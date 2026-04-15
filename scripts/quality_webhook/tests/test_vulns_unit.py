"""Unit tests for vulnerability extraction edge cases.

Requirements: 13.1, 13.3, 13.6
"""

import json
import pathlib
import tempfile
from datetime import datetime, timedelta
from unittest.mock import patch

import pytest

from scripts.quality_webhook.vulns import (
    RECURRING_THRESHOLD,
    RECURRING_WINDOW_DAYS,
    MAX_RECORDS_PER_CRATE,
    extract_vuln_info,
    record_vuln_failure,
    get_recurring_vulns,
    get_vuln_summary,
)
from scripts.quality_webhook.history import _load_fix_history


@pytest.fixture()
def tmp_history():
    tmp = tempfile.NamedTemporaryFile(
        mode="w", suffix=".json", delete=False, encoding="utf-8",
    )
    tmp.write("{}")
    tmp.close()
    path = pathlib.Path(tmp.name)
    with patch("scripts.quality_webhook.history._FIX_HISTORY_FILE", path):
        yield path
    path.unlink(missing_ok=True)


def test_extract_rustsec_pattern():
    error_log = (
        "error: found vulnerability in crate `openssl-sys` (RUSTSEC-2024-0001)\n"
        "advisory: RUSTSEC-2024-0001\n"
    )
    result = extract_vuln_info(error_log)
    assert len(result) >= 1
    crates = [c for c, _ in result]
    assert "openssl-sys" in crates
    advisories = result[0][1]
    assert "RUSTSEC-2024-0001" in advisories


def test_extract_ghsa_pattern():
    error_log = (
        "crate `tokio` has advisory GHSA-abcd-efgh-ijkl\n"
    )
    result = extract_vuln_info(error_log)
    assert len(result) >= 1
    crates = [c for c, _ in result]
    assert "tokio" in crates
    all_advisories = []
    for _, advs in result:
        all_advisories.extend(advs)
    assert "GHSA-abcd-efgh-ijkl" in all_advisories


def test_extract_cve_pattern():
    error_log = (
        "package `hyper` affected by CVE-2024-12345\n"
    )
    result = extract_vuln_info(error_log)
    assert len(result) >= 1
    crates = [c for c, _ in result]
    assert "hyper" in crates
    all_advisories = []
    for _, advs in result:
        all_advisories.extend(advs)
    assert "CVE-2024-12345" in all_advisories


def test_extract_cargo_deny_format():
    error_log = (
        "openssl-sys = 0.9.80 has advisory RUSTSEC-2024-0002\n"
    )
    result = extract_vuln_info(error_log)
    assert len(result) >= 1
    crates = [c for c, _ in result]
    assert "openssl-sys" in crates


def test_extract_no_crate_name():
    error_log = "RUSTSEC-2024-0001 found in unknown location"
    result = extract_vuln_info(error_log)
    assert result == []


def test_extract_no_advisories():
    error_log = "crate `serde` has some issue"
    result = extract_vuln_info(error_log)
    if result:
        for crate, advisories in result:
            assert advisories == []


def test_extract_empty_log():
    result = extract_vuln_info("")
    assert result == []


def test_recurring_threshold_exactly_3_in_30_days(tmp_history):
    crate = "recurring-crate"
    now = datetime.now()

    for i in range(RECURRING_THRESHOLD):
        with patch("scripts.quality_webhook.vulns.datetime") as mock_dt:
            mock_dt.now.return_value = now - timedelta(days=i)
            mock_dt.fromisoformat = datetime.fromisoformat
            record_vuln_failure(crate, ["RUSTSEC-2024-0001"])

    recurring = get_recurring_vulns()
    recurring_crates = [r.crate for r in recurring]
    assert crate in recurring_crates

    record = next(r for r in recurring if r.crate == crate)
    assert record.failure_count_30d >= RECURRING_THRESHOLD
    assert record.is_recurring is True


def test_failures_older_than_30_days_excluded(tmp_history):
    crate = "old-crate"
    now = datetime.now()

    history = {"vuln_tracking": {}}
    failures = []
    for i in range(5):
        old_ts = (now - timedelta(days=RECURRING_WINDOW_DAYS + 1 + i)).isoformat()
        failures.append({"ts": old_ts})

    history["vuln_tracking"][crate] = {
        "advisory_ids": ["CVE-2024-0001"],
        "failures": failures,
    }
    tmp_history.write_text(json.dumps(history, indent=2), encoding="utf-8")

    recurring = get_recurring_vulns()
    recurring_crates = [r.crate for r in recurring]
    assert crate not in recurring_crates


def test_two_failures_not_recurring(tmp_history):
    crate = "almost-recurring"
    for _ in range(RECURRING_THRESHOLD - 1):
        record_vuln_failure(crate, ["RUSTSEC-2024-0001"])

    recurring = get_recurring_vulns()
    recurring_crates = [r.crate for r in recurring]
    assert crate not in recurring_crates


def test_get_vuln_summary_structure(tmp_history):
    for _ in range(RECURRING_THRESHOLD):
        record_vuln_failure("summary-crate", ["GHSA-aaaa-bbbb-cccc"])

    summary = get_vuln_summary()
    assert "recurring_crates" in summary
    assert len(summary["recurring_crates"]) >= 1
    entry = summary["recurring_crates"][0]
    assert "crate" in entry
    assert "failures_30d" in entry
    assert "advisories" in entry


def test_advisory_ids_accumulated(tmp_history):
    crate = "multi-advisory"
    record_vuln_failure(crate, ["RUSTSEC-2024-0001"])
    record_vuln_failure(crate, ["GHSA-aaaa-bbbb-cccc"])
    record_vuln_failure(crate, ["CVE-2024-9999"])

    data = _load_fix_history()
    entry = data["vuln_tracking"][crate]
    assert "RUSTSEC-2024-0001" in entry["advisory_ids"]
    assert "GHSA-aaaa-bbbb-cccc" in entry["advisory_ids"]
    assert "CVE-2024-9999" in entry["advisory_ids"]


def test_multiple_crates_in_single_log():
    error_log = (
        "crate `openssl-sys` has RUSTSEC-2024-0001\n"
        "crate `hyper` has CVE-2024-5678\n"
    )
    result = extract_vuln_info(error_log)
    crates = [c for c, _ in result]
    assert "openssl-sys" in crates
    assert "hyper" in crates


def test_records_trimmed_to_max(tmp_history):
    crate = "trim-crate"
    for i in range(MAX_RECORDS_PER_CRATE + 10):
        record_vuln_failure(crate, [])

    data = _load_fix_history()
    entry = data["vuln_tracking"][crate]
    assert len(entry["failures"]) <= MAX_RECORDS_PER_CRATE
