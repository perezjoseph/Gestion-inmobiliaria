# Implementation Plan: Agent Guidance Redesign

## Overview

Replace the monolithic `system_prompt` freetext with structured, categorized guidance rules. Hide `AgentConfig` from users. Update frontend to show toggleable rules grouped by category.

## Tasks

- [x] 1. Database migration and data model
  - [x] 1.1 Create migration `m20260607_000001_add_guidance_rules_to_chatbot_config.rs`
    - Add column `guidance_rules` (JSONB NOT NULL DEFAULT '[]') to `chatbot_config` table
    - _Requirements: 1.1, 1.3_

  - [x] 1.2 Create migration `m20260607_000002_seed_guidance_rule_templates.rs`
    - For each existing chatbot_config row, populate `guidance_rules` with the 16 predefined template rules (all enabled)
    - If `system_prompt` is non-empty, append a custom rule in `politicas` category with the existing text (truncated to 500 chars)
    - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 8.1, 8.2_

  - [x] 1.3 Update the `chatbot_config` entity to include the `guidance_rules` field
    - Add `guidance_rules: serde_json::Value` to the SeaORM entity
    - _Requirements: 1.1, 1.3_

- [x] 2. Backend DTOs and types
  - [x] 2.1 Add guidance types to `models/chatbot.rs`
    - Define `GuidanceRule` struct (id, category, instruction, enabled, is_template, sort_order, created_at, updated_at)
    - Define `GuidanceCategory` enum (EstiloComunicacion, ContextoClarificacion, Escalamiento, Politicas)
    - Define `CreateGuidanceRuleRequest`, `UpdateGuidanceRuleRequest`, `BatchUpdateItem`, `BatchUpdateRequest`
    - Define `GuidanceRuleResponse` (same as GuidanceRule, serialized with camelCase)
    - _Requirements: 1.1, 1.2, 4.3, 4.4, 4.5, 4.6_

  - [x] 2.2 Update `ChatbotConfigResponse` and `ChatbotConfigUpdateRequest`
    - Add `guidance_rules: Vec<GuidanceRuleResponse>` to response
    - Remove `agent_config` from response
    - Remove `system_prompt` and `agent_config` from update request
    - _Requirements: 4.1, 4.2, 6.1, 6.4_

- [x] 3. Backend service logic
  - [x] 3.1 Implement guidance rule CRUD in `services/chatbot.rs`
    - `create_guidance_rule`: validate instruction (non-empty, max 500 chars), validate active count < 30, assign UUID and timestamps, append to JSONB array
    - `update_guidance_rule`: find by ID, validate template constraints (cannot change instruction on templates), validate active count on enable, update fields
    - `delete_guidance_rule`: find by ID, reject if `is_template`, remove from JSONB array
    - `batch_update_guidance_rules`: validate each item, apply enabled/sort_order changes in one write
    - _Requirements: 1.4, 1.5, 1.6, 3.1, 3.2, 3.3, 3.4, 4.3, 4.4, 4.5, 4.6_

  - [x] 3.2 Implement `seed_template_rules` function
    - Returns the default Vec of 16 template GuidanceRule structs
    - Called during org chatbot config creation (existing `create_chatbot_config` path)
    - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 2.6_

  - [x] 3.3 Update `compose_system_prompt` in `services/ai_module/mod.rs`
    - Accept `&[GuidanceRule]` instead of `system_prompt: Option<&str>`
    - Filter to enabled rules, group by category, format into prompt sections
    - Omit category sections with no enabled rules
    - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5, 5.6_

  - [x] 3.4 Update `process_message` in AI module
    - Parse `guidance_rules` from chatbot config model into `Vec<GuidanceRule>`
    - Pass enabled rules to updated `compose_system_prompt`
    - Remove `agent_config` from `ProcessMessageContext` (use hardcoded defaults internally)
    - _Requirements: 5.1, 6.2, 6.3_

  - [x] 3.5 Update chatbot config service functions
    - `get_chatbot_config`: deserialize guidance_rules from JSONB, include in response
    - `create_chatbot_config`: call `seed_template_rules` to populate initial rules
    - `update_chatbot_config`: stop reading/writing `system_prompt` and `agent_config` from user input
    - _Requirements: 4.1, 4.2, 6.1, 6.4, 8.3, 8.4_

- [x] 4. Backend handlers and routes
  - [x] 4.1 Add handler functions in `handlers/chatbot.rs`
    - `create_guidance_rule_handler`: POST, requires WriteAccess, validates input, calls service
    - `update_guidance_rule_handler`: PUT with path param, requires WriteAccess
    - `delete_guidance_rule_handler`: DELETE with path param, requires WriteAccess
    - `batch_update_guidance_rules_handler`: PUT, requires WriteAccess
    - _Requirements: 4.3, 4.4, 4.5, 4.6, 4.7_

  - [x] 4.2 Register new routes in `routes.rs`
    - Add route scope under `/api/v1/chatbot/guidance-rules`
    - _Requirements: 4.3, 4.4, 4.5, 4.6_

- [x] 5. Checkpoint — Backend compiles and tests pass
  - Ensure all existing chatbot tests still pass with updated DTOs.
  - Verify compose_system_prompt produces correct output with sample guidance rules.

- [x] 6. Frontend types and API
  - [x] 6.1 Add guidance rule types to `types/chatbot.rs`
    - `GuidanceRule` struct with all fields
    - `GuidanceCategory` enum
    - `CreateGuidanceRuleRequest`, `UpdateGuidanceRuleRequest`, `BatchUpdateRequest`
    - _Requirements: 7.1_

  - [x] 6.2 Add API functions to `services/chatbot.rs`
    - `create_guidance_rule`, `update_guidance_rule`, `delete_guidance_rule`, `batch_update_guidance_rules`
    - _Requirements: 7.1_

  - [x] 6.3 Update `ChatbotConfigResponse` type
    - Add `guidance_rules: Vec<GuidanceRule>` field
    - Remove `system_prompt` and `agent_config` fields
    - _Requirements: 7.1_

- [x] 7. Frontend component
  - [x] 7.1 Create `components/chatbot/guidance_rules_step.rs`
    - Main component with collapsible category sections
    - Each rule row: toggle switch, instruction text, lock icon (templates), edit/delete buttons (custom)
    - Active count display ("X/30 reglas activas")
    - "Agregar regla" button per category
    - Inline form for creating/editing custom rules (textarea, max 500 chars)
    - Auto-save on toggle changes (debounced via parent save_config callback)
    - _Requirements: 7.1, 7.2, 7.3, 7.4, 7.5, 7.6, 7.7, 7.8_

  - [x] 7.2 Update `pages/chatbot_config.rs`
    - Import and render `GuidanceRulesStep` as a new section
    - Pass guidance_rules from config state and save callback
    - _Requirements: 7.1_

  - [x] 7.3 Update `components/chatbot/persona_step.rs`
    - Remove the "Instrucciones adicionales" textarea (system_prompt field)
    - Remove system_prompt from `PersonaUpdate` struct
    - _Requirements: 7.1_

  - [x] 7.4 Register component in `components/chatbot/mod.rs`
    - Add `pub mod guidance_rules_step;`
    - _Requirements: 7.1_

- [~] 8. Checkpoint — Frontend compiles and renders correctly
  - Verify the guidance rules section displays rules grouped by category.
  - Verify toggle changes persist via API.
  - Verify custom rule creation and deletion works.

- [x] 9. Audit and non-functional
  - [x] 9.1 Add audit logging for guidance rule mutations
    - Log create, update, delete operations through existing `registros_auditoria` system
    - _Requirements: 9.1_

  - [x] 9.2 Add accessibility attributes to GuidanceRulesStep
    - ARIA labels on toggles, keyboard navigation for expand/collapse, role attributes on rule list
    - _Requirements: 9.3_

- [~] 10. Final checkpoint — Full build passes
  - Backend: cargo fmt, clippy, tests pass.
  - Frontend: cargo fmt, clippy pass.
  - Guidance rules display, toggle, create, delete all working end-to-end.

## Notes

- The `system_prompt` column is NOT dropped — it's deprecated and retained for rollback safety. A future migration will remove it.
- The `agent_config` JSONB column remains for internal use but is invisible to the API.
- Template rules are seeded once per org. If new templates are added in the future, a separate migration seeds them.
- The 30-rule limit is generous for property management. Industry standard (Intercom) allows 100 but those are enterprise-scale. 30 is appropriate for this domain.
- All UI text is in Spanish per project localization rules.

## Task Dependency Graph

```json
{
  "waves": [
    { "id": 0, "tasks": ["1.1", "1.2", "1.3"] },
    { "id": 1, "tasks": ["2.1", "2.2"] },
    { "id": 2, "tasks": ["3.1", "3.2", "3.3", "3.4", "3.5"] },
    { "id": 3, "tasks": ["4.1", "4.2"] },
    { "id": 4, "tasks": ["6.1", "6.2", "6.3"] },
    { "id": 5, "tasks": ["7.1", "7.2", "7.3", "7.4"] },
    { "id": 6, "tasks": ["9.1", "9.2"] }
  ]
}
```
