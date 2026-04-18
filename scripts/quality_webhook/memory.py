"""
Semantic memory layer for CI fix attempts.

Uses SimpleMem's local embedding model (Qwen3-Embedding-0.6B, 1024-d) for
vector similarity search over past fix attempts. Stores memories in LanceDB
(embedded, file-based). No LLM or API key required — embeddings run locally.

Falls back gracefully when SimpleMem isn't installed.
"""

import json
import re
import threading
from datetime import datetime

from .config import WIN_PROJECT_DIR, log

_SIMPLEMEM_AVAILABLE = False
_embedder = None
_db = None
_table = None
_init_lock = threading.Lock()
_init_attempted = False

LANCEDB_PATH = str(WIN_PROJECT_DIR / ".simplemem-data")
TABLE_NAME = "ci_fix_attempts"


def _get_components():
    """Lazy-init embedding model and LanceDB table. Returns (embedder, table) or (None, None)."""
    global _embedder, _db, _table, _init_attempted, _SIMPLEMEM_AVAILABLE
    if _init_attempted:
        return _embedder, _table
    with _init_lock:
        if _init_attempted:
            return _embedder, _table
        _init_attempted = True
        try:
            from simplemem.utils.embedding import EmbeddingModel
            import lancedb

            _embedder = EmbeddingModel()
            _db = lancedb.connect(LANCEDB_PATH)

            try:
                _table = _db.open_table(TABLE_NAME)
            except Exception:
                import pyarrow as pa
                schema = pa.schema([
                    pa.field("vector", pa.list_(pa.float32(), 1024)),
                    pa.field("text", pa.string()),
                    pa.field("job", pa.string()),
                    pa.field("error_class", pa.string()),
                    pa.field("success", pa.bool_()),
                    pa.field("timestamp", pa.string()),
                ])
                _table = _db.create_table(TABLE_NAME, schema=schema)

            _SIMPLEMEM_AVAILABLE = True
            log.info(f"SimpleMem memory initialized (LanceDB at {LANCEDB_PATH})")
        except Exception as e:
            log.warning(f"SimpleMem unavailable, using hash-based history only: {e}")
            _embedder = None
            _table = None
    return _embedder, _table


def _rebuild_table():
    """Recreate the LanceDB table when data files are corrupted."""
    global _table, _db
    try:
        import lancedb
        import pyarrow as pa

        log.warning("SimpleMem: rebuilding corrupted LanceDB table")
        if _db is None:
            _db = lancedb.connect(LANCEDB_PATH)
        _db.drop_table(TABLE_NAME, ignore_missing=True)
        schema = pa.schema([
            pa.field("vector", pa.list_(pa.float32(), 1024)),
            pa.field("text", pa.string()),
            pa.field("job", pa.string()),
            pa.field("error_class", pa.string()),
            pa.field("success", pa.bool_()),
            pa.field("timestamp", pa.string()),
        ])
        _table = _db.create_table(TABLE_NAME, schema=schema)
        log.info("SimpleMem: table rebuilt successfully")
    except Exception as e:
        log.warning(f"SimpleMem: rebuild failed: {e}")
        _table = None


def store_fix_attempt(job, step, error_class, error_log, attempt, success):
    """Store a fix attempt with its embedding for semantic search."""
    embedder, table = _get_components()
    if embedder is None or table is None:
        return

    try:
        status = "SUCCEEDED" if success else "FAILED"
        ts = datetime.now().strftime("%Y-%m-%dT%H:%M:%S")
        error_preview = error_log[:600].replace("\n", " ").strip()

        text = (
            f"[{status}] CI fix attempt {attempt} for job={job} step={step} "
            f"class={error_class}. Error: {error_preview}"
        )

        vector = embedder.encode(text)
        if hasattr(vector, 'shape') and len(vector.shape) > 1:
            vector = vector[0]

        table.add([{
            "vector": vector.tolist(),
            "text": text,
            "job": job,
            "error_class": error_class,
            "success": success,
            "timestamp": ts,
        }])
        log.debug(f"SimpleMem: stored fix attempt ({job}/{step}, {status})")
    except Exception as e:
        log.warning(f"SimpleMem store failed: {e}")
        if "Not found" in str(e) and ".lance" in str(e):
            _rebuild_table()


def store_fix_outcome(job, error_class, files_changed="", strategy=""):
    """Store the outcome/strategy of a successful fix for future reference."""
    embedder, table = _get_components()
    if embedder is None or table is None:
        return

    try:
        ts = datetime.now().strftime("%Y-%m-%dT%H:%M:%S")
        text = (
            f"[RESOLUTION] Fixed {job} ({error_class}). "
            f"Strategy: {strategy[:500]}. "
            f"Files: {files_changed[:300]}."
        )

        vector = embedder.encode(text)
        if hasattr(vector, 'shape') and len(vector.shape) > 1:
            vector = vector[0]

        table.add([{
            "vector": vector.tolist(),
            "text": text,
            "job": job,
            "error_class": error_class,
            "success": True,
            "timestamp": ts,
        }])
        log.debug(f"SimpleMem: stored fix outcome for {job}")
    except Exception as e:
        log.warning(f"SimpleMem outcome store failed: {e}")


def search_relevant_context(job, error_log, max_results=5):
    """Search for relevant past fix context using vector similarity.

    Returns a formatted string to inject into the prompt, or empty string.
    """
    embedder, table = _get_components()
    if embedder is None or table is None:
        return ""

    try:
        error_preview = error_log[:500].replace("\n", " ").strip()
        query = f"CI fix for job={job}: {error_preview}"

        query_vector = embedder.encode(query)
        if hasattr(query_vector, 'shape') and len(query_vector.shape) > 1:
            query_vector = query_vector[0]
        results = table.search(query_vector.tolist()).limit(max_results).to_list()

        if not results:
            return ""

        lines = ["SEMANTIC MEMORY (past fixes for similar errors):"]
        for i, row in enumerate(results, 1):
            text = row.get("text", "")
            dist = row.get("_distance", 0)
            if text and len(text) > 20 and dist < 1.5:
                lines.append(f"  {i}. [dist={dist:.2f}] {text[:300]}")

        if len(lines) <= 1:
            return ""

        log.info(f"SimpleMem: found {len(lines) - 1} relevant memories for {job}")
        return "\n".join(lines)

    except Exception as e:
        log.warning(f"SimpleMem search failed: {e}")
        if "Not found" in str(e) and ".lance" in str(e):
            _rebuild_table()
        return ""


def get_memory_stats():
    """Return memory system stats for the /health endpoint."""
    _, table = _get_components()
    if table is None:
        return {"status": "unavailable"}

    try:
        count = table.count_rows()
        return {
            "status": "active",
            "total_memories": count,
            "backend": "simplemem-vector",
            "db_path": LANCEDB_PATH,
        }
    except Exception as e:
        return {"status": "error", "error": str(e)}


def extract_outcome_from_output(kiro_output):
    """Parse kiro-cli output to extract what the agent actually did.

    Looks for git commit messages, file modifications, and cargo/gradle
    commands to build a summary of the fix strategy.
    """
    if not kiro_output:
        return {"files_changed": "", "strategy": "", "commit_msg": ""}

    tail = kiro_output[-8000:]

    commit_msg = ""
    commit_matches = re.findall(r"git commit.*?-m\s+['\"](.+?)['\"]", tail)
    if commit_matches:
        commit_msg = commit_matches[-1][:200]

    files = set()
    for pattern in [
        r"(?:modified|changed|created|wrote|updated):\s*(\S+\.(?:rs|toml|kt|kts|yml|yaml|xml|json))",
        r"strReplace.*?path[\"']:\s*[\"']([^\"']+)[\"']",
        r"fsWrite.*?path[\"']:\s*[\"']([^\"']+)[\"']",
    ]:
        for m in re.findall(pattern, tail, re.IGNORECASE):
            if len(m) < 200 and not m.startswith("."):
                files.add(m)

    git_diff = re.findall(r"(\d+) files? changed", tail)
    if git_diff and not files:
        files.add(f"({git_diff[-1]} files from git diff)")

    strategy_hints = []
    for pattern, label in [
        (r"cargo update -p (\S+)", "updated crate: {}"),
        (r"cargo audit", "ran cargo audit"),
        (r"cargo fmt", "ran cargo fmt"),
        (r"cargo clippy", "ran cargo clippy"),
        (r"cargo test", "ran cargo test"),
        (r"gradlew (\S+)", "ran gradlew {}"),
        (r"Updated? (\S+) (?:from|version) (\S+) to (\S+)", "updated {} from {} to {}"),
        (r"added.*suppress", "added suppression"),
        (r"\.cargo/audit\.toml", "modified audit.toml ignore list"),
    ]:
        matches = re.findall(pattern, tail, re.IGNORECASE)
        if matches:
            last = matches[-1]
            if isinstance(last, tuple):
                strategy_hints.append(label.format(*last))
            else:
                strategy_hints.append(label.format(last) if "{}" in label else label)

    pushed = bool(re.search(r"git push.*origin", tail))
    if pushed:
        strategy_hints.append("pushed to origin")

    return {
        "files_changed": ", ".join(sorted(files)[:10]),
        "strategy": "; ".join(strategy_hints[:8]) or commit_msg or "unknown",
        "commit_msg": commit_msg,
    }
