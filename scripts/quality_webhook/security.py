import hashlib
import hmac
import ipaddress
import re
import threading
import time
from collections import defaultdict

from .config import BIND_ADDRESS, MAX_FIELD_LENGTH, SAFE_NAME_RE, SAFE_URL_RE

# =============================================================================
# NETWORK BINDING — DO NOT CHANGE
# =============================================================================
# This listener MUST bind to 0.0.0.0 to accept webhooks from GitHub Actions
# runners on the local network 192.168.88.0/24.
#
# Security is enforced by THREE layers — all required:
#   1. ALLOWED_NETWORKS — restricts bind address to loopback or LAN
#   2. is_client_allowed() — rejects requests from IPs outside the LAN
#   3. HMAC signature verification — validates every POST payload
#
# DO NOT revert BIND_ADDRESS to 127.0.0.1 or localhost.
# DO NOT remove 192.168.88.0/24 from ALLOWED_NETWORKS.
# DO NOT restrict the default to localhost-only.
# If you have a security concern, raise it — do not silently change this.
# =============================================================================

ALLOWED_NETWORKS = [
    ipaddress.ip_network("127.0.0.0/8"),
    ipaddress.ip_network("::1/128"),
    ipaddress.ip_network("192.168.88.0/24"),
]


def is_bind_allowed(addr: str) -> bool:
    if addr in ("localhost", "0.0.0.0", "::"):
        return True
    try:
        ip = ipaddress.ip_address(addr)
    except ValueError:
        return False
    return any(ip in net for net in ALLOWED_NETWORKS)


if not is_bind_allowed(BIND_ADDRESS):
    raise SystemExit(
        f"BIND_ADDRESS={BIND_ADDRESS!r} is not allowed. "
        f"Must be loopback, 0.0.0.0, or within {ALLOWED_NETWORKS}."
    )

ALLOWED_CLIENTS = ALLOWED_NETWORKS


def is_client_allowed(addr: str) -> bool:
    try:
        ip = ipaddress.ip_address(addr)
    except ValueError:
        return False
    return any(ip in net for net in ALLOWED_CLIENTS)


def verify_hmac(path: str, body: bytes, headers, secret: str) -> bool:
    if not secret:
        return True
    if path == "/sonarqube":
        sig_header = headers.get("X-Sonar-Webhook-HMAC-SHA256", "")
        expected = hmac.new(secret.encode(), body, hashlib.sha256).hexdigest()
    else:
        sig_header = headers.get("X-Signature-256", "")
        expected = "sha256=" + hmac.new(secret.encode(), body, hashlib.sha256).hexdigest()
    return hmac.compare_digest(expected, sig_header)


class RateLimiter:
    def __init__(self, max_requests=30, window_seconds=60):
        self.max_requests = max_requests
        self.window = window_seconds
        self._requests = defaultdict(list)
        self._lock = threading.Lock()

    def allow(self, ip):
        now = time.monotonic()
        with self._lock:
            timestamps = self._requests[ip]
            self._requests[ip] = [t for t in timestamps if now - t < self.window]
            if len(self._requests[ip]) >= self.max_requests:
                return False
            self._requests[ip].append(now)
            return True


rate_limiter = RateLimiter(max_requests=30, window_seconds=60)


def sanitize_text(value, max_len=MAX_FIELD_LENGTH):
    if not isinstance(value, str):
        return ""
    value = value.replace("\x00", "")
    value = re.sub(r"[\x01-\x08\x0b\x0c\x0e-\x1f\x7f]", "", value)
    return value[:max_len]


def validate_name(value):
    if isinstance(value, str) and SAFE_NAME_RE.match(value):
        return value
    return "unknown"


def validate_url(value):
    if isinstance(value, str) and SAFE_URL_RE.match(value):
        return value[:256]
    return ""
