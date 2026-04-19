import json
import sqlite3
import threading
import time
from datetime import datetime
from pathlib import Path

from .config import WIN_PROJECT_DIR, log

DB_PATH = str(WIN_PROJECT_DIR / ".webhook-queue.db")
_local = threading.local()


def _get_conn() -> sqlite3.Connection:
    if not hasattr(_local, "conn") or _local.conn is None:
        conn = sqlite3.connect(DB_PATH, timeout=10)
        conn.execute("PRAGMA journal_mode=WAL")
        conn.execute("PRAGMA busy_timeout=5000")
        conn.execute("""
            CREATE TABLE IF NOT EXISTS webhook_queue (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                endpoint TEXT NOT NULL,
                payload TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'pending',
                created_at TEXT NOT NULL,
                started_at TEXT,
                completed_at TEXT,
                error TEXT,
                retries INTEGER NOT NULL DEFAULT 0
            )
        """)
        conn.execute("""
            CREATE INDEX IF NOT EXISTS idx_queue_status
            ON webhook_queue(status)
        """)
        conn.commit()
        _local.conn = conn
    return _local.conn


def enqueue(endpoint: str, payload: dict) -> int:
    conn = _get_conn()
    cur = conn.execute(
        "INSERT INTO webhook_queue (endpoint, payload, status, created_at) "
        "VALUES (?, ?, 'pending', ?)",
        (endpoint, json.dumps(payload), datetime.now().isoformat()),
    )
    conn.commit()
    return cur.lastrowid


def dequeue() -> tuple[int, str, dict] | None:
    conn = _get_conn()
    row = conn.execute(
        "SELECT id, endpoint, payload FROM webhook_queue "
        "WHERE status = 'pending' ORDER BY id LIMIT 1",
    ).fetchone()
    if row is None:
        return None
    row_id, endpoint, payload_str = row
    conn.execute(
        "UPDATE webhook_queue SET status = 'processing', started_at = ? WHERE id = ?",
        (datetime.now().isoformat(), row_id),
    )
    conn.commit()
    return row_id, endpoint, json.loads(payload_str)


def mark_done(row_id: int) -> None:
    conn = _get_conn()
    conn.execute(
        "UPDATE webhook_queue SET status = 'done', completed_at = ? WHERE id = ?",
        (datetime.now().isoformat(), row_id),
    )
    conn.commit()


def mark_failed(row_id: int, error: str) -> None:
    conn = _get_conn()
    conn.execute(
        "UPDATE webhook_queue SET status = 'failed', completed_at = ?, error = ?, "
        "retries = retries + 1 WHERE id = ?",
        (datetime.now().isoformat(), error[:1000], row_id),
    )
    conn.commit()


def requeue_stale(max_age_s: int = 1800) -> int:
    conn = _get_conn()
    cutoff = datetime.fromtimestamp(time.time() - max_age_s).isoformat()
    cur = conn.execute(
        "UPDATE webhook_queue SET status = 'pending', started_at = NULL "
        "WHERE status = 'processing' AND started_at < ? AND retries < 3",
        (cutoff,),
    )
    conn.commit()
    count = cur.rowcount
    if count > 0:
        log.info(f"Requeued {count} stale webhook(s) older than {max_age_s}s")
    return count


def pending_count() -> int:
    conn = _get_conn()
    row = conn.execute(
        "SELECT COUNT(*) FROM webhook_queue WHERE status IN ('pending', 'processing')",
    ).fetchone()
    return row[0] if row else 0


def cleanup_old(days: int = 7) -> int:
    conn = _get_conn()
    cutoff = datetime.fromtimestamp(time.time() - days * 86400).isoformat()
    cur = conn.execute(
        "DELETE FROM webhook_queue WHERE status = 'done' AND completed_at < ?",
        (cutoff,),
    )
    conn.commit()
    count = cur.rowcount
    if count > 0:
        log.info(f"Cleaned up {count} completed webhook(s) older than {days} days")
    return count


def get_stats() -> dict:
    conn = _get_conn()
    rows = conn.execute(
        "SELECT status, COUNT(*) FROM webhook_queue GROUP BY status"
    ).fetchall()
    stats = {row[0]: row[1] for row in rows}
    stats["db_path"] = DB_PATH
    return stats
