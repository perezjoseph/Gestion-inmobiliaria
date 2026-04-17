from dataclasses import dataclass, field
from datetime import datetime, timedelta

from .config import log
from .history import _fix_history_lock, _load_fix_history, _save_fix_history


FEASIBILITY_THRESHOLD = 0.10
FEASIBILITY_WARN_THRESHOLD = 0.30
MIN_ATTEMPTS_FOR_FEASIBILITY = 5
MAX_STRATEGIES_IN_PROMPT = 5
STRATEGY_WINDOW_DAYS = 30
REPRO_PROFILE_HISTORY = 10
REPRO_SKIP_PASS_RATE = 1.0
CO_FAILURE_THRESHOLD = 0.70
CO_FAILURE_MIN_EVENTS = 5
CO_FAILURE_HOLD_S = 30
CO_FAILURE_WINDOW_S = 120


@dataclass
class StrategyRecord:
    strategy: str
    successes: int
    failures: int
    last_used: str

    @property
    def success_rate(self) -> float:
        total = self.successes + self.failures
        if total == 0:
            return 0.0
        return self.successes / total


@dataclass
class FeasibilityResult:
    score: float
    sample_count: int
    recommendation: str


@dataclass
class ReproProfile:
    step_name: str
    outcomes: list[bool] = field(default_factory=list)
    pass_rate: float = 0.0
    skip_recommended: bool = False


@dataclass
class CoFailurePair:
    job_a: str
    job_b: str
    co_failures: int
    total_failures_a: int
    total_failures_b: int
    co_failure_rate: float
    is_correlated: bool


@dataclass
class FixDecision:
    feasibility: FeasibilityResult
    ranked_strategies: list[tuple[str, float]] = field(default_factory=list)
    steps_to_skip: list[str] = field(default_factory=list)
    hold_for_jobs: list[str] = field(default_factory=list)
    prompt_additions: str = ""


def record_strategy_outcome(job: str, error_class: str, strategy: str, success: bool | None) -> None:
    key = f"{job}|{error_class}"
    now = datetime.now().isoformat()
    with _fix_history_lock:
        history = _load_fix_history()
        stats = history.setdefault("strategy_stats", {})
        entries = stats.setdefault(key, [])
        entry = {"strategy": strategy, "ts": now}
        if success is not None:
            entry["success"] = success
        else:
            entry["shadow"] = True
        entries.append(entry)
        stats[key] = entries
        _save_fix_history(history)


def get_ranked_strategies(job: str, error_class: str) -> list[tuple[str, float]]:
    key = f"{job}|{error_class}"
    with _fix_history_lock:
        history = _load_fix_history()
    stats = history.get("strategy_stats", {})
    entries = stats.get(key, [])
    if not entries:
        log.info("no strategy history for %s", key)
        return []
    cutoff = (datetime.now() - timedelta(days=STRATEGY_WINDOW_DAYS)).isoformat()
    recent = [e for e in entries if e.get("ts", "") >= cutoff]
    if not recent:
        log.info("no strategy history for %s", key)
        return []
    totals: dict[str, list[bool]] = {}
    for e in recent:
        totals.setdefault(e["strategy"], []).append(e["success"])
    ranked = []
    for strat, outcomes in totals.items():
        rate = sum(outcomes) / len(outcomes)
        ranked.append((strat, rate))
    ranked.sort(key=lambda x: x[1], reverse=True)
    return ranked[:MAX_STRATEGIES_IN_PROMPT]


def compute_feasibility(job: str, error_class: str) -> FeasibilityResult:
    key = f"{job}|{error_class}"
    with _fix_history_lock:
        history = _load_fix_history()
    entries = history.get("strategy_stats", {}).get(key, [])
    recent = entries[-20:]
    total = len(recent)
    if total < MIN_ATTEMPTS_FOR_FEASIBILITY:
        return FeasibilityResult(score=1.0, sample_count=total, recommendation="proceed")
    successes = sum(1 for e in recent if e.get("success"))
    score = successes / total
    if score < FEASIBILITY_THRESHOLD:
        recommendation = "skip"
    elif score < FEASIBILITY_WARN_THRESHOLD:
        recommendation = "warn"
    else:
        recommendation = "proceed"
    return FeasibilityResult(score=score, sample_count=total, recommendation=recommendation)


def record_repro_outcome(job: str, step_name: str, passed: bool) -> None:
    with _fix_history_lock:
        history = _load_fix_history()
        profiles = history.setdefault("repro_profiles", {})
        job_profiles = profiles.setdefault(job, {})
        outcomes = job_profiles.get(step_name, [])
        outcomes.append(passed)
        job_profiles[step_name] = outcomes[-REPRO_PROFILE_HISTORY:]
        _save_fix_history(history)


def get_steps_to_skip(job: str) -> list[str]:
    with _fix_history_lock:
        history = _load_fix_history()
    job_profiles = history.get("repro_profiles", {}).get(job, {})
    skip = []
    for step_name, outcomes in job_profiles.items():
        if len(outcomes) == REPRO_PROFILE_HISTORY and all(outcomes):
            skip.append(step_name)
    return skip


def get_repro_profile(job: str) -> dict[str, dict]:
    with _fix_history_lock:
        history = _load_fix_history()
    job_profiles = history.get("repro_profiles", {}).get(job, {})
    result: dict[str, dict] = {}
    for step_name, outcomes in job_profiles.items():
        total = len(outcomes)
        passes = sum(outcomes)
        pass_rate = passes / total if total > 0 else 0.0
        skip_recommended = total == REPRO_PROFILE_HISTORY and all(outcomes)
        result[step_name] = {
            "outcomes": outcomes,
            "pass_rate": pass_rate,
            "skip_recommended": skip_recommended,
        }
    return result


def record_co_failure_event(commit: str, failed_jobs: list[str]) -> None:
    if len(failed_jobs) < 2:
        return
    sorted_jobs = sorted(set(failed_jobs))
    pairs = []
    for i in range(len(sorted_jobs)):
        for j in range(i + 1, len(sorted_jobs)):
            pairs.append((sorted_jobs[i], sorted_jobs[j]))
    with _fix_history_lock:
        history = _load_fix_history()
        matrix = history.setdefault("co_failure_matrix", {})
        for job_a, job_b in pairs:
            key = f"{job_a}|{job_b}"
            entry = matrix.get(key, {"co_failures": 0, "total_a": 0, "total_b": 0})
            entry["co_failures"] = entry.get("co_failures", 0) + 1
            entry["total_a"] = entry.get("total_a", 0) + 1
            entry["total_b"] = entry.get("total_b", 0) + 1
            matrix[key] = entry
        _save_fix_history(history)


def get_correlated_jobs(job: str) -> list[str]:
    with _fix_history_lock:
        history = _load_fix_history()
    matrix = history.get("co_failure_matrix", {})
    correlated = []
    for key, entry in matrix.items():
        parts = key.split("|", 1)
        if len(parts) != 2:
            continue
        job_a, job_b = parts
        if job_a != job and job_b != job:
            continue
        co = entry.get("co_failures", 0)
        if co < CO_FAILURE_MIN_EVENTS:
            continue
        total_a = entry.get("total_a", 0)
        total_b = entry.get("total_b", 0)
        denominator = min(total_a, total_b)
        if denominator == 0:
            continue
        rate = co / denominator
        if rate > CO_FAILURE_THRESHOLD:
            partner = job_b if job_a == job else job_a
            correlated.append(partner)
    return correlated


def should_hold_for_correlation(job: str) -> tuple[bool, list[str], float]:
    correlated = get_correlated_jobs(job)
    if correlated:
        return (True, correlated, CO_FAILURE_HOLD_S)
    return (False, [], 0)


def build_decision(job: str, error_class: str) -> FixDecision:
    feasibility = compute_feasibility(job, error_class)
    ranked_strategies = get_ranked_strategies(job, error_class)
    steps_to_skip = get_steps_to_skip(job)
    hold, hold_for_jobs, _ = should_hold_for_correlation(job)

    parts: list[str] = []
    if ranked_strategies:
        lines = ["PREFERRED STRATEGIES (by historical success rate):"]
        for strategy, rate in ranked_strategies:
            lines.append(f"  - {strategy}: {rate:.0%}")
        parts.append("\n".join(lines))
    if feasibility.recommendation == "warn":
        parts.append(
            f"WARNING: Low historical success rate ({feasibility.score:.0%}). "
            "Try a fundamentally different approach."
        )

    return FixDecision(
        feasibility=feasibility,
        ranked_strategies=ranked_strategies,
        steps_to_skip=steps_to_skip,
        hold_for_jobs=hold_for_jobs if hold else [],
        prompt_additions="\n\n".join(parts),
    )


def get_decision_cache_summary() -> dict:
    with _fix_history_lock:
        history = _load_fix_history()

    raw_stats = history.get("strategy_stats", {})
    strategy_stats: dict[str, dict] = {}
    for key, entries in raw_stats.items():
        totals: dict[str, int] = {}
        for e in entries:
            totals[e["strategy"]] = totals.get(e["strategy"], 0) + 1
        top_strategy = max(totals, key=totals.get) if totals else None
        strategy_stats[key] = {
            "strategy_count": len(totals),
            "top_strategy": top_strategy,
        }

    feasibility: dict[str, dict] = {}
    for key in raw_stats:
        parts = key.split("|", 1)
        if len(parts) != 2:
            continue
        job, error_class = parts
        result = compute_feasibility(job, error_class)
        feasibility[key] = {
            "score": result.score,
            "sample_count": result.sample_count,
            "recommendation": result.recommendation,
        }

    raw_profiles = history.get("repro_profiles", {})
    repro_profiles: dict[str, dict] = {}
    for job in raw_profiles:
        repro_profiles[job] = get_repro_profile(job)

    co_failure_pairs = history.get("co_failure_matrix", {})

    return {
        "strategy_stats": strategy_stats,
        "feasibility": feasibility,
        "repro_profiles": repro_profiles,
        "co_failure_pairs": co_failure_pairs,
    }
