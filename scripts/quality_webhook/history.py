import hashlib
import json
import re
import threading
from datetime import datetime

from .config import WIN_PROJECT_DIR, log

_FIX_HISTORY_FILE = WIN_PROJECT_DIR / ".fix-history.json"
_FIX_HISTORY_MAX_ENTRIES = 200
_fix_history_lock = threading.Lock()


def _error_hash(job, error_log):
    normalized = re.sub(r"\d+", "N", error_log[:2000]).strip().lower()
    return hashlib.sha256(f"{job}:{normalized}".encode()).hexdigest()[:16]


def _load_fix_history():
    try:
        if _FIX_HISTORY_FILE.is_file():
            data = json.loads(_FIX_HISTORY_FILE.read_text(encoding="utf-8"))
            if isinstance(data, dict):
                return data
    except (json.JSONDecodeError, OSError) as e:
        log.warning(f"Fix history load failed: {e}")
    return {}


def _save_fix_history(history):
    if len(history) > _FIX_HISTORY_MAX_ENTRIES:
        sorted_keys = sorted(
            history.keys(),
            key=lambda k: history[k].get("last_seen", ""),
        )
        for k in sorted_keys[: len(history) - _FIX_HISTORY_MAX_ENTRIES]:
            del history[k]
    try:
        _FIX_HISTORY_FILE.write_text(
            json.dumps(history, indent=2, ensure_ascii=False),
            encoding="utf-8",
            newline="\n",
        )
    except OSError as e:
        log.warning(f"Fix history save failed: {e}")


def record_fix_attempt(job, error_log, attempt, success, strategy_notes="", timing=None, **kwargs):
    h = _error_hash(job, error_log)
    now = datetime.now().isoformat()
    with _fix_history_lock:
        history = _load_fix_history()
        entry = history.get(h, {
            "job": job,
            "first_seen": now,
            "attempts": [],
            "total_attempts": 0,
            "successes": 0,
        })
        entry["last_seen"] = now
        entry["total_attempts"] = entry.get("total_attempts", 0) + 1
        if success:
            entry["successes"] = entry.get("successes", 0) + 1
        attempt_record = {
            "ts": now,
            "attempt": attempt,
            "success": success,
            "notes": strategy_notes[:500],
        }
        if timing is not None:
            attempt_record["timing"] = timing
        entry["attempts"] = entry.get("attempts", [])[-9:] + [attempt_record]
        history[h] = entry
        _save_fix_history(history)


def get_past_attempts(job, error_log):
    h = _error_hash(job, error_log)
    with _fix_history_lock:
        history = _load_fix_history()
        entry = history.get(h)
    if not entry:
        return ""
    total = entry.get("total_attempts", 0)
    successes = entry.get("successes", 0)
    if total == 0:
        return ""
    recent = entry.get("attempts", [])[-3:]
    lines = [f"FIX HISTORY: {total} previous attempts for this error pattern, {successes} succeeded."]
    for a in recent:
        status = "succeeded" if a.get("success") else "failed"
        lines.append(f"  - {a.get('ts', '?')}: attempt {a.get('attempt', '?')} {status}")
        if a.get("notes"):
            lines.append(f"    Notes: {a['notes']}")
    if total > 0 and successes == 0:
        lines.append("  WARNING: No previous attempt has succeeded. Try a fundamentally different approach.")
    return "\n".join(lines)
