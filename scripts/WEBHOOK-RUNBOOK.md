# Webhook Listener Runbook

Operational guide for `scripts/quality-webhook-listener.py` — the local webhook that receives CI/CD events and triggers kiro-cli auto-fixes.

---

## Architecture

```
GitHub Actions CI ──POST──▶ webhook listener (:9090) ──subprocess──▶ WSL bash ──▶ kiro-cli
SonarQube server ──POST──▶       (Windows Python)                    (Ubuntu)
```

The listener runs as a **Windows Python process**. It writes prompt files to the project directory, then calls `wsl -u jperez -d Ubuntu-22.04 bash -lc "..."` to invoke kiro-cli inside WSL.

### Key files

| File | Purpose |
|---|---|
| `scripts/quality-webhook-listener.py` | The listener itself |
| `.env` | `WEBHOOK_SECRET` and other config |
| `kiro-debug.log` | Full kiro-cli output for each invocation |
| `.kiro-prompt-*.txt` | Temp prompt files (auto-deleted after use) |
| `.github/workflows/ci.yml` | CI workflow that sends webhooks |
| `.github/workflows/android-ci.yml` | Android CI workflow that sends webhooks |

### Endpoints

| Method | Path | Source | Purpose |
|---|---|---|---|
| GET | `/health` | Manual/monitoring | Health check + lock status |
| POST | `/sonarqube` | SonarQube webhook | Quality gate failure auto-fix |
| POST | `/ci-failure` | GitHub Actions | CI job failure auto-fix |
| POST | `/ci-improve` | GitHub Actions | Pipeline self-improvement |
| POST | `/sonar-fix` | GitHub Actions | Batch SonarQube issue resolution |

---

## Starting the Listener

```bash
# From Windows (PowerShell or cmd)
cd D:\realestate
python scripts/quality-webhook-listener.py

# Or in background
python scripts/quality-webhook-listener.py > webhook.log 2>&1 &
```

Required environment:
- `WEBHOOK_SECRET` must be set (in `.env` or environment)
- Port 9090 must be free
- WSL distro `Ubuntu-22.04` with user `jperez` must be available
- `kiro-cli` must be in the WSL user's PATH

---

## Debugging

### 1. Check if the listener is running

```bash
# From WSL
curl -s http://127.0.0.1:9090/health | jq .
```

Expected response:
```json
{
  "status": "ok",
  "timestamp": "2026-04-12T17:00:00.000000",
  "locks": {
    "fix": false,
    "sonar_fix": false,
    "improve": false
  }
}
```

If a lock is `true`, a kiro-cli invocation is in progress for that category. The listener will skip duplicate requests while a lock is held.

### 2. Check if webhooks are arriving

Look at the listener's stdout/stderr for log lines like:
```
[INFO] [abc123def456] POST /ci-failure from 127.0.0.1 (1234 bytes)
```

If you see nothing, the webhook isn't reaching the listener. Check:
- Is `WEBHOOK_HOST` set correctly in GitHub Actions secrets/vars?
- Is the self-hosted runner on the same network as the listener?
- Is port 9090 open? (`ss -tlnp | grep 9090`)

### 3. Check HMAC signature failures

```
[WARNING] [abc123] Invalid HMAC signature on /ci-failure from 127.0.0.1
```

This means the `WEBHOOK_SECRET` in `.env` doesn't match the one in GitHub Actions secrets. They must be identical.

For `/sonarqube`, the signature header is `X-Sonar-Webhook-HMAC-SHA256` (raw hex). For all other endpoints, it's `X-Signature-256: sha256=<hex>`.

### 4. Check if kiro-cli is receiving the prompt

```bash
# Check the debug log
tail -100 kiro-debug.log
```

If you see `ERROR: prompt file empty or unreadable`, the prompt file couldn't be read from WSL. Causes:
- Prompt file written to a directory WSL can't access (fixed: now writes to project dir)
- WSL drvfs mount issue — restart WSL: `wsl --shutdown` then retry

If you see `Tool approval required` or `denied list`, kiro-cli needs `--trust-all-tools` (already set) or the agent config is wrong.

### 5. Check if kiro-cli is timing out

```
[WARNING] kiro-cli timed out after 60 minutes
```

The timeout is `KIRO_TIMEOUT = 3600` seconds (1 hour). If kiro-cli hangs, check:
- Is the WSL distro responsive? (`wsl -d Ubuntu-22.04 echo ok`)
- Is kiro-cli waiting for interactive input? (`--no-interactive` should prevent this)

### 6. Test a webhook manually

```bash
# Set your secret
SECRET="bf6ae5a797fcdd69294420054f3bc692957ef9fd63231117cecce1a23e8c0a9b"

# Build payload
PAYLOAD='{"job":"lint","step":"clippy","error_log":"test error","commit":"abc","branch":"main","actor":"test"}'

# Sign it
SIG=$(printf '%s' "$PAYLOAD" | openssl dgst -sha256 -hmac "$SECRET" | awk '{print $NF}')

# Send
curl -v -X POST http://127.0.0.1:9090/ci-failure \
  -H "Content-Type: application/json" \
  -H "X-Signature-256: sha256=$SIG" \
  -d "$PAYLOAD"
```

For SonarQube webhooks (different signature header):
```bash
PAYLOAD='{"project":{"key":"gestion-inmobiliaria"},"qualityGate":{"status":"ERROR","conditions":[{"status":"ERROR","metric":"coverage","value":"40","errorThreshold":"80"}]}}'
SIG=$(printf '%s' "$PAYLOAD" | openssl dgst -sha256 -hmac "$SECRET" | awk '{print $NF}')

curl -v -X POST http://127.0.0.1:9090/sonarqube \
  -H "Content-Type: application/json" \
  -H "X-Sonar-Webhook-HMAC-SHA256: $SIG" \
  -d "$PAYLOAD"
```

### 7. Common failure modes

| Symptom | Cause | Fix |
|---|---|---|
| `Connection refused` on :9090 | Listener not running | Start it |
| `401 Invalid signature` | Secret mismatch | Sync `WEBHOOK_SECRET` between `.env` and GitHub Actions |
| `429 Too many requests` | Rate limit (30 req/min/IP) | Wait 60s or adjust `RateLimiter` |
| `413 Payload too large` | Payload > 512KB | Truncate error logs in CI before sending |
| Lock stuck at `true` | kiro-cli hung or crashed | Kill the stuck process, restart listener |
| `prompt file empty or unreadable` | WSL can't read the file | Check WSL mount: `ls /mnt/d/realestate/` |
| `kiro-cli exited with unexpected code` | kiro-cli crash | Check `kiro-debug.log` for stack trace |

---

## Adding a New Route

### Step 1: Define the handler method

Add a `_handle_<name>` method to `WebhookHandler`. Follow the existing pattern:

```python
def _handle_my_feature(self, payload):
    # 1. Extract and sanitize fields from payload
    some_field = _sanitize_text(payload.get("some_field", ""))
    safe_name = _validate_name(payload.get("name", "unknown"))
    url = _validate_url(payload.get("run_url", ""))

    log.info(f"My feature webhook: name={safe_name}")

    # 2. Build the kiro-cli prompt
    prompt = (
        f"Task description based on {safe_name}.\n\n"
        f"Details:\n{some_field}\n\n"
        "INSTRUCTIONS:\n"
        "1. Do the thing.\n"
        "2. Verify with cargo test.\n"
        "3. Commit and push:\n"
        "   git add -A && git commit -m 'fix: my feature (auto-fix)' && git push origin main\n"
    )

    # 3. Call run_kiro (with retry if needed)
    for attempt in range(1, MAX_RETRIES + 1):
        if run_kiro(prompt, f"My feature attempt {attempt}"):
            return
    log.error("My feature fix failed after all retries.")
```

### Step 2: Register the route

Add the path to the `handlers` dict in `do_POST`:

```python
handlers = {
    "/sonarqube": self._handle_sonarqube,
    "/ci-failure": self._handle_ci_failure,
    "/ci-improve": self._handle_ci_improve,
    "/sonar-fix": self._handle_sonar_fix,
    "/my-feature": self._handle_my_feature,  # ← add here
}
```

### Step 3: Add a concurrency lock (if needed)

If the handler does long-running work and you want to prevent duplicate concurrent runs:

```python
# At module level, next to the other locks
_my_feature_lock = threading.Lock()
```

Then in the handler:
```python
def _handle_my_feature(self, payload):
    if not _my_feature_lock.acquire(blocking=False):
        log.warning("Another my-feature run is in progress -- skipping")
        return
    try:
        # ... handler logic ...
    finally:
        _my_feature_lock.release()
```

Expose the lock in `/health` by adding it to the `locks` dict in `do_GET`:
```python
"locks": {
    "fix": _fix_lock.locked(),
    "sonar_fix": _sonar_fix_lock.locked(),
    "improve": _improve_lock.locked(),
    "my_feature": _my_feature_lock.locked(),
}
```

### Step 4: Add the CI workflow step

In `.github/workflows/ci.yml` (or `android-ci.yml`), add a step that sends the webhook:

```yaml
- name: Notify -- my feature
  if: always() && <condition> && steps.webhook-check.outputs.skip != 'true'
  env:
    SOME_DATA: ${{ steps.previous-step.outputs.data }}
  run: |
    PAYLOAD=$(jq -n \
      --arg some_field "$SOME_DATA" \
      --arg name "my-feature" \
      --arg run_url "${GH_SERVER_URL}/${GH_REPOSITORY}/actions/runs/${GH_RUN_ID}" \
      '{some_field: $some_field, name: $name, run_url: $run_url}')

    SIGNATURE=$(printf '%s' "$PAYLOAD" | openssl dgst -sha256 -hmac "${WEBHOOK_SECRET}" | awk '{print $NF}')
    curl -s --connect-timeout 5 --max-time 10 -X POST http://${WEBHOOK_HOST}:9090/my-feature \
      -H "Content-Type: application/json" \
      -H "X-Signature-256: sha256=$SIGNATURE" \
      -d "$PAYLOAD" || echo "Webhook unreachable -- skipping"
```

### Step 5: Update the docstring and startup log

Add the new endpoint to the module docstring at the top of the file and to the `main()` startup log:

```python
log.info(f"  My feature: http://{local_ip}:{PORT}/my-feature")
```

### Checklist for new routes

- [ ] Handler method sanitizes all input (`_sanitize_text`, `_validate_name`, `_validate_url`)
- [ ] Route registered in `handlers` dict
- [ ] Concurrency lock added if handler is long-running
- [ ] Lock exposed in `/health` response
- [ ] CI workflow step sends signed payload with `jq -n --arg`
- [ ] Payload uses `jq --arg` (not string interpolation) to prevent JSON injection
- [ ] Error logs sanitized for secrets before sending (use `sanitize_payload` function in CI)
- [ ] Module docstring updated
- [ ] Startup log updated
- [ ] Tested manually with `curl` (see "Test a webhook manually" above)

---

## Security Notes

- All POST endpoints require HMAC-SHA256 signature verification
- SonarQube uses `X-Sonar-Webhook-HMAC-SHA256` (raw hex); all others use `X-Signature-256: sha256=<hex>`
- Rate limited to 30 requests/minute per IP
- Max payload size: 512KB
- Max 4 concurrent handler threads (`_thread_semaphore`)
- Listener binds to `127.0.0.1` by default (localhost only)
- Input sanitization strips null bytes and control characters
- Names validated against `^[a-zA-Z0-9_\-]{1,64}$`
- URLs validated against GitHub Actions run URL pattern only
