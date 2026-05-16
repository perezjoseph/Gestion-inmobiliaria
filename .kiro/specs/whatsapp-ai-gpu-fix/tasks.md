# Implementation Plan

- [x] 1. Write bug condition exploration test
  - **Property 1: Bug Condition** - Agent Loop Does Not Execute Tool Calls
  - **CRITICAL**: This test MUST FAIL on unfixed code - failure confirms the bug exists
  - **DO NOT attempt to fix the test or the code when it fails**
  - **NOTE**: This test encodes the expected behavior - it will validate the fix when it passes after implementation
  - **GOAL**: Surface counterexamples that demonstrate the agent loop bug exists
  - **Scoped PBT Approach**: Scope the property to concrete failing cases - mock LLM responses containing tool_calls and verify tools are actually executed
  - Test that `invoke_agent` with a mock LLM response containing `tool_calls: [{"name": "extract_receipt", ...}]` actually dispatches to the tool implementation
  - Assert expected behavior: when LLM returns tool_calls, the agent loop SHALL execute each tool, collect results, feed them back to the LLM, and continue until a final text response is produced
  - Run test on UNFIXED code
  - **EXPECTED OUTCOME**: Test FAILS (this is correct - it proves the bug exists)
  - Document counterexamples found: agent collects tool names but never dispatches them, returns placeholder string
  - Mark task complete when test is written, run, and failure is documented
  - _Requirements: 1.5, 1.6, 1.7, 1.8_

- [x] 2. Write preservation property tests (BEFORE implementing fix)
  - **Property 2: Preservation** - Non-Tool-Call Responses Pass Through Unchanged
  - **IMPORTANT**: Follow observation-first methodology
  - Observe: when LLM returns a final text response (no tool_calls), `invoke_agent` passes the reply through unchanged on unfixed code
  - Observe: `compose_system_prompt` produces identical output with persona, tenant context, FAQs, policies, handoff keywords on unfixed code
  - Observe: tool definitions are gated by tenant capabilities on unfixed code
  - Write property-based test: for all mock LLM responses that contain NO tool_calls (final text only), the response text is returned directly without entering a tool execution loop
  - Write property-based test: for all `Capabilities` configurations, tool registration matches exactly the enabled capabilities
  - Write property-based test: for all conversation histories without tool calls, `compose_system_prompt` output is identical before and after fix
  - Verify tests pass on UNFIXED code
  - **EXPECTED OUTCOME**: Tests PASS (this confirms baseline behavior to preserve)
  - Mark task complete when tests are written, run, and passing on unfixed code
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 3.6_

- [x] 3. Update OVMS deployment for GPU
  - Change image from `openvino/model_server:latest` to `openvino/model_server:latest-gpu` in `infra/k8s/app/ovms.yml`
  - Add `gpu.intel.com/xe: "1"` to resource requests and limits
  - Remove `OPENVINO_DEVICE` and `TARGET_DEVICE` environment variables (device configured in graph.pbtxt)
  - Update model name references from `"Qwen3-30B-A3B"` to `"Qwen3.6-35B-A3B"`
  - _Requirements: 1.1, 1.9, 2.1, 2.9, 3.1_

- [x] 4. Add tool_parser and reasoning_parser to graph.pbtxt
  - Add `tool_parser: "qwen3coder"` to `LLMCalculatorOptions` in `infra/k8s/app/ovms/graph.pbtxt`
  - Add `reasoning_parser: "qwen3"` to `LLMCalculatorOptions`
  - Update `models_path` to `"/models/Qwen3.6-35B-A3B"`
  - _Requirements: 1.2, 2.2, 3.5_

- [x] 5. Optimize LLM serving parameters
  - Set `plugin_config: '{"KV_CACHE_PRECISION": "u8", "DYNAMIC_QUANTIZATION_GROUP_SIZE": "32"}'` for KV cache compression
  - Set `cache_size: 0` for dynamic allocation
  - Set `dynamic_split_fuse: false`
  - Set `max_num_batched_tokens: 8192`
  - _Requirements: 1.3, 2.3_

- [x] 6. Update OVMS config.json model name
  - Change `"Qwen3-30B-A3B"` to `"Qwen3.6-35B-A3B"` in both `name` and `base_path` fields in `infra/k8s/app/ovms/config.json`
  - _Requirements: 1.9, 2.9_

- [x] 7. Implement multi-turn agent loop in invoke_agent
  - Replace single-shot request/response with a loop (max 5 turns) in `backend/src/services/ai_module.rs`
  - If response contains tool_calls: execute each tool, append tool results to chat history, continue loop
  - If response contains final text: return it
  - If turn limit reached: return fallback message
  - Pass runtime context (`db`, `organizacion_id`, `sender_phone`) to tool instances
  - Wire tool dispatch: match tool call names to actual tool implementations
  - _Requirements: 1.5, 1.8, 2.5, 2.8, 3.3_

- [x] 8. Wire ExtractReceiptTool to OcrClient
  - In `ExtractReceiptTool::call()`, construct an `OcrClient`, decode `image_base64`, call `ocr_client.extract()`
  - Map `OcrResult` to `PaymentReceipt` struct (bank, amount, currency, date, reference, sender_name, recipient, confidence)
  - Add `organizacion_id` and `sender_phone` fields to the struct for scoped context
  - _Requirements: 1.6, 2.6_

- [x] 9. Wire GetPaymentHistoryTool to database
  - In `GetPaymentHistoryTool::call()`, query pagos for the tenant's contracts using SeaORM
  - Map results to `PaymentHistoryEntry` structs
  - Add `organizacion_id` and `sender_phone` fields to the struct for scoped queries
  - _Requirements: 1.7, 2.7_

- [x] 10. Update config default model name
  - Change `"qwen3.6"` default to `"Qwen3.6-35B-A3B"` in `backend/src/config.rs`
  - Update test fixtures referencing old model name
  - _Requirements: 1.9, 2.9_

- [x] 11. Verify bug condition exploration test now passes
  - **Property 1: Expected Behavior** - Agent Loop Executes Tool Calls
  - **IMPORTANT**: Re-run the SAME test from task 1 - do NOT write a new test
  - The test from task 1 encodes the expected behavior (tools are executed, results fed back to LLM)
  - When this test passes, it confirms the agent loop bug is fixed
  - Run bug condition exploration test from step 1
  - **EXPECTED OUTCOME**: Test PASSES (confirms bug is fixed)
  - _Requirements: 2.5, 2.6, 2.7, 2.8_

- [x] 12. Verify preservation tests still pass
  - **Property 2: Preservation** - Non-Tool-Call Responses Pass Through Unchanged
  - **IMPORTANT**: Re-run the SAME tests from task 2 - do NOT write new tests
  - Run preservation property tests from step 2
  - **EXPECTED OUTCOME**: Tests PASS (confirms no regressions)
  - Confirm all preservation tests still pass after fix (no regressions in text responses, health probes, capability gating, system prompt)

- [x] 13. Add IR conversion to convert_models.py
  - After ONNX export, call `openvino.save_model()` to produce `.xml` + `.bin` IR files in `ocr-service/convert_models.py`
  - _Requirements: 1.10, 2.10_

- [x] 14. Update ocr_engine.py to load IR instead of ONNX
  - Change `IR_MODEL_FILE = "inference.onnx"` to `IR_MODEL_FILE = "inference.xml"` in `ocr-service/ocr_engine.py`
  - _Requirements: 1.10, 2.10_

- [x] 15. Checkpoint - Ensure all tests pass
  - Run full test suite: `cargo test` in backend
  - Verify exploration test (Property 1) passes - agent loop executes tools correctly
  - Verify preservation tests (Property 2) pass - non-tool-call behavior unchanged
  - Verify OVMS config files are syntactically valid (graph.pbtxt, config.json)
  - Ensure all tests pass, ask the user if questions arise

## Task Dependency Graph

```json
{
  "waves": [
    ["1", "2"],
    ["3", "4", "5", "6", "7", "8", "9", "10"],
    ["11", "12", "13", "14"],
    ["15"]
  ]
}
```

## Notes

- The OVMS endpoint remains `/v1` (confirmed working). Do not change to `/v3`.
- Tasks 1 and 2 must be completed BEFORE any implementation tasks.
- Tasks 3-10 can be worked on in parallel after tests are written.
- Property-based tests use the `proptest` crate (already in backend dependencies).
