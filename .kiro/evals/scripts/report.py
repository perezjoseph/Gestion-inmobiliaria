"""Generate a markdown report from benchmark.json."""

import json
import sys
from pathlib import Path


def generate_report(benchmark_path: str) -> None:
    path = Path(benchmark_path)
    if not path.exists():
        print(f"Benchmark file not found: {benchmark_path}")
        sys.exit(1)

    with open(path) as f:
        data = json.load(f)

    scores = data["scores"]
    lines = [
        "# Agent Eval Report",
        "",
        f"Run: `{data['run_dir']}`",
        f"Total evals: {data['total_evals']}",
        "",
        "## Summary",
        "",
        "| Variant | Pass Rate | Std Dev | N |",
        "|---------|-----------|---------|---|",
        f"| Specialist | {scores['specialist']['mean']:.1%} | ±{scores['specialist']['stdev']:.1%} | {scores['specialist']['n']} |",
        f"| Baseline (general-task-execution) | {scores['baseline']['mean']:.1%} | ±{scores['baseline']['stdev']:.1%} | {scores['baseline']['n']} |",
        "",
        f"**Delta: {scores['delta']:+.1%}** — {scores['verdict']}",
        "",
        "## Per-Eval Breakdown",
        "",
        "| Agent | Eval | Specialist | Baseline | Delta |",
        "|-------|------|-----------|----------|-------|",
    ]

    for entry in data.get("per_eval", []):
        spec_score = next((v["score"] for v in entry["variants"] if v["variant"] == "specialist"), 0)
        base_score = next((v["score"] for v in entry["variants"] if v["variant"] == "baseline"), 0)
        delta = spec_score - base_score
        lines.append(
            f"| {entry.get('agent', '?')} | {entry.get('eval_name', entry.get('eval_id', '?'))} "
            f"| {spec_score:.0%} | {base_score:.0%} | {delta:+.0%} |"
        )

    lines.extend(["", "## Assertion Details", ""])

    for entry in data.get("per_eval", []):
        lines.append(f"### {entry.get('eval_name', entry.get('eval_id', '?'))} ({entry.get('agent', '?')})")
        lines.append("")
        for variant in entry.get("variants", []):
            lines.append(f"**{variant['variant']}** (score: {variant['score']:.0%})")
            lines.append("")
            for a in variant.get("assertions", []):
                icon = "✓" if a["passed"] else "✗"
                lines.append(f"- {icon} {a['text']}")
                if a.get("evidence"):
                    lines.append(f"  - Evidence: {a['evidence']}")
            lines.append("")

    report_path = path.parent / "report.md"
    with open(report_path, "w", encoding="utf-8") as f:
        f.write("\n".join(lines))

    print(f"Report written to {report_path}")


if __name__ == "__main__":
    if len(sys.argv) != 2:
        print("Usage: python report.py <benchmark.json>")
        sys.exit(1)
    generate_report(sys.argv[1])
