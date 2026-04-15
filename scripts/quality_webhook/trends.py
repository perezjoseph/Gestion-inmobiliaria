from dataclasses import dataclass, field
from datetime import datetime

from .config import log
from .history import _fix_history_lock, _load_fix_history, _save_fix_history


@dataclass
class DurationSample:
    timestamp: str
    duration_s: float
    success: bool


@dataclass
class JobTrend:
    job: str
    samples: list[DurationSample] = field(default_factory=list)
    moving_avg_s: float = 0.0
    timeout_risk: bool = False
    sample_count: int = 0


JOB_TIMEOUTS = {
    "lint": 1200,
    "test-backend": 1800,
    "test-frontend": 1200,
    "coverage-backend": 1800,
    "quality-gate": 600,
    "secret-scan": 600,
    "build-frontend": 1200,
    "build-backend": 1200,
}

MAX_SAMPLES = 20
MOVING_AVG_WINDOW = 10
TIMEOUT_RISK_THRESHOLD = 0.80
MIN_SAMPLES_FOR_ANALYSIS = 5
DEFAULT_TIMEOUT_S = 1800


def record_duration(job: str, duration_s: float, success: bool) -> None:
    sample = {
        "ts": datetime.now().isoformat(),
        "duration_s": duration_s,
        "success": success,
    }
    with _fix_history_lock:
        history = _load_fix_history()
        duration_history = history.get("duration_history", {})
        job_samples = duration_history.get(job, [])
        job_samples.append(sample)
        if len(job_samples) > MAX_SAMPLES:
            job_samples = job_samples[-MAX_SAMPLES:]
        duration_history[job] = job_samples
        history["duration_history"] = duration_history
        _save_fix_history(history)


def analyze_trend(job: str) -> JobTrend:
    with _fix_history_lock:
        history = _load_fix_history()

    duration_history = history.get("duration_history", {})
    raw_samples = duration_history.get(job, [])

    samples = [
        DurationSample(
            timestamp=s.get("ts", ""),
            duration_s=s.get("duration_s", 0.0),
            success=s.get("success", True),
        )
        for s in raw_samples
    ]

    trend = JobTrend(job=job, samples=samples, sample_count=len(samples))

    if trend.sample_count < MIN_SAMPLES_FOR_ANALYSIS:
        return trend

    window = samples[-MOVING_AVG_WINDOW:]
    trend.moving_avg_s = sum(s.duration_s for s in window) / len(window)

    timeout_s = JOB_TIMEOUTS.get(job, DEFAULT_TIMEOUT_S)
    trend.timeout_risk = trend.moving_avg_s >= TIMEOUT_RISK_THRESHOLD * timeout_s

    return trend


def get_all_trends() -> dict[str, JobTrend]:
    with _fix_history_lock:
        history = _load_fix_history()

    duration_history = history.get("duration_history", {})
    trends = {}
    for job in duration_history:
        trends[job] = _analyze_trend_from_data(job, duration_history)
    return trends


def _analyze_trend_from_data(job: str, duration_history: dict) -> JobTrend:
    raw_samples = duration_history.get(job, [])
    samples = [
        DurationSample(
            timestamp=s.get("ts", ""),
            duration_s=s.get("duration_s", 0.0),
            success=s.get("success", True),
        )
        for s in raw_samples
    ]

    trend = JobTrend(job=job, samples=samples, sample_count=len(samples))

    if trend.sample_count < MIN_SAMPLES_FOR_ANALYSIS:
        return trend

    window = samples[-MOVING_AVG_WINDOW:]
    trend.moving_avg_s = sum(s.duration_s for s in window) / len(window)

    timeout_s = JOB_TIMEOUTS.get(job, DEFAULT_TIMEOUT_S)
    trend.timeout_risk = trend.moving_avg_s >= TIMEOUT_RISK_THRESHOLD * timeout_s

    return trend


def check_and_alert_trends() -> list[str]:
    trends = get_all_trends()
    at_risk = []
    for job, trend in trends.items():
        if trend.timeout_risk:
            at_risk.append(job)
            log.warning(
                f"Job '{job}' trending toward timeout: "
                f"moving_avg={trend.moving_avg_s:.1f}s, "
                f"timeout={JOB_TIMEOUTS.get(job, DEFAULT_TIMEOUT_S)}s, "
                f"samples={trend.sample_count}"
            )
    return at_risk
