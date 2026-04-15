import re
import threading
from dataclasses import dataclass, field
from datetime import datetime, timedelta

from .config import log
from .history import _fix_history_lock, _load_fix_history, _save_fix_history

RECURRING_THRESHOLD = 3
RECURRING_WINDOW_DAYS = 30
MAX_RECORDS_PER_CRATE = 50

RUSTSEC_RE = re.compile(r"(RUSTSEC-\d{4}-\d{4,})")
GHSA_RE = re.compile(r"(GHSA-[a-z0-9]{4}-[a-z0-9]{4}-[a-z0-9]{4})")
CVE_RE = re.compile(r"(CVE-\d{4}-\d{4,})")
CRATE_RE = re.compile(r"(?:crate|package)\s+`?(\S+?)`?\s+", re.IGNORECASE)
CARGO_DENY_CRATE_RE = re.compile(
    r"(\S+)\s+(?:=|@)\s*[\d.]+.*(?:RUSTSEC|GHSA|CVE|advisory)", re.IGNORECASE,
)


@dataclass
class VulnRecord:
    crate: str
    advisory_ids: list[str] = field(default_factory=list)
    failure_timestamps: list[str] = field(default_factory=list)
    failure_count_30d: int = 0
    is_recurring: bool = False


def extract_vuln_info(error_log: str) -> list[tuple[str, list[str]]]:
    advisory_ids: list[str] = []
    for pattern in (RUSTSEC_RE, GHSA_RE, CVE_RE):
        advisory_ids.extend(pattern.findall(error_log))
    advisory_ids = sorted(set(advisory_ids))

    crate_names: list[str] = []
    for match in CARGO_DENY_CRATE_RE.finditer(error_log):
        name = match.group(1).strip("`'\"")
        if name and name not in crate_names:
            crate_names.append(name)

    for match in CRATE_RE.finditer(error_log):
        name = match.group(1).strip("`'\"")
        if name and name not in crate_names:
            crate_names.append(name)

    if not crate_names:
        if advisory_ids:
            log.warning("Found advisory IDs but no crate names in error log")
        return []

    if not advisory_ids:
        return [(crate, []) for crate in crate_names]

    return [(crate, list(advisory_ids)) for crate in crate_names]


def record_vuln_failure(crate: str, advisory_ids: list[str]) -> None:
    now = datetime.now().isoformat()
    cutoff = datetime.now() - timedelta(days=RECURRING_WINDOW_DAYS)

    with _fix_history_lock:
        history = _load_fix_history()
        vuln_tracking = history.get("vuln_tracking", {})
        entry = vuln_tracking.get(crate, {"advisory_ids": [], "failures": []})

        existing_advisories = set(entry.get("advisory_ids", []))
        existing_advisories.update(advisory_ids)
        entry["advisory_ids"] = sorted(existing_advisories)

        failures = entry.get("failures", [])
        failures.append({"ts": now})
        if len(failures) > MAX_RECORDS_PER_CRATE:
            failures = failures[-MAX_RECORDS_PER_CRATE:]
        entry["failures"] = failures

        count_30d = 0
        for f in failures:
            try:
                ts = datetime.fromisoformat(f["ts"])
                if ts >= cutoff:
                    count_30d += 1
            except (ValueError, KeyError):
                pass

        entry["failure_count_30d"] = count_30d
        entry["is_recurring"] = count_30d >= RECURRING_THRESHOLD

        vuln_tracking[crate] = entry
        history["vuln_tracking"] = vuln_tracking
        _save_fix_history(history)


def get_recurring_vulns() -> list[VulnRecord]:
    with _fix_history_lock:
        history = _load_fix_history()

    vuln_tracking = history.get("vuln_tracking", {})
    recurring = []
    cutoff = datetime.now() - timedelta(days=RECURRING_WINDOW_DAYS)

    for crate, entry in vuln_tracking.items():
        failures = entry.get("failures", [])
        count_30d = 0
        timestamps = []
        for f in failures:
            try:
                ts = datetime.fromisoformat(f["ts"])
                timestamps.append(f["ts"])
                if ts >= cutoff:
                    count_30d += 1
            except (ValueError, KeyError):
                pass

        is_recurring = count_30d >= RECURRING_THRESHOLD
        if is_recurring:
            recurring.append(VulnRecord(
                crate=crate,
                advisory_ids=entry.get("advisory_ids", []),
                failure_timestamps=timestamps,
                failure_count_30d=count_30d,
                is_recurring=True,
            ))

    return recurring


def get_vuln_summary() -> dict:
    with _fix_history_lock:
        history = _load_fix_history()

    vuln_tracking = history.get("vuln_tracking", {})
    cutoff = datetime.now() - timedelta(days=RECURRING_WINDOW_DAYS)
    recurring_crates = []

    for crate, entry in vuln_tracking.items():
        failures = entry.get("failures", [])
        count_30d = 0
        for f in failures:
            try:
                ts = datetime.fromisoformat(f["ts"])
                if ts >= cutoff:
                    count_30d += 1
            except (ValueError, KeyError):
                pass

        if count_30d >= RECURRING_THRESHOLD:
            recurring_crates.append({
                "crate": crate,
                "failures_30d": count_30d,
                "advisories": entry.get("advisory_ids", []),
            })

    return {
        "recurring_crates": recurring_crates,
    }
