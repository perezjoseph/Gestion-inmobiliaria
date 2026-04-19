import json
import re
import threading
from dataclasses import dataclass, field, asdict
from datetime import datetime, timedelta
from enum import Enum

from .config import (
    WIN_PROJECT_DIR, PIPELINE_BUDGET, PIPELINE_STATE_TTL_HOURS,
    BACKOFF_SCHEDULE, log,
)
from .runner import wsl_bash


class JobState(Enum):
    UNKNOWN = "unknown"
    PASSED = "passed"
    FAILED = "failed"
    FIXING = "fixing"
    SKIPPED = "skipped"


@dataclass
class QualitySnapshot:
    clippy_warning_count: int = 0
    test_count: int = 0
    sonar_issue_count: int = 0
    vuln_count: int = 0
    timestamp: str = ""


@dataclass
class PipelineRun:
    lineage_id: str
    current_commit: str
    round_number: int = 1
    job_states: dict[str, JobState] = field(default_factory=dict)
    deployed: bool = False
    sonarqube_passed: bool = False
    budget_total: int = PIPELINE_BUDGET
    budget_remaining: int = PIPELINE_BUDGET
    files_changed: dict[int, list[str]] = field(default_factory=dict)
    pending_strategies: dict[str, str] = field(default_factory=dict)
    quality_snapshots: dict[int, QualitySnapshot] = field(default_factory=dict)
    last_run_url: str = ""
    created_at: datetime = field(default_factory=datetime.now)
    last_updated: datetime = field(default_factory=datetime.now)


_PIPELINE_STATE_FILE = WIN_PROJECT_DIR / ".pipeline-state.json"
_LINEAGE_TAG_RE = re.compile(r"\[lineage:([a-f0-9]{7,40})\]")
_ISO_FORMAT = "%Y-%m-%dT%H:%M:%S.%f"


def _serialize_pipeline(run: PipelineRun) -> dict:
    return {
        "lineage_id": run.lineage_id,
        "current_commit": run.current_commit,
        "round_number": run.round_number,
        "job_states": {k: v.value for k, v in run.job_states.items()},
        "deployed": run.deployed,
        "sonarqube_passed": run.sonarqube_passed,
        "budget_total": run.budget_total,
        "budget_remaining": run.budget_remaining,
        "files_changed": {str(k): v for k, v in run.files_changed.items()},
        "pending_strategies": run.pending_strategies,
        "quality_snapshots": {
            str(k): asdict(v) for k, v in run.quality_snapshots.items()
        },
        "last_run_url": run.last_run_url,
        "created_at": run.created_at.strftime(_ISO_FORMAT),
        "last_updated": run.last_updated.strftime(_ISO_FORMAT),
    }


def _deserialize_pipeline(data: dict) -> PipelineRun:
    job_states = {
        k: JobState(v) for k, v in data.get("job_states", {}).items()
    }
    files_changed = {
        int(k): v for k, v in data.get("files_changed", {}).items()
    }
    quality_snapshots = {
        int(k): QualitySnapshot(**v)
        for k, v in data.get("quality_snapshots", {}).items()
    }
    try:
        created_at = datetime.strptime(data["created_at"], _ISO_FORMAT)
    except (KeyError, ValueError):
        created_at = datetime.now()
    try:
        last_updated = datetime.strptime(data["last_updated"], _ISO_FORMAT)
    except (KeyError, ValueError):
        last_updated = datetime.now()

    return PipelineRun(
        lineage_id=data["lineage_id"],
        current_commit=data.get("current_commit", ""),
        round_number=data.get("round_number", 1),
        job_states=job_states,
        deployed=data.get("deployed", False),
        sonarqube_passed=data.get("sonarqube_passed", False),
        budget_total=data.get("budget_total", PIPELINE_BUDGET),
        budget_remaining=data.get("budget_remaining", PIPELINE_BUDGET),
        files_changed=files_changed,
        pending_strategies=data.get("pending_strategies", {}),
        quality_snapshots=quality_snapshots,
        last_run_url=data.get("last_run_url", ""),
        created_at=created_at,
        last_updated=last_updated,
    )


class PipelineTracker:
    def __init__(self):
        self._lock = threading.Lock()
        self._pipelines: dict[str, PipelineRun] = {}
        self._commit_to_lineage: dict[str, str] = {}
        self._load()

    def _load(self):
        try:
            if _PIPELINE_STATE_FILE.is_file():
                data = json.loads(
                    _PIPELINE_STATE_FILE.read_text(encoding="utf-8")
                )
                if isinstance(data, dict):
                    cutoff = datetime.now() - timedelta(
                        hours=PIPELINE_STATE_TTL_HOURS
                    )
                    for lineage_id, entry in data.get("pipelines", {}).items():
                        run = _deserialize_pipeline(entry)
                        if run.last_updated >= cutoff:
                            self._pipelines[lineage_id] = run
                            self._commit_to_lineage[run.current_commit] = lineage_id
                    pruned = len(data.get("pipelines", {})) - len(self._pipelines)
                    if pruned > 0:
                        log.info(f"Pruned {pruned} expired pipeline entries")
                    log.info(
                        f"Loaded {len(self._pipelines)} pipeline entries "
                        f"from {_PIPELINE_STATE_FILE.name}"
                    )
        except (json.JSONDecodeError, OSError) as e:
            log.warning(f"Pipeline state load failed: {e}")

    def _save(self):
        data = {
            "pipelines": {
                lid: _serialize_pipeline(run)
                for lid, run in self._pipelines.items()
            }
        }
        try:
            _PIPELINE_STATE_FILE.write_text(
                json.dumps(data, indent=2, ensure_ascii=False),
                encoding="utf-8",
                newline="\n",
            )
        except OSError as e:
            log.warning(f"Pipeline state save failed: {e}")

    def _resolve_lineage(self, commit: str) -> str | None:
        if commit in self._commit_to_lineage:
            return self._commit_to_lineage[commit]
        for lid, run in self._pipelines.items():
            if run.current_commit == commit:
                self._commit_to_lineage[commit] = lid
                return lid
        return None

    def _parse_lineage_from_commit_message(self, commit: str) -> str | None:
        if not commit:
            return None
        try:
            result = wsl_bash(
                f"git log -1 --format=%B {commit}", timeout=10
            )
            if result.returncode == 0 and result.stdout:
                match = _LINEAGE_TAG_RE.search(result.stdout)
                if match:
                    return match.group(1)
        except Exception as e:
            log.debug(f"Failed to parse commit message for {commit[:8]}: {e}")
        return None

    def get_or_create(self, commit: str) -> PipelineRun:
        with self._lock:
            lineage_id = self._resolve_lineage(commit)
            if lineage_id and lineage_id in self._pipelines:
                run = self._pipelines[lineage_id]
                run.current_commit = commit
                run.last_updated = datetime.now()
                self._commit_to_lineage[commit] = lineage_id
                self._save()
                return run

            parsed_tag = self._parse_lineage_from_commit_message(commit)
            if parsed_tag and parsed_tag in self._pipelines:
                run = self._pipelines[parsed_tag]
                run.current_commit = commit
                run.last_updated = datetime.now()
                self._commit_to_lineage[commit] = parsed_tag
                self._save()
                return run

            lineage_id = commit[:12] if commit else datetime.now().strftime("%Y%m%d%H%M%S")
            run = PipelineRun(
                lineage_id=lineage_id,
                current_commit=commit,
            )
            self._pipelines[lineage_id] = run
            self._commit_to_lineage[commit] = lineage_id
            self._save()
            log.info(f"Created new pipeline lineage {lineage_id} for commit {commit[:8]}")
            return run

    def register_failure(self, commit: str, job: str, error_log: str, run_url: str = "") -> PipelineRun:
        with self._lock:
            run = self._get_or_create_unlocked(commit)
            run.job_states[job] = JobState.FAILED
            if run_url and run_url != run.last_run_url and run.last_run_url:
                run.round_number += 1
                run.budget_remaining = max(0, run.budget_total - run.round_number + 1)
                log.info(
                    f"Lineage {run.lineage_id}: new CI run detected, "
                    f"round={run.round_number}, budget_remaining={run.budget_remaining}"
                )
            if run_url:
                run.last_run_url = run_url
            run.last_updated = datetime.now()
            self._save()
            return run

    def update_from_report(self, commit: str, job_results: dict[str, str]) -> PipelineRun:
        with self._lock:
            run = self._get_or_create_unlocked(commit)
            state_map = {
                "success": JobState.PASSED,
                "failure": JobState.FAILED,
                "skipped": JobState.SKIPPED,
                "cancelled": JobState.SKIPPED,
            }
            for job, result in job_results.items():
                new_state = state_map.get(result, JobState.UNKNOWN)
                current = run.job_states.get(job)
                if current == JobState.FIXING and new_state == JobState.FAILED:
                    pass
                else:
                    run.job_states[job] = new_state
            run.round_number += 1
            run.budget_remaining = max(0, run.budget_total - run.round_number + 1)
            run.last_updated = datetime.now()
            self._save()
            log.info(
                f"Lineage {run.lineage_id}: updated from report, "
                f"round={run.round_number}, jobs={len(job_results)}"
            )
            return run

    def mark_deployed(self, commit: str) -> None:
        with self._lock:
            run = self._get_or_create_unlocked(commit)
            run.deployed = True
            run.last_updated = datetime.now()
            self._save()
            log.info(f"Lineage {run.lineage_id}: marked as deployed")

    def mark_job_fixing(self, commit: str, job: str) -> None:
        with self._lock:
            run = self._get_or_create_unlocked(commit)
            run.job_states[job] = JobState.FIXING
            run.last_updated = datetime.now()
            self._save()

    def mark_job_fixed(
        self, commit: str, job: str, new_commit: str,
        files: list[str], strategy: str,
    ) -> None:
        with self._lock:
            run = self._get_or_create_unlocked(commit)
            run.job_states[job] = JobState.PASSED
            run.current_commit = new_commit
            self._commit_to_lineage[new_commit] = run.lineage_id
            if run.round_number not in run.files_changed:
                run.files_changed[run.round_number] = []
            existing = set(run.files_changed[run.round_number])
            existing.update(files)
            run.files_changed[run.round_number] = sorted(existing)
            run.pending_strategies[job] = strategy
            run.last_updated = datetime.now()
            self._save()
            log.info(
                f"Lineage {run.lineage_id}: job {job} fixed, "
                f"new_commit={new_commit[:8]}, files={len(files)}"
            )

    def is_budget_exhausted(self, commit: str) -> bool:
        with self._lock:
            run = self._get_or_create_unlocked(commit)
            return run.round_number >= run.budget_total

    def should_backoff(self, commit: str, job: str, run_url: str = "") -> tuple[bool, float]:
        with self._lock:
            run = self._get_or_create_unlocked(commit)
            if run_url and run_url != run.last_run_url:
                return False, 0.0
            idx = min(run.round_number, len(BACKOFF_SCHEDULE) - 1)
            wait_seconds = BACKOFF_SCHEDULE[idx] if idx >= 0 else 0
            if wait_seconds > 0:
                log.info(
                    f"Lineage {run.lineage_id}: backoff {wait_seconds}s "
                    f"for round {run.round_number}"
                )
                return True, float(wait_seconds)
            return False, 0.0

    def detect_regression(
        self, commit: str, failed_job: str, error_files: set[str],
    ) -> list[str]:
        with self._lock:
            run = self._get_or_create_unlocked(commit)
            prev_round = run.round_number - 1
            if prev_round < 1:
                return []
            prev_files = set(run.files_changed.get(prev_round, []))
            if not prev_files or not error_files:
                return []
            overlap = prev_files & error_files
            if not overlap:
                return []
            regressed_jobs = []
            for job, state in run.job_states.items():
                if job == failed_job:
                    continue
                job_files = set(run.files_changed.get(prev_round, []))
                if job_files & overlap:
                    regressed_jobs.append(job)
            if regressed_jobs:
                log.warning(
                    f"Lineage {run.lineage_id}: regression detected for {failed_job}, "
                    f"overlapping files from jobs: {regressed_jobs}"
                )
            return regressed_jobs

    def confirm_strategy(self, commit: str, job: str, success: bool) -> None:
        with self._lock:
            run = self._get_or_create_unlocked(commit)
            strategy = run.pending_strategies.pop(job, None)
            if strategy:
                outcome = "confirmed_success" if success else "confirmed_failure"
                log.info(
                    f"Lineage {run.lineage_id}: strategy for {job} "
                    f"{outcome}: {strategy[:80]}"
                )
                from .decisions import record_strategy_outcome as _record
                error_class = strategy.split("_")[0] if "_" in strategy else "unknown"
                _record(job, error_class, strategy, success, confirmed=success)
            run.last_updated = datetime.now()
            self._save()

    def record_quality_snapshot(
        self, commit: str, round_num: int, snapshot: QualitySnapshot,
    ) -> None:
        with self._lock:
            run = self._get_or_create_unlocked(commit)
            snapshot.timestamp = datetime.now().isoformat()
            run.quality_snapshots[round_num] = snapshot
            run.last_updated = datetime.now()
            self._save()

    def get_summary(self) -> dict:
        with self._lock:
            pipelines = {}
            for lid, run in self._pipelines.items():
                latest_snapshot = None
                if run.quality_snapshots:
                    latest_round = max(run.quality_snapshots.keys())
                    snap = run.quality_snapshots[latest_round]
                    latest_snapshot = {
                        "clippy_warning_count": snap.clippy_warning_count,
                        "test_count": snap.test_count,
                        "sonar_issue_count": snap.sonar_issue_count,
                        "vuln_count": snap.vuln_count,
                        "timestamp": snap.timestamp,
                    }
                pipelines[lid] = {
                    "current_commit": run.current_commit[:8] if run.current_commit else "",
                    "round_number": run.round_number,
                    "deployed": run.deployed,
                    "sonarqube_passed": run.sonarqube_passed,
                    "budget_remaining": run.budget_remaining,
                    "job_states": {k: v.value for k, v in run.job_states.items()},
                    "pending_strategies": list(run.pending_strategies.keys()),
                    "quality_snapshot": latest_snapshot,
                    "last_updated": run.last_updated.strftime(_ISO_FORMAT),
                }
            return {
                "active_lineages": len(self._pipelines),
                "deployed_count": sum(
                    1 for r in self._pipelines.values() if r.deployed
                ),
                "budget_exhausted_count": sum(
                    1 for r in self._pipelines.values()
                    if r.round_number >= r.budget_total
                ),
                "pipelines": pipelines,
            }

    def _get_or_create_unlocked(self, commit: str) -> PipelineRun:
        lineage_id = self._resolve_lineage(commit)
        if lineage_id and lineage_id in self._pipelines:
            run = self._pipelines[lineage_id]
            run.current_commit = commit
            return run

        parsed_tag = self._parse_lineage_from_commit_message(commit)
        if parsed_tag and parsed_tag in self._pipelines:
            run = self._pipelines[parsed_tag]
            run.current_commit = commit
            self._commit_to_lineage[commit] = parsed_tag
            return run

        lineage_id = commit[:12] if commit else datetime.now().strftime("%Y%m%d%H%M%S")
        run = PipelineRun(
            lineage_id=lineage_id,
            current_commit=commit,
        )
        self._pipelines[lineage_id] = run
        self._commit_to_lineage[commit] = lineage_id
        log.info(f"Created new pipeline lineage {lineage_id} for commit {commit[:8]}")
        return run


_tracker_instance: PipelineTracker | None = None
_tracker_init_lock = threading.Lock()


def get_tracker() -> PipelineTracker:
    global _tracker_instance
    if _tracker_instance is None:
        with _tracker_init_lock:
            if _tracker_instance is None:
                _tracker_instance = PipelineTracker()
    return _tracker_instance


def capture_quality_snapshot() -> QualitySnapshot:
    snapshot = QualitySnapshot(timestamp=datetime.now().isoformat())

    try:
        clippy_result = wsl_bash(
            "cargo clippy --locked -p realestate-backend --message-format=json -- -D warnings 2>&1 || true",
            timeout=120,
        )
        if clippy_result.stdout:
            snapshot.clippy_warning_count = clippy_result.stdout.count('"level":"warning"')
    except Exception:
        pass

    try:
        test_result = wsl_bash(
            "cargo test --locked -p realestate-backend --all-targets -- --list 2>&1 || true",
            timeout=60,
        )
        if test_result.stdout:
            import re as _re
            count_match = _re.search(r"(\d+) tests?", test_result.stdout)
            if count_match:
                snapshot.test_count = int(count_match.group(1))
    except Exception:
        pass

    try:
        audit_result = wsl_bash("cargo audit --json 2>/dev/null || true", timeout=60)
        if audit_result.stdout:
            snapshot.vuln_count = audit_result.stdout.count('"kind":"vulnerability"')
    except Exception:
        pass

    return snapshot


def compare_snapshots(before: QualitySnapshot, after: QualitySnapshot) -> list[str]:
    regressions = []
    if after.clippy_warning_count > before.clippy_warning_count:
        regressions.append(
            f"clippy warnings: {before.clippy_warning_count} → {after.clippy_warning_count}"
        )
    if after.test_count < before.test_count and before.test_count > 0:
        regressions.append(
            f"test count decreased: {before.test_count} → {after.test_count}"
        )
    if after.vuln_count > before.vuln_count:
        regressions.append(
            f"vulnerabilities: {before.vuln_count} → {after.vuln_count}"
        )
    return regressions
