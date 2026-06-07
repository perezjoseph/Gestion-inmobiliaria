# Requirements Document

## Introduction

Replace the freetext `system_prompt` field in chatbot configuration with a structured guidance rules system. Property managers configure agent behavior through categorized, individually toggleable natural-language rules instead of writing raw prompts. Technical parameters (`AgentConfig`) are hidden from users entirely.

Industry standard (Intercom Fin, Zendesk AI Agents, Freshdesk Freddy) has converged on structured guidance rules over monolithic system prompts. Users don't write prompts — they enable predefined rules and optionally write custom ones in natural language. This reduces configuration complexity, enables per-rule analytics, and prevents users from breaking agent behavior with malformed prompts.

## Glossary

- **Guidance Rule**: A single, natural-language behavioral instruction for the WhatsApp chatbot agent.
- **Template Rule**: A predefined rule seeded by the system that can be toggled but not deleted.
- **Custom Rule**: A user-created rule that can be fully managed (created, edited, deleted).
- **AgentConfig**: Internal backend struct controlling LLM parameters (temperature, max_turns). Not user-facing.
- **Prompt Assembly**: The process of composing the final system prompt from persona, FAQs, policies, and enabled guidance rules.

## Requirements

### 1. Guidance Rule Data Model

1.1 Each guidance rule has: id (UUID), category, instruction text (max 500 chars), enabled flag, sort order, and timestamps.

1.2 Categories are: `estilo_comunicacion` (tone, terminology), `contexto_clarificacion` (when to ask follow-ups), `escalamiento` (when to hand off to humans), `politicas` (business policies and restrictions).

1.3 Rules are stored per organization in a JSONB column on `chatbot_config` (field: `guidance_rules`), replacing the existing `system_prompt` field.

1.4 Maximum 30 active (enabled) rules per organization. Attempts to enable a 31st rule return a 422 error.

1.5 Each rule instruction is max 500 characters. Reject longer instructions at the service layer.

1.6 Rules have a `is_template` boolean flag. Template rules are seeded on first chatbot config creation and can be toggled but not deleted by users. Custom rules can be deleted.

### 2. Predefined Templates

2.1 The system provides predefined template rules for Dominican Republic property management covering all four categories.

2.2 Templates for `estilo_comunicacion`: formal address (usted), currency formatting (RD$/US$), short message length, Spanish language enforcement.

2.3 Templates for `contexto_clarificacion`: identity verification before sharing data, unit number confirmation before balance queries, contract verification for payment questions.

2.4 Templates for `escalamiento`: immediate handoff on legal keywords (abogado, tribunal, demanda), handoff on repeated questions (3x same question), handoff on emergencies (inundación, gas, incendio, electrical), handoff on explicit request (hablar con alguien, humano, agente).

2.5 Templates for `politicas`: never share owner bank details, never disclose other tenants' info, never confirm payments without system verification, never give legal/financial advice, never share contrato terms with non-parties.

2.6 All templates are enabled by default on new organizations. Users can disable any template.

### 3. Custom Rules

3.1 Users with `admin` or `gerente` role can create custom guidance rules in any category.

3.2 Custom rules are created with an instruction text and a category. The system assigns UUID and timestamps.

3.3 Custom rules can be edited, enabled/disabled, reordered, and deleted.

3.4 Custom rules are validated: non-empty instruction, max 500 chars, valid category enum.

### 4. API Changes

4.1 The `ChatbotConfigResponse` includes a `guidance_rules` array with all rules (templates + custom), sorted by category then sort_order.

4.2 The `ChatbotConfigUpdateRequest` no longer accepts `system_prompt`. The field is removed from the API.

4.3 New endpoint: `POST /api/v1/chatbot/guidance-rules` — create a custom rule.

4.4 New endpoint: `PUT /api/v1/chatbot/guidance-rules/{id}` — update a rule (instruction, enabled, sort_order).

4.5 New endpoint: `DELETE /api/v1/chatbot/guidance-rules/{id}` — delete a custom rule. Returns 403 if rule is a template.

4.6 New endpoint: `PUT /api/v1/chatbot/guidance-rules/batch` — batch update enabled/sort_order for multiple rules (used by drag-to-reorder and bulk toggle).

4.7 All guidance endpoints require `WriteAccess` extractor (admin or gerente).

### 5. System Prompt Assembly

5.1 The existing `compose_system_prompt` function is updated to assemble the prompt from enabled guidance rules instead of a raw `system_prompt` string.

5.2 Rules are grouped by category and injected into the system prompt with clear section headers.

5.3 Disabled rules are excluded from prompt assembly entirely.

5.4 The assembled prompt is never exposed to the user — it's an internal implementation detail.

5.5 Existing persona fields (tone, greeting, display_name) continue to be included in prompt assembly as before.

5.6 FAQs and policies continue to be injected as before — guidance rules complement them, not replace them.

### 6. AgentConfig Simplification

6.1 Remove `agent_config` from `ChatbotConfigUpdateRequest` — it's no longer user-configurable.

6.2 The `AgentConfig` struct remains internal with hardcoded sensible defaults (max_turns: 5, temperature: none, tool_registration: selective).

6.3 Guardrail overrides (`max_receipt_amount_dop`, `max_receipt_amount_usd`, `blocked_patterns`) remain internal and are derived from organization settings, not user input.

6.4 The `ChatbotConfigResponse` no longer includes `agent_config`.

### 7. Frontend Changes

7.1 Replace the "Instrucciones adicionales" textarea in PersonaStep with a new GuidanceRulesStep component.

7.2 GuidanceRulesStep displays rules grouped by category with collapsible sections.

7.3 Each rule shows: toggle switch (enabled/disabled), instruction text, edit button (for custom rules), delete button (for custom rules).

7.4 Template rules show a lock icon and cannot be deleted, only toggled.

7.5 Each category section has an "Agregar regla" button to create custom rules within that category.

7.6 Creating a custom rule opens an inline form with a textarea (max 500 chars) and category selector (pre-filled from the section).

7.7 A counter shows "X/30 reglas activas" to indicate limit proximity.

7.8 All UI text is in Spanish.

### 8. Migration Strategy

8.1 Existing organizations with a non-empty `system_prompt` get it converted into a single custom rule in the `politicas` category with the instruction being the existing system_prompt text (truncated to 500 chars if needed).

8.2 All existing organizations receive the predefined template rules (enabled by default).

8.3 The `system_prompt` column is retained but deprecated (nullable, not used in prompt assembly). It will be dropped in a future migration.

8.4 The `agent_config` JSONB column remains for internal use but is no longer exposed via the API.

### 9. Non-Functional Requirements

9.1 Guidance rules CRUD operations must be audited (existing auditoria system).

9.2 Rule changes are auto-saved (debounced like other chatbot config changes).

9.3 The guidance rules UI must be accessible (ARIA labels, keyboard navigation for toggles and reorder).

9.4 The prompt assembly must handle 0 enabled rules gracefully (agent still functions with just persona + FAQs + policies).
