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
    scores = [g["score"] for g in gradings if "score" in g]

    if not scores:
        return {"mean": 0, "stdev": 0, "n": 0}

    m = mean(scores)
    s = stdev(scores) if len(scores) > 1 else 0
    return {"mean": round(m, 3), "stdev": round(s, 3), "n": len(scores)}


def compute_per_agent(gradings: list[dict]) -> dict:
    by_agent = {}
    for g in gradings:
        agent = g.get("agent", "unknown")
        if agent not in by_agent:
            by_agent[agent] = []
        by_agent[agent].append(g["score"])

    result = {}
    for agent, scores in by_agent.items():
        m = mean(scores)
        s = stdev(scores) if len(scores) > 1 else 0
        result[agent] = {"mean": round(m, 3), "stdev": round(s, 3), "n": len(scores)}
    return result


def aggregate(run_dir: str) -> None:
    run_path = Path(run_dir)
    if not run_path.exists():
        print(f"Run directory not found: {run_dir}")
        sys.exit(1)

    gradings = load_gradings(run_path)
    if not gradings:
        print(f"No grading.json files found in {run_dir}")
        sys.exit(1)

    scores = compute_scores(gradings)
    per_agent = compute_per_agent(gradings)

    contaminated = [g for g in gradings if g.get("contamination")]

    benchmark = {
        "run_dir": str(run_path),
        "total_evals": len(gradings),
        "overall": scores,
        "per_agent": per_agent,
        "contamination_count": len(contaminated),
        "per_eval": gradings
    }

    out_path = run_path / "benchmark.json"
    with open(out_path, "w") as f:
        json.dump(benchmark, f, indent=2)

    print(f"Benchmark written to {out_path}")
    print(f"  Overall: {scores['mean']:.1%} ± {scores['stdev']:.1%} (n={scores['n']})")
    print(f"  Contamination: {len(contaminated)}/{len(gradings)} evals")
    print()
    for agent, stats in per_agent.items():
        print(f"  {agent}: {stats['mean']:.1%} (n={stats['n']})")


if __name__ == "__main__":
    if len(sys.argv) != 2:
        print("Usage: python aggregate.py <run-directory>")
        sys.exit(1)
    aggregate(sys.argv[1])
