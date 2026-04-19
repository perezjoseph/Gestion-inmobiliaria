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

_worktree_lock = threading.Lock()
_active_worktrees: dict[str, float] = {}
_worktree_usage_locks: dict[str, threading.Lock] = {}

_SAFE_BRANCH_RE = re.compile(r"[^a-zA-Z0-9_\-]")


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


def setup_worktree(branch: str, commit: str) -> tuple[str, str]:
    """Create an ephemeral worktree for the given branch/commit.

    Returns (wsl_path, name) on success, ("", "") on failure.
    Handles the case where the branch is already checked out in the
    main worktree by using detached HEAD + branch reassignment.
    """
    name = _worktree_name(branch, commit)
    wsl_path = _wsl_worktree_path(name)
    win_path = _win_worktree_path(name)

    with _worktree_lock:
        if name in _active_worktrees:
            usage_lock = _worktree_usage_locks.get(name)
            if usage_lock and usage_lock.locked():
                log.warning(f"Worktree {name} is in use by another fix, creating fresh")
            else:
                log.info(f"Reusing existing worktree: {name}")
                _pull_in_worktree(wsl_path, branch)
                if name not in _worktree_usage_locks:
                    _worktree_usage_locks[name] = threading.Lock()
                return wsl_path, name

        _enforce_max_count()

    log.info(f"Creating worktree: {name} for branch={branch} commit={commit[:12]}")

    fetch_ref = branch
    checkout_ref = f"origin/{branch}"
    is_pr_ref = "/" in branch and branch.split("/")[0].isdigit()
    if is_pr_ref:
        pr_number = branch.split("/")[0]
        fetch_ref = f"pull/{pr_number}/head:pr-{pr_number}"
        checkout_ref = f"pr-{pr_number}"

    try:
        wsl_bash(f"git fetch origin {fetch_ref} --no-tags", timeout=60)
    except subprocess.TimeoutExpired:
        log.error(f"Timeout fetching {fetch_ref} for worktree {name}")
        return "", ""

    try:
        result = wsl_bash(
            f"git worktree add '{wsl_path}' '{checkout_ref}'",
            timeout=60,
        )
    except subprocess.TimeoutExpired:
        log.error(f"Timeout creating worktree {name}")
        return "", ""

    if result.returncode != 0:
        stderr = result.stderr or ""
        try:
            if "already exists" in stderr:
                log.info(f"Stale worktree path exists, removing and retrying: {name}")
                wsl_bash(f"git worktree remove --force '{wsl_path}'", timeout=30)
                wsl_bash(f"rm -rf '{wsl_path}'", timeout=15)
                result = wsl_bash(
                    f"git worktree add '{wsl_path}' '{checkout_ref}'",
                    timeout=60,
                )
                if result.returncode != 0:
                    stderr = result.stderr or ""

            if result.returncode != 0 and ("already checked out" in stderr or "is already used by" in stderr):
                log.info(f"Branch '{branch}' already checked out, using detached HEAD")
                target = commit if commit and commit != "HEAD" else checkout_ref
                result = wsl_bash(
                    f"git worktree add --detach '{wsl_path}' '{target}'",
                    timeout=60,
                )
                if result.returncode != 0:
                    log.error(f"Failed to create detached worktree: {result.stderr}")
                    return "", ""

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
            elif result.returncode != 0:
                log.error(f"Failed to create worktree: {stderr}")
                return "", ""
        except subprocess.TimeoutExpired:
            log.error(f"Timeout during worktree recovery for {name}")
            return "", ""

    with _worktree_lock:
        _active_worktrees[name] = time.monotonic()
        if name not in _worktree_usage_locks:
            _worktree_usage_locks[name] = threading.Lock()

    log.info(f"Worktree ready: {wsl_path}")
    return wsl_path, name


def _pull_in_worktree(wsl_path: str, branch: str) -> None:
    wsl_bash(
        f"git -C '{wsl_path}' fetch origin {branch} --no-tags && "
        f"git -C '{wsl_path}' reset --hard 'origin/{branch}'",
        timeout=60,
    )


def commit_and_push(wsl_path: str, branch: str, message: str) -> tuple[bool, str]:
    """Stage, commit, and push from a worktree. Returns (success, stderr)."""
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


def cleanup_worktree(name: str) -> None:
    """Remove a worktree and clean up tracking."""
    wsl_path = _wsl_worktree_path(name)
    log.info(f"Cleaning up worktree: {name}")

    wsl_bash(f"git worktree remove --force '{wsl_path}'", timeout=30)

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
    to_remove = []

    with _worktree_lock:
        for name, created_at in list(_active_worktrees.items()):
            if now - created_at > max_age_s:
                to_remove.append(name)

    for name in to_remove:
        cleanup_worktree(name)
        log.info(f"Pruned stale worktree: {name}")

    wsl_bash("git worktree prune", timeout=15)


def _enforce_max_count() -> None:
    """Remove oldest worktrees if we're at the limit."""
    while len(_active_worktrees) >= WORKTREE_MAX_COUNT:
        oldest_name = min(_active_worktrees, key=_active_worktrees.get)
        log.info(f"Evicting oldest worktree to make room: {oldest_name}")
        cleanup_worktree(oldest_name)
