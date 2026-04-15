import json
import re
import threading
import time
from collections import Counter
from dataclasses import dataclass, field
from datetime import datetime

from .config import WIN_PROJECT_DIR, log
from .history import _load_fix_history, _save_fix_history, _fix_history_lock


@dataclass
class FailureEvent:
    job: str
    step: str
    error_log: str
    commit: str
    timestamp: datetime
    error_class: str


@dataclass
class CorrelationGroup:
    commit: str
    failures: list[FailureEvent] = field(default_factory=list)
    shared_root_cause: str | None = None
    shared_files: list[str] = field(default_factory=list)
    shared_patterns: list[str] = field(default_factory=list)
    dispatched: bool = False


CORRELATION_WINDOW_S = 60
MIN_FAILURES_FOR_CORRELATION = 2
FILE_PATH_RE = re.compile(
    r"(?:^|\s)((?:backend|frontend|android)/\S+\.(?:rs|kt|toml|yml))",
    re.MULTILINE,
)

_PRUNE_AGE_S = 600
_MAX_GROUPS = 50

_lock = threading.Lock()
_groups: dict[str, CorrelationGroup] = {}
_group_created_at: dict[str, float] = {}
_group_counter = 0


def _prune_stale_groups():
    now = time.monotonic()
    stale = [
        gid for gid, created in _group_created_at.items()
        if now - created > _PRUNE_AGE_S
    ]
    for gid in stale:
        _groups.pop(gid, None)
        _group_created_at.pop(gid, None)


def _extract_files(error_log: str) -> set[str]:
    return set(FILE_PATH_RE.findall(error_log))


def register_failure(
    job: str,
    step: str,
    error_log: str,
    commit: str,
    error_class: str,
) -> str | None:
    global _group_counter

    now_mono = time.monotonic()
    now_dt = datetime.now()
    event = FailureEvent(
        job=job,
        step=step,
        error_log=error_log,
        commit=commit,
        timestamp=now_dt,
        error_class=error_class,
    )

    with _lock:
        _prune_stale_groups()

        for gid, group in _groups.items():
            if group.commit != commit:
                continue
            created = _group_created_at.get(gid, 0)
            if now_mono - created <= CORRELATION_WINDOW_S:
                group.failures.append(event)
                return gid

        if len(_groups) >= _MAX_GROUPS:
            return None

        _group_counter += 1
        gid = f"cg-{_group_counter}"
        _groups[gid] = CorrelationGroup(commit=commit, failures=[event])
        _group_created_at[gid] = now_mono
        return gid


def check_correlation(group_id: str) -> CorrelationGroup | None:
    with _lock:
        group = _groups.get(group_id)
        if group is None:
            return None

        if len(group.failures) < MIN_FAILURES_FOR_CORRELATION:
            return group

        file_sets = [_extract_files(f.error_log) for f in group.failures]
        if file_sets:
            shared = file_sets[0]
            for fs in file_sets[1:]:
                shared = shared & fs
            group.shared_files = sorted(shared)

        class_counts = Counter(f.error_class for f in group.failures)
        group.shared_patterns = [
            cls for cls, cnt in class_counts.items()
            if cnt >= MIN_FAILURES_FOR_CORRELATION
        ]

        if group.shared_files or group.shared_patterns:
            parts = []
            if group.shared_files:
                parts.append(f"shared files: {', '.join(group.shared_files)}")
            if group.shared_patterns:
                parts.append(f"shared error patterns: {', '.join(group.shared_patterns)}")
            group.shared_root_cause = "; ".join(parts)

        return group


def should_dispatch_correlated_fix(group_id: str) -> bool:
    with _lock:
        group = _groups.get(group_id)
        if group is None:
            return False
        if len(group.failures) < MIN_FAILURES_FOR_CORRELATION:
            return False
        if group.shared_root_cause is None:
            return False
        return True


def build_correlated_fix_prompt(group: CorrelationGroup) -> str:
    jobs = [f.job for f in group.failures]
    lines = [
        f"CORRELATED CI FAILURES (commit {group.commit[:8]})",
        f"Affected jobs: {', '.join(jobs)}",
    ]
    if group.shared_root_cause:
        lines.append(f"Shared root cause: {group.shared_root_cause}")
    if group.shared_files:
        lines.append(f"Shared files: {', '.join(group.shared_files)}")

    lines.append("")
    for i, failure in enumerate(group.failures, 1):
        lines.append(f"--- Failure {i}: {failure.job}/{failure.step} (class: {failure.error_class}) ---")
        lines.append(f"Error preview:\n```\n{failure.error_log[:2000]}\n```")
        lines.append("")

    lines.append("Fix the shared root cause that affects ALL listed jobs.")
    lines.append("Verify against all affected jobs before staging.")
    return "\n".join(lines)


def track_cache_health(job: str, cache_hit: bool) -> None:
    with _fix_history_lock:
        history = _load_fix_history()
        cache_health = history.get("cache_health", {})
        entry = cache_health.get(job, {"runs": []})
        entry["runs"] = entry.get("runs", [])
        entry["runs"].append(cache_hit)
        entry["runs"] = entry["runs"][-20:]

        total = len(entry["runs"])
        hits = sum(1 for r in entry["runs"] if r)
        entry["hit_rate"] = round(hits / total, 2) if total > 0 else 1.0

        consecutive_misses = 0
        for r in reversed(entry["runs"]):
            if not r:
                consecutive_misses += 1
            else:
                break
        entry["consecutive_misses"] = consecutive_misses

        miss_rate = 1.0 - entry["hit_rate"]
        entry["degraded"] = miss_rate > 0.50 and total >= 3

        cache_health[job] = entry
        history["cache_health"] = cache_health
        _save_fix_history(history)


def get_correlation_summary() -> dict:
    with _lock:
        active_groups = len(_groups)

    with _fix_history_lock:
        history = _load_fix_history()
        cache_health_raw = history.get("cache_health", {})

    cache_health = {}
    for job, entry in cache_health_raw.items():
        if not isinstance(entry, dict):
            continue
        cache_health[job] = {
            "hit_rate": entry.get("hit_rate", 1.0),
            "consecutive_misses": entry.get("consecutive_misses", 0),
        }

    return {
        "active_groups": active_groups,
        "cache_health": cache_health,
    }
