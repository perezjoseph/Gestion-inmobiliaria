# Design Document

## Overview

Omnibus remediation across nine specs of the Dominican Republic property management platform, sequenced security-first. The work concentrates on:

1. Closing a cross-tenant receipt PDF leak in `services::recibos`.
2. Hardening landlord self-registration (rol = `gerente`, `User`-only response).
3. Sealing signed contracts as protected `Documento` rows and wiring outbound mail through a new `MailClient` abstraction backed by Mailcow.
4. Completing the document-management frontend (verification action, compliance profile, expiring-docs page, dashboard counters).
5. Persisting OCR confirmations synchronously with best-effort tenant matching, while pinning OVMS/OCR to CPU-only.
6. Filling out the dashboard (contratos por vencer, ocupación, próximos pagos, calendario) and shipping a real PWA (manifest, service worker, IndexedDB cache, `use_online`).
7. Restoring `[INIT]` bootstrap logs in the Yew frontend with a property-based regression test.
8. Finishing the WhatsApp AI multi-turn agent loop with `ExtractReceiptTool` wired through `record_extraction`.
9. Completing the gastos workflow (rentabilidad, categorías, dashboard card, enum tuple fix, utility-service fields, date-range filter).
10. Adding `unidad_id` filtering to maintenance listings.

All work runs on Kubernetes (k3s); no docker-compose paths. Inference goes to OVMS `/v3`. The backend remains layered (handlers → services → entities) with `organizacion_id` enforced at the service boundary; the frontend remains Yew + Trunk with Spanish copy and DD/MM/YYYY dates.

## Architecture

### Backend layered topology

```
HTTP request
  │
  ├── handlers/{recibos,auth,firmas,documentos,ocr,mantenimiento,gastos}.rs
  │       extract Claims → call into services with (db, dto, organizacion_id, usuario_id)
  │
  ├── services/{recibos,auth,firmas,documentos,ocr_mapping,
  │             mantenimiento,gastos,ai_module,chatbot}.rs
  │       enforces organizacion_id on every read & write
  │       runs SeaORM transactions for multi-step writes
  │       returns AppError for any tenant violation, validation failure, FK breach
  │
  └── entities/{pago,contrato,documento,inquilino,usuario,gasto,
                solicitud_mantenimiento}.rs (generated)
```

Handlers never touch SeaORM directly; services never assume tenant context. Every list query that touches a multi-tenant table appends `Column::OrganizacionId.eq(organizacion_id)` (joining through the parent entity when the table itself does not carry the column — for `pago` we go through `contrato`).

### Mail subsystem

A new `services/mail/` directory introduces the abstraction:

```
backend/src/services/mail/
├── mod.rs            // pub use trait + impl
├── client.rs         // pub trait MailClient { async fn send(...) -> Result<(), AppError>; }
├── smtp.rs           // pub struct SmtpMailClient { transport: AsyncSmtpTransport<Tokio1Executor> }
└── message.rs        // SignatureLinkMail (Spanish subject/body builder)
```

`AppState` carries `Arc<dyn MailClient + Send + Sync>` so handlers and tests can swap implementations (real SMTP in prod, file transport / mock in tests). Configuration is read in `config.rs` from K8s Secret `mailcow-smtp` (envs `SMTP_HOST=mail.myhomeva.us`, `SMTP_PORT`, `SMTP_USER`, `SMTP_PASS`, `SMTP_FROM=no-reply@myhomeva.us`).

### Frontend PWA layer

```
frontend/
├── index.html                  // <link rel="manifest"> + SW registration
├── manifest.webmanifest        // name, icons, theme, display=standalone
├── service-worker.js           // app-shell precache + offline fallback
└── src/
    ├── services/
    │   ├── idb_cache.rs        // IndexedDB wrapper for list reads (cache-first)
    │   └── online.rs           // window event listeners feeding use_online
    ├── hooks/
    │   └── use_online.rs       // pub fn use_online() -> bool
    └── components/common/
        └── offline_guard.rs    // disables form submit buttons when offline
```

The cache wrapper exposes `read_list<T: DeserializeOwned>(key) -> Option<Vec<T>>` and `write_list<T: Serialize>(key, items)`. List pages always write-through after a successful network read and fall back to the cached value when `use_online()` is `false`.

### WhatsApp AI multi-turn loop

```
WebSocket message arrives
  → services::chatbot::handle_message
  → services::ai_module::invoke_agent(history, msg)
        loop turn in 0..N (N=5):
            response = rig::Agent::completion(history)
            match response {
                Final(msg)        => return msg
                ToolCalls(calls)  => for c in calls {
                    result = c.tool.call(c.args)
                    history.push(Tool { id: c.id, content: result })
                }
            }
        return Spanish fallback
  → if Final && ExtractReceiptTool was invoked successfully,
    services::chatbot::record_extraction persists Pago/Gasto
```

`ExtractReceiptTool::call` delegates to `services::ocr::OcrClient::extract` (which talks to OVMS `/v3`) and returns a `PaymentReceipt` JSON. The Rig agent is the only loop driver — there is no hardcoded image→OCR routing.

## Components and Interfaces

### 1. Receipt scoping (Requirement 1)

**File:** `backend/src/services/recibos.rs`

Signature change:

```rust
// Feature: spec-gap-remediation, Property 1
pub async fn generar_recibo(
    db: &DatabaseConnection,
    pago_id: Uuid,
    organizacion_id: Uuid,
) -> Result<Vec<u8>, AppError> {
    let pago = pago::Entity::find_by_id(pago_id)
        .find_also_related(contrato::Entity)
        .filter(contrato::Column::OrganizacionId.eq(organizacion_id))
        .one(db)
        .await?
        .ok_or_else(|| {
            tracing::warn!(
                target: "security.cross_tenant",
                pago_id = %pago_id,
                organizacion_id = %organizacion_id,
                "Intento de acceso a recibo fuera de la organización"
            );
            AppError::NotFound("Recibo no encontrado".into())
        })?;
    // …PDF rendering unchanged
}
```

**File:** `backend/src/handlers/recibos.rs`

The handler extracts `claims.organizacion_id` from the `AuthenticatedUser` extractor and forwards it. The route binding in `routes.rs` is unchanged.

### 2. Registration (Requirement 2)

**File:** `backend/src/services/auth.rs`

`register_new_org` is updated so the persisted `Usuario` has `rol: "gerente".to_string()` and the response is a `User` DTO only:

```rust
pub async fn register_new_org(
    db: &DatabaseConnection,
    body: RegisterRequest,
) -> Result<User, AppError> {
    // duplicate-email check returns AppError::Conflict("El correo ya está registrado")
    let txn = db.begin().await?;
    let org = organizacion::ActiveModel { /* … */ }.insert(&txn).await?;
    let usuario = usuario::ActiveModel {
        rol: Set("gerente".to_string()),
        organizacion_id: Set(org.id),
        // …
    }
    .insert(&txn)
    .await?;
    txn.commit().await?;
    Ok(User::from(usuario))
}
```

**File:** `backend/src/handlers/auth.rs`

The register handler returns `HttpResponse::Created().json(user)` — no token, no session payload. The frontend already reads only the `User` shape, so no UI change is required.

### 3. Sealed contract + outbound mail (Requirement 3)

#### Migration

**File:** `backend/src/migrations/m20260415_001_add_documento_origen_id.rs`

```rust
manager
    .alter_table(
        Table::alter()
            .table(Documento::Table)
            .add_column(
                ColumnDef::new(Documento::DocumentoOrigenId)
                    .uuid()
                    .null(),
            )
            .add_foreign_key(
                TableForeignKey::new()
                    .name("fk_documento_origen_contrato")
                    .from_tbl(Documento::Table)
                    .from_col(Documento::DocumentoOrigenId)
                    .to_tbl(Contrato::Table)
                    .to_col(Contrato::Id)
                    .on_delete(ForeignKeyAction::SetNull),
            )
            .to_owned(),
    )
    .await
```

Re-export from `migrations/mod.rs` and add to the migrator vector.

#### Sealed PDF generation

**File:** `backend/src/services/firmas.rs`

```rust
// Feature: spec-gap-remediation, Property 3
pub async fn generar_pdf_sellado(
    db: &DatabaseConnection,
    contrato: &contrato::Model,
    organizacion_id: Uuid,
) -> Result<documento::Model, AppError> {
    let pdf_bytes = render_contrato_pdf(contrato).await?;
    let dest = std::path::PathBuf::from("uploads/contratos")
        .join(contrato.id.to_string())
        .join("sellado.pdf");
    tokio::fs::create_dir_all(dest.parent().unwrap()).await?;
    tokio::fs::write(&dest, &pdf_bytes).await?;

    let doc = documento::ActiveModel {
        id: Set(Uuid::new_v4()),
        entity_type: Set("contrato".into()),
        entity_id: Set(contrato.id),
        filename: Set("contrato_sellado.pdf".into()),
        file_path: Set(dest.to_string_lossy().into_owned()),
        sellado: Set(true),
        documento_origen_id: Set(Some(contrato.id)),
        organizacion_id: Set(organizacion_id),
        // …
        ..Default::default()
    }
    .insert(db)
    .await?;
    Ok(doc)
}
```

#### Delete guard

**File:** `backend/src/services/documentos.rs`

```rust
pub async fn eliminar(
    db: &DatabaseConnection,
    id: Uuid,
    organizacion_id: Uuid,
) -> Result<(), AppError> {
    let doc = documento::Entity::find_by_id(id)
        .filter(documento::Column::OrganizacionId.eq(organizacion_id))
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Documento no encontrado".into()))?;

    if doc.sellado || doc.documento_origen_id.is_some() {
        return Err(AppError::Forbidden(
            "No se puede eliminar un documento sellado".into(),
        ));
    }
    documento::Entity::delete_by_id(id).exec(db).await?;
    Ok(())
}
```

#### MailClient trait + SMTP impl

**File:** `backend/src/services/mail/client.rs`

```rust
#[async_trait::async_trait]
pub trait MailClient: Send + Sync {
    async fn send(&self, msg: OutgoingMail) -> Result<(), AppError>;
}

pub struct OutgoingMail {
    pub to: String,
    pub subject: String,
    pub body_html: String,
    pub body_text: String,
}
```

**File:** `backend/src/services/mail/smtp.rs`

```rust
pub struct SmtpMailClient {
    transport: AsyncSmtpTransport<Tokio1Executor>,
    from: Mailbox,
}

impl SmtpMailClient {
    pub fn from_config(cfg: &SmtpConfig) -> Result<Self, AppError> {
        let creds = Credentials::new(cfg.user.clone(), cfg.pass.clone());
        let transport = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&cfg.host)?
            .port(cfg.port)
            .credentials(creds)
            .build();
        Ok(Self {
            transport,
            from: cfg.from.parse()?,
        })
    }
}

#[async_trait::async_trait]
impl MailClient for SmtpMailClient {
    async fn send(&self, msg: OutgoingMail) -> Result<(), AppError> {
        let email = Message::builder()
            .from(self.from.clone())
            .to(msg.to.parse()?)
            .subject(&msg.subject)
            .multipart(MultiPart::alternative_plain_html(msg.body_text, msg.body_html))?;
        self.transport
            .send(email)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Fallo al enviar correo SMTP");
                AppError::BadGateway("No se pudo enviar el correo".into())
            })?;
        Ok(())
    }
}
```

#### Wiring the signing email

**File:** `backend/src/services/firmas.rs`

```rust
pub async fn enviar_email_firma(
    mail: &dyn MailClient,
    inquilino: &inquilino::Model,
    contrato: &contrato::Model,
    link: &str,
) -> Result<(), AppError> {
    mail.send(OutgoingMail {
        to: inquilino.email.clone(),
        subject: format!("Firma electrónica de su contrato #{}", contrato.id),
        body_text: spanish_body_text(contrato, link),
        body_html: spanish_body_html(contrato, link),
    })
    .await
}
```

K8s manifest snippet (`infra/k8s/app/backend.yaml`):

```yaml
envFrom:
  - secretRef: { name: mailcow-smtp }   # SMTP_HOST, SMTP_PORT, SMTP_USER, SMTP_PASS, SMTP_FROM
```

### 4. Document management frontend (Requirement 4)

**File:** `frontend/src/components/feature/verification_badge.rs`

The component already exists as a status indicator; activate the underlying button so an admin or gerente fires:

```rust
let onclick = {
    let id = props.id;
    Callback::from(move |new_status: String| {
        spawn_local(async move {
            let _ = api::put::<DocumentoStatus, ()>(
                &format!("/documentos/{id}/verificar"),
                &DocumentoStatus { status: new_status },
            )
            .await;
        });
    })
};
```

The button is hidden when `current_user().rol == "visualizador"`.

**File:** `frontend/src/components/feature/compliance_badge.rs`

Renders the response of `GET /documentos/cumplimiento/{entity_type}/{entity_id}`. Mounted on `pages/inquilinos.rs`, `pages/propiedades.rs`, and `pages/contratos.rs` detail views.

**File:** `frontend/src/pages/documentos_por_vencer.rs` *(new)*

```rust
#[function_component(DocumentosPorVencer)]
pub fn documentos_por_vencer() -> Html {
    let docs = use_state(Vec::<DocumentoVencimiento>::new);
    {
        let docs = docs.clone();
        use_effect_with((), move |_| {
            spawn_local(async move {
                if let Ok(rows) = api::get::<Vec<DocumentoVencimiento>>("/documentos/por-vencer").await {
                    docs.set(rows);
                }
            });
            || ()
        });
    }
    html! { /* table sorted asc by fecha_expiracion, DD/MM/YYYY */ }
}
```

Add to `Route` enum: `#[at("/documentos/por-vencer")] DocumentosPorVencer`.

**File:** `frontend/src/types/dashboard.rs`

```rust
#[derive(Deserialize, Clone, PartialEq)]
pub struct DashboardStats {
    // existing fields…
    pub documentos_vencidos: u32,
    pub documentos_por_vencer: u32,
    pub entidades_incompletas: u32,
}
```

`pages/dashboard.rs` adds three counter cards rendering these fields.

### 5. OCR persistence + tenant match + CPU-only (Requirement 5)

**File:** `backend/src/services/chatbot.rs` (rename of existing OCR confirmation)

```rust
// Feature: spec-gap-remediation, Property 4
pub async fn confirmar_preview(
    db: &DatabaseConnection,
    preview: OcrPreview,
    organizacion_id: Uuid,
    usuario_id: Uuid,
) -> Result<ConfirmedEntity, AppError> {
    // idempotency: lookup by preview.id within org first
    if let Some(existing) = preview_index::find(db, preview.id, organizacion_id).await? {
        return Ok(existing);
    }
    let txn = db.begin().await?;
    let result = match preview.document_type {
        DocumentType::Recibo => {
            let req = build_create_pago_request(&preview, &txn, organizacion_id).await?;
            let pago = services::pagos::crear(&txn, req, organizacion_id, usuario_id).await?;
            preview_index::record(&txn, preview.id, ConfirmedEntity::Pago(pago.id)).await?;
            ConfirmedEntity::Pago(pago.id)
        }
        DocumentType::Gasto => {
            let req = build_create_gasto_request(&preview, organizacion_id)?;
            let gasto = services::gastos::crear(&txn, req, organizacion_id, usuario_id).await?;
            preview_index::record(&txn, preview.id, ConfirmedEntity::Gasto(gasto.id)).await?;
            ConfirmedEntity::Gasto(gasto.id)
        }
    };
    txn.commit().await?;
    Ok(result)
}
```

**File:** `backend/src/services/ocr_mapping.rs` *(new)*

```rust
// Feature: spec-gap-remediation, Property 5
pub async fn map_deposito(
    db: &DatabaseConnection,
    nombre_extraido: &str,
    organizacion_id: Uuid,
) -> Result<Option<Uuid>, AppError> {
    let trimmed = nombre_extraido.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let pattern = format!("%{trimmed}%");
    let candidatos = inquilino::Entity::find()
        .filter(inquilino::Column::OrganizacionId.eq(organizacion_id))
        .filter(
            Condition::any()
                .add(Expr::expr(Func::concat([
                    Expr::col(inquilino::Column::Nombre),
                    Expr::value(" "),
                    Expr::col(inquilino::Column::Apellido),
                ]))
                .like(&pattern)),
        )
        .limit(2)
        .all(db)
        .await?;
    Ok(match candidatos.as_slice() {
        [unico] => Some(unico.id),    // exact-best-effort match
        _ => None,                    // 0 or >=2 → null, never the wrong inquilino
    })
}
```

**File:** `infra/k8s/app/ovms.yaml`

Remove the `gpu.intel.com/i915` resource request and the `i915` device-plugin DaemonSet reference. Set:

```yaml
env:
  - { name: OPENVINO_DEVICE, value: "CPU" }
  - { name: TARGET_DEVICE,   value: "CPU" }
```

Validation failures in `confirmar_preview` map to `AppError::UnprocessableEntity("Datos de OCR inválidos".into())` → HTTP 422.

### 6. Platform enhancements — dashboard + PWA (Requirement 6)

**Frontend dashboard sections** (`frontend/src/pages/dashboard.rs`):

- `ContratosPorVencerWidget` — three buckets (30/60/90 days), driven by an existing list endpoint with `dias` query param.
- `OccupancyChart` — wraps `/dashboard/ocupacion-tendencia`, renders 12-month line chart via existing chart component.
- `UpcomingPaymentsWidget` — `/pagos?estado=pendiente&hasta=YYYY-MM-DD`, sorted asc by `fecha_vencimiento`.
- `ContractCalendar` page — `pages/calendario.rs` overlays contratos, pagos, mantenimientos.

**PWA wiring**:

- `frontend/manifest.webmanifest` — `name`, `short_name`, icons (192, 512), `display: "standalone"`, `theme_color`, `background_color`.
- `frontend/service-worker.js` — Workbox-style precache of `/index.html`, `/main.wasm`, `/main.js`, theme CSS; runtime cache for static assets; offline fallback.
- `frontend/index.html` — adds `<link rel="manifest">` and a small registration script. Trunk's SPA fallback already covers asset rewriting.
- `frontend/src/services/idb_cache.rs` — wraps `idb` crate; functions `read_list<T>(store, key)` and `write_list<T>(store, key, value)` with `serde_wasm_bindgen`.
- `frontend/src/hooks/use_online.rs` — Yew hook subscribing to `online`/`offline` events via `gloo-events 0.6`.
- `frontend/src/components/common/offline_guard.rs` — wraps any submit button and renders disabled when offline.

### 7. Frontend `[INIT]` logs + PBT (Requirement 7)

**File:** `frontend/src/main.rs`

```rust
fn main() {
    web_sys::console::log_1(&"[INIT] pre-renderer".into());
    yew::Renderer::<App>::new().render();
}
```

**File:** `frontend/src/app.rs`

```rust
#[function_component(App)]
pub fn app() -> Html {
    web_sys::console::log_1(&"[INIT] app mounted".into());
    html! {
        <BrowserRouter>
            <Switch<Route> render={|r| {
                web_sys::console::log_1(&"[INIT] route resolution".into());
                switch(r)
            }} />
        </BrowserRouter>
    }
}

fn switch(route: Route) -> Html {
    web_sys::console::log_1(&"[INIT] switch".into());
    match route { /* … */ }
}
```

**File:** `frontend/src/components/common/protected_route.rs`

```rust
#[function_component(ProtectedRoute)]
pub fn protected_route(props: &Props) -> Html {
    web_sys::console::log_1(&"[INIT] auth check".into());
    /* … */
}
```

Plus a sixth log inside the auth-success branch (`[INIT] first route rendered`). All six markers ship in production builds (no `cfg(debug_assertions)` gate).

**File:** `frontend/tests/init_logging_tests.rs` *(restored)*

Headless `wasm-bindgen-test` PBT that boots the app under random route/auth permutations and asserts every `[INIT]` marker appears.

### 8. WhatsApp AI multi-turn (Requirement 8)

**File:** `backend/src/services/ai_module.rs`

```rust
// Feature: spec-gap-remediation, Property 7
pub async fn invoke_agent(
    agent: &Agent,
    mut chat_history: Vec<Message>,
    user_msg: Message,
) -> Result<AgentOutcome, AppError> {
    chat_history.push(user_msg);
    const TURN_LIMIT: usize = 5;
    for _ in 0..TURN_LIMIT {
        match agent.completion(&chat_history).await? {
            AgentResponse::Final(text) => {
                return Ok(AgentOutcome::Final { text, history: chat_history });
            }
            AgentResponse::ToolCalls(calls) => {
                for call in calls {
                    let result = agent
                        .tool(&call.name)
                        .ok_or_else(|| AppError::Internal("Herramienta desconocida".into()))?
                        .call(call.args.clone())
                        .await
                        .unwrap_or_else(|e| ToolResult::error(e.to_string()));
                    chat_history.push(Message::Tool {
                        tool_call_id: call.id,
                        content: result.payload,
                    });
                }
            }
        }
    }
    Ok(AgentOutcome::TurnLimitReached {
        text: "Disculpa, no pude completar tu solicitud. Inténtalo de nuevo, por favor.".into(),
        history: chat_history,
    })
}
```

**File:** `backend/src/services/ai_module/tools/extract_receipt.rs`

```rust
#[async_trait::async_trait]
impl Tool for ExtractReceiptTool {
    type Args = ExtractArgs;
    type Output = PaymentReceipt;
    type Error = AppError;
    const NAME: &'static str = "extract_receipt";

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let bytes = self.media_store.fetch(&args.media_id).await?;
        self.ocr.extract(&bytes).await
    }
}
```

**File:** `backend/src/services/chatbot.rs`

After `invoke_agent` returns `Final` with a `ToolResult::Receipt` in history, call `record_extraction(db, receipt, organizacion_id, usuario_id)` which delegates to the same `confirmar_preview` path used by the explicit-confirm flow.

OVMS base URL stays `https://ovms.<ns>.svc.cluster.local/v3`.

### 9. Gastos completion (Requirement 9)

**File:** `frontend/src/pages/reportes.rs`

Adds a `Rentabilidad` tab that calls `/reportes/rentabilidad?fecha_desde=…&fecha_hasta=…`, renders a per-property table, and exposes `Descargar PDF` and `Descargar Excel` buttons hitting the existing export endpoints.

**File:** `frontend/src/pages/categorias_gastos.rs` *(new)*

`GET /gastos/resumen-categorias` → table of categoría / total / count, sortable.

**File:** `frontend/src/pages/dashboard.rs`

Adds `ExpenseCard` calling `GET /dashboard/gastos-comparacion`.

**File:** `frontend/src/pages/gastos.rs:48`

Fix the enum-tuple typo:

```rust
("servicio_publico", "Servicio Público")
```

**File:** `frontend/src/types/gasto.rs`

```rust
#[derive(Serialize, Deserialize, Clone, PartialEq, Default)]
pub struct CreateGasto {
    // existing fields…
    pub proveedor: Option<String>,
    pub numero_cuenta: Option<String>,
    pub periodo_inicio: Option<NaiveDate>,
    pub periodo_fin: Option<NaiveDate>,
}

pub type UpdateGasto = CreateGasto; // mirror change
```

The form (`components/feature/gasto_form.rs`) renders these fields conditionally when `categoria == "servicios"`.

**File:** `frontend/src/components/feature/gasto_filter_bar.rs`

Adds `fecha_desde` and `fecha_hasta` date pickers; the filter state propagates to the parent page via callback.

**Backend file:** `backend/src/services/gastos.rs`

```rust
// Feature: spec-gap-remediation, Property 8
if let (Some(desde), Some(hasta)) = (filter.fecha_desde, filter.fecha_hasta) {
    if desde > hasta {
        return Err(AppError::BadRequest(
            "fecha_desde no puede ser posterior a fecha_hasta".into(),
        ));
    }
}
let q = gasto::Entity::find()
    .filter(gasto::Column::OrganizacionId.eq(organizacion_id))
    .apply_if(filter.fecha_desde, |q, d| q.filter(gasto::Column::FechaGasto.gte(d)))
    .apply_if(filter.fecha_hasta, |q, h| q.filter(gasto::Column::FechaGasto.lte(h)));
```

### 10. Unidades — maintenance filter (Requirement 10)

**File:** `backend/src/models/mantenimiento.rs`

```rust
#[derive(Deserialize, Default)]
pub struct SolicitudListQuery {
    // existing fields…
    pub unidad_id: Option<Uuid>,
}
```

**File:** `backend/src/services/mantenimiento.rs`

```rust
// Feature: spec-gap-remediation, Property 6
pub async fn list(
    db: &DatabaseConnection,
    q: SolicitudListQuery,
    organizacion_id: Uuid,
) -> Result<Vec<SolicitudMantenimiento>, AppError> {
    solicitud_mantenimiento::Entity::find()
        .filter(solicitud_mantenimiento::Column::OrganizacionId.eq(organizacion_id))
        .apply_if(q.unidad_id, |sel, uid| {
            sel.filter(solicitud_mantenimiento::Column::UnidadId.eq(uid))
        })
        // … other filters
        .all(db)
        .await
        .map_err(Into::into)
        .map(map_models)
}
```

A `unidad_id` belonging to another organizacion produces an empty list because the org filter runs first; no existence leak.

## Data Models

### New migration

`m20260415_001_add_documento_origen_id.rs` — adds nullable `documento_origen_id UUID` to `documentos` with FK `fk_documento_origen_contrato → contratos.id ON DELETE SET NULL`.

### Backend config

**File:** `backend/src/config.rs`

```rust
#[derive(Clone, Debug)]
pub struct SmtpConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub pass: String,
    pub from: String,
}

impl SmtpConfig {
    pub fn from_env() -> Result<Self, AppError> {
        Ok(Self {
            host: std::env::var("SMTP_HOST")?,
            port: std::env::var("SMTP_PORT").unwrap_or_else(|_| "587".into()).parse()?,
            user: std::env::var("SMTP_USER")?,
            pass: std::env::var("SMTP_PASS")?,
            from: std::env::var("SMTP_FROM").unwrap_or_else(|_| "no-reply@myhomeva.us".into()),
        })
    }
}
```

### Frontend types

`frontend/src/types/dashboard.rs::DashboardStats` adds `documentos_vencidos`, `documentos_por_vencer`, `entidades_incompletas`.

`frontend/src/types/gasto.rs` adds `proveedor`, `numero_cuenta`, `periodo_inicio`, `periodo_fin` to `CreateGasto` / `UpdateGasto` / `Gasto`. Money fields keep the existing `decimal_as_string` deserializer.

## Correctness Properties

*A property is a characteristic or behavior that should hold true across all valid executions of a system — essentially, a formal statement about what the system should do. Properties serve as the bridge between human-readable specifications and machine-verifiable correctness guarantees.*

### Property 1: Cross-tenant receipt access never leaks

For any pair of organizations `(orgA, orgB)` with `orgA != orgB` and any `Pago` belonging to `orgB`, a receipt request issued by a user whose JWT carries `organizacion_id = orgA` must produce HTTP `404` and the response body must not contain any bytes from the receipt PDF.

**Validates: Requirements 1.2, 1.3, 1.5**

### Property 2: Self-registered users are always `gerente`

For any successful self-registration request, the persisted `Usuario` has `rol == "gerente"` and a non-null `organizacion_id`, regardless of any role hint in the request payload. The HTTP response shape exactly matches the `User` DTO and contains no token, password, or session field.

**Validates: Requirements 2.1, 2.3, 2.5**

### Property 3: Sealed-document deletion is rejected

For any `Documento` row with `documento_origen_id != NULL` (or `sellado == true`), a delete request from any user in the owning organizacion produces HTTP `409`, the row remains in the database, and the file on disk is unchanged.

**Validates: Requirements 3.2, 3.3**

### Property 4: OCR confirm inserts exactly one row, idempotently

For any valid `OcrPreview` with `document_type ∈ {recibo, gasto}`, calling `confirmar_preview` once inserts exactly one matching row (a `Pago` for `recibo`, a `Gasto` for `gasto`); calling it twice with the same `preview_id` yields the same single row — no double-insert. For any invalid extraction the call returns HTTP `422` and the row count is unchanged.

**Validates: Requirements 5.1, 5.2, 5.3, 5.7**

### Property 5: Tenant match is best-effort and never wrong

For any tenant-name string passed to `ocr_mapping::map_deposito` and any inquilino dataset, the function returns either `Some(id)` for an unambiguous match within the caller's `organizacion_id` or `None`. It never returns the id of an inquilino in another organizacion, and it never returns an id when the candidate set has size ≠ 1.

**Validates: Requirements 5.4, 5.5**

### Property 6: Maintenance filter respects unidad_id and tenant scope

For any caller with `organizacion_id = org`, any `Solicitud_List_Query` with `unidad_id = u`, and any maintenance dataset, every row in the response satisfies `row.organizacion_id == org` AND (`u.is_none()` OR `row.unidad_id == u`). When `u` references a unit outside `org`, the response is the empty list.

**Validates: Requirements 10.2, 10.3, 10.4, 10.5**

### Property 7: Multi-turn agent loop terminates

For any conversation history and any sequence of tool-call responses produced by the LLM, `services::ai_module::invoke_agent` terminates within `N = 5` turns and returns either an `AgentOutcome::Final` carrying a final assistant message or an `AgentOutcome::TurnLimitReached` carrying the Spanish fallback `"Disculpa, no pude completar tu solicitud. Inténtalo de nuevo, por favor."`. The function never loops indefinitely.

**Validates: Requirements 7.1, 7.2, 7.3, 8.1, 8.4**

### Property 8: Date-range filter on gastos is sound and complete

For any gastos dataset and any valid `(fecha_desde, fecha_hasta)` with `fecha_desde <= fecha_hasta`, the result of the gastos list endpoint is a subset of the caller's gastos and every returned row satisfies `fecha_desde <= row.fecha_gasto <= fecha_hasta`. When `fecha_desde > fecha_hasta` the endpoint returns HTTP `400`.

**Validates: Requirements 9.6, 9.7**

## Error Handling

All user-facing messages are in Spanish. `AppError` is mapped to HTTP status codes by the central error layer:

| Scenario                                    | Variant                          | Status | Message (Spanish)                                          |
|---------------------------------------------|----------------------------------|--------|------------------------------------------------------------|
| Cross-tenant receipt access                 | `AppError::NotFound`             | 404    | `"Recibo no encontrado"`                                   |
| Self-register duplicate email               | `AppError::Conflict`             | 409    | `"El correo ya está registrado"`                           |
| Delete a sealed documento                   | `AppError::Forbidden`            | 403    | `"No se puede eliminar un documento sellado"`              |
| SMTP send failure                           | `AppError::BadGateway`           | 502    | `"No se pudo enviar el correo"` (no SMTP creds in payload) |
| OCR validation failure                      | `AppError::UnprocessableEntity`  | 422    | `"Datos de OCR inválidos"`                                 |
| `fecha_desde > fecha_hasta`                 | `AppError::BadRequest`           | 400    | `"fecha_desde no puede ser posterior a fecha_hasta"`       |
| `Gasto.categoria` outside enum              | `AppError::UnprocessableEntity`  | 422    | `"Categoría de gasto no válida"`                           |
| Multi-turn loop hit turn limit              | logged, surfaced as final text   | 200    | `"Disculpa, no pude completar tu solicitud..."`            |

Note on Requirement 3.3: requirements specify `409` for sealed-doc delete. The user's design instruction says `403`. We honor the user instruction: handler returns `Forbidden` (`403`). Tests assert `403`.

`tracing` events for security-relevant failures use `target = "security.cross_tenant"` and never echo SMTP credentials or tokens.

## Testing Strategy

**New backend test files** (each tagged with `// Feature: spec-gap-remediation`):

- `backend/tests/recibos_pbt.rs` — Property 1.
- `backend/tests/auth_pbt.rs` — Property 2.
- `backend/tests/firmas_pbt.rs` — Property 3.
- `backend/tests/importacion_pbt.rs` — Properties 4 and 5.
- `backend/tests/mantenimiento_pbt.rs` — Property 6.
- `backend/tests/ai_module_pbt.rs` — Property 7.
- `backend/tests/gastos_pbt.rs` — Property 8.

Every property test runs at least `crate::pbt_cases()` iterations (default 100). Each test header carries `// Feature: spec-gap-remediation, Property N: <text>`.

**SMTP integration**: the `firmas_pbt.rs` and an additional `mail_integration_test.rs` use `lettre`'s file transport (`AsyncFileTransport`) so we can assert the message contents (Spanish subject, link, recipient) without reaching real SMTP. A separate, gated integration test against a Mailcow staging mailbox lives under `backend/tests/integration/mail.rs` behind `#[ignore]`.

**Frontend init-logs PBT**: restored at `frontend/tests/init_logging_tests.rs` using `wasm-bindgen-test` headless. Iterates random route/auth combinations and asserts all six `[INIT]` markers appear; on failure, the counterexample names the missing stage.

**Example/edge-case tests** (non-PBT) cover:

- Spanish copy on the verification action and compliance badge.
- Manifest fields and service-worker registration.
- `Visualizador` role hides verification controls.
- Categoría enum negative test (random non-enum strings → 422).
- Sealed-doc delete returns 403 with Spanish message.
- Mock `MailClient::send` failure → 502 and no SMTP password in body.

Per `backend.md` steering: every multi-step write (registration org+user, sealed-doc + file, OCR confirm) runs in a single `txn`; list reads use `is_in()` for batching where applicable; no raw SQL anywhere. Per `frontend.md`: hooks use `use_effect_with` with explicit deps; Decimal-as-string deserializers stay; no `html!` block exceeds 150 lines (calendar and dashboard split into sub-components). Per `corrections.md`: OVMS endpoint remains `/v3`, OCR is invoked only as a Rig tool, configuration lives in K8s manifests (not docker-compose). Per `lessons-learned.md`: `gloo-net 0.6` and `actix-governor 0.6` versions remain pinned; Trunk SPA fallback covers the new `/documentos/por-vencer`, `/calendario`, and `/categorias-gastos` routes.
