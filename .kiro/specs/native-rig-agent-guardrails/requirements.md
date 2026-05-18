# Requirements Document

## Introduction

This document defines the requirements for refactoring the `AiModule` to use Rig's native `AgentBuilder` pattern with `PromptHook` guardrails. The refactoring replaces the manual multi-turn agent loop with Rig's built-in orchestration, introduces a centralized guardrail hook for argument validation, side-effect capture, and output safety filtering, and adds per-organization agent configuration and an evals subsystem.

## Glossary

- **AiModule**: The stateless service that builds a Rig agent per invocation and returns the agent response.
- **AgentBuilder**: Rig's native builder pattern for constructing agents with tools, hooks, and configuration.
- **RentalGuardrailHook**: The `PromptHook` implementation that intercepts the agent loop for argument validation, side-effect capture, and output safety filtering.
- **PromptHook**: Rig's trait for intercepting agent execution at four points: before completion, after completion, before tool execution, and after tool execution.
- **ToolDyn**: Rig's dynamic tool trait enabling runtime-conditional tool registration via `Vec<Box<dyn ToolDyn>>`.
- **Capabilities**: A per-organization struct indicating which tools are enabled (receipt_ocr, balance_queries, maintenance_requests, human_handoff).
- **AgentConfig**: Per-organization configuration for agent behavior (max_turns, temperature, max_tokens, guardrail overrides) stored in `chatbot_config.agent_config` JSONB.
- **GuardrailConfig**: Configuration struct holding validation limits and blocked output patterns for the guardrail hook.
- **PaymentReceipt**: Structured data extracted from a receipt image by the `extract_receipt` tool.
- **AgentResponse**: The response struct containing reply text, tools invoked, and optionally an extracted receipt.
- **OVMS**: OpenVINO Model Server — the LLM inference backend.
- **EvalSuite**: A collection of test cases with associated metrics for evaluating agent quality.
- **EvalRun**: A single execution of an eval suite producing scored results.

## Requirements

### Requirement 1: Native Agent Orchestration

**User Story:** As a developer, I want the AiModule to use Rig's native AgentBuilder with multi_turn orchestration, so that tool dispatch and conversation looping are handled by the framework rather than manual code.

#### Acceptance Criteria

1. WHEN a user message is received, THE AiModule SHALL build a Rig agent using AgentBuilder with the composed system prompt, registered tools, and guardrail hook
2. WHEN the agent is built, THE AiModule SHALL invoke the agent using `.chat(msg, history).multi_turn(max_turns).send()` where max_turns is resolved from AgentConfig, wrapped in a `tokio::time::timeout` of `timeout_secs` seconds (default: 30, valid range: 5–120, configured via environment)
3. WHEN the agent completes successfully with a non-empty reply (at least 1 character of non-whitespace text), THE AiModule SHALL return an AgentResponse containing the reply text, the list of tool names that were executed during the multi-turn loop, and the extracted receipt if the extract_receipt tool produced a valid PaymentReceipt
4. WHEN the agent invocation exceeds the configured timeout, THE AiModule SHALL return a fallback AgentResponse with a Spanish-language message indicating temporary service unavailability, an empty tools_invoked list, no extracted receipt, and no error propagation to the caller
5. IF the OVMS endpoint returns an error, THEN THE AiModule SHALL return an AppError::Internal and log the error details including the OVMS response status via tracing at error level
6. IF the agent completes but returns an empty or whitespace-only reply, THEN THE AiModule SHALL return an AppError::Internal indicating an unexpected empty response from the model

### Requirement 2: Selective Tool Registration

**User Story:** As a platform operator, I want tools to be registered on the agent only when the organization has the corresponding capability enabled, so that the LLM never sees tool definitions for disabled features and no tokens are wasted.

#### Acceptance Criteria

1. WHEN building the agent, THE AiModule SHALL register only tools whose corresponding capability flag is enabled in the organization's Capabilities struct, such that the LLM prompt contains no tool definitions for disabled capabilities
2. WHEN receipt_ocr capability is enabled, THE AiModule SHALL register the ExtractReceiptTool
3. WHEN balance_queries capability is enabled, THE AiModule SHALL register QueryBalanceTool and GetPaymentHistoryTool
4. WHEN maintenance_requests capability is enabled, THE AiModule SHALL register CreateMaintenanceRequestTool
5. WHEN human_handoff capability is enabled, THE AiModule SHALL register HandoffToHumanTool
6. WHEN a capability is disabled, THE AiModule SHALL exclude the corresponding tool from the agent's tool vector so that the tool count in the vector equals the sum of tools mapped to enabled capabilities
7. IF all capability flags are disabled, THEN THE AiModule SHALL build the agent with an empty tool vector and the agent SHALL still return a valid text response without error
8. IF AgentConfig.tool_registration is set to AllWithHookGating, THEN THE AiModule SHALL register all tools regardless of capability flags and delegate gating to the RentalGuardrailHook

### Requirement 3: Argument Validation in Guardrail Hook

**User Story:** As a platform operator, I want the guardrail hook to validate tool arguments before execution, so that malformed or oversized payloads are rejected before reaching backend services.

#### Acceptance Criteria

1. WHEN the LLM calls create_maintenance_request with a description shorter than 2 characters (measured by UTF-8 byte length of the string value), THE RentalGuardrailHook SHALL return ToolCallHookAction::Skip with a reason indicating the description is below the minimum length
2. WHEN the LLM calls create_maintenance_request with a description longer than the configured max_description_length (default 1000 characters, measured by UTF-8 byte length), THE RentalGuardrailHook SHALL return ToolCallHookAction::Skip with a reason indicating the description exceeds the maximum length
3. WHEN the LLM calls create_maintenance_request with a description whose UTF-8 byte length is between 2 and max_description_length inclusive, THE RentalGuardrailHook SHALL return ToolCallHookAction::Continue
4. WHEN the LLM provides tool arguments that fail to deserialize into the expected Args struct (malformed JSON or missing required fields), THE RentalGuardrailHook SHALL return ToolCallHookAction::Continue to let Rig's native deserialization produce the error
5. THE RentalGuardrailHook SHALL never return ToolCallHookAction::Terminate for argument validation failures
6. THE GuardrailConfig SHALL expose a max_description_length field (default: 1000) that controls the upper bound used in description length validation

### Requirement 4: Side-Effect Capture

**User Story:** As a developer, I want the guardrail hook to capture structured side-effects from tool results, so that extracted data (like payment receipts) is available in the final AgentResponse without modifying tool implementations.

#### Acceptance Criteria

1. WHEN the extract_receipt tool returns a result that successfully deserializes to a valid PaymentReceipt via serde_json::from_str, THE RentalGuardrailHook SHALL store the receipt in its shared captured_receipt state
2. WHEN the extract_receipt tool returns a result that does not deserialize to a valid PaymentReceipt, THE RentalGuardrailHook SHALL leave captured_receipt unchanged (retaining any previously captured value)
3. IF the extract_receipt tool is invoked multiple times in a single multi-turn session, THEN THE RentalGuardrailHook SHALL overwrite captured_receipt with the most recent successfully deserialized PaymentReceipt
4. WHEN any tool result is received, THE RentalGuardrailHook SHALL append the tool name to the tools_invoked list in the order results are received
5. THE RentalGuardrailHook on_tool_result method SHALL always return HookAction::Continue regardless of capture success or failure

### Requirement 5: Output Safety Filtering

**User Story:** As a platform operator, I want the guardrail hook to check agent responses against configurable blocked patterns, so that sensitive data or harmful content is never sent to end users.

#### Acceptance Criteria

1. WHEN the agent produces a response whose concatenated text content (all AssistantContent::Text items joined with a space) matches any compiled Regex in blocked_output_patterns, THE RentalGuardrailHook SHALL return HookAction::Terminate with a reason string "Response blocked by safety filter"
2. WHEN the agent produces a response whose concatenated text content does not match any blocked pattern, THE RentalGuardrailHook SHALL return HookAction::Continue
3. WHEN HookAction::Terminate is returned by the output safety check, THE AiModule SHALL return an AgentResponse with a safe fallback message in Spanish indicating the response was filtered, and SHALL log the event at warn level with the organizacion_id and matched pattern
4. THE RentalGuardrailHook output safety check SHALL perform read-only inspection without mutating the response content
5. IF blocked_output_patterns is empty, THEN THE RentalGuardrailHook SHALL always return HookAction::Continue from on_completion_response without performing any regex matching

### Requirement 6: Per-Organization Agent Configuration

**User Story:** As a platform operator, I want to customize agent behavior per organization (multi-turn depth, temperature, guardrail thresholds), so that different organizations can have tailored agent experiences.

#### Acceptance Criteria

1. THE AgentConfig SHALL be stored in the chatbot_config table as a JSONB column named agent_config with a default value of '{}'
2. IF AgentConfig.max_turns is provided, THEN THE AiModule SHALL use the value clamped to the range 1–15 (inclusive) as the multi_turn depth
3. IF AgentConfig.max_turns is absent, THEN THE AiModule SHALL default to 5 turns
4. IF AgentConfig.temperature is provided and within range 0.0–2.0 (inclusive), THEN THE AiModule SHALL pass it to the AgentBuilder
5. IF AgentConfig.temperature is absent or outside the range 0.0–2.0, THEN THE AiModule SHALL omit temperature from the AgentBuilder (use model default)
6. IF AgentConfig.max_tokens is provided and within range 1–4096 (inclusive), THEN THE AiModule SHALL pass it to the AgentBuilder
7. IF AgentConfig.max_tokens is absent or outside the range 1–4096, THEN THE AiModule SHALL omit max_tokens from the AgentBuilder
8. IF AgentConfig.guardrails.blocked_patterns contains an invalid regex, THEN THE chatbot configuration service SHALL reject the update with an HTTP 400 response and an error message indicating which pattern failed compilation
9. THE AgentConfig SHALL be updatable via the existing PUT /api/v1/chatbot/config endpoint
10. WHEN the PUT /api/v1/chatbot/config endpoint receives an agent_config payload, THE chatbot configuration service SHALL replace the entire agent_config JSONB value (full replacement, not field-level merge)
11. IF AgentConfig.guardrails.blocked_patterns contains more than 20 entries, THEN THE chatbot configuration service SHALL reject the update with an HTTP 400 response and an error message indicating the maximum allowed count

### Requirement 7: Tool Schema Auto-Generation

**User Story:** As a developer, I want tool argument schemas to be auto-generated from Rust type definitions, so that schemas stay in sync with code and manual JSON schema maintenance is eliminated.

#### Acceptance Criteria

1. THE tool Args types SHALL derive schemars::JsonSchema for automatic schema generation
2. WHEN a tool's definition() method is called, THE tool SHALL use schemars::schema_for! to produce the JSON schema instead of hand-written JSON
3. THE generated schema SHALL include field descriptions derived from doc comments on the Args struct fields; fields without doc comments SHALL have no description in the generated schema
4. FOR ALL tool Args types, serializing a valid instance to JSON via serde_json::to_value and then deserializing back via serde_json::from_value SHALL produce a value equal to the original (round-trip compatibility)
5. THE generated JSON schema SHALL conform to JSON Schema Draft 2020-12 as produced by schemars v1

### Requirement 8: Evals Subsystem

**User Story:** As a platform operator, I want to run quality evaluations against the agent using predefined test suites, so that I can measure and track agent performance across changes.

#### Acceptance Criteria

1. THE Evals subsystem SHALL be compiled only when the `evals` feature flag is enabled
2. WHEN a POST request is made to /api/v1/chatbot/evals/run with a suite_id that exists and belongs to the requesting organization, THE system SHALL execute all cases in the suite against the agent with a default concurrency of 3, set the run status to "running", and store results upon completion with status "completed"
3. IF a POST request to /api/v1/chatbot/evals/run references a suite_id that does not exist or does not belong to the requesting organization, THEN THE system SHALL reject the request with a not-found error
4. WHEN an eval run completes, THE system SHALL persist results including per-case scores (each a decimal value from 0.0 to 1.0), reasoning text, and a summary containing pass_rate (0.0 to 1.0) and average_score (0.0 to 1.0)
5. IF an individual eval case fails due to agent error or timeout, THEN THE system SHALL record that case with a score of 0.0 and reasoning indicating the failure, and continue executing remaining cases
6. WHEN an eval case is executed, THE system SHALL build the agent using the same AgentBuilder pattern as production including selective tool registration and guardrail hook, applying the optional agent_config_override from the run request if provided
7. THE eval-runner CLI binary SHALL accept a suite_id argument, execute all cases in the suite, persist results to the chatbot_eval_run table, and print the pass rate to stdout
8. WHEN listing eval suites via GET /api/v1/chatbot/evals/suites, THE system SHALL return only suites whose organizacion_id matches the requesting user's organization
9. WHEN retrieving eval run results via GET /api/v1/chatbot/evals/runs/{id}, THE system SHALL return the run status, per-case outcomes (score, reasoning), and summary metrics (pass_rate, average_score, by_metric breakdown)

### Requirement 9: Database Schema Changes

**User Story:** As a developer, I want the database schema to support agent configuration and eval data, so that per-organization settings and evaluation history are persisted.

#### Acceptance Criteria

1. THE migration SHALL add an agent_config JSONB column with NOT NULL default '{}' to the chatbot_config table
2. THE migration SHALL create a chatbot_eval_suite table with columns: id (UUID PK), organizacion_id (UUID NOT NULL), name (VARCHAR(100) NOT NULL), description (TEXT), cases (JSONB NOT NULL DEFAULT '[]'), metrics (JSONB NOT NULL DEFAULT '[]'), created_at (TIMESTAMPTZ NOT NULL), and updated_at (TIMESTAMPTZ NOT NULL)
3. THE migration SHALL create a chatbot_eval_run table with columns: id (UUID PK), suite_id (UUID NOT NULL), organizacion_id (UUID NOT NULL), status (VARCHAR(20) NOT NULL), results (JSONB), summary (JSONB), agent_config_snapshot (JSONB NOT NULL DEFAULT '{}'), started_at (TIMESTAMPTZ NOT NULL), and completed_at (TIMESTAMPTZ)
4. THE chatbot_eval_suite.organizacion_id SHALL reference the organizaciones table as a foreign key with ON DELETE CASCADE
5. THE chatbot_eval_run.suite_id SHALL reference the chatbot_eval_suite table as a foreign key with ON DELETE CASCADE
6. THE chatbot_eval_run.organizacion_id SHALL reference the organizaciones table as a foreign key with ON DELETE CASCADE
7. THE migration SHALL create indexes on chatbot_eval_suite.organizacion_id, chatbot_eval_run.suite_id, and chatbot_eval_run.organizacion_id
