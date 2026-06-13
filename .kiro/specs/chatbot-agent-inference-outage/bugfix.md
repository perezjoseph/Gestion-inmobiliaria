# Bugfix Requirements Document

## Introduction

The agentic WhatsApp chatbot can no longer generate any replies in production
(`https://gestion.myhomeva.us`). The chatbot configuration page
(`/configuracion/chatbot`) renders correctly, but the core agent capability —
generating a response to a tenant message — is completely non-functional.

When an admin opens the "Probar conversación" (test conversation) panel and sends
a message, the request `POST /api/v1/chatbot/test/stream` returns HTTP 500 and the
UI surfaces a dismissible alert "Error del servidor (código 500)". The agent never
replies. The same failure affects real WhatsApp replies, because both paths depend
on the same inference dependency.

The backend handler `test_chat_stream` (`backend/src/handlers/chatbot.rs`) calls the
inference service through `OvmsCompletionModel`
(`backend/src/services/ai_module/mod.rs`) at
`http://vllm-inference.realestate.svc.cluster.local:8000/v1/chat/completions`.
The backend logs `ProviderError: Error conectando a OVMS stream: error sending
request ...` because the `vllm` pods in the `realestate` production namespace are
all unhealthy.

Two distinct contributing failures cause the inference dependency to be
unavailable, and both must be addressed:

1. **Model unavailability at runtime** — vLLM attempts to download the model
   `Intel/Qwen3-30B-A3B-Instruct-2507-int4-AutoRound` from `huggingface.co` at
   startup. Production has no egress to `huggingface.co`, so the pod fails with
   `requests.exceptions.ConnectionError: ... [Errno 101] Network is unreachable`
   and enters `CrashLoopBackOff`.
2. **GPU device plugin unhealthy** — other vLLM pods are rejected with
   `UnexpectedAdmissionError: Pod was rejected: Allocate failed due to no healthy
   devices present; cannot allocate unhealthy devices gpu.intel.com/xe`. The Intel
   GPU device plugin reports `gpu.intel.com/xe` devices as unhealthy, preventing
   pod admission.

This bugfix restores the vLLM inference dependency to a healthy, reachable state so
that agent reply generation works again, without altering the backend's existing
error-handling behavior.

**Scope note:** This spec (Bug A) is scoped to restoring chatbot reply generation
via vLLM inference availability. The separate `chatbot/status` 500 polling issue
(Bug B) is out of scope. The `ocr-service` pods are also crash-looping; that is
flagged as *related* (it may share the same GPU or model-provisioning root cause)
but remains out of scope unless restoring vLLM provisioning also restores it.

## Bug Analysis

### Current Behavior (Defect)

When a chatbot reply is requested in production, the inference dependency is
unreachable and the request fails.

1.1 WHEN an admin sends a message in the "Probar conversación" panel THEN the system returns HTTP 500 from `POST /api/v1/chatbot/test/stream` and the UI shows "Error del servidor (código 500)" with no agent reply

1.2 WHEN the backend `test_chat_stream` handler calls `OvmsCompletionModel` at `http://vllm-inference.realestate.svc.cluster.local:8000/v1/chat/completions` THEN the request fails with `ProviderError: Error conectando a OVMS stream: error sending request` because no healthy vLLM pod is serving the endpoint

1.3 WHEN a vLLM pod starts in the `realestate` namespace THEN it attempts to download `Intel/Qwen3-30B-A3B-Instruct-2507-int4-AutoRound` from `huggingface.co`, fails with `ConnectionError: ... [Errno 101] Network is unreachable`, and enters `CrashLoopBackOff` because production has no egress to `huggingface.co`

1.4 WHEN the Kubernetes scheduler attempts to admit a vLLM pod requesting `gpu.intel.com/xe` THEN admission is rejected with `UnexpectedAdmissionError: Allocate failed due to no healthy devices present` because the Intel GPU device plugin reports the devices as unhealthy

1.5 WHEN a tenant sends a real WhatsApp message that should be answered by the agent THEN no reply is generated because the same vLLM inference dependency is unavailable

### Expected Behavior (Correct)

For the same conditions, the inference dependency is healthy and reachable, and the
agent generates replies.

2.1 WHEN an admin sends a message in the "Probar conversación" panel THEN the system SHALL return HTTP 200 from `POST /api/v1/chatbot/test/stream` and stream an agent reply for a valid configured organization

2.2 WHEN the backend `test_chat_stream` handler calls `OvmsCompletionModel` at the in-cluster vLLM URL THEN the request SHALL succeed and receive a streamed completion from a healthy vLLM pod

2.3 WHEN a vLLM pod starts in the `realestate` namespace THEN it SHALL obtain the model `Intel/Qwen3-30B-A3B-Instruct-2507-int4-AutoRound` from a pre-provisioned local source (baked into the image, mounted from a PVC/local model cache, or loaded with HuggingFace offline / `local-files-only` mode) WITHOUT requiring egress to `huggingface.co`, and SHALL reach a `Running`/`Ready` state

2.4 WHEN the Kubernetes scheduler attempts to admit a vLLM pod requesting `gpu.intel.com/xe` THEN the Intel GPU device plugin SHALL report healthy devices and the pod SHALL be admitted and scheduled

2.5 WHEN a tenant sends a real WhatsApp message that should be answered by the agent THEN the agent SHALL generate and return a reply through the restored vLLM inference dependency

### Unchanged Behavior (Regression Prevention)

Behavior that does not depend on the inference outage must be preserved exactly.

3.1 WHEN the chatbot configuration page `/configuracion/chatbot` is loaded THEN the system SHALL CONTINUE TO render the configuration UI correctly

3.2 WHEN vLLM is reachable and healthy THEN the system SHALL CONTINUE TO serve all other chatbot endpoints and the `test/stream` flow exactly as before

3.3 WHEN inference genuinely fails (e.g., a real downstream error after the dependency is restored) THEN the backend SHALL CONTINUE TO surface the error gracefully to the UI as a dismissible alert — the existing error-handling path is preserved, not removed

3.4 WHEN the backend resolves chatbot configuration, persona, FAQs, policies, handoff keywords, and guidance rules for an organization THEN the system SHALL CONTINUE TO compose the system prompt and conversation history exactly as before

3.5 WHEN other AI-dependent features that do not depend on vLLM are exercised THEN the system SHALL CONTINUE TO behave as before (this bugfix changes only the vLLM inference availability, not unrelated AI features)

## Bug Condition Methodology

### Definitions

- **F**: The original (unfixed) system — vLLM pods crash-loop or are rejected, the in-cluster inference endpoint is unreachable, and reply generation returns HTTP 500.
- **F'**: The fixed system — vLLM obtains its model from a pre-provisioned local source with healthy GPU devices, the in-cluster endpoint is reachable, and reply generation succeeds.
- **Input X**: A chatbot reply request (test-conversation simulator request or real WhatsApp inbound message) for a valid, configured organization, evaluated against the production cluster state.

### Bug Condition

```pascal
FUNCTION isBugCondition(X)
  INPUT: X of type ChatbotReplyRequest
  OUTPUT: boolean

  // The bug is triggered whenever a reply is requested while the vLLM
  // inference dependency is unavailable: either the model cannot be
  // provisioned without huggingface.co egress, or no GPU device is healthy
  // enough to admit a vLLM pod. In both cases the in-cluster endpoint is
  // unreachable.
  RETURN X.requestsAgentReply
     AND ( vllmModelUnavailableWithoutEgress()
        OR intelGpuDevicesUnhealthy()
        OR NOT vllmEndpointReachable() )
END FUNCTION
```

### Property: Fix Checking

For every reply request that previously triggered the bug, the fixed system must
produce a successful, streamed reply rather than an HTTP 500.

```pascal
// Property: Fix Checking - Restore agent reply generation
FOR ALL X WHERE isBugCondition(X) DO
  result ← processChatbotReply'(X)
  ASSERT result.httpStatus = 200
     AND result.streamsAgentReply = true
     AND vllmPodHealthy() = true
     AND intelGpuDevicesHealthy() = true
     AND modelLoadedWithoutHuggingFaceEgress() = true
END FOR
```

### Property: Preservation Checking

For every input that does not trigger the bug (the inference dependency is already
healthy and reachable), the fixed system must behave identically to the original.

```pascal
// Property: Preservation Checking - No regression when vLLM is healthy
FOR ALL X WHERE NOT isBugCondition(X) DO
  ASSERT F(X) = F'(X)
END FOR
```

This guarantees that restoring the dependency does not change the behavior of
config-page rendering, other chatbot endpoints, prompt/history composition, the
graceful error-surfacing path for genuine inference failures, or unrelated
AI-dependent features.

### Counterexample (demonstrates the bug)

Sending `"¿Cuánto debo este mes?"` via the "Probar conversación" panel as an admin
issues `POST /api/v1/chatbot/test/stream`, which returns HTTP 500
("Error del servidor (código 500)") because every vLLM pod in the `realestate`
namespace is either in `CrashLoopBackOff` (model download from `huggingface.co`
fails — network unreachable) or rejected by admission
(`gpu.intel.com/xe` devices unhealthy).
