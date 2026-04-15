# Feature: ci-pipeline-optimization, Property 1, 2, 3: Webhook payload, sanitization, HMAC
"""
Property-based tests for webhook payload construction, sanitization, and HMAC
signature verification.

**Validates: Requirements 1.3, 1.4, 1.5**

Property 1: For any job name, step name, error log string, run URL, commit SHA,
branch name, and actor, the constructed webhook payload SHALL contain all seven
required fields, and the error_log field SHALL be at most 3000 bytes.

Property 2: For any string containing secret-like patterns, sanitize_payload
SHALL replace every occurrence with a redacted placeholder, and the output SHALL
contain none of the original secret values.

Property 3: For any valid webhook payload (bytes) and any non-empty secret,
computing the HMAC-SHA256 signature and then verifying it with verify_hmac SHALL
return True. Verifying with a different secret or corrupted signature SHALL
return False.
"""

import hashlib
import hmac as hmac_mod
import json
import re

from hypothesis import given, settings, assume
from hypothesis import strategies as st

from scripts.quality_webhook.security import verify_hmac, sanitize_text


REQUIRED_FIELDS = {"job", "step", "error_log", "run_url", "commit", "branch", "actor"}
MAX_ERROR_LOG_BYTES = 3000


def build_webhook_payload(
    job: str,
    step: str,
    error_log: str,
    run_url: str,
    commit: str,
    branch: str,
    actor: str,
) -> dict:
    """Mirror the bash jq payload construction with error_log truncation.

    The bash pipeline does:
      ERROR_LOG=$(printf '%s' "$ERROR_LOG" | sanitize_payload | head -c 3000)
      PAYLOAD=$(jq -n --arg job ... '{job:$job, ...}')

    This function replicates that: truncate error_log to 3000 bytes, then
    build the JSON payload dict with all 7 required fields.
    """
    error_log_bytes = error_log.encode("utf-8")[:MAX_ERROR_LOG_BYTES]
    truncated_error_log = error_log_bytes.decode("utf-8", errors="ignore")

    return {
        "job": job,
        "step": step,
        "error_log": truncated_error_log,
        "run_url": run_url,
        "commit": commit,
        "branch": branch,
        "actor": actor,
    }


SECRET_PREFIXES = ["ghp_", "ghs_", "gho_", "ghu_", "github_pat_"]
SENSITIVE_KEYS = ["password", "secret", "token", "key"]

REDACTED = "[REDACTED]"


def sanitize_payload(text: str) -> str:
    """Mirror the bash sed redaction patterns for testability.

    The bash sanitize_payload function does:
      sed -E 's/(ghp_|ghs_|gho_|ghu_|github_pat_)[A-Za-z0-9_]+/[REDACTED]/g;
              s/Bearer [A-Za-z0-9._-]+/Bearer [REDACTED]/g;
              s|postgresql://[^ ]+|postgresql://[REDACTED]|g;
              s/(password|secret|token|key)=[^ ]+/\\1=[REDACTED]/gi'
    """
    text = re.sub(
        r"(ghp_|ghs_|gho_|ghu_|github_pat_)[A-Za-z0-9_]+",
        REDACTED,
        text,
    )
    text = re.sub(
        r"Bearer [A-Za-z0-9._-]+",
        f"Bearer {REDACTED}",
        text,
    )
    text = re.sub(
        r"postgresql://[^ ]+",
        f"postgresql://{REDACTED}",
        text,
    )
    text = re.sub(
        r"(password|secret|token|key)=[^ ]+",
        rf"\1={REDACTED}",
        text,
        flags=re.IGNORECASE,
    )
    return text


def compute_hmac(payload: bytes, secret: str) -> str:
    """Mirror the bash openssl HMAC computation.

    The bash pipeline does:
      SIGNATURE=$(printf '%s' "$PAYLOAD" | openssl dgst -sha256 -hmac "$SECRET" | awk '{print $NF}')
      curl ... -H "X-Signature-256: sha256=$SIGNATURE"

    This returns the full header value: "sha256=<hex_digest>"
    """
    digest = hmac_mod.new(secret.encode(), payload, hashlib.sha256).hexdigest()
    return f"sha256={digest}"


# ---------------------------------------------------------------------------
# Property 1: Webhook payload field completeness and truncation
# ---------------------------------------------------------------------------

@given(
    job=st.text(),
    step=st.text(),
    error_log_len=st.integers(min_value=0, max_value=10000),
    error_log_char=st.text(min_size=1, max_size=1),
    run_url=st.text(),
    commit=st.text(),
    branch=st.text(),
    actor=st.text(),
)
@settings(max_examples=200, deadline=None)
def test_payload_contains_all_fields_and_truncates_error_log(
    job, step, error_log_len, error_log_char, run_url, commit, branch, actor
):
    """Property 1: Constructed payload contains all 7 required fields and
    error_log is at most 3000 bytes.

    **Validates: Requirements 1.3**
    """
    error_log = error_log_char * error_log_len

    payload = build_webhook_payload(job, step, error_log, run_url, commit, branch, actor)

    assert set(payload.keys()) == REQUIRED_FIELDS, (
        f"Payload missing fields: {REQUIRED_FIELDS - set(payload.keys())}"
    )

    error_log_byte_len = len(payload["error_log"].encode("utf-8"))
    assert error_log_byte_len <= MAX_ERROR_LOG_BYTES, (
        f"error_log is {error_log_byte_len} bytes, exceeds {MAX_ERROR_LOG_BYTES}"
    )


@given(
    job=st.text(min_size=0, max_size=50),
    step=st.text(min_size=0, max_size=50),
    error_log=st.text(min_size=0, max_size=10000),
    run_url=st.text(min_size=0, max_size=100),
    commit=st.text(min_size=0, max_size=50),
    branch=st.text(min_size=0, max_size=50),
    actor=st.text(min_size=0, max_size=50),
)
@settings(max_examples=200, deadline=None)
def test_payload_serializes_to_valid_json(
    job, step, error_log, run_url, commit, branch, actor
):
    """Property 1 (serialization): The payload round-trips through JSON.

    **Validates: Requirements 1.3**
    """
    payload = build_webhook_payload(job, step, error_log, run_url, commit, branch, actor)

    serialized = json.dumps(payload)
    deserialized = json.loads(serialized)

    assert set(deserialized.keys()) == REQUIRED_FIELDS
    assert len(deserialized["error_log"].encode("utf-8")) <= MAX_ERROR_LOG_BYTES


# ---------------------------------------------------------------------------
# Property 2: Payload sanitization redacts all secret patterns
# ---------------------------------------------------------------------------

token_alphabet = st.sampled_from(
    list("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789_")
)
token_body = st.text(alphabet=token_alphabet, min_size=5, max_size=30)


@st.composite
def secret_with_context(draw):
    """Generate a string containing at least one secret pattern surrounded by
    random text, returning both the full string and the raw secret value."""
    prefix_text = draw(st.text(min_size=0, max_size=20))
    suffix_text = draw(st.text(min_size=0, max_size=20))
    body = draw(token_body)

    pattern_type = draw(st.sampled_from([
        "github_token", "bearer", "postgresql", "key_value",
    ]))

    if pattern_type == "github_token":
        prefix = draw(st.sampled_from(SECRET_PREFIXES))
        secret_value = f"{prefix}{body}"
    elif pattern_type == "bearer":
        bearer_chars = re.sub(r"[^A-Za-z0-9._-]", "x", body)
        secret_value = f"Bearer {bearer_chars}"
    elif pattern_type == "postgresql":
        secret_value = f"postgresql://user:{body}@host/db"
    else:
        key_name = draw(st.sampled_from(SENSITIVE_KEYS))
        secret_value = f"{key_name}={body}"

    full_text = f"{prefix_text}{secret_value}{suffix_text}"
    return full_text, secret_value, pattern_type


@given(data=secret_with_context())
@settings(max_examples=200, deadline=None)
def test_sanitize_payload_redacts_secret_patterns(data):
    """Property 2: sanitize_payload removes all secret values from output.

    **Validates: Requirements 1.4**
    """
    full_text, secret_value, pattern_type = data

    sanitized = sanitize_payload(full_text)

    if pattern_type == "github_token":
        for prefix in SECRET_PREFIXES:
            remaining = re.findall(
                rf"(?:{re.escape(prefix)})[A-Za-z0-9_]+", sanitized
            )
            assert not remaining, (
                f"GitHub token pattern still present after sanitization: {remaining}"
            )
    elif pattern_type == "bearer":
        remaining = re.findall(r"Bearer [A-Za-z0-9._-]+", sanitized)
        for match in remaining:
            assert match == f"Bearer {REDACTED}", (
                f"Bearer token not fully redacted: {match}"
            )
    elif pattern_type == "postgresql":
        remaining = re.findall(r"postgresql://[^ ]+", sanitized)
        for match in remaining:
            assert match == f"postgresql://{REDACTED}", (
                f"PostgreSQL URI not fully redacted: {match}"
            )
    else:
        for key_name in SENSITIVE_KEYS:
            remaining = re.findall(
                rf"(?i){re.escape(key_name)}=[^ ]+", sanitized
            )
            for match in remaining:
                assert match.lower().endswith(f"={REDACTED}".lower()), (
                    f"Key=value pattern not redacted: {match}"
                )


@given(
    secrets_list=st.lists(secret_with_context(), min_size=1, max_size=5),
    separator=st.text(min_size=1, max_size=10),
)
@settings(max_examples=200, deadline=None)
def test_sanitize_payload_redacts_multiple_secrets(secrets_list, separator):
    """Property 2 (multiple secrets): All secret patterns are redacted when
    multiple appear in the same string.

    **Validates: Requirements 1.4**
    """
    combined = separator.join(item[0] for item in secrets_list)
    sanitized = sanitize_payload(combined)

    for prefix in SECRET_PREFIXES:
        remaining = re.findall(rf"(?:{re.escape(prefix)})[A-Za-z0-9_]+", sanitized)
        assert not remaining, (
            f"GitHub token pattern still present: {remaining}"
        )

    bearer_matches = re.findall(r"Bearer [A-Za-z0-9._-]+", sanitized)
    for match in bearer_matches:
        assert match == f"Bearer {REDACTED}", (
            f"Bearer token not fully redacted: {match}"
        )


# ---------------------------------------------------------------------------
# Property 3: HMAC signature round-trip verification
# ---------------------------------------------------------------------------

class _FakeHeaders(dict):
    """Dict subclass that mimics request headers for verify_hmac."""
    def get(self, key, default=""):
        return super().get(key, default)


@given(
    payload=st.binary(min_size=1, max_size=5000),
    secret=st.text(min_size=1, max_size=100),
)
@settings(max_examples=200, deadline=None)
def test_hmac_sign_then_verify_returns_true(payload, secret):
    """Property 3: Computing HMAC and verifying with the same secret returns True.

    **Validates: Requirements 1.5**
    """
    signature = compute_hmac(payload, secret)
    headers = _FakeHeaders({"X-Signature-256": signature})

    result = verify_hmac("/ci-failure", payload, headers, secret)
    assert result is True, (
        f"HMAC verification failed for valid signature. "
        f"Secret length={len(secret)}, payload length={len(payload)}"
    )


@given(
    payload=st.binary(min_size=1, max_size=5000),
    secret=st.text(min_size=1, max_size=100),
    wrong_secret=st.text(min_size=1, max_size=100),
)
@settings(max_examples=200, deadline=None)
def test_hmac_wrong_secret_returns_false(payload, secret, wrong_secret):
    """Property 3 (wrong secret): Verifying with a different secret returns False.

    **Validates: Requirements 1.5**
    """
    assume(secret != wrong_secret)

    signature = compute_hmac(payload, secret)
    headers = _FakeHeaders({"X-Signature-256": signature})

    result = verify_hmac("/ci-failure", payload, headers, wrong_secret)
    assert result is False, (
        f"HMAC verification should fail with wrong secret. "
        f"Correct secret length={len(secret)}, wrong secret length={len(wrong_secret)}"
    )


@given(
    payload=st.binary(min_size=1, max_size=5000),
    secret=st.text(min_size=1, max_size=100),
)
@settings(max_examples=200, deadline=None)
def test_hmac_corrupted_signature_returns_false(payload, secret):
    """Property 3 (corrupted signature): Verifying with a corrupted signature
    returns False.

    **Validates: Requirements 1.5**
    """
    signature = compute_hmac(payload, secret)

    if signature.endswith("0"):
        corrupted = signature[:-1] + "1"
    else:
        corrupted = signature[:-1] + "0"

    headers = _FakeHeaders({"X-Signature-256": corrupted})

    result = verify_hmac("/ci-failure", payload, headers, secret)
    assert result is False, (
        f"HMAC verification should fail with corrupted signature"
    )
