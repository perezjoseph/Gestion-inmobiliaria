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


def record_strategy_outcome(job: str, error_class: str, strategy: str, success: bool) -> None:
    key = f"{job}|{error_class}"
    now = datetime.now().isoformat()
    with _fix_history_lock:
        history = _load_fix_history()
        stats = history.setdefault("strategy_stats", {})
        entries = stats.setdefault(key, [])
        entries.append({"strategy": strategy, "ts": now, "success": success})
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
