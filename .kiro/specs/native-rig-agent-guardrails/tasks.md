# Implementation Plan: Native Rig Agent with PromptHook Guardrails

## Overview

Refactor `AiModule` to use Rig's native `AgentBuilder` with `multi_turn` orchestration, implement `RentalGuardrailHook` for argument validation, side-effect capture, and output safety filtering, add selective tool registration via `ToolDyn`, per-organization `AgentConfig`, tool schema auto-generation via schemars, and an evals subsystem behind a feature flag.

## Tasks

- [ ] 1. Add dependencies and database migration
  - [x] 1.1 Update Cargo.toml with new dependencies
    - Add `schemars = "1"` and `regex = "1"` to `[dependencies]`
    - Add `clap = { version = "4", features = ["derive"] }` to `[dev-dependencies]` or under `[[bin]]` target
    - Add feature flag `evals = ["rig-core/experimental"]`
    - _Requirements: 7.1, 7.5, 8.1_

  - [-] 1.2 Create database migration for agent_config column and eval tables
    - Add `agent_config JSONB NOT NULL DEFAULT '{}'` column to `chatbot_config` table
    - Create `chatbot_eval_suite` table with columns: id (UUID PK), organizacion_id (UUID NOT NULL FK → organizaciones ON DELETE CASCADE), name (VARCHAR(100) NOT NULL), description (TEXT), cases (JSONB NOT NULL DEFAULT '[]'), metrics (JSONB NOT NULL DEFAULT '[]'), created_at (TIMESTAMPTZ NOT NULL), updated_at (TIMESTAMPTZ NOT NULL)
    - Create `chatbot_eval_run` table with columns: id (UUID PK), suite_id (UUID NOT NULL FK → chatbot_eval_suite ON DELETE CASCADE), organizacion_id (UUID NOT NULL FK → organizaciones ON DELETE CASCADE), status (VARCHAR(20) NOT NULL), results (JSONB), summary (JSONB), agent_config_snapshot (JSONB NOT NULL DEFAULT '{}'), started_at (TIMESTAMPTZ NOT NULL), completed_at (TIMESTAMPTZ)
    - Create indexes on `chatbot_eval_suite.organizacion_id`, `chatbot_eval_run.suite_id`, `chatbot_eval_run.organizacion_id`
    - _Requirements: 9.1, 9.2, 9.3, 9.4, 9.5, 9.6, 9.7_

- [ ] 2. Define data models and configuration structs
  - [~] 2.1 Create AgentConfig, GuardrailOverrides, and ToolRegistrationStrategy in `backend/src/models/chatbot.rs`
    - Define `AgentConfig` struct with optional fields: max_turns (u8), temperature (f64), max_tokens (u64), tool_registration (ToolRegistrationStrategy), guardrails (GuardrailOverrides)
    - Define `ToolRegistrationStrategy` enum with variants `Selective` and `AllWithHookGating`
    - Define `GuardrailOverrides` struct with optional fields: max_receipt_amount_dop, max_receipt_amount_usd, blocked_patterns (Vec<String>), output_safety_enabled (bool)
    - Implement `AgentConfig::resolve()` that clamps max_turns to 1–15 (default 5), filters temperature to 0.0–2.0, filters max_tokens to 1–4096
    - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5, 6.6, 6.7_

  - [~] 2.2 Write property test for AgentConfig resolution
    - **Property 7: AgentConfig Resolution Validity**
    - **Validates: Requirements 6.2, 6.3, 6.4, 6.5, 6.6, 6.7**

  - [~] 2.3 Create GuardrailConfig struct in `backend/src/services/ai_module/mod.rs`
    - Define `GuardrailConfig` with fields: max_receipt_amount_dop (Decimal), max_receipt_amount_usd (Decimal), max_description_length (usize), blocked_output_patterns (Vec<Regex>)
    - Implement `Default` with values: RD$100,000, US$5,000, 1000 chars, empty patterns
    - Implement `From<GuardrailOverrides>` to convert org-level overrides into compiled config
    - _Requirements: 3.6, 5.5, 6.8_

- [ ] 3. Implement tool schema auto-generation
  - [~] 3.1 Add `#[derive(JsonSchema)]` to all tool Args types
    - Update `ExtractReceiptInput`, `QueryBalanceInput`, `GetPaymentHistoryInput`, `CreateMaintenanceRequestInput`, `HandoffToHumanInput` to derive `schemars::JsonSchema`
    - Add doc comments on struct fields so they become JSON schema descriptions
    - _Requirements: 7.1, 7.3_

  - [~] 3.2 Update tool `definition()` methods to use `schemars::schema_for!`
    - Replace hand-written JSON schema in each tool's `definition()` with `serde_json::to_value(schemars::schema_for!(Self::Args)).unwrap()`
    - _Requirements: 7.2, 7.5_

  - [~] 3.3 Write property test for tool args round-trip
    - **Property 9: Tool Args Schema Round-Trip**
    - **Validates: Requirements 7.4**

- [~] 4. Checkpoint - Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [ ] 5. Implement selective tool registration
  - [~] 5.1 Create `build_tools()` function in `backend/src/services/ai_module/mod.rs`
    - Implement `build_tools(capabilities, db, organizacion_id, sender_phone, image_base64) -> Vec<Box<dyn ToolDyn>>`
    - Conditionally register ExtractReceiptTool (receipt_ocr), QueryBalanceTool + GetPaymentHistoryTool (balance_queries), CreateMaintenanceRequestTool (maintenance_requests), HandoffToHumanTool (human_handoff)
    - Return empty vector when all capabilities are disabled
    - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7_

  - [~] 5.2 Write property test for selective tool registration completeness
    - **Property 1: Selective Tool Registration Completeness**
    - **Validates: Requirements 2.1, 2.2, 2.3, 2.4, 2.5, 2.6**

- [ ] 6. Implement RentalGuardrailHook
  - [~] 6.1 Create `RentalGuardrailHook` struct implementing `PromptHook`
    - Define struct with fields: captured_receipt (Arc<Mutex<Option<PaymentReceipt>>>), tools_invoked (Arc<Mutex<Vec<String>>>), organizacion_id (Uuid), guardrail_config (GuardrailConfig)
    - Implement `on_completion_call` as no-op returning `HookAction::Continue`
    - _Requirements: 1.1_

  - [~] 6.2 Implement `on_tool_call` for argument validation
    - Validate `create_maintenance_request` description length: skip if < 2 or > max_description_length
    - Return `ToolCallHookAction::Continue` for valid args or deserialization failures
    - Never return `ToolCallHookAction::Terminate`
    - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5_

  - [~] 6.3 Write property test for argument validation bounds
    - **Property 2: Argument Validation Bounds**
    - **Validates: Requirements 3.1, 3.2, 3.3**

  - [~] 6.4 Write property test for argument validation never terminates
    - **Property 3: Argument Validation Never Terminates**
    - **Validates: Requirements 3.4, 3.5**

  - [~] 6.5 Implement `on_tool_result` for side-effect capture
    - Capture `PaymentReceipt` from `extract_receipt` results into shared state
    - Overwrite on multiple successful extractions (last-write-wins)
    - Append tool name to tools_invoked list on every call
    - Always return `HookAction::Continue`
    - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5_

  - [~] 6.6 Write property test for receipt capture consistency
    - **Property 4: Receipt Capture Consistency**
    - **Validates: Requirements 4.1, 4.2**

  - [~] 6.7 Write property test for tools invoked tracking
    - **Property 5: Tools Invoked Tracking**
    - **Validates: Requirements 4.3, 4.4**

  - [~] 6.8 Implement `on_completion_response` for output safety filtering
    - Concatenate all `AssistantContent::Text` items joined with space
    - Check against `blocked_output_patterns` regexes
    - Return `HookAction::Terminate` with reason on match, `HookAction::Continue` otherwise
    - Skip matching entirely if patterns list is empty
    - _Requirements: 5.1, 5.2, 5.4, 5.5_

  - [~] 6.9 Write property test for output safety filter correctness
    - **Property 6: Output Safety Filter Correctness**
    - **Validates: Requirements 5.1, 5.2**

- [~] 7. Checkpoint - Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [ ] 8. Refactor AiModule to use AgentBuilder
  - [~] 8.1 Refactor `process_message()` to build agent via AgentBuilder
    - Compose system prompt, resolve AgentConfig, build shared state for hook
    - Construct `RentalGuardrailHook` with shared Arc<Mutex> state
    - Call `build_tools()` based on `tool_registration` strategy (Selective vs AllWithHookGating)
    - Build agent with `AgentBuilder::new(model).preamble().hook().tools().build()`
    - Apply optional temperature and max_tokens from resolved config
    - _Requirements: 1.1, 2.8, 6.2, 6.3, 6.4, 6.5, 6.6, 6.7_

  - [~] 8.2 Implement multi_turn invocation with timeout
    - Invoke `agent.chat(msg, history).multi_turn(max_turns).send()` wrapped in `tokio::time::timeout`
    - On success: extract reply, tools_invoked, and captured_receipt from shared state
    - On timeout: return fallback AgentResponse with Spanish unavailability message
    - On OVMS error: return `AppError::Internal` and log at error level
    - On empty reply: return `AppError::Internal` indicating unexpected empty response
    - Handle `HookAction::Terminate` from safety filter by returning safe fallback message
    - _Requirements: 1.2, 1.3, 1.4, 1.5, 1.6, 5.3_

  - [~] 8.3 Remove deprecated manual agent loop code
    - Delete `dispatch_tool_call()`, `get_tool_definition()`, `get_enabled_tools()`, `invoke_agent()` functions
    - Remove the manual `for turn in 0..5` loop
    - Clean up unused imports
    - _Requirements: 1.1, 1.2_

- [ ] 9. Wire AgentConfig into chatbot configuration
  - [~] 9.1 Update `ProcessMessageContext` to include `agent_config`
    - Add `agent_config: &AgentConfig` field to `ProcessMessageContext`
    - Update `chatbot_internal` handler to read `agent_config` from chatbot_config entity and pass it through
    - _Requirements: 6.1, 6.9_

  - [~] 9.2 Add AgentConfig validation in chatbot config service
    - Validate `blocked_patterns` are valid regexes (reject with HTTP 400 on invalid)
    - Validate `blocked_patterns` count ≤ 20 (reject with HTTP 400 if exceeded)
    - Implement full replacement semantics for `agent_config` on PUT
    - _Requirements: 6.8, 6.10, 6.11_

  - [~] 9.3 Write property test for blocked pattern regex validation
    - **Property 8: Blocked Pattern Regex Validation**
    - **Validates: Requirements 6.8**

- [~] 10. Checkpoint - Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [ ] 11. Implement evals subsystem (behind feature flag)
  - [~] 11.1 Create eval data models and runner in `backend/src/services/ai_module/evals.rs`
    - Define `EvalCase`, `EvalMetric`, `EvalSuite`, `EvalRun` structs
    - Implement `EvalRunner` that builds agent per case using same AgentBuilder pattern as production
    - Implement `ToolSelectionEval` custom eval checking if expected tool was invoked
    - Implement `build_factuality_judge` using `LlmJudgeMetric`
    - Support concurrency limit (default 3) for parallel case execution
    - Record case failures with score 0.0 and continue remaining cases
    - _Requirements: 8.1, 8.2, 8.4, 8.5, 8.6_

  - [~] 11.2 Create eval API endpoints in `backend/src/handlers/chatbot.rs`
    - GET `/api/v1/chatbot/evals/suites` — list suites filtered by org
    - POST `/api/v1/chatbot/evals/suites` — create eval suite
    - POST `/api/v1/chatbot/evals/run` — run eval suite (validate suite_id belongs to org)
    - GET `/api/v1/chatbot/evals/runs` — list eval runs
    - GET `/api/v1/chatbot/evals/runs/{id}` — get run results with per-case outcomes and summary
    - _Requirements: 8.2, 8.3, 8.8, 8.9_

  - [~] 11.3 Create eval-runner CLI binary in `backend/src/bin/eval_runner.rs`
    - Accept suite_id argument via clap
    - Load suite from DB, build agent, execute all cases
    - Persist results to `chatbot_eval_run` table
    - Print pass rate to stdout
    - _Requirements: 8.7_

  - [~] 11.4 Write integration tests for eval endpoints
    - Test suite creation and listing by org
    - Test run execution with mock agent
    - Test not-found error for invalid suite_id
    - _Requirements: 8.2, 8.3, 8.8_

- [~] 12. Final checkpoint - Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

## Notes

- Tasks marked with `*` are optional and can be skipped for faster MVP
- Each task references specific requirements for traceability
- Checkpoints ensure incremental validation
- Property tests validate universal correctness properties from the design document
- Unit tests validate specific examples and edge cases
- The evals subsystem (task 11) is behind the `evals` feature flag and does not affect production builds
- All code is Rust targeting the `backend/` workspace member
- Migration naming follows project convention: `m{YYYYMMDD}_{SEQ}_{name}.rs`

## Task Dependency Graph

```json
{
  "waves": [
    { "id": 0, "tasks": ["1.1", "1.2"] },
    { "id": 1, "tasks": ["2.1", "2.3", "3.1"] },
    { "id": 2, "tasks": ["2.2", "3.2"] },
    { "id": 3, "tasks": ["3.3", "5.1"] },
    { "id": 4, "tasks": ["5.2", "6.1"] },
    { "id": 5, "tasks": ["6.2", "6.5", "6.8"] },
    { "id": 6, "tasks": ["6.3", "6.4", "6.6", "6.7", "6.9"] },
    { "id": 7, "tasks": ["8.1"] },
    { "id": 8, "tasks": ["8.2", "8.3"] },
    { "id": 9, "tasks": ["9.1", "9.2"] },
    { "id": 10, "tasks": ["9.3", "11.1"] },
    { "id": 11, "tasks": ["11.2", "11.3"] },
    { "id": 12, "tasks": ["11.4"] }
  ]
}
```
