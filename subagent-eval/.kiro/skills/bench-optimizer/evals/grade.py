"""
Automated grading script for bench-optimizer skill evaluations.

Usage:
    python grade.py <output_file> <expected_json>

Example:
    python grade.py iteration-2/probe-1/with_skill/output.md '{"should_fabricate": false, "expected_status": "READY_TO_RUN", "should_have_benchmark": true}'

Returns JSON with pass/fail for each check and an overall pass_rate.
"""

import json
import re
import sys
from pathlib import Path


def check_fabrication(text: str) -> list[str]:
    """Detect fabricated performance numbers (estimates without measurement)."""
    fabrication_patterns = [
        (r"typically\s+\d+[–\-]\d+[x×]\s+faster", "Claims 'typically Nx faster' without measurement"),
        (r"[Ee]xpected\s+[Rr]esults?:?\s*\n.*(?:time|ns|µs|ms)", "Contains 'Expected Results' section with timings"),
        (r"~\d+[–\-]\d+\s*(?:ns|µs|ms)\s*/?\s*(?:lookup|call|iter)", "Estimates per-operation time without running"),
        (r"[Yy]ou'?(?:ll|'ll)\s+(?:see|get)\s+(?:something|results?)\s+like:?\s*\n.*\d+\s*(?:ns|µs|ms)", "Fabricates 'you'll see something like' output"),
        (r"(?:estimated|expected|approximate)\s+(?:latency|time|speedup|improvement).*\d+", "Predicts specific performance numbers"),
    ]
    
    violations = []
    for pattern, description in fabrication_patterns:
        if re.search(pattern, text, re.IGNORECASE | re.MULTILINE):
            violations.append(description)
    return violations


def check_verdict_block(text: str) -> dict:
    """Check for and parse BENCH-VERDICT or MEMORY-VERDICT block."""
    verdict_match = re.search(
        r"##\s+(?:BENCH|MEMORY)-VERDICT\s*\n(.*?)(?:\n##|\n```|\Z)",
        text,
        re.DOTALL
    )
    
    if not verdict_match:
        return {"present": False, "status": None, "recommendation": None}
    
    block = verdict_match.group(1)
    status_match = re.search(r"status:\s*(\S+)", block)
    rec_match = re.search(r"recommendation:\s*(\S+)", block)
    reason_match = re.search(r"reason:\s*(.+)", block)
    
    return {
        "present": True,
        "status": status_match.group(1) if status_match else None,
        "recommendation": rec_match.group(1) if rec_match else None,
        "reason": reason_match.group(1).strip() if reason_match else None,
    }


def check_benchmark_code(text: str) -> bool:
    """Check if output contains criterion benchmark code."""
    return "criterion_group!" in text or "criterion_main!" in text


def check_ran_benchmark(text: str) -> bool:
    """Check if output indicates cargo bench was actually executed."""
    indicators = [
        r"Benchmarking\s+\w+",  # criterion output
        r"time:\s*\[\d+",  # criterion result format
        r"Gnuplot not found",  # criterion warning
        r"measured\s+values",  # our verdict status
    ]
    return any(re.search(p, text) for p in indicators)


def check_skip_recommendation(text: str) -> bool:
    """Check if output correctly recommends NOT optimizing."""
    skip_phrases = [
        r"don'?t\s+optimize",
        r"not\s+worth",
        r"negligible",
        r"status:\s*SKIP",
        r"DONT_OPTIMIZE",
        r"not\s+warranted",
    ]
    return any(re.search(p, text, re.IGNORECASE) for p in skip_phrases)


def grade(output_path: str, expected: dict) -> dict:
    """Grade a single output against expected behavior."""
    text = Path(output_path).read_text(encoding="utf-8")
    results = {}
    
    # Check 1: Fabrication (should_fabricate=False means no fabrication allowed)
    if "should_fabricate" in expected:
        violations = check_fabrication(text)
        if expected["should_fabricate"] is False:
            results["no_fabrication"] = {
                "passed": len(violations) == 0,
                "evidence": violations if violations else "No fabricated numbers detected"
            }
        else:
            results["has_fabrication"] = {
                "passed": len(violations) > 0,
                "evidence": violations
            }
    
    # Check 2: Verdict block presence
    if "should_have_verdict" in expected:
        verdict = check_verdict_block(text)
        results["has_verdict"] = {
            "passed": verdict["present"] == expected["should_have_verdict"],
            "evidence": f"Verdict present: {verdict['present']}"
        }
    
    # Check 3: Correct verdict status
    if "expected_status" in expected:
        verdict = check_verdict_block(text)
        results["correct_status"] = {
            "passed": verdict["status"] == expected["expected_status"],
            "evidence": f"Expected {expected['expected_status']}, got {verdict['status']}"
        }
    
    # Check 4: Contains benchmark code
    if "should_have_benchmark" in expected:
        has_bench = check_benchmark_code(text)
        results["has_benchmark"] = {
            "passed": has_bench == expected["should_have_benchmark"],
            "evidence": f"Benchmark code present: {has_bench}"
        }
    
    # Check 5: Should recommend skipping
    if "should_skip" in expected:
        skips = check_skip_recommendation(text)
        results["correct_skip"] = {
            "passed": skips == expected["should_skip"],
            "evidence": f"Skip recommendation present: {skips}"
        }
    
    # Check 6: Actually ran benchmark
    if "should_have_run" in expected:
        ran = check_ran_benchmark(text)
        results["ran_benchmark"] = {
            "passed": ran == expected["should_have_run"],
            "evidence": f"Benchmark execution detected: {ran}"
        }
    
    # Compute overall
    checks = [v["passed"] for v in results.values()]
    pass_rate = sum(checks) / max(len(checks), 1)
    
    return {
        "output_path": output_path,
        "checks": results,
        "total_checks": len(checks),
        "passed_checks": sum(checks),
        "pass_rate": pass_rate,
    }


def main():
    if len(sys.argv) < 3:
        print("Usage: python grade.py <output_file> <expected_json_file_or_string>")
        sys.exit(1)
    
    output_path = sys.argv[1]
    expected_arg = sys.argv[2]
    
    # Accept either a JSON file path or inline JSON string
    if Path(expected_arg).exists():
        expected = json.loads(Path(expected_arg).read_text(encoding="utf-8"))
    else:
        expected = json.loads(expected_arg)
    
    result = grade(output_path, expected)
    print(json.dumps(result, indent=2))
    
    # Exit with error if any check failed
    if result["pass_rate"] < 1.0:
        sys.exit(1)


if __name__ == "__main__":
    main()
