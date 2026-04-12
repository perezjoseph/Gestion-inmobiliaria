"""
Semantic memory layer for CI fix attempts using SimpleMem.

Stores fix attempts as structured memories and retrieves relevant past
context via semantic search. Falls back to hash-based history.py when
SimpleMem is unavailable.

SimpleMem runs with local embeddings (Qwen3-Embedding-0.6B) — no API
key needed for storage and retrieval. An OpenAI-compatible LLM is only
needed for the `ask()` method, which we don't use here.
"""

import threading
from datetime import datetime

from .config import WIN_PROJECT_DIR, log

_SIMPLEMEM_AVAILABLE = False
_system = None
_init_lock = threading.Lock()
_init_attempted = False

LANCEDB_PATH = WIN_PROJECT_DIR / ".simplemem-data"


def _get_system():
    """Lazy-init SimpleMem. Returns the system or None."""
    global _system, _init_attempted, _SIMPLEMEM_AVAILABLE
    if _init_attempted:
        return _system
    with _init_lock:
        if _init_attempted:
            return _system
        _init_attempted = True
        try:
            import simplemem
            config = simplemem.get_config()
            config.lancedb_path = str(LANCEDB_PATH)
            config.memory_table_name = "ci_fix_memory"
            simplemem.set_config(config)
            _system = simplemem.create_system()
            _SIMPLEMEM_AVAILABLE = True
            log.info("SimpleMem initialized (semantic memory enabled)")
        except Exception as e:
            log.warning(f"SimpleMem unavailable, using hash-based history only: {e}")
            _system = None
    return _system


def store_fix_attempt(job, step, error_class, error_log, attempt, success):
    """Store a fix attempt as a SimpleMem dialogue entry."""
    sys = _get_system()
    if sys is None:
        return

    try:
        status = "SUCCEEDED" if success else "FAILED"
        ts = datetime.now().strftime("%Y-%m-%dT%H:%M:%S")
        error_preview = error_log[:800].replace("\n", " ").strip()

        sys.add_dialogue(
            speaker="ci-pipeline",
            text=(
                f"[{status}] CI fix attempt {attempt} for job={job} step={step} "
                f"class={error_class}. Error: {error_preview}"
            ),
            timestamp=ts,
        )
        log.debug(f"SimpleMem: stored fix attempt ({job}/{step}, {status})")
    except Exception as e:
        log.warning(f"SimpleMem store failed: {e}")


def store_fix_outcome(job, error_class, files_changed="", strategy=""):
    """Store the outcome/strategy of a successful fix for future reference."""
    sys = _get_system()
    if sys is None:
        return

    try:
        ts = datetime.now().strftime("%Y-%m-%dT%H:%M:%S")
        sys.add_dialogue(
            speaker="fix-agent",
            text=(
                f"[RESOLUTION] Fixed {job} ({error_class}). "
                f"Strategy: {strategy[:500]}. "
                f"Files: {files_changed[:300]}."
            ),
            timestamp=ts,
        )
        log.debug(f"SimpleMem: stored fix outcome for {job}")
    except Exception as e:
        log.warning(f"SimpleMem outcome store failed: {e}")


def search_relevant_context(job, error_log, max_results=5):
    """Search for relevant past fix context using semantic similarity.

    Returns a formatted string to inject into the prompt, or empty string.
    """
    sys = _get_system()
    if sys is None:
        return ""

    try:
        error_preview = error_log[:500].replace("\n", " ").strip()
        query = f"CI fix for job={job}: {error_preview}"

        sys.finalize()
        results = sys.hybrid_retriever.retrieve(query, top_k=max_results)

        if not results:
            return ""

        lines = ["SEMANTIC MEMORY (past fixes for similar errors):"]
        for i, entry in enumerate(results, 1):
            text = entry.get("text", entry.get("content", str(entry)))
            if isinstance(text, str) and len(text) > 20:
                lines.append(f"  {i}. {text[:300]}")

        if len(lines) <= 1:
            return ""

        log.info(f"SimpleMem: found {len(lines) - 1} relevant memories for {job}")
        return "\n".join(lines)

    except Exception as e:
        log.warning(f"SimpleMem search failed: {e}")
        return ""


def get_memory_stats():
    """Return memory system stats for the /health endpoint."""
    sys = _get_system()
    if sys is None:
        return {"status": "unavailable"}

    try:
        memories = sys.get_all_memories()
        return {
            "status": "active",
            "total_memories": len(memories) if memories else 0,
            "backend": "simplemem",
            "db_path": str(LANCEDB_PATH),
        }
    except Exception as e:
        return {"status": "error", "error": str(e)}
