# Design: Agent Guidance Redesign

## Overview

This design replaces the monolithic `system_prompt` freetext field in chatbot configuration with a structured guidance rules system. Property managers configure agent behavior through categorized, individually toggleable natural-language rules. Technical parameters (`AgentConfig`) are internalized and no longer user-facing. The prompt assembly pipeline reads enabled rules at invocation time and composes the final system prompt internally.

## Architecture

The guidance rules system replaces the monolithic `system_prompt` freetext with structured, categorized, individually manageable rules stored as JSONB in `chatbot_config`. The prompt assembly pipeline reads enabled rules at invocation time and composes the final system prompt internally.

```
┌─────────────────────────────────────────────────────────────┐
│  Frontend (GuidanceRulesStep component)                     │
│  - Displays rules grouped by category                       │
│  - Toggle enable/disable per rule                           │
│  - Create/edit/delete custom rules                          │
│  - Auto-save on change (debounced)                          │
└─────────────────────┬───────────────────────────────────────┘
                      │ REST API
                      ▼
┌─────────────────────────────────────────────────────────────┐
│  Backend (handlers/chatbot.rs + services/chatbot.rs)        │
│  - CRUD endpoints for guidance rules                        │
│  - Validation (500 char max, 30 active limit, categories)   │
│  - Audit logging                                            │
└─────────────────────┬───────────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────────┐
│  Database (chatbot_config.guidance_rules JSONB)             │
│  - Array of GuidanceRule objects                            │
│  - Templates seeded on org creation                         │
└─────────────────────┬───────────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────────┐
│  AI Module (compose_system_prompt)                          │
│  - Reads enabled rules at message processing time           │
│  - Groups by category → injects into system prompt          │
│  - Persona + FAQs + Policies + Guidance Rules = final prompt│
└─────────────────────────────────────────────────────────────┘
```

## Components and Interfaces

### Backend Components

- **`models/chatbot.rs`** — `GuidanceRule`, `GuidanceCategory`, `CreateGuidanceRuleRequest`, `UpdateGuidanceRuleRequest`, `BatchUpdateRequest` DTOs.
- **`services/chatbot.rs`** — CRUD functions: `create_guidance_rule`, `update_guidance_rule`, `delete_guidance_rule`, `batch_update_guidance_rules`, `seed_template_rules`. Validation logic for max 30 active rules, 500 char limit.
- **`handlers/chatbot.rs`** — HTTP handler functions for the four new endpoints, wired with `WriteAccess` extractor.
- **`services/ai_module/mod.rs`** — Updated `compose_system_prompt` to assemble from enabled guidance rules.

### Frontend Components

- **`components/chatbot/guidance_rules_step.rs`** — Main component displaying categorized rules with toggles, CRUD actions, and active count.
- **`types/chatbot.rs`** — `GuidanceRule`, `GuidanceCategory` TypeScript-equivalent types for frontend.
- **`services/chatbot.rs`** — API functions: `create_guidance_rule`, `update_guidance_rule`, `delete_guidance_rule`, `batch_update_guidance_rules`.

### Interface Contracts

- Frontend communicates with backend via REST JSON endpoints under `/api/v1/chatbot/guidance-rules`.
- Backend stores rules as JSONB array in existing `chatbot_config` table.
- AI module reads rules from the `chatbot_config` model at message processing time (no caching layer).

## Data Models

### GuidanceRule (stored as JSONB array in `chatbot_config.guidance_rules`)

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GuidanceRule {
    pub id: Uuid,
    pub category: GuidanceCategory,
    pub instruction: String,
    pub enabled: bool,
    pub is_template: bool,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GuidanceCategory {
    EstiloComunicacion,
    ContextoClarificacion,
    Escalamiento,
    Politicas,
}
```

### Storage

- Column: `chatbot_config.guidance_rules` (JSONB, NOT NULL, DEFAULT '[]')
- The existing `system_prompt` column remains nullable but is no longer read by prompt assembly.
- The `agent_config` JSONB column remains but is no longer exposed in API responses or updates.

## API Design

### Modified Endpoints

**GET /api/v1/chatbot/config** — Response now includes `guidanceRules` array, excludes `agentConfig` and `systemPrompt`.

**PUT /api/v1/chatbot/config** — No longer accepts `systemPrompt` or `agentConfig` fields. Other fields unchanged.

### New Endpoints

```
POST   /api/v1/chatbot/guidance-rules          → CreateGuidanceRuleRequest → GuidanceRule
PUT    /api/v1/chatbot/guidance-rules/{id}      → UpdateGuidanceRuleRequest → GuidanceRule
DELETE /api/v1/chatbot/guidance-rules/{id}      → 204 No Content
PUT    /api/v1/chatbot/guidance-rules/batch     → BatchUpdateRequest → Vec<GuidanceRule>
```

### Request/Response DTOs

```rust
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateGuidanceRuleRequest {
    pub category: GuidanceCategory,
    pub instruction: String,
    pub enabled: Option<bool>,  // defaults to true
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateGuidanceRuleRequest {
    pub instruction: Option<String>,
    pub enabled: Option<bool>,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchUpdateItem {
    pub id: Uuid,
    pub enabled: Option<bool>,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchUpdateRequest {
    pub rules: Vec<BatchUpdateItem>,
}
```

## Prompt Assembly

The `compose_system_prompt` function is updated to include a guidance section:

```
[Persona section — name, tone, greeting]

## Reglas de comportamiento

### Estilo de comunicación
- {rule 1 instruction}
- {rule 2 instruction}

### Contexto y clarificación
- {rule 3 instruction}

### Escalamiento
- {rule 4 instruction}
- {rule 5 instruction}

### Políticas
- {rule 6 instruction}
- {rule 7 instruction}

[FAQs section]
[Policies section]
[Tenant context section]
[Handoff keywords section]
```

Only enabled rules are included. Categories with no enabled rules are omitted entirely.

## Template Rules (Seeded Defaults)

### estilo_comunicacion
1. "Tratar a todos los inquilinos de 'usted', nunca de 'tú'"
2. "Incluir siempre el símbolo de moneda (RD$ o US$) al mencionar montos"
3. "Mantener mensajes cortos: máximo 3 oraciones por respuesta"
4. "Responder siempre en español, sin importar el idioma del mensaje recibido"

### contexto_clarificacion
5. "Antes de compartir cualquier dato financiero, confirmar la identidad del inquilino pidiendo nombre y número de unidad"
6. "Si el inquilino pregunta por un balance sin especificar unidad, preguntar cuál unidad antes de responder"
7. "Si hay ambigüedad sobre cuál contrato se refiere, listar los contratos activos y pedir que elija"

### escalamiento
8. "Si el inquilino menciona 'abogado', 'tribunal', 'demanda' o 'acción legal', transferir inmediatamente a un humano sin hacer más preguntas"
9. "Si el inquilino reporta una emergencia (inundación, fuga de gas, incendio, fallo eléctrico), transferir a humano inmediatamente"
10. "Si el inquilino pide hablar con una persona real o dice 'humano', 'agente' o 'hablar con alguien', respetar su solicitud y transferir"
11. "Si el inquilino repite la misma pregunta 3 veces sin obtener la respuesta deseada, ofrecer transferencia a un humano"

### politicas
12. "Nunca compartir datos bancarios del propietario o la administración"
13. "Nunca revelar información personal de otros inquilinos (nombres, balances, unidades)"
14. "No confirmar la recepción de un pago sin verificar primero en el sistema"
15. "No dar consejos legales ni financieros — derivar al profesional correspondiente"
16. "No compartir términos de contrato con personas que no sean parte del contrato"

## Migration Plan

### Phase 1: Database Migration

1. Add `guidance_rules JSONB NOT NULL DEFAULT '[]'` to `chatbot_config`.
2. For each existing org:
   - Seed all 16 template rules (enabled = true).
   - If `system_prompt` is non-empty, create a custom rule in `politicas` category with the existing text (truncated to 500 chars).
3. Mark `system_prompt` as deprecated (keep column, stop reading).

### Phase 2: Backend Changes

1. Add `GuidanceRule`, `GuidanceCategory` to `models/chatbot.rs`.
2. Add CRUD functions to `services/chatbot.rs`.
3. Add handler endpoints to `handlers/chatbot.rs`.
4. Update `compose_system_prompt` to use guidance rules.
5. Remove `agent_config` and `system_prompt` from API response/request DTOs.
6. Register new routes.

### Phase 3: Frontend Changes

1. Create `components/chatbot/guidance_rules_step.rs`.
2. Remove `system_prompt` textarea from `PersonaStep`.
3. Add `GuidanceRulesStep` as a new section in the chatbot config page.
4. Add types for guidance rules in `types/chatbot.rs`.
5. Add API calls in `services/chatbot.rs`.

## Frontend Component Design

```
┌─────────────────────────────────────────────────────────────┐
│  Reglas del agente (12/30 activas)                          │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ▼ Estilo de comunicación (4 activas)                       │
│  ┌─────────────────────────────────────────────────────────┐│
│  │ 🔒 [✓] Tratar a todos los inquilinos de 'usted'...     ││
│  │ 🔒 [✓] Incluir siempre el símbolo de moneda...         ││
│  │ 🔒 [ ] Mantener mensajes cortos: máximo 3 oraciones... ││
│  │ 🔒 [✓] Responder siempre en español...                 ││
│  │    [+ Agregar regla]                                    ││
│  └─────────────────────────────────────────────────────────┘│
│                                                             │
│  ▼ Contexto y clarificación (3 activas)                     │
│  ┌─────────────────────────────────────────────────────────┐│
│  │ 🔒 [✓] Antes de compartir datos financieros...         ││
│  │ 🔒 [✓] Si pregunta por balance sin unidad...           ││
│  │    [✓] Mi regla personalizada aquí... [✏️] [🗑️]        ││
│  │    [+ Agregar regla]                                    ││
│  └─────────────────────────────────────────────────────────┘│
│                                                             │
│  ▶ Escalamiento (4 activas)                                 │
│  ▶ Políticas (5 activas)                                    │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

## Security Considerations

- All guidance rule endpoints require `WriteAccess` (admin or gerente).
- Template rules cannot be deleted (only toggled).
- Instruction text is sanitized: no prompt injection patterns (instructions like "ignore previous instructions" are rejected).
- Guidance rules are scoped to the authenticated organization (multi-tenancy enforced).
- Rule changes are audit-logged.

## Correctness Properties

### Property 1: Active Rule Count Invariant
For all organizations, the count of rules where `enabled = true` never exceeds 30. Enforced at the service layer before any enable/create operation.
**Validates: Requirements 1.4**

### Property 2: Template Immutability
Template rules (`is_template = true`) can never be deleted via the API. Only `enabled` and `sort_order` can be modified on template rules.
**Validates: Requirements 1.6, 3.3**

### Property 3: Instruction Length Bound
No rule instruction exceeds 500 characters and no instruction is empty. Validated on create and update.
**Validates: Requirements 1.1, 1.5, 3.4**

### Property 4: Category Validity
Every rule has a valid `GuidanceCategory` enum value. Deserialization rejects unknown values.
**Validates: Requirements 1.2, 3.4**

### Property 5: Prompt Assembly Completeness
The assembled system prompt always includes all enabled rules and never includes disabled rules. For any set of rules R, the prompt contains rule.instruction if and only if rule.enabled is true.
**Validates: Requirements 5.1, 5.2, 5.3**

### Property 6: Organization Isolation
Guidance rules for organization A are never visible or modifiable by organization B. All CRUD operations are scoped to the authenticated organization.
**Validates: Requirements 4.7**

### Property 7: UUID Uniqueness
Each rule has a globally unique ID. No two rules within an org share an ID.
**Validates: Requirements 1.1, 3.2**

## Error Handling

- **422 Unprocessable Entity**: Returned when attempting to enable a 31st rule, or when instruction exceeds 500 chars, or when instruction is empty.
- **403 Forbidden**: Returned when attempting to delete a template rule, or when a `visualizador` role user tries to create/update/delete rules.
- **404 Not Found**: Returned when a rule ID doesn't exist within the authenticated org.
- **400 Bad Request**: Returned for malformed JSON, invalid category enum, or missing required fields.
- **Graceful degradation**: If `guidance_rules` JSONB is corrupted or unparseable, the AI module falls back to an empty rule set (agent still functions with persona + FAQs + policies).

## Testing Strategy

- Unit tests: GuidanceRule validation (length, category, max active count).
- Unit tests: compose_system_prompt with various rule combinations (0 rules, all disabled, mixed).
- Property-based tests: rule count invariant (never exceed 30 active), template immutability.
- Integration tests: CRUD lifecycle, batch update, migration correctness.
- Frontend: component renders rules grouped by category, toggle updates state correctly.
