import logging
import os
import pathlib
import re
import socket

import json

_env_path = pathlib.Path(__file__).resolve().parent.parent.parent / ".env"
if _env_path.is_file():
    with open(_env_path) as _f:
        for _line in _f:
            _line = _line.strip()
            if _line and not _line.startswith("#") and "=" in _line:
                _k, _, _v = _line.partition("=")
                os.environ.setdefault(_k.strip(), _v.strip())

PORT = 9090
BIND_ADDRESS = os.environ.get("BIND_ADDRESS", "0.0.0.0")
WEBHOOK_SECRET = os.environ.get("WEBHOOK_SECRET", "")

WIN_PROJECT_DIR = pathlib.Path(__file__).resolve().parent.parent.parent
MAX_RETRIES = 3
KIRO_TIMEOUT = 900
MAX_CONCURRENT_FIXES = 3
THREAD_POOL_SIZE = 8
REQUEST_QUEUE_SIZE = 16
MAX_PAYLOAD_BYTES = 512 * 1024
MAX_FIELD_LENGTH = 50_000
MAX_LOG_BYTES = 50 * 1024 * 1024

DEDUP_WINDOW_MINUTES = 60
SONAR_PARALLEL_GROUPS = 3

WSL_DISTRO = "Ubuntu-22.04"
WSL_USER = "jperez"
KIRO_CLI = f"/home/{WSL_USER}/.local/bin/kiro-cli"

SAFE_NAME_RE = re.compile(r"^[a-zA-Z0-9_\-]{1,64}$")
SAFE_URL_RE = re.compile(
    r"^https://github\.com/[a-zA-Z0-9._\-]+/[a-zA-Z0-9._\-]+/actions/runs/\d+$"
)

SCOPE_CONSTRAINTS = (
    "\n\nSCOPE CONSTRAINTS (mandatory):\n"
    "- Do NOT refactor unrelated code. Fix only what the error requires.\n"
    "- Do NOT add new crate dependencies unless absolutely necessary for the fix.\n"
    "- Do NOT modify CI pipeline YAML files (.github/workflows/) unless the job is specifically about pipeline config.\n"
    "- Do NOT modify scripts/quality-webhook-listener.py.\n"
    "- Prefer minimal, targeted edits over broad changes.\n"
)


def to_wsl_path(win_path: pathlib.Path) -> str:
    drive = win_path.drive
    if drive and len(drive) == 2 and drive[1] == ":":
        rest = win_path.as_posix()[2:]
        return f"/mnt/{drive[0].lower()}{rest}"
    return str(win_path)


PROJECT_DIR = to_wsl_path(WIN_PROJECT_DIR)

_STRUCTURED_LOGGING = os.environ.get("LOG_FORMAT", "text").lower() == "json"


class _JsonFormatter(logging.Formatter):
    def format(self, record):
        entry = {
            "ts": self.formatTime(record, self.datefmt),
            "level": record.levelname,
            "msg": record.getMessage(),
            "logger": record.name,
        }
        if record.exc_info and record.exc_info[0]:
            entry["exception"] = self.formatException(record.exc_info)
        return json.dumps(entry, ensure_ascii=False)


_handler = logging.StreamHandler()
if _STRUCTURED_LOGGING:
    _handler.setFormatter(_JsonFormatter(datefmt="%Y-%m-%dT%H:%M:%S"))
else:
    _handler.setFormatter(logging.Formatter(
        "%(asctime)s [%(levelname)s] %(message)s",
        datefmt="%Y-%m-%d %H:%M:%S",
    ))

logging.basicConfig(level=logging.DEBUG, handlers=[_handler])
log = logging.getLogger("webhook")


def get_local_ip():
    try:
        sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        sock.settimeout(2)
        sock.connect(("8.8.8.8", 80))
        ip = sock.getsockname()[0]
        sock.close()
        return ip
    except Exception:
        return "127.0.0.1"
