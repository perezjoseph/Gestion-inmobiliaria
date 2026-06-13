"""
Agent runner for subagent orchestration evaluation.

Instruments kiro-cli subprocess calls externally using opentelemetry-sdk.
Writes OTLP JSON spans directly to eval/otel-traces.jsonl (no collector needed).

Usage:
    python eval/agent_runner.py                    # Run all test cases
    python eval/agent_runner.py --case tc-001      # Single test case
    python eval/agent_runner.py --dry-run          # Preview without executing
"""

import json
import subprocess
import sys
import time
from pathlib import Path

from opentelemetry import trace
from opentelemetry.sdk.trace import TracerProvider
from opentelemetry.sdk.trace.export import SimpleSpanProcessor, SpanExporter, SpanExportResult
from opentelemetry.sdk.resources import Resource

EVAL_DIR = Path(__file__).parent
REPO_ROOT = EVAL_DIR.parent
TEST_CASES_FILE = EVAL_DIR / "test-cases.jsonl"
TRACES_DIR = EVAL_DIR / "traces"
OTEL_TRACES_FILE = EVAL_DIR / "otel-traces.jsonl"


class JsonFileExporter(SpanExporter):
    """Exports spans as OTLP-compatible JSON lines to a file."""

    def __init__(self, file_path: Path):
        self.file_path = file_path
        self.file_path.parent.mkdir(parents=True, exist_ok=True)

    def export(self, spans):
        with open(self.file_path, "a", encoding="utf-8") as f:
            for span in spans:
                resource_attrs = [
                    {"key": k, "value": {"stringValue": str(v)}}
                    for k, v in span.resource.attributes.items()
                ]
                span_attrs = [
                    {"key": k, "value": {"stringValue": str(v)} if isinstance(v, str)
                     else {"intValue": str(v)} if isinstance(v, int)
                     else {"doubleValue": v} if isinstance(v, float)
                     else {"stringValue": str(v)}}
                    for k, v in span.attributes.items()
                ]
                otlp = {
                    "resourceSpans": [{
                        "resource": {"attributes": resource_attrs},
                        "scopeSpans": [{
                            "scope": {"name": span.instrumentation_scope.name if hasattr(span, 'instrumentation_scope') else "kiro-eval"},
                            "spans": [{
                                "traceId": format(span.context.trace_id, '032x'),
                                "spanId": format(span.context.span_id, '016x'),
                                "name": span.name,
                                "kind": 1,
                                "startTimeUnixNano": str(span.start_time),
                                "endTimeUnixNano": str(span.end_time),
                                "attributes": span_attrs,
                            }]
                        }]
                    }]
                }
                f.write(json.dumps(otlp) + "\n")
        return SpanExportResult.SUCCESS

    def shutdown(self):
        pass


def setup_tracing() -> trace.Tracer:
    if OTEL_TRACES_FILE.exists():
        OTEL_TRACES_FILE.unlink()

    resource = Resource.create({"service.name": "kiro-subagent-eval"})
    exporter = JsonFileExporter(OTEL_TRACES_FILE)
    provider = TracerProvider(resource=resource)
    provider.add_span_processor(SimpleSpanProcessor(exporter))
    trace.set_tracer_provider(provider)
    return trace.get_tracer("kiro-eval")


def load_test_cases(case_id: str | None = None) -> list[dict]:
    cases = []
    with open(TEST_CASES_FILE) as f:
        for line in f:
            if line.strip():
                tc = json.loads(line)
                if case_id is None or tc["id"] == case_id:
                    cases.append(tc)
    return cases


def run_test_case(tracer: trace.Tracer, tc: dict) -> dict:
    prompt = tc["input"]
    expected_agent = tc.get("expected_agent", "kiro_default")

    with tracer.start_as_current_span("agent_invocation") as span:
        span.set_attribute("traceloop.entity.name", expected_agent)
        span.set_attribute("traceloop.entity.input", json.dumps({"prompt": prompt}))
        span.set_attribute("eval.test_case_id", tc["id"])
        span.set_attribute("eval.expected_agent", expected_agent)

        print(f"  [{tc['id']}] Running: {prompt[:70]}...")
        start = time.time()

        try:
            result = subprocess.run(
                ["kiro-cli", "chat", "--no-interactive", "--trust-all-tools", prompt],
                capture_output=True, text=True, timeout=600,
                cwd=str(REPO_ROOT),
            )
            stdout = result.stdout
            exit_code = result.returncode
        except subprocess.TimeoutExpired:
            stdout = ""
            exit_code = -1
            print(f"  [{tc['id']}] TIMEOUT")
        except FileNotFoundError:
            stdout = ""
            exit_code = -2
            print(f"  [{tc['id']}] kiro-cli not found")

        elapsed = time.time() - start
        span.set_attribute("traceloop.entity.output", json.dumps({"response": stdout[:10000]}))
        span.set_attribute("eval.exit_code", exit_code)
        span.set_attribute("eval.duration_seconds", round(elapsed, 2))

    print(f"  [{tc['id']}] Done in {elapsed:.1f}s (exit={exit_code})")
    return {
        "test_case_id": tc["id"],
        "input": prompt,
        "expected_agent": expected_agent,
        "stdout": stdout,
        "exit_code": exit_code,
        "elapsed_seconds": round(elapsed, 2),
    }


def process_traces():
    processor = REPO_ROOT / "subagent-eval" / ".evalkit" / "tracing" / "trace-processor.py"
    if not OTEL_TRACES_FILE.exists():
        print("  Warning: No otel-traces.jsonl found")
        return
    TRACES_DIR.mkdir(exist_ok=True)
    subprocess.run(
        [sys.executable, str(processor),
         "--input", str(OTEL_TRACES_FILE),
         "--output-dir", str(TRACES_DIR),
         "--pretty"],
        cwd=str(REPO_ROOT), check=True,
    )
    count = len(list(TRACES_DIR.glob("*.json")))
    print(f"  Processed {count} trace file(s) → {TRACES_DIR}/")


def main():
    import argparse

    parser = argparse.ArgumentParser()
    parser.add_argument("--case", type=str, help="Run specific test case ID")
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args()

    cases = load_test_cases(args.case)
    if not cases:
        print(f"No test cases found" + (f" matching '{args.case}'" if args.case else ""))
        sys.exit(1)

    if args.dry_run:
        for tc in cases:
            print(f"[DRY RUN] {tc['id']}: kiro-cli chat --no-interactive --trust-all-tools \"{tc['input'][:60]}...\"")
        return

    print(f"\n{'='*60}")
    print(f"Running {len(cases)} test case(s)")
    print(f"{'='*60}\n")

    print("[1/3] Initializing OpenTelemetry tracer (file exporter)...")
    tracer = setup_tracing()

    print("\n[2/3] Executing test cases...")
    results = []
    for tc in cases:
        results.append(run_test_case(tracer, tc))

    # Flush
    trace.get_tracer_provider().force_flush()

    print("\n[3/3] Processing traces...")
    process_traces()

    results_file = EVAL_DIR / "results" / "agent_runs.json"
    results_file.parent.mkdir(exist_ok=True)
    results_file.write_text(json.dumps(results, indent=2), encoding="utf-8")

    print(f"\n{'='*60}")
    print(f"Complete. Results: {results_file}")
    print(f"Traces:  {TRACES_DIR}/")
    print(f"Raw OTLP: {OTEL_TRACES_FILE}")
    print(f"{'='*60}")


if __name__ == "__main__":
    main()
