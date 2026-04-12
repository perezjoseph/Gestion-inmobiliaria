"""
agentmemory REST client for semantic fix memory.

Talks to a local agentmemory server (port 3111) to store and recall
CI fix observations. All calls are fire-and-forget with timeouts —
if agentmemory is down, the webhook listener continues without it.

Start the server separately:
    npx @agentmemory/agentmemory
"""

import json
import logging
import os
import urllib.request
import urllib.error
from datetime import datetime

log = logging.getLogger("webhook.memory")

AGENTMEMORY_URL = os.environ.get("AGENTMEMORY_URL", "http://127.0.0.1:3111")
AGENTMEMORY_SECRET = os.environ.get("AGENTMEMORY_SECRET", "")
_TIMEOUT = 5  # seconds — fast fail if server is down
_SESSION_ID = f"webhook-{datetime.now().strftime('%Y%m%d-%H%M%S')}"
_available = None  # tri-state: None=unknown, True/False
_last_health_check = 0.0
_HEALTH_RECHECK_INTERVAL = 120  # re-check every 2 minutes if unavailable


def _headers():
    h = {"Content-Type": "application/json"}
    if AGENTMEMORY_SECRET:
        h["Authorization"] = f"Bearer {AGENTMEMORY_SECRET}"
    return h


def _post(path, payload):
    """POST JSON to agentmemory. Returns parsed response or None on failure."""
    global _available
    url = f"{AGENTMEMORY_URL}/agentmemory{path}"
    body = json.dumps(payload).encode("utf-8")
    req = urllib.request.Request(url, data=body, headers=_headers(), method="POST")
    try:
        with urllib.request.urlopen(req, timeout=_TIMEOUT) as resp:
            _available = True
            return json.loads(resp.read().decode("utf-8"))
    except urllib.error.URLError as e:
        if _available is not False:
            log.info(f"agentmemory unavailable ({e.reason}) — running without semantic memory")
            _available = False
        return None
    except Exception as e:
        if _available is not False:
            log.debug(f"agentmemory request failed: {e}")
            _available = False
        return None


def _get(path):
    """GET from agentmemory. Returns parsed response or None on failure."""
    global _available
    url = f"{AGENTMEMORY_URL}/agentmemory{path}"
    req = urllib.request.Request(url, headers=_headers(), method="GET")
    try:
        with urllib.request.urlopen(req, timeout=_TIMEOUT) as resp:
            _available = True
            return json.loads(resp.read().decode("utf-8"))
    except Exception:
        return None


def is_available():
    """Check if agentmemory server is reachable. Re-checks periodically if down."""
    global _available, _last_health_check
    import time
    now = time.monotonic()
    if _available is True:
        return True
    if _available is False and (now - _last_health_check) < _HEALTH_RECHECK_INTERVAL:
        return False
    _last_health_check = now
    result = _get("/health")
    _available = result is not None and isinstance(result, dict)
    if _available:
        log.info("agentmemory connected — semantic memory enabled")
    return _available


def observe_fix_attempt(job, step, error_class, error_log_preview,
                        attempt, success, files_changed=""):
    """Record a fix attempt as an observation in agentmemory."""
    if not is_available():
        return

    status = "succeeded" if success else "failed"
    content = (
        f"CI fix attempt for {job}/{step} (attempt {attempt}): {status}. "
        f"Error class: {error_class}. "
    )
    if files_changed:
        content += f"Files changed: {files_changed}. "
    content += f"Error preview: {error_log_preview[:500]}"

    _post("/observe", {
        "sessionId": _SESSION_ID,
        "tool": "ci-fix",
        "input": {
            "job": job,
            "step": step,
            "error_class": error_class,
            "attempt": attempt,
        },
        "output": {
            "success": success,
            "content": content,
        },
    })
    log.debug(f"agentmemory: observed fix attempt {job}/{step} #{attempt} ({status})")


def remember_successful_fix(job, error_class, error_summary, fix_description):
    """Save a successful fix as a long-term memory for future recall."""
    if not is_available():
        return

    _post("/remember", {
        "content": (
            f"Successfully fixed {job} ({error_class}): {fix_description}. "
            f"Original error: {error_summary}"
        ),
        "metadata": {
            "type": "ci-fix-success",
            "job": job,
            "error_class": error_class,
        },
    })
    log.debug(f"agentmemory: remembered successful fix for {job}")


def search_similar_fixes(job, error_log, error_class):
    """Search agentmemory for similar past fixes. Returns context string or empty."""
    if not is_available():
        return ""

    query = f"{job} {error_class} fix: {error_log[:300]}"
    result = _post("/smart-search", {
        "query": query,
        "limit": 5,
    })

    if not result:
        return ""

    memories = result.get("results") or result.get("memories") or []
    if not memories:
        return ""

    lines = ["SEMANTIC MEMORY (similar past fixes from agentmemory):"]
    for i, mem in enumerate(memories[:5], 1):
        content = ""
        if isinstance(mem, dict):
            content = mem.get("content") or mem.get("memory") or mem.get("text") or str(mem)
        elif isinstance(mem, str):
            content = mem
        if content:
            lines.append(f"  {i}. {content[:300]}")

    if len(lines) == 1:
        return ""

    lines.append("  Use these past fixes as reference. Adapt the approach if the error is similar.")
    return "\n".join(lines)


def observe_sonar_fix(sonar_report_preview, attempt, success):
    """Record a SonarQube fix attempt."""
    if not is_available():
        return

    _post("/observe", {
        "sessionId": _SESSION_ID,
        "tool": "sonar-fix",
        "input": {"report_preview": sonar_report_preview[:300], "attempt": attempt},
        "output": {"success": success},
    })


def observe_pipeline_improve(focus, attempt, success):
    """Record a pipeline improvement attempt."""
    if not is_available():
        return

    _post("/observe", {
        "sessionId": _SESSION_ID,
        "tool": "pipeline-improve",
        "input": {"focus": focus, "attempt": attempt},
        "output": {"success": success},
    })
