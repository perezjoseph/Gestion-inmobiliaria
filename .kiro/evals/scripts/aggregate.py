"""Aggregate eval grading results into a benchmark summary."""

import json
import sys
from pathlib import Path
from statistics import mean, stdev


def load_gradings(run_dir: Path) -> list[dict]:
    results = []
    for grading_file in run_dir.rglob("grading.json"):
        with open(grading_file) as f:
            results.append(json.load(f))
    return results


def compute_scores(gradings: list[dict]) -> dict:
    specialist_scores = []
    baseline_scores = []

    for g in gradings:
        for variant in g.get("variants", []):
            score = variant["score"]
            if variant["variant"] == "specialist":
                specialist_scores.append(score)
            elif variant["variant"] == "baseline":
                baseline_scores.append(score)

    def stats(scores):
        if not scores:
            return {"mean": 0, "stdev": 0, "n": 0}
        m = mean(scores)
        s = stdev(scores) if len(scores) > 1 else 0
        return {"mean": round(m, 3), "stdev": round(s, 3), "n": len(scores)}

    spec_stats = stats(specialist_scores)
    base_stats = stats(baseline_scores)
    delta = round(spec_stats["mean"] - base_stats["mean"], 3)

    return {
        "specialist": spec_stats,
        "baseline": base_stats,
        "delta": delta,
        "verdict": "SPECIALIST_WINS" if delta > 0.1 else "NO_DIFFERENCE" if abs(delta) <= 0.1 else "BASELINE_WINS"
    }


def aggregate(run_dir: str) -> None:
    run_path = Path(run_dir)
    if not run_path.exists():
        print(f"Run directory not found: {run_dir}")
        sys.exit(1)

    gradings = load_gradings(run_path)
    if not gradings:
        print(f"No grading.json files found in {run_dir}")
        sys.exit(1)

    benchmark = {
        "run_dir": str(run_path),
        "total_evals": len(gradings),
        "scores": compute_scores(gradings),
        "per_eval": []
    }

    for g in gradings:
        eval_entry = {
            "eval_id": g.get("eval_id"),
            "eval_name": g.get("eval_name"),
            "agent": g.get("agent"),
            "variants": g.get("variants", [])
        }
        benchmark["per_eval"].append(eval_entry)

    out_path = run_path / "benchmark.json"
    with open(out_path, "w") as f:
        json.dump(benchmark, f, indent=2)

    print(f"Benchmark written to {out_path}")
    print(f"  Specialist: {benchmark['scores']['specialist']['mean']:.1%} (n={benchmark['scores']['specialist']['n']})")
    print(f"  Baseline:   {benchmark['scores']['baseline']['mean']:.1%} (n={benchmark['scores']['baseline']['n']})")
    print(f"  Delta:      {benchmark['scores']['delta']:+.1%}")
    print(f"  Verdict:    {benchmark['scores']['verdict']}")


if __name__ == "__main__":
    if len(sys.argv) != 2:
        print("Usage: python aggregate.py <run-directory>")
        sys.exit(1)
    aggregate(sys.argv[1])
