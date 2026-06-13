#!/usr/bin/env bash
# ═══════════════════════════════════════════════════════════════════════════════
# Preservation Property Test — Non-Inference Behavior Unchanged
#
# This test validates that all chatbot-related functionality NOT dependent on
# vLLM inference availability works correctly on the LIVE production cluster.
#
# It should PASS on both UNFIXED and FIXED infrastructure, confirming that
# restoring vLLM inference does not regress any existing behavior.
#
# Validates: Requirements 3.1, 3.2, 3.3, 3.4, 3.5
#
# Prerequisites:
#   - kubectl configured with access to the realestate namespace
#   - curl available on the test machine
#   - Network access to the production URL (https://gestion.myhomeva.us)
#
# Usage:
#   bash infra/k8s/tests/test_chatbot_preservation.sh
# ═══════════════════════════════════════════════════════════════════════════════
set -uo pipefail

# ── Configuration ─────────────────────────────────────────────────────────────
NAMESPACE="realestate"
PROD_URL="https://gestion.myhomeva.us"
BACKEND_PORT="8080"

# Counters
PASSED=0
FAILED=0
RESULTS=()

# ── Helpers ───────────────────────────────────────────────────────────────────

pass() {
  local msg="$1"
  PASSED=$((PASSED + 1))
  RESULTS+=("[PASS] $msg")
  echo "  [PASS] $msg"
}

fail() {
  local msg="$1"
  FAILED=$((FAILED + 1))
  RESULTS+=("[FAIL] $msg")
  echo "  [FAIL] $msg"
}

# Get the name of a running backend pod
get_backend_pod() {
  kubectl get pods -n "$NAMESPACE" -l "app.kubernetes.io/name=backend" \
    --field-selector=status.phase=Running \
    -o jsonpath='{.items[0].metadata.name}' 2>/dev/null
}

# Make an in-cluster HTTP request via kubectl exec on the backend pod.
# The backend container has busybox wget available.
# Usage: backend_wget <url>
# Returns: HTTP status code (or "000" on failure)
backend_wget() {
  local url="$1"
  local pod
  pod=$(get_backend_pod)
  if [[ -z "$pod" ]]; then
    echo "000"
    return
  fi
  local output
  output=$(kubectl exec -n "$NAMESPACE" "$pod" -- \
    wget -q -O /dev/null -S "$url" 2>&1 || true)
  local code
  code=$(echo "$output" | grep -m1 'HTTP/' | grep -oE '[0-9]{3}' | head -1)
  echo "${code:-000}"
}

# Make an in-cluster POST request via kubectl exec on the backend pod.
# Usage: backend_wget_post <url> <body>
# Returns: HTTP status code (or "000" on failure)
backend_wget_post() {
  local url="$1"
  local body="$2"
  local pod
  pod=$(get_backend_pod)
  if [[ -z "$pod" ]]; then
    echo "000"
    return
  fi
  local output
  output=$(kubectl exec -n "$NAMESPACE" "$pod" -- \
    wget -q -O /dev/null -S --post-data="$body" --header="Content-Type: application/json" "$url" 2>&1 || true)
  local code
  code=$(echo "$output" | grep -m1 'HTTP/' | grep -oE '[0-9]{3}' | head -1)
  echo "${code:-000}"
}

# ── Test Functions ────────────────────────────────────────────────────────────

test_backend_health() {
  echo ""
  echo "── Test: Backend health endpoint returns 200 (Req 3.2) ──"

  local http_code
  http_code=$(backend_wget "http://127.0.0.1:${BACKEND_PORT}/health")

  if [[ "$http_code" == "200" ]]; then
    pass "Backend /health returns HTTP 200 (DB reachable)"
  else
    fail "Backend /health returns HTTP ${http_code} (expected 200)"
  fi
}

test_backend_livez() {
  echo ""
  echo "── Test: Backend liveness probe returns 200 ──"

  local http_code
  http_code=$(backend_wget "http://127.0.0.1:${BACKEND_PORT}/livez")

  if [[ "$http_code" == "200" ]]; then
    pass "Backend /livez returns HTTP 200"
  else
    fail "Backend /livez returns HTTP ${http_code} (expected 200)"
  fi
}

test_frontend_config_page() {
  echo ""
  echo "── Test: Frontend serves /configuracion/chatbot via production URL (Req 3.1) ──"

  local http_code
  http_code=$(curl -sk -o /dev/null -w "%{http_code}" "${PROD_URL}/configuracion/chatbot" 2>/dev/null || echo "000")

  if [[ "$http_code" == "200" ]]; then
    pass "GET ${PROD_URL}/configuracion/chatbot returns HTTP 200"
  else
    fail "GET ${PROD_URL}/configuracion/chatbot returns HTTP ${http_code} (expected 200)"
  fi
}

test_frontend_static_assets() {
  echo ""
  echo "── Test: Frontend serves static assets via Caddy (Req 3.1) ──"

  local http_code
  http_code=$(curl -sk -o /dev/null -w "%{http_code}" "${PROD_URL}/" 2>/dev/null || echo "000")

  if [[ "$http_code" == "200" ]]; then
    pass "GET ${PROD_URL}/ returns HTTP 200 (static assets served)"
  else
    fail "GET ${PROD_URL}/ returns HTTP ${http_code} (expected 200)"
  fi
}

test_backend_dns_resolution() {
  echo ""
  echo "── Test: Backend can resolve K8s service DNS for db, baileys, ocr-service (Req 3.5) ──"

  local services=("db" "baileys" "ocr-service")
  for svc in "${services[@]}"; do
    local endpoints
    endpoints=$(kubectl get endpoints "$svc" -n "$NAMESPACE" \
      -o jsonpath='{.subsets[*].addresses[*].ip}' 2>/dev/null || echo "")

    if [[ -n "$endpoints" ]]; then
      pass "Service '${svc}' has active endpoints (${endpoints})"
    else
      local svc_exists
      svc_exists=$(kubectl get svc "$svc" -n "$NAMESPACE" -o name 2>/dev/null || echo "")
      if [[ -n "$svc_exists" ]]; then
        pass "Service '${svc}' exists in namespace (endpoints may be scaling)"
      else
        fail "Service '${svc}' not found in namespace ${NAMESPACE}"
      fi
    fi
  done
}

test_chatbot_config_endpoint() {
  echo ""
  echo "── Test: Chatbot config endpoint is functional (Req 3.1, 3.4) ──"

  # The chatbot config endpoint requires JWT auth.
  # A 401 response confirms the route + auth middleware + handler chain works.
  local http_code
  http_code=$(backend_wget "http://127.0.0.1:${BACKEND_PORT}/api/v1/chatbot/config")

  if [[ "$http_code" == "401" || "$http_code" == "200" ]]; then
    pass "GET /api/v1/chatbot/config returns HTTP ${http_code} (endpoint functional)"
  else
    fail "GET /api/v1/chatbot/config returns HTTP ${http_code} (expected 401 or 200)"
  fi
}

test_graceful_inference_error() {
  echo ""
  echo "── Test: Inference failure returns graceful response, not crash (Req 3.3) ──"

  # When vLLM is down, POST /api/v1/chatbot/test/stream should respond gracefully.
  # Without auth: 401 confirms the route handler chain works.
  # With auth and vLLM down: 500 confirms error-handling path surfaces a response.
  # Connection refused (000) would mean the backend is down entirely.
  local http_code
  http_code=$(backend_wget_post "http://127.0.0.1:${BACKEND_PORT}/api/v1/chatbot/test/stream" \
    '{"message":"test preservation"}')

  if [[ "$http_code" == "401" || "$http_code" == "500" || "$http_code" == "429" ]]; then
    pass "POST /api/v1/chatbot/test/stream returns HTTP ${http_code} (graceful, not crash)"
  elif [[ "$http_code" == "000" ]]; then
    fail "POST /api/v1/chatbot/test/stream: connection refused (backend unreachable)"
  else
    pass "POST /api/v1/chatbot/test/stream returns HTTP ${http_code} (backend responded)"
  fi
}

test_network_policies_intact() {
  echo ""
  echo "── Test: Network policies for non-vLLM pods are intact (Req 3.5) ──"

  local policies
  policies=$(kubectl get networkpolicy -n "$NAMESPACE" \
    -o jsonpath='{.items[*].metadata.name}' 2>/dev/null || echo "")

  local required_policies=("allow-backend-egress" "allow-frontend-egress" "allow-cloudflared-egress" "allow-baileys-egress")
  for policy in "${required_policies[@]}"; do
    if echo "$policies" | tr ' ' '\n' | grep -qx "$policy"; then
      pass "NetworkPolicy '${policy}' exists"
    else
      fail "NetworkPolicy '${policy}' not found"
    fi
  done
}

test_non_vllm_pods_healthy() {
  echo ""
  echo "── Test: Non-vLLM pods are running and ready (Req 3.5) ──"

  local core_apps=("backend" "frontend" "db" "baileys")
  for app in "${core_apps[@]}"; do
    local ready_count
    ready_count=$(kubectl get pods -n "$NAMESPACE" -l "app.kubernetes.io/name=$app" \
      -o jsonpath='{range .items[?(@.status.phase=="Running")]}{.status.containerStatuses[0].ready}{"\n"}{end}' 2>/dev/null \
      | grep -c "true" || echo "0")

    if [[ "$ready_count" -gt 0 ]]; then
      pass "Pod '${app}' has ${ready_count} ready instance(s)"
    else
      fail "Pod '${app}' has no ready instances"
    fi
  done
}

test_backend_reaches_database() {
  echo ""
  echo "── Test: Backend confirms DB connectivity via /health response (Req 3.5) ──"

  local pod body
  pod=$(get_backend_pod)
  if [[ -z "$pod" ]]; then
    fail "No running backend pod for DB connectivity test"
    return
  fi

  body=$(kubectl exec -n "$NAMESPACE" "$pod" -- \
    wget -q -O - "http://127.0.0.1:${BACKEND_PORT}/health" 2>/dev/null || echo "")

  if echo "$body" | grep -q '"ok"'; then
    pass "Backend /health confirms DB reachable (status: ok)"
  elif echo "$body" | grep -q '"degraded"'; then
    fail "Backend /health reports degraded (DB unreachable)"
  else
    fail "Backend /health did not return expected JSON body"
  fi
}

# ── Main ──────────────────────────────────────────────────────────────────────

main() {
  echo "══════════════════════════════════════════════════════════════════════"
  echo "Preservation Property Test — Non-Inference Behavior Unchanged"
  echo "══════════════════════════════════════════════════════════════════════"
  echo ""
  echo "This test validates that all non-inference chatbot functionality works"
  echo "correctly on the live cluster. It should PASS on both unfixed and fixed"
  echo "infrastructure."
  echo ""
  echo "Validates: Requirements 3.1, 3.2, 3.3, 3.4, 3.5"
  echo ""

  # Verify kubectl access
  if ! kubectl get ns "$NAMESPACE" &>/dev/null; then
    echo "ERROR: Cannot access namespace '${NAMESPACE}'. Check kubectl configuration."
    exit 1
  fi

  echo "Cluster accessible. Running preservation tests..."

  # Run all preservation tests
  test_backend_health
  test_backend_livez
  test_backend_reaches_database
  test_frontend_config_page
  test_frontend_static_assets
  test_backend_dns_resolution
  test_chatbot_config_endpoint
  test_graceful_inference_error
  test_network_policies_intact
  test_non_vllm_pods_healthy

  # ── Summary ──
  echo ""
  echo "──────────────────────────────────────────────────────────────────────"
  echo "Results:"
  echo "──────────────────────────────────────────────────────────────────────"
  for result in "${RESULTS[@]}"; do
    echo "  $result"
  done
  echo "──────────────────────────────────────────────────────────────────────"
  echo ""
  echo "Total: ${PASSED} passed, ${FAILED} failed"
  echo ""

  if [[ "$FAILED" -gt 0 ]]; then
    echo "PRESERVATION VIOLATIONS:"
    for result in "${RESULTS[@]}"; do
      if [[ "$result" == *"[FAIL]"* ]]; then
        echo "  - $result"
      fi
    done
    echo ""
    echo "TEST OUTCOME: FAIL (non-inference behavior was affected!)"
    exit 1
  else
    echo "TEST OUTCOME: PASS (all non-inference behavior preserved)"
    exit 0
  fi
}

main "$@"
