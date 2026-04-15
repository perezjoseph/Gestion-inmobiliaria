import json
import re
import threading
from dataclasses import dataclass, field
from datetime import datetime

from .config import WIN_PROJECT_DIR, log

_FLAKY_TESTS_FILE = WIN_PROJECT_DIR / ".flaky-tests.json"
_flaky_lock = threading.Lock()

MAX_OUTCOMES_PER_TEST = 20
FLAKINESS_THRESHOLD = 0.30
MIN_RUNS_FOR_FLAKY = 10

TEST_PASSED_RE = re.compile(r"^test\s+(\S+)\s+\.\.\.\s+ok$", re.MULTILINE)
TEST_FAILED_RE = re.compile(r"^test\s+(\S+)\s+\.\.\.\s+FAILED$", re.MULTILINE)


@dataclass
class TestOutcome:
    timestamp: str
    passed: bool


@dataclass
class TestRecord:
    test_name: str
    outcomes: list[TestOutcome] = field(default_factory=list)
    flakiness_score: float = 0.0
    is_flaky: bool = False
    run_count: int = 0


def _load_flaky_data() -> dict:
    try:
        if _FLAKY_TESTS_FILE.is_file():
            data = json.loads(_FLAKY_TESTS_FILE.read_text(encoding="utf-8"))
            if isinstance(data, dict):
                return data
    except (json.JSONDecodeError, OSError) as e:
        log.warning(f"Flaky test data load failed: {e}")
    return {}


def _save_flaky_data(data: dict) -> None:
    try:
        _FLAKY_TESTS_FILE.write_text(
            json.dumps(data, indent=2, ensure_ascii=False),
            encoding="utf-8",
            newline="\n",
        )
    except OSError as e:
        log.warning(f"Flaky test data save failed: {e}")


def parse_test_results(error_log: str) -> dict[str, bool]:
    results: dict[str, bool] = {}
    for match in TEST_PASSED_RE.finditer(error_log):
        results[match.group(1)] = True
    for match in TEST_FAILED_RE.finditer(error_log):
        results[match.group(1)] = False
    return results


def record_test_outcomes(job: str, results: dict[str, bool]) -> None:
    if not results:
        return
    now = datetime.now().isoformat()
    with _flaky_lock:
        data = _load_flaky_data()
        tests = data.get("tests", {})
        for test_name, passed in results.items():
            entry = tests.get(test_name, {"outcomes": []})
            outcomes = entry.get("outcomes", [])
            outcomes.append({"ts": now, "passed": passed})
            if len(outcomes) > MAX_OUTCOMES_PER_TEST:
                outcomes = outcomes[-MAX_OUTCOMES_PER_TEST:]
            total = len(outcomes)
            failures = sum(1 for o in outcomes if not o["passed"])
            score = failures / total if total > 0 else 0.0
            entry["outcomes"] = outcomes
            entry["flakiness_score"] = score
            entry["is_flaky"] = score > FLAKINESS_THRESHOLD and total >= MIN_RUNS_FOR_FLAKY
            entry["run_count"] = total
            tests[test_name] = entry
        data["tests"] = tests
        _save_flaky_data(data)


def are_all_failures_flaky(error_log: str) -> tuple[bool, list[str]]:
    failing_tests = [
        match.group(1) for match in TEST_FAILED_RE.finditer(error_log)
    ]
    if not failing_tests:
        return False, []

    with _flaky_lock:
        data = _load_flaky_data()

    tests = data.get("tests", {})
    flaky_names = []
    for test_name in failing_tests:
        entry = tests.get(test_name)
        if entry and entry.get("is_flaky", False):
            flaky_names.append(test_name)
        else:
            return False, flaky_names

    return True, flaky_names


def get_flaky_tests() -> list[TestRecord]:
    with _flaky_lock:
        data = _load_flaky_data()

    tests = data.get("tests", {})
    flaky = []
    for name, entry in tests.items():
        if entry.get("is_flaky", False):
            outcomes = [
                TestOutcome(timestamp=o.get("ts", ""), passed=o.get("passed", True))
                for o in entry.get("outcomes", [])
            ]
            flaky.append(TestRecord(
                test_name=name,
                outcomes=outcomes,
                flakiness_score=entry.get("flakiness_score", 0.0),
                is_flaky=True,
                run_count=entry.get("run_count", 0),
            ))
    return flaky


def get_flaky_test_summary() -> dict:
    with _flaky_lock:
        data = _load_flaky_data()

    tests = data.get("tests", {})
    total_tracked = len(tests)
    flaky_entries = []
    for name, entry in tests.items():
        if entry.get("is_flaky", False):
            flaky_entries.append({
                "name": name,
                "score": entry.get("flakiness_score", 0.0),
                "runs": entry.get("run_count", 0),
            })

    return {
        "total_tracked": total_tracked,
        "flaky_count": len(flaky_entries),
        "tests": flaky_entries,
    }
