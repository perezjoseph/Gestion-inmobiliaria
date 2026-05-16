# WhatsApp AI GPU Fix — Bugfix Design

## Overview

The WhatsApp AI chatbot is non-functional due to a cluster of interrelated issues spanning infrastructure (OVMS GPU migration), model configuration (missing parsers, suboptimal serving params), and application logic (broken agent loop, stub tools, missing runtime context). This design formalizes the bug conditions, identifies root causes, and plans a minimal set of targeted fixes to restore full chatbot functionality with GPU-accelerated inference on the Intel Arc GPU.

## Glossary

- **Bug_Condition (C)**: The set of conditions that cause the chatbot to fail — GPU not utilized, tool calls not parsed/executed, stubs returning errors
- **Property (P)**: The desired behavior — GPU inference, working tool calling with Qwen3.6 parsers, functional agent loop with real tool execution
- **Preservation**: Existing behaviors that must remain unchanged — health probes, webhook auth, final-text responses, capability gating, system prompt composition
- **OVMS**: OpenVINO Model Server — serves the Qwen3.6-35B-A3B model for chat completions
- **graph.pbtxt**: MediaPipe graph configuration for OVMS LLM serving pipeline
- **invoke_agent**: The method in `backend/src/services/ai_module.rs` that builds a Rig `CompletionRequest` and sends it to OVMS
- **OcrClient**: HTTP client in `backend/src/services/ocr_client.rs` that calls the PaddleOCR service
- **IR (Intermediate Representation)**: OpenVINO's optimized model format (`.xml` + `.bin`) that enables full graph optimizations vs raw ONNX

## Bug Details

### Bug Condition

The bug manifests across three layers: (1) infrastructure — OVMS runs on CPU without GPU resources or proper model config, (2) model serving — missing `tool_parser`/`reasoning_parser` and suboptimal LLM parameters, and (3) application — the agent loop collects tool call names but never dispatches them, and tool implementations are stubs or lack runtime context.

**Formal Specification:**
```
FUNCTION isBugCondition(input)
  INPUT: input of type ChatRequest (user message arriving via WhatsApp webhook)
  OUTPUT: boolean

  // Layer 1: Infrastructure — OVMS not using GPU
  infraBroken := deployment.image == "openvino/model_server:latest"
                 AND NOT resources.requests.contains("gpu.intel.com/xe")
                 AND graph.device == "CPU"

  // Layer 2: Model serving — parsers missing, params suboptimal
  parsersMissing := NOT graph.node_options.contains("tool_parser")
                    AND NOT graph.node_options.contains("reasoning_parser")
  paramsSuboptimal := graph.plugin_config == "{}"
                      AND graph.cache_size == 8 (static)

  // Layer 3: Application — agent loop broken
  agentBroken := llmResponse.hasToolCalls()
                 AND NOT toolsActuallyExecuted(llmResponse.tool_calls)

  // Layer 4: Tool stubs
  toolsStubbed := tool.name IN ["extract_receipt", "get_payment_history"]
                  AND tool.call() returns stub/error

  RETURN infraBroken OR parsersMissing OR paramsSuboptimal
         OR agentBroken OR toolsStubbed
END FUNCTION
```

### Examples

- **GPU not utilized**: OVMS pod starts with `openvino/model_server:latest` (CPU-only image), inference runs on CPU at ~2 tokens/sec instead of GPU at ~30 tokens/sec, causing timeouts
- **Tool calls not parsed**: LLM outputs `<tool_call>{"name":"extract_receipt",...}</tool_call>` but OVMS returns raw text instead of structured tool_calls array because `tool_parser: "qwen3coder"` is missing
- **Agent loop broken**: LLM returns tool_calls in response, `invoke_agent` extracts names into `tools_invoked` vec but returns `"Herramientas invocadas: extract_receipt"` as a string instead of executing the tool
- **ExtractReceiptTool stub**: When invoked, returns `Err("OCR service not yet wired")` instead of calling `http://ocr-service:8000/ocr/extract`
- **GetPaymentHistoryTool stub**: Returns empty `PaymentHistoryResult { payments: vec![], total_count: 0 }` instead of querying the database

## Expected Behavior

### Preservation Requirements

**Unchanged Behaviors:**
- Health probes (`/v1/config` on port 8000) must continue to work for OVMS readiness/liveness
- WhatsApp webhook authentication, sender policy enforcement, and conversation history persistence must remain unchanged
- When the LLM returns a final text response (no tool calls), the reply is sent directly without entering a tool execution loop
- Tool definitions are gated by tenant capabilities (`receipt_ocr`, `balance_queries`, `maintenance_requests`, `human_handoff`)
- System prompt composition (persona, tenant context, FAQs, policies, handoff keywords) remains unchanged
- The OVMS provider's handling of missing `id` field in responses continues to work

**Scope:**
All inputs that do NOT trigger tool calls (simple text Q&A, greetings, FAQ lookups) should be completely unaffected by the agent loop fix. Infrastructure changes only affect the OVMS pod scheduling and performance, not the API contract.

## Hypothesized Root Cause

Based on the code analysis, the root causes are:

1. **OVMS Deployment Image & Resources**: The deployment uses `openvino/model_server:latest` (CPU-only) instead of the `-gpu` variant, and lacks `gpu.intel.com/xe: "1"` resource requests. The `OPENVINO_DEVICE=CPU` and `TARGET_DEVICE=CPU` env vars force CPU execution even if GPU were available.

2. **Missing Parsers in graph.pbtxt**: The `LLMCalculatorOptions` in `graph.pbtxt` has no `tool_parser` or `reasoning_parser` fields. Without these, OVMS cannot parse Qwen3.6's tool call format (`<tool_call>...</tool_call>`) or reasoning blocks (`<think>...</think>`) into structured response fields.

3. **Suboptimal LLM Serving Parameters**: `plugin_config: "{}"` means no KV cache compression (u8 precision saves ~4x VRAM). `cache_size: 8` pre-reserves 8GB statically which may cause OOM with the larger Qwen3.6-35B-A3B model on 32GB VRAM.

4. **Agent Loop Never Executes Tools**: In `invoke_agent()` (line ~230), after receiving the `CompletionResponse`, the code iterates `response.choice` and pushes tool call names to `tools_invoked` but never actually calls the tool implementations. It returns a placeholder string `"Herramientas invocadas: ..."` instead of executing tools and feeding results back to the LLM in a multi-turn loop.

5. **Tools Lack Runtime Context**: `ExtractReceiptTool` has a `db` field but no `OcrClient` or image data access. `GetPaymentHistoryTool` has `db` but its `call()` method ignores the args and returns empty results. Neither tool receives `organizacion_id` or `sender_phone` needed for scoped queries.

6. **OCR Service Reads ONNX at Runtime**: `ocr_engine.py` sets `IR_MODEL_FILE = "inference.onnx"` and calls `core.read_model(str(det_path))` on ONNX files. OpenVINO performs on-the-fly ONNX→IR conversion each time, missing graph optimizations available with pre-compiled IR.

## Correctness Properties

Property 1: Bug Condition - GPU Inference and Tool Calling Functional

_For any_ chat request where the OVMS deployment is running with the fixed configuration (GPU image, GPU resources, tool_parser, reasoning_parser, optimized params), the system SHALL serve inference on the Intel Arc GPU with structured tool call parsing, and the agent loop SHALL execute returned tool calls with real implementations, feeding results back to the LLM until a final text response is produced.

**Validates: Requirements 2.1, 2.2, 2.3, 2.5, 2.6, 2.7, 2.8**

Property 2: Preservation - Non-Tool-Call Behavior and Health Probes

_For any_ input where the LLM returns a final text response (no tool calls), or where health probes fire, or where webhook authentication occurs, the fixed system SHALL produce exactly the same behavior as the original system, preserving health check paths, authentication flow, capability gating, and direct text responses.

**Validates: Requirements 3.1, 3.2, 3.3, 3.4, 3.5, 3.6**

## Fix Implementation

### Changes Required

Assuming our root cause analysis is correct:

**File**: `infra/k8s/app/ovms.yml`

**Changes**:
1. **GPU Image**: Change `image: openvino/model_server:latest` to `image: openvino/model_server:latest-gpu`
2. **GPU Resources**: Add `gpu.intel.com/xe: "1"` to resource requests and limits
3. **Remove CPU Env Vars**: Remove `OPENVINO_DEVICE` and `TARGET_DEVICE` environment variables (device is configured in graph.pbtxt)
4. **Update ConfigMap model name**: Change `"Qwen3-30B-A3B"` references to `"Qwen3.6-35B-A3B"` in both `config.json` and `graph.pbtxt` sections

---

**File**: `infra/k8s/app/ovms/graph.pbtxt`

**Changes**:
1. **Device**: Already `"GPU"` — confirm correct
2. **Add tool_parser**: Add `tool_parser: "qwen3coder"` to `LLMCalculatorOptions`
3. **Add reasoning_parser**: Add `reasoning_parser: "qwen3"` to `LLMCalculatorOptions`
4. **KV Cache Compression**: Set `plugin_config: '{"KV_CACHE_PRECISION": "u8", "DYNAMIC_QUANTIZATION_GROUP_SIZE": "32"}'`
5. **Dynamic Cache**: Set `cache_size: 0` (dynamic allocation)
6. **Disable split fuse**: Add `dynamic_split_fuse: false`
7. **Batch tokens**: Set `max_num_batched_tokens: 8192`
8. **Update models_path**: Change to `"/models/Qwen3.6-35B-A3B"`

---

**File**: `infra/k8s/app/ovms/config.json`

**Changes**:
1. **Update model name**: Change `"Qwen3-30B-A3B"` to `"Qwen3.6-35B-A3B"` in both `name` and `base_path`

---

**File**: `backend/src/services/ai_module.rs`

**Function**: `invoke_agent`

**Changes**:
1. **Implement multi-turn agent loop**: Replace the single-shot request/response with a loop (max 5 turns) that:
   - Sends the completion request to OVMS
   - If response contains tool calls: execute each tool, append tool results to chat history, continue loop
   - If response contains final text: return it
   - If turn limit reached: return fallback message
2. **Pass runtime context to tools**: Accept `db`, `organizacion_id`, `sender_phone` in `invoke_agent` and construct tool instances with full context
3. **Wire tool dispatch**: Match tool call names to actual tool implementations and call them with deserialized args

---

**File**: `backend/src/services/ai_module.rs`

**Struct**: `ExtractReceiptTool`

**Changes**:
1. **Wire OcrClient**: In `call()`, construct an `OcrClient`, decode `image_base64`, call `ocr_client.extract()`, and map the `OcrResult` to a `PaymentReceipt`

---

**File**: `backend/src/services/ai_module.rs`

**Struct**: `GetPaymentHistoryTool`

**Changes**:
1. **Wire DB query**: In `call()`, query pagos for the tenant's contracts using SeaORM, map results to `PaymentHistoryEntry` structs
2. **Add runtime context fields**: Add `organizacion_id` and `sender_phone` fields to the struct

---

**File**: `backend/src/config.rs`

**Changes**:
1. **Update default model name**: Change `"qwen3.6"` default to `"Qwen3.6-35B-A3B"`

---

**File**: `ocr-service/convert_models.py`

**Changes**:
1. **Add IR conversion step**: After ONNX export, call `openvino.save_model()` to produce `.xml` + `.bin` IR files

---

**File**: `ocr-service/ocr_engine.py`

**Changes**:
1. **Load IR instead of ONNX**: Change `IR_MODEL_FILE = "inference.onnx"` to `IR_MODEL_FILE = "inference.xml"` so pre-compiled IR is loaded directly

---

**File**: `backend/tests/` (various test fixtures)

**Changes**:
1. **Update model name references**: Change `"qwen3.6"` to `"Qwen3.6-35B-A3B"` in test fixtures

## Testing Strategy

### Validation Approach

The testing strategy follows a two-phase approach: first, surface counterexamples that demonstrate the bugs on unfixed code, then verify the fixes work correctly and preserve existing behavior.

### Exploratory Bug Condition Checking

**Goal**: Surface counterexamples that demonstrate the bugs BEFORE implementing fixes. Confirm or refute the root cause analysis.

**Test Plan**: Write tests that exercise the agent loop with mock LLM responses containing tool calls, and verify that tools are NOT executed on the unfixed code.

**Test Cases**:
1. **Agent Loop No-Dispatch Test**: Send a mock LLM response with tool_calls → verify `invoke_agent` returns placeholder string instead of executing tools (will fail to execute on unfixed code)
2. **ExtractReceiptTool Stub Test**: Call `ExtractReceiptTool::call()` → verify it returns `Err("OCR service not yet wired")` (confirms stub on unfixed code)
3. **GetPaymentHistoryTool Empty Test**: Call `GetPaymentHistoryTool::call()` → verify it returns empty results (confirms stub on unfixed code)
4. **Graph Config Missing Parsers Test**: Parse `graph.pbtxt` → verify `tool_parser` and `reasoning_parser` fields are absent (confirms missing config)

**Expected Counterexamples**:
- Agent loop returns `"Herramientas invocadas: extract_receipt"` instead of executing the tool
- Tool calls are collected but never dispatched to implementations
- Possible causes: single-shot request without loop, no tool instance construction, no result feedback

### Fix Checking

**Goal**: Verify that for all inputs where the bug condition holds, the fixed functions produce the expected behavior.

**Pseudocode:**
```
FOR ALL input WHERE isBugCondition(input) DO
  result := invoke_agent_fixed(input)
  ASSERT result.tools_invoked.len() > 0 IMPLIES tools_were_executed(result)
  ASSERT result.reply != "Herramientas invocadas: ..."
  ASSERT result.extracted_receipt.is_some() WHEN tool == "extract_receipt"
END FOR
```

### Preservation Checking

**Goal**: Verify that for all inputs where the bug condition does NOT hold, the fixed functions produce the same result as the original.

**Pseudocode:**
```
FOR ALL input WHERE NOT isBugCondition(input) DO
  ASSERT invoke_agent_original(input) == invoke_agent_fixed(input)
END FOR
```

**Testing Approach**: Property-based testing is recommended for preservation checking because:
- It generates many random conversation histories and user messages that don't trigger tool calls
- It catches edge cases in system prompt composition and response parsing
- It provides strong guarantees that non-tool-call paths are unchanged

**Test Plan**: Observe behavior on UNFIXED code for simple text responses, then write property-based tests capturing that behavior.

**Test Cases**:
1. **Final Text Preservation**: Verify that when LLM returns text-only responses, the reply is passed through unchanged after fix
2. **Health Probe Preservation**: Verify `/v1/config` continues to work as readiness/liveness endpoint
3. **Capability Gating Preservation**: Verify that disabled capabilities still prevent tool registration
4. **System Prompt Preservation**: Verify `compose_system_prompt` output is identical before and after fix

### Unit Tests

- Test multi-turn agent loop terminates within 5 turns for any sequence of tool calls
- Test `ExtractReceiptTool::call()` successfully calls OcrClient and maps response
- Test `GetPaymentHistoryTool::call()` queries DB and returns actual payment records
- Test tool dispatch correctly matches tool names to implementations
- Test graph.pbtxt contains `tool_parser` and `reasoning_parser` fields

### Property-Based Tests

- Generate random sequences of LLM responses (Final, ToolCalls, errors) and verify the agent loop always terminates within TURN_LIMIT and produces either a final text or fallback message
- Generate random `Capabilities` configurations and verify tool registration matches exactly the enabled capabilities
- Generate random conversation histories without tool calls and verify the response path is unchanged

### Integration Tests

- Test full WhatsApp message flow: webhook → auth → AI module → tool execution → reply
- Test OCR tool end-to-end: image message → ExtractReceiptTool → OcrClient → PaymentReceipt
- Test agent loop with real OVMS responses containing tool calls (requires running OVMS instance)
