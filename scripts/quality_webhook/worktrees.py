import re
import subprocess
import threading
import time
from pathlib import Path

from .config import (
    WIN_PROJECT_DIR, PROJECT_DIR, WORKTREE_BASE, WORKTREE_MAX_AGE_H,
    WORKTREE_MAX_COUNT, log, to_wsl_path,
)
from .runner import wsl_bash

_GIT_WORKTREE_PRUNE = "git worktree prune"

_worktree_lock = threading.Lock()
_active_worktrees: dict[str, float] = {}
_worktree_usage_locks: dict[str, threading.Lock] = {}
# Per-name locks that serialise setup/cleanup for the same worktree name
# so two threads targeting the same (branch, commit) don't race through
# git operations on the same path.
_name_locks: dict[str, threading.Lock] = {}

_SAFE_BRANCH_RE = re.compile(r"[^a-zA-Z0-9_\-]")
_SHELL_SAFE_REF_RE = re.compile(r"^[a-zA-Z0-9_./@\-]+$")


def _validate_branch_for_shell(branch: str) -> str:
    """Validate that *branch* is safe to interpolate into shell commands.

    Raises ``ValueError`` for empty strings or strings containing shell
    metacharacters (quotes, backticks, semicolons, spaces, etc.).
    Returns the branch unchanged when valid.
    """
    if not branch:
        raise ValueError("branch name must not be empty")
    if not _SHELL_SAFE_REF_RE.match(branch):
        raise ValueError(
            f"branch name contains unsafe characters: {branch!r}"
        )
    return branch


def _get_name_lock(name: str) -> threading.Lock:
    """Return (and lazily create) a per-worktree-name lock.

    Must be called while ``_worktree_lock`` is held.
    """
    lock = _name_locks.get(name)
    if lock is None:
        lock = threading.Lock()
        _name_locks[name] = lock
    return lock


def _retry_wsl_bash(cmd: str, *, timeout: int, max_attempts: int,
                     label: str) -> subprocess.CompletedProcess | None:
    """Run a wsl_bash command with retries on timeout. Return None if all attempts fail."""
    for attempt in range(1, max_attempts + 1):
        try:
            return wsl_bash(cmd, timeout=timeout)
        except subprocess.TimeoutExpired:
            if attempt < max_attempts:
                log.warning(f"Timeout {label} (attempt {attempt}/{max_attempts}, retrying in 5s)")
                time.sleep(5)
            else:
                log.error(f"Timeout {label} after {max_attempts} attempts")
    return None


def _cleanup_partial_worktree(wsl_path: str, name: str) -> None:
    """Best-effort removal of a worktree left behind by a timed-out add."""
    log.info(f"Cleaning up partial worktree after timeout: {name}")
    try:
        wsl_bash(f"git worktree remove --force '{wsl_path}' 2>/dev/null; "
                 f"rm -rf '{wsl_path}' 2>/dev/null; "
                 f"{_GIT_WORKTREE_PRUNE}", timeout=30)
    except subprocess.TimeoutExpired:
        log.warning(f"Cleanup of partial worktree {name} also timed out")


def _sanitize_branch_name(branch: str) -> str:
    return _SAFE_BRANCH_RE.sub("-", branch)[:64]


def _worktree_name(branch: str, commit: str) -> str:
    safe_branch = _sanitize_branch_name(branch)
    short_sha = commit[:8] if commit else "latest"
    return f"fix-{safe_branch}-{short_sha}"


def _win_worktree_path(name: str) -> Path:
    return WIN_PROJECT_DIR / WORKTREE_BASE / name


def _wsl_worktree_path(name: str) -> str:
    return to_wsl_path(_win_worktree_path(name))


def _resolve_fetch_refs(branch: str) -> tuple[str, str, bool]:
    """Return (fetch_ref, checkout_ref, is_pr_ref) for the given branch."""
    is_pr_ref = "/" in branch and branch.split("/")[0].isdigit()
    if not is_pr_ref:
        return branch, f"origin/{branch}", False
    pr_number = branch.split("/")[0]
    return f"pull/{pr_number}/head:pr-{pr_number}", f"pr-{pr_number}", True


def _try_reuse_worktree(name: str) -> bool:
    """Check whether the worktree already exists and is free.

    Must be called while ``_worktree_lock`` is held.
    Returns True if the worktree can be reused (caller must still refresh
    it *after* releasing the global lock), False otherwise.
    """
    if name not in _active_worktrees:
        return False
    usage_lock = _worktree_usage_locks.get(name)
    if usage_lock and usage_lock.locked():
        log.warning(f"Worktree {name} is in use by another fix, creating fresh")
        return False
    return True


def _remove_stale_lock(name: str) -> None:
    """Remove a leftover index.lock inside .git/worktrees/<name>/.

    A timed-out or crashed ``git worktree add`` can leave this lock behind,
    causing subsequent adds to fail with "File exists".
    """
    lock_path = WIN_PROJECT_DIR / ".git" / "worktrees" / name / "index.lock"
    try:
        lock_path.unlink(missing_ok=True)
    except OSError as exc:
        log.warning(f"Could not remove {lock_path}: {exc}")


def _ensure_worktree_base() -> None:
    """Create the worktree base directory if it doesn't exist."""
    base = WIN_PROJECT_DIR / WORKTREE_BASE
    base.mkdir(parents=True, exist_ok=True)


def _recover_stale_path(wsl_path: str, name: str, checkout_ref: str) -> subprocess.CompletedProcess:
    """Remove a stale worktree path and retry the add."""
    log.info(f"Stale worktree path exists, removing and retrying: {name}")
    _remove_stale_lock(name)
    # Prune git's internal bookkeeping first so stale entries are cleared
    wsl_bash(_GIT_WORKTREE_PRUNE, timeout=15)
    wsl_bash(f"git worktree remove --force '{wsl_path}' 2>/dev/null || true", timeout=30)
    wsl_bash(f"rm -rf '{wsl_path}'", timeout=15)
    return wsl_bash(f"git worktree add '{wsl_path}' '{checkout_ref}'", timeout=120)


def _add_detached_worktree(
    wsl_path: str, branch: str, commit: str,
    checkout_ref: str, is_pr_ref: bool,
) -> subprocess.CompletedProcess | None:
    """Create a detached-HEAD worktree and optionally reassign the branch.

    Returns the final result, or None on failure.
    """
    log.info(f"Branch '{branch}' already checked out, using detached HEAD")
    target = commit if commit and commit != "HEAD" else checkout_ref
    result = wsl_bash(f"git worktree add --detach '{wsl_path}' '{target}'", timeout=60)
    if result.returncode != 0:
        log.error(f"Failed to create detached worktree: {result.stderr}")
        return None

    if not is_pr_ref:
        checkout_result = wsl_bash(
            f"git -C '{wsl_path}' checkout -B '{branch}' 'origin/{branch}'",
            timeout=30,
        )
        if checkout_result.returncode != 0:
            log.warning(
                f"Could not reassign branch in worktree, "
                f"will push from detached HEAD: {checkout_result.stderr}"
            )
    return result


def _handle_worktree_add_failure(
    result: subprocess.CompletedProcess,
    wsl_path: str, name: str, branch: str, commit: str,
    checkout_ref: str, is_pr_ref: bool,
) -> tuple[str, str] | None:
    """Attempt recovery when ``git worktree add`` fails.

    Returns (wsl_path, name) on success, ("", "") on unrecoverable failure,
    or None when the initial add actually succeeded (returncode == 0).
    """
    if result.returncode == 0:
        return None

    stderr = result.stderr or ""

    if "already exists" in stderr or "File exists" in stderr:
        result = _recover_stale_path(wsl_path, name, checkout_ref)
        stderr = result.stderr or ""

    if result.returncode == 0:
        return None

    if "already checked out" in stderr or "is already used by" in stderr:
        detach_result = _add_detached_worktree(
            wsl_path, branch, commit, checkout_ref, is_pr_ref,
        )
        if detach_result is None:
            return "", ""
        return None

    log.error(f"Failed to create worktree: {stderr}")
    return "", ""


def setup_worktree(branch: str, commit: str) -> tuple[str, str]:
    """Create an ephemeral worktree for the given branch/commit.

    Returns (wsl_path, name) on success, ("", "") on failure.
    Handles the case where the branch is already checked out in the
    main worktree by using detached HEAD + branch reassignment.

    Thread-safety: a per-name lock serialises all git operations for a
    given worktree name so two concurrent requests for the same
    (branch, commit) don't race.  The global ``_worktree_lock`` is only
    held briefly for bookkeeping (never during I/O).
    """
    try:
        _validate_branch_for_shell(branch)
    except ValueError as exc:
        log.error(f"Rejecting worktree setup: {exc}")
        return "", ""

    name = _worktree_name(branch, commit)
    wsl_path = _wsl_worktree_path(name)

    # --- bookkeeping under global lock (fast, no I/O) ---
    with _worktree_lock:
        name_lock = _get_name_lock(name)
        can_reuse = _try_reuse_worktree(name)
        if can_reuse:
            # Mark reuse intent so _enforce_max_count won't evict it.
            _active_worktrees[name] = time.monotonic()
            _worktree_usage_locks.setdefault(name, threading.Lock())

    # --- per-name lock: serialises git I/O for this worktree name ---
    with name_lock:
        if can_reuse:
            log.info(f"Reusing existing worktree: {name}")
            _pull_in_worktree(wsl_path, branch)
            return wsl_path, name

        # Evict oldest worktrees if at the limit.  We hold only the
        # global lock while mutating the dict, then release it for the
        # slow disk removal.
        with _worktree_lock:
            _enforce_max_count()

        log.info(f"Creating worktree: {name} for branch={branch} "
                 f"commit={commit[:12] if len(commit) > 12 else commit}")

        # Ensure the worktree base directory exists on disk so git can
        # create the checkout inside it (git worktree add doesn't create
        # parents).
        _ensure_worktree_base()

        # Remove any stale index.lock left by a previous timed-out add.
        _remove_stale_lock(name)

        # Prune stale worktree bookkeeping before attempting to add, so
        # git doesn't reject the add due to a leftover entry.
        try:
            wsl_bash(_GIT_WORKTREE_PRUNE, timeout=15)
        except subprocess.TimeoutExpired:
            log.warning("git worktree prune timed out, continuing anyway")

        fetch_ref, checkout_ref, is_pr_ref = _resolve_fetch_refs(branch)

        result = _retry_wsl_bash(
            f"git fetch origin {fetch_ref} --no-tags",
            timeout=90, max_attempts=3,
            label=f"fetching {fetch_ref} for worktree {name}",
        )
        if result is None:
            return "", ""

        result = _retry_wsl_bash(
            f"git worktree add '{wsl_path}' '{checkout_ref}'",
            timeout=120, max_attempts=2,
            label=f"creating worktree {name}",
        )
        if result is None:
            _cleanup_partial_worktree(wsl_path, name)
            return "", ""

        try:
            failure = _handle_worktree_add_failure(
                result, wsl_path, name, branch, commit,
                checkout_ref, is_pr_ref,
            )
            if failure is not None:
                return failure
        except subprocess.TimeoutExpired:
            log.error(f"Timeout during worktree recovery for {name}")
            _cleanup_partial_worktree(wsl_path, name)
            return "", ""

        with _worktree_lock:
            _active_worktrees[name] = time.monotonic()
            _worktree_usage_locks.setdefault(name, threading.Lock())

        log.info(f"Worktree ready: {wsl_path}")
        return wsl_path, name


def _pull_in_worktree(wsl_path: str, branch: str) -> None:
    _validate_branch_for_shell(branch)
    wsl_bash(
        f"git -C '{wsl_path}' fetch origin {branch} --no-tags && "
        f"git -C '{wsl_path}' reset --hard 'origin/{branch}'",
        timeout=60,
    )


def commit_and_push(wsl_path: str, branch: str, message: str) -> tuple[bool, str]:
    """Stage, commit, and push from a worktree. Returns (success, stderr)."""
    try:
        _validate_branch_for_shell(branch)
    except ValueError as exc:
        return False, f"unsafe branch name: {exc}"

    add_result = wsl_bash(f"git -C '{wsl_path}' add -A", timeout=30)
    if add_result.returncode != 0:
        return False, f"git add failed: {add_result.stderr}"

    nothing_check = wsl_bash(
        f"git -C '{wsl_path}' diff --cached --quiet",
        timeout=15,
    )
    if nothing_check.returncode == 0:
        log.info("Nothing to commit in worktree")
        return True, "nothing to commit"

    safe_message = message.replace("'", "'\\''")
    commit_result = wsl_bash(
        f"git -C '{wsl_path}' commit -m '{safe_message}'",
        timeout=30,
    )
    if commit_result.returncode != 0:
        return False, f"git commit failed: {commit_result.stderr}"

    push_result = wsl_bash(
        f"git -C '{wsl_path}' push origin '{branch}'",
        timeout=120,
    )
    if push_result.returncode == 0:
        return True, ""

    log.info(f"Push failed, attempting fetch + rebase: {push_result.stderr}")
    fetch_result = wsl_bash(
        f"git -C '{wsl_path}' fetch origin '{branch}' --no-tags",
        timeout=60,
    )
    if fetch_result.returncode != 0:
        return False, f"fetch before rebase failed: {fetch_result.stderr}"

    rebase_result = wsl_bash(
        f"git -C '{wsl_path}' rebase 'origin/{branch}'",
        timeout=120,
    )
    if rebase_result.returncode != 0:
        log.warning(f"Rebase failed (likely conflicts), aborting: {rebase_result.stderr}")
        wsl_bash(f"git -C '{wsl_path}' rebase --abort", timeout=15)
        return False, f"rebase conflicts: {rebase_result.stderr}"

    retry_push = wsl_bash(
        f"git -C '{wsl_path}' push origin '{branch}'",
        timeout=120,
    )
    if retry_push.returncode == 0:
        return True, ""

    return False, f"push failed after rebase: {retry_push.stderr}"


def _remove_worktree_on_disk(name: str) -> None:
    """Run ``git worktree remove`` without touching in-memory tracking."""
    wsl_path = _wsl_worktree_path(name)
    log.info(f"Cleaning up worktree: {name}")
    wsl_bash(f"git worktree remove --force '{wsl_path}'", timeout=30)


def cleanup_worktree(name: str) -> None:
    """Remove a worktree and clean up tracking.

    Acquires the per-name lock to avoid racing with a concurrent
    ``setup_worktree`` for the same name, then updates bookkeeping
    under the global lock.
    """
    with _worktree_lock:
        name_lock = _get_name_lock(name)

    with name_lock:
        _remove_worktree_on_disk(name)

    with _worktree_lock:
        _active_worktrees.pop(name, None)
        _worktree_usage_locks.pop(name, None)


def discard_changes(wsl_path: str) -> None:
    """Reset staged and unstaged changes in a worktree without removing it."""
    wsl_bash(
        f"git -C '{wsl_path}' reset HEAD -- . 2>/dev/null; "
        f"git -C '{wsl_path}' checkout -- . 2>/dev/null; "
        f"git -C '{wsl_path}' clean -fd 2>/dev/null",
        timeout=15,
    )


def get_head_sha(wsl_path: str) -> str:
    result = wsl_bash(f"git -C '{wsl_path}' rev-parse HEAD", timeout=10)
    return result.stdout.strip() if result.returncode == 0 else ""


def prune_stale_worktrees() -> None:
    """Remove worktrees older than WORKTREE_MAX_AGE_H."""
    max_age_s = WORKTREE_MAX_AGE_H * 3600
    now = time.monotonic()

    with _worktree_lock:
        to_remove = [
            name for name, created_at in _active_worktrees.items()
            if now - created_at > max_age_s
        ]
        # Remove from tracking while still holding the lock so no other
        # thread can hand out a worktree we're about to delete.
        for name in to_remove:
            _active_worktrees.pop(name, None)
            _worktree_usage_locks.pop(name, None)

    for name in to_remove:
        with _worktree_lock:
            name_lock = _get_name_lock(name)
        with name_lock:
            try:
                _remove_worktree_on_disk(name)
            except Exception:
                log.warning(f"Failed to remove stale worktree {name}", exc_info=True)
        log.info(f"Pruned stale worktree: {name}")

    wsl_bash(_GIT_WORKTREE_PRUNE, timeout=15)


def _enforce_max_count() -> None:
    """Remove oldest worktrees if we're at the limit.

    MUST be called while ``_worktree_lock`` is held.  Collects eviction
    targets and removes them from tracking while holding the lock, then
    releases the lock for the slow disk removal and re-acquires it.
    This prevents other threads from seeing stale counts.
    """
    to_evict: list[str] = []
    while len(_active_worktrees) >= WORKTREE_MAX_COUNT:
        oldest_name = min(_active_worktrees, key=_active_worktrees.get)
        log.info(f"Evicting oldest worktree to make room: {oldest_name}")
        _active_worktrees.pop(oldest_name, None)
        _worktree_usage_locks.pop(oldest_name, None)
        to_evict.append(oldest_name)

    if to_evict:
        _worktree_lock.release()
        try:
            for name in to_evict:
                _remove_worktree_on_disk(name)
        finally:
            _worktree_lock.acquire()
