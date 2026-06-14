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

    overall = data["overall"]
    per_agent = data.get("per_agent", {})

    lines = [
        "# Agent Eval Report",
        "",
        f"Run: `{data['run_dir']}`",
        f"Total evals: {data['total_evals']}",
        f"Contamination: {data['contamination_count']}/{data['total_evals']} evals",
        "",
        "## Overall",
        "",
        f"**Pass rate: {overall['mean']:.1%}** ± {overall['stdev']:.1%} (n={overall['n']})",
        "",
        "## Per Agent",
        "",
        "| Agent | Pass Rate | Std Dev | N |",
        "|-------|-----------|---------|---|",
    ]

    for agent, stats in per_agent.items():
        lines.append(f"| {agent} | {stats['mean']:.1%} | ±{stats['stdev']:.1%} | {stats['n']} |")

    lines.extend(["", "## Per-Eval Breakdown", ""])
    lines.append("| Agent | Eval | Score | Contamination |")
    lines.append("|-------|------|-------|---------------|")

    for entry in data.get("per_eval", []):
        contam = ", ".join(entry.get("contamination", [])) or "—"
        lines.append(
            f"| {entry.get('agent', '?')} | {entry.get('eval_name', entry.get('eval_id', '?'))} "
            f"| {entry['score']:.0%} | {contam} |"
        )

    lines.extend(["", "## Assertion Details", ""])

    for entry in data.get("per_eval", []):
        lines.append(f"### {entry.get('eval_name', entry.get('eval_id', '?'))} ({entry.get('agent', '?')})")
        lines.append("")
        lines.append(f"Score: {entry['score']:.0%}")
        lines.append("")
        for a in entry.get("assertions", []):
            icon = "✓" if a["passed"] else "✗"
            lines.append(f"- {icon} {a['text']}")
            if a.get("evidence"):
                lines.append(f"  - {a['evidence']}")
        if entry.get("contamination"):
            lines.append(f"- ⚠ Contamination: {', '.join(entry['contamination'])}")
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
