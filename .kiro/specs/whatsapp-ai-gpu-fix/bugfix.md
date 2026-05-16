# Bugfix Requirements Document

## Introduction

The WhatsApp AI chatbot is non-functional due to multiple issues spanning GPU migration, missing tool/reasoning parser configuration, suboptimal LLM serving parameters, endpoint misconfiguration, and an incomplete agent loop. The OVMS deployment uses the CPU-only image without GPU resources, lacks the `tool_parser` and `reasoning_parser` settings required for Qwen3.6 tool calling and thinking mode, uses unoptimized LLM serving config (no KV cache compression, static cache allocation, no GPU-specific low-concurrency tuning), and defaults to the wrong API endpoint path. The model is being upgraded from Qwen3-30B-A3B to Qwen3.6-35B-A3B (exported to OpenVINO IR format with INT4 quantization for the 32GB Intel Arc GPU). The AI agent loop also never executes tool calls returned by the LLM, and tool implementations are stubs lacking runtime context. Together these prevent the chatbot from responding to users or performing any tool-based actions.

## Bug Analysis

### Current Behavior (Defect)

1.1 WHEN the OVMS deployment runs THEN the system uses the CPU-only image (`openvino/model_server:latest`) without GPU resource requests, sets `OPENVINO_DEVICE=CPU` and `TARGET_DEVICE=CPU` env vars, and configures `device: "CPU"` in graph.pbtxt — failing to utilize the Intel Arc GPU on the inference node

1.2 WHEN the OVMS graph.pbtxt LLMCalculatorOptions is configured THEN the system does NOT include a `tool_parser` field NOR a `reasoning_parser` field, so OVMS cannot parse tool calls from Qwen3.6 model output and cannot extract reasoning content — tool calling and thinking mode are broken at the inference server level

1.3 WHEN the OVMS LLM serving runs THEN the system uses an empty `plugin_config: "{}"` (no KV cache compression) and a static `cache_size: 8` (8GB pre-reserved), risking VRAM preemption or OOM on GPU with the Qwen3.6-35B-A3B model

1.4 WHEN the backend sends chat completion requests to OVMS THEN it defaults `OVMS_ENDPOINT` to `/v1` but OVMS 2026.1 chat completions API is at `/v3/chat/completions`

1.5 WHEN the LLM returns tool calls in its response THEN the system collects tool names but does NOT dispatch them to actual tool implementations — it returns a placeholder string instead of executing the tools and feeding results back to the LLM

1.6 WHEN `ExtractReceiptTool::call()` is invoked THEN the system returns `Err("OCR service not yet wired")` instead of calling the existing PaddleOCR service (running on the coreos node's Intel HD Graphics iGPU at `http://ocr-service:8000`) via the `OcrClient` and mapping the response to a `PaymentReceipt`

1.7 WHEN `GetPaymentHistoryTool::call()` is invoked THEN the system returns empty results instead of querying the database

1.8 WHEN tools are instantiated THEN the system only passes `ToolDefinition` schemas to the LLM request without providing runtime context (`db`, `organizacion_id`, `sender_phone`) needed for actual execution

1.9 WHEN the OVMS migration is complete THEN the system has dead code and deprecated artifacts remaining: `OPENVINO_DEVICE` and `TARGET_DEVICE` env vars still present in the deployment YAML, test fixtures hardcoding the `/v1` endpoint instead of `/v3`, the `ovms_chat_model` config value still referencing the old model name "qwen3.6" instead of the actual exported model name, the old Qwen3-30B-A3B model directory still present on the PVC, and dead code paths that reference the old model name "Qwen3-30B-A3B"

1.10 WHEN the OCR service loads PaddleOCR models THEN the system reads ONNX files directly at runtime (`inference.onnx`), causing OpenVINO to perform on-the-fly ONNX-to-IR conversion which offers fewer graph optimizations and lower inference performance on the coreos node's Intel HD Graphics iGPU

### Expected Behavior (Correct)

2.1 WHEN the OVMS deployment runs THEN the system SHALL use the GPU image (`openvino/model_server:latest-gpu`), request `gpu.intel.com/xe: "1"` in K8s resources, remove `OPENVINO_DEVICE` and `TARGET_DEVICE` env vars, and set `device: "GPU"` in graph.pbtxt — running Qwen3.6-35B-A3B inference on the Intel Arc GPU

2.2 WHEN the OVMS graph.pbtxt LLMCalculatorOptions is configured THEN the system SHALL include `tool_parser: "qwen3coder"` AND `reasoning_parser: "qwen3"` so OVMS can parse tool calls and extract reasoning content from Qwen3.6-35B-A3B model output

2.3 WHEN the OVMS LLM serving runs THEN the system SHALL set `plugin_config: '{"KV_CACHE_PRECISION": "u8", "DYNAMIC_QUANTIZATION_GROUP_SIZE": "32"}'` for KV cache compression, set `cache_size: 0` for dynamic allocation, set `dynamic_split_fuse: false`, and set `max_num_batched_tokens: 8192` — reducing VRAM pressure, preventing preemption, and optimizing for GPU with low concurrency (single-user WhatsApp chatbot)

2.4 WHEN the backend sends chat completion requests to OVMS THEN it SHALL default `OVMS_ENDPOINT` to `http://ovms:8000/v3`

2.5 WHEN the LLM returns tool calls in its response THEN the system SHALL execute each tool with the appropriate runtime context, collect results, feed them back to the LLM, and continue the agent loop until the LLM produces a final text response

2.6 WHEN `ExtractReceiptTool::call()` is invoked THEN the system SHALL call the PaddleOCR service at `http://ocr-service:8000` (running on the coreos node's Intel HD Graphics iGPU via OpenVINO) using the existing `OcrClient`, pass the base64-encoded image, and map the OCR extraction response to a `PaymentReceipt` struct with bank, amount, currency, date, reference, sender_name, recipient, and confidence fields

2.7 WHEN `GetPaymentHistoryTool::call()` is invoked THEN the system SHALL query the database for the sender's payment history and return actual results

2.8 WHEN tools are instantiated for execution THEN the system SHALL provide runtime context (`db`, `organizacion_id`, `sender_phone`) so tools can perform real database queries and service calls

2.9 WHEN the OVMS migration is complete THEN the system SHALL remove all dead code and deprecated artifacts: remove `OPENVINO_DEVICE` and `TARGET_DEVICE` env vars from the deployment YAML (GPU device is configured in graph.pbtxt), update all test fixtures that hardcode `ovms_endpoint: "http://ovms:8000/v1"` to use `/v3`, update the `ovms_chat_model` config value from "qwen3.6" to match the actual exported model name (e.g. "Qwen3.6-35B-A3B"), remove the old Qwen3-30B-A3B model directory from the PVC after the new model is deployed and verified, and remove any dead code paths that referenced the old model name

2.10 WHEN the OCR service Docker image is built THEN the system SHALL pre-convert PaddleOCR models from ONNX to OpenVINO IR format (`.xml` + `.bin`) using `openvino.save_model()` during the Docker build step, and the `ocr_engine.py` SHALL load the pre-compiled IR files instead of ONNX — enabling full graph optimizations and faster inference on the Intel HD Graphics iGPU

### Unchanged Behavior (Regression Prevention)

3.1 WHEN the OVMS readiness/liveness probes fire THEN the system SHALL CONTINUE TO use `/v1/config` on port 8000 as the health check path

3.2 WHEN a WhatsApp message arrives via webhook THEN the system SHALL CONTINUE TO authenticate the request, enforce sender policy, and persist conversation history before invoking the AI module

3.3 WHEN the LLM returns a final text response (no tool calls) THEN the system SHALL CONTINUE TO send the reply back to the user via Baileys without entering a tool execution loop

3.4 WHEN tool definitions are registered THEN the system SHALL CONTINUE TO gate them by the tenant's configured capabilities

3.5 WHEN the OVMS provider parses a response THEN the system SHALL CONTINUE TO handle the missing `id` field and parse tool calls correctly

3.6 WHEN the system composes the system prompt THEN the system SHALL CONTINUE TO include persona, tenant context, FAQs, policies, and handoff keywords as before
