#!/usr/bin/env bash
# =============================================================================
# Bug Condition Exploration Test: vLLM Inference Unavailability
# =============================================================================
# Validates: Requirements 1.1, 1.2, 1.3, 1.4, 1.5
#
# This test encodes the EXPECTED BEHAVIOR after the fix is applied:
#   vllmPodHealthy() = true
#   AND intelGpuDevicesHealthy() = true
#   AND modelLoadedWithoutHuggingFaceEgress() = true
#   AND result.httpStatus = 200
#
# On UNFIXED infrastructure, this test is EXPECTED TO FAIL — failure confirms
# the bug exists. After the fix is applied, this test should PASS.
# =============================================================================

set -euo pipefail

NAMESPACE="realestate"
VLLM_LABEL="app.kubernetes.io/name=vllm"
INFERENCE_NODE="inference"
BACKEND_URL="https://gestion.myhomeva.us"
TEST_ENDPOINT="/api/v1/chatbot/test/stream"

PASS=0
FAIL=0
COUNTEREXAMPLES=""

# Utility: record a check result
check() {
  local name="$1"
  local result="$2"  # "pass" or "fail"
  local detail="$3"

  if [[ "$result" == "pass" ]]; then
    PASS=$((PASS + 1))
    echo "[PASS] $name"
  else
    FAIL=$((FAIL + 1))
    echo "[FAIL] $name"
    echo "       Detail: $detail"
    COUNTEREXAMPLES="${COUNTEREXAMPLES}\n- ${name}: ${detail}"
  fi
}

echo "============================================="
echo "Bug Condition Exploration Test"
echo "vLLM Inference Unavailability"
echo "============================================="
echo ""
echo "Running on: $(date -u '+%Y-%m-%dT%H:%M:%SZ')"
echo "Namespace:  $NAMESPACE"
echo "Node:       $INFERENCE_NODE"
echo ""

# ---------------------------------------------------------------------------
# Property Check 1: vLLM pod is Running/Ready
# Expected behavior: at least one pod with status Running and all containers Ready
# ---------------------------------------------------------------------------
echo "--- Check 1: vLLM Pod Running/Ready ---"

POD_OUTPUT=$(kubectl get pods -n "$NAMESPACE" -l "$VLLM_LABEL" --no-headers 2>&1 || true)
echo "$POD_OUTPUT"

if echo "$POD_OUTPUT" | grep -qE '\s+([0-9]+)/\1\s+Running'; then
  check "vLLM pod Running/Ready" "pass" ""
else
  POD_STATUS=$(echo "$POD_OUTPUT" | awk '{print $3}' | head -1)
  check "vLLM pod Running/Ready" "fail" "Pod status: ${POD_STATUS:-no pods found}. Expected: Running with all containers Ready."
fi

echo ""

# ---------------------------------------------------------------------------
# Property Check 2: Intel GPU devices healthy (non-zero allocatable)
# Expected behavior: gpu.intel.com/xe shows non-zero allocatable count
# ---------------------------------------------------------------------------
echo "--- Check 2: Intel GPU Devices Healthy ---"

GPU_OUTPUT=$(kubectl describe node "$INFERENCE_NODE" 2>&1 | grep -A2 "gpu.intel.com/xe" || true)
echo "$GPU_OUTPUT"

GPU_ALLOCATABLE=$(kubectl describe node "$INFERENCE_NODE" 2>&1 | sed -n '/^Allocatable:/,/^System Info:/p' | grep "gpu.intel.com/xe" | awk '{print $2}' || echo "0")

if [[ -n "$GPU_ALLOCATABLE" && "$GPU_ALLOCATABLE" != "0" ]]; then
  check "Intel GPU devices allocatable" "pass" ""
else
  check "Intel GPU devices allocatable" "fail" "gpu.intel.com/xe allocatable: ${GPU_ALLOCATABLE:-not found}. Expected: non-zero (e.g., 2)."
fi

echo ""

# ---------------------------------------------------------------------------
# Property Check 3: vLLM pod logs do NOT contain HuggingFace egress errors
# Expected behavior: no ConnectionError or Network is unreachable to huggingface.co
# ---------------------------------------------------------------------------
echo "--- Check 3: No HuggingFace Egress Errors in Logs ---"

# Get the most recent vLLM pod name (even if CrashLoopBackOff)
VLLM_POD=$(kubectl get pods -n "$NAMESPACE" -l "$VLLM_LABEL" --no-headers -o custom-columns=":metadata.name" 2>/dev/null | head -1 || true)

if [[ -n "$VLLM_POD" ]]; then
  # Check both current and previous container logs
  POD_LOGS=$(kubectl logs -n "$NAMESPACE" "$VLLM_POD" --tail=100 2>&1 || true)
  POD_LOGS_PREV=$(kubectl logs -n "$NAMESPACE" "$VLLM_POD" --previous --tail=100 2>&1 || true)
  ALL_LOGS="${POD_LOGS}\n${POD_LOGS_PREV}"

  if echo -e "$ALL_LOGS" | grep -qiE "(ConnectionError|Network is unreachable).*(huggingface|hf)"; then
    EGRESS_ERROR=$(echo -e "$ALL_LOGS" | grep -iE "(ConnectionError|Network is unreachable).*(huggingface|hf)" | head -3)
    check "No HuggingFace egress errors" "fail" "Found egress error in logs: $EGRESS_ERROR"
  elif echo -e "$ALL_LOGS" | grep -qiE "(ConnectionError|Network is unreachable)"; then
    EGRESS_ERROR=$(echo -e "$ALL_LOGS" | grep -iE "(ConnectionError|Network is unreachable)" | head -3)
    check "No HuggingFace egress errors" "fail" "Found network error in logs (likely huggingface download): $EGRESS_ERROR"
  else
    check "No HuggingFace egress errors" "pass" ""
  fi
else
  check "No HuggingFace egress errors" "fail" "No vLLM pod found to inspect logs."
fi

echo ""

# ---------------------------------------------------------------------------
# Property Check 4: Inference endpoint returns HTTP 200 with streamed reply
# Expected behavior: POST /api/v1/chatbot/test/stream returns 200
# ---------------------------------------------------------------------------
echo "--- Check 4: Inference Endpoint Returns HTTP 200 ---"

# Send a test request to the chatbot test stream endpoint
# Using a simple test message payload
HTTP_STATUS=$(curl -sk -o /dev/null -w "%{http_code}" \
  -X POST "${BACKEND_URL}${TEST_ENDPOINT}" \
  -H "Content-Type: application/json" \
  -d '{"message": "Hola, esta es una prueba de inferencia"}' \
  --max-time 30 2>&1 || echo "000")

echo "HTTP Status: $HTTP_STATUS"

if [[ "$HTTP_STATUS" == "200" ]]; then
  check "Inference endpoint HTTP 200" "pass" ""
else
  check "Inference endpoint HTTP 200" "fail" "HTTP status: $HTTP_STATUS. Expected: 200 with streamed agent reply."
fi

echo ""

# ---------------------------------------------------------------------------
# Summary
# ---------------------------------------------------------------------------
echo "============================================="
echo "RESULTS SUMMARY"
echo "============================================="
echo "Passed: $PASS"
echo "Failed: $FAIL"
echo "Total:  $((PASS + FAIL))"
echo ""

if [[ $FAIL -gt 0 ]]; then
  echo "STATUS: FAIL (bug condition confirmed)"
  echo ""
  echo "Counterexamples found:"
  echo -e "$COUNTEREXAMPLES"
  echo ""
  echo "These failures confirm the bug exists:"
  echo "  - vLLM pods cannot reach Running/Ready state"
  echo "  - GPU devices reported unhealthy (allocatable = 0)"
  echo "  - Model download fails due to blocked egress to huggingface.co"
  echo "  - Inference endpoint unreachable, returns non-200"
  exit 1
else
  echo "STATUS: PASS (expected behavior satisfied)"
  echo ""
  echo "All properties hold:"
  echo "  - vllmPodHealthy() = true"
  echo "  - intelGpuDevicesHealthy() = true"
  echo "  - modelLoadedWithoutHuggingFaceEgress() = true"
  echo "  - result.httpStatus = 200"
  exit 0
fi

# =============================================================================
# COUNTEREXAMPLES (documented from unfixed infrastructure runs):
#
# 1. vLLM pod logs show:
#    requests.exceptions.ConnectionError: HTTPSConnectionPool(host='huggingface.co', ...)
#    [Errno 101] Network is unreachable
#
# 2. kubectl describe pod shows event:
#    UnexpectedAdmissionError: Pod was rejected: Allocate failed due to no healthy
#    devices present; cannot allocate unhealthy devices gpu.intel.com/xe
#
# 3. Node 'inference' reports:
#    gpu.intel.com/xe: 0  (allocatable)
#
# 4. POST /api/v1/chatbot/test/stream returns:
#    HTTP 500 with ProviderError: Error conectando a OVMS stream
# =============================================================================
