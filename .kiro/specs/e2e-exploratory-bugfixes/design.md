# E2E Exploratory Bugfixes — Bugfix Design

## Overview

A production exploratory E2E pass against https://gestion.myhomeva.us surfaced nine confirmed
defects across the Rust/Actix-web + SeaORM backend and the Rust/Yew (WASM) frontend. This design
formalizes each defect using the bug-condition methodology (`F` = current behavior, `F'` = fixed
behavior), identifies the concrete root cause and fix location from the actual source, and defines
how each fix maps to the Fix Check and Preservation Check from `bugfix.md`.

The nine bugs group as follows:

- **Frontend behavior/layout (1–4):** a global 401 interceptor swallows login errors; registration
  name validation is sticky; the Pagos page renders pagination twice; and a flexbox overlap lets the
  navbar-right group intercept taps on the mobile hamburger.
- **Backend RBAC/serialization/routing (5–8):** NCF reads are blocked by a service-layer *fiscal*
  gate (not RBAC); Invitaciones returns a bare JSON array instead of a `PaginatedResponse`; the
  Servicios Públicos property dropdown called a non-existent route (already corrected in source);
  and document thumbnails request a non-existent URL because of a no-op string trim.
- **Configuration/CSP (9):** the frontend Caddy CSP that reaches the browser omits the Cloudflare
  Insights origins (the repo Caddyfile already adds them; the deployed ConfigMap is stale).

Each fix is scoped to the smallest change that satisfies the corresponding Expected Behavior clause
while preserving the Unchanged Behavior clauses. No fix weakens authentication, authorization, or
input validation.

> Note: One frontend bug (7) is **already corrected in the current source tree**. The defect existed
> in the deployed build. This design documents the root cause, confirms the source is aligned, and
> adds a regression guard, rather than introducing a redundant change.

## Glossary

- **Bug_Condition (C)**: The predicate `isBugCondition(X)` identifying inputs/states that trigger a
  defect. `F(X)` is wrong exactly when `C(X)` holds.
- **Property (P)**: The desired behavior `P(result)` for inputs where `C(X)` holds.
- **Preservation**: For every input where `¬C(X)`, the fixed system must match the original:
  `F(X) = F'(X)`.
- **`F` / `F'`**: Original (unfixed) and fixed behavior, respectively.
- **`AdminOnly` / `WriteAccess`**: Actix `FromRequest` RBAC extractors in
  `backend/src/middleware/rbac.rs`. `AdminOnly` allows `rol == "admin"`; `WriteAccess` allows
  `admin` or `gerente`.
- **Fiscal-access gate**: `verificar_acceso_fiscal` / `obtener_org_con_acceso_fiscal` in
  `backend/src/services/fiscal.rs`, which returns `403 Forbidden` when `organizacion.tipo_fiscal ==
  "informal"`. Distinct from RBAC.
- **`PaginatedResponse<T>`**: The camelCase pagination envelope `{ data, total, page, perPage }`
  (backend: `backend/src/models/mod.rs`; frontend: `frontend/src/types`).
- **`handle_response`**: The shared HTTP response handler in `frontend/src/services/api.rs` that
  applies global 401 handling and error humanization.
- **`serve_upload`**: The authenticated file-serving route `GET /uploads/{path:.*}` in
  `backend/src/app.rs` that streams files from `UPLOAD_DIR`.
- **`gi-navbar-left` / `gi-navbar-right`**: Flex containers in the navbar (`frontend/src/components/layout/navbar.rs`,
  styled in `frontend/styles/tailwind.css`).

## Bug Details

For each bug below: the formal Bug Condition, concrete examples, the confirmed/hypothesized root
cause with the exact file/function, the fix, and the preservation scope.

---

### Bug 1 — Login 401 redirects to landing instead of showing an error (HIGH)

_Requirements: 1.1 / 2.1 / preserve 3.1_

**Bug Condition**

```
FUNCTION isBugCondition_login(X)
  INPUT: X = { email: valid-format, password, authResult }
  OUTPUT: boolean
  RETURN X.authResult = HTTP_401 AND X.route_before = "/login"
END FUNCTION
```

**Examples**

- Submit `/login` with `user@example.com` + wrong password → backend `POST /api/v1/auth/login`
  returns 401 → app navigates to `/` with no error. **Expected:** stay on `/login`, show
  "Credenciales inválidas".

**Root Cause (confirmed)**

`frontend/src/services/api.rs::handle_response` treats **every** 401 as an expired session:

```rust
if response.status() == 401 {
    clear_token_and_redirect();           // window.location.set_href("/")
    return Err("Sesión expirada. Redirigiendo al inicio de sesión.".into());
}
```

`login()` calls `api_post("/auth/login", ..)`, which routes through `handle_response`. On a failed
login the 401 triggers a hard redirect to `/` before `LoginForm`'s `friendly_server_error` (which
already maps 401 → "Correo o contraseña incorrectos…") can render. The redirect is correct for an
*expired session on an authenticated request*, but wrong for the login endpoint, where no session
exists yet.

**Fix** — `frontend/src/services/api.rs`, `handle_response`

Only perform the clear-and-redirect when a token is actually present (i.e., a genuine session
expiry on an authenticated request). When there is no token (login/registration attempts), fall
through to the normal error path so the form surfaces the message and stays on `/login`:

```rust
if response.status() == 401 {
    if get_token().is_some() {
        clear_token_and_redirect();
        return Err("Sesión expirada. Redirigiendo al inicio de sesión.".into());
    }
    // no token → not a session expiry; fall through to error humanization
}
```

The existing `!response.ok()` branch then returns a 401 message (the `_` arm of `humanize_error`
yields a string containing "401", which `LoginForm::friendly_server_error` maps to a friendly
Spanish message). Optionally add an explicit `401 =>` arm to `humanize_error` returning
"Credenciales inválidas." for clarity. No change to the success path.

**Preservation (¬C):** valid credentials still authenticate and navigate to the dashboard (3.1);
expired sessions on authenticated pages still clear the token and redirect to `/` (token present).

---

### Bug 2 — Registration "El nombre es obligatorio" is sticky (MEDIUM)

_Requirements: 1.2 / 2.2 / preserve 3.2, 3.3_

**Bug Condition**

```
FUNCTION isBugCondition_nombre(X)
  INPUT: X = { nombre, priorEmptySubmission: boolean }
  OUTPUT: boolean
  RETURN nonEmpty(X.nombre) AND X.priorEmptySubmission = true
END FUNCTION
```

**Examples**

- Submit `/registro` with empty Nombre → "El nombre es obligatorio" shown. Type "Test User", submit
  again → error still shown. **Expected:** error clears and the name validation passes.

**Root Cause (hypothesized — confirm in exploratory phase)**

`frontend/src/components/auth/register_form.rs` validates `nombre` **only on submit**
(`validate_nombre(&nombre)` inside `on_submit`); there is no per-field revalidation on input. The
`nombre` field uses the generic `make_handler` (sets state only), so the "El nombre es obligatorio"
banner persists after the first failed submit until the next full submit. The reported persistence
*after* re-submit indicates the displayed `nombre_error` is not being recomputed against the
current field value at the moment of the second submit (sticky validation state / stale value
captured by the field error rendering).

Two candidate mechanisms (the exploratory test will confirm or refute):
1. The error state is updated only on submit and the user perceives/asserts persistence; or
2. The submitted value read by `validate_nombre` does not reflect the latest input at re-submit.

**Fix** — `frontend/src/components/auth/register_form.rs`

Add live revalidation for `nombre` so the error clears as soon as a non-empty value is entered, and
ensure the submit path re-reads the current value. Replace the generic handler for `nombre` with one
that revalidates:

```rust
let on_nombre_change = {
    let nombre = nombre.clone();
    let nombre_error = nombre_error.clone();
    Callback::from(move |e: InputEvent| {
        let input: web_sys::HtmlInputElement = e.target_unchecked_into();
        let value = input.value();
        nombre_error.set(validate_nombre(&value)); // clears once non-empty
        nombre.set(value);
    })
};
```

This guarantees the Fix Check (non-empty name after a prior empty submit ⇒ no error) regardless of
which mechanism is at play, and is consistent with the live-validation pattern.

**Preservation (¬C):** an empty/whitespace Nombre still errors on submit (3.3 — `validate_nombre`
unchanged and still runs in `on_submit`); fully valid submissions still create the account (3.2).

---

### Bug 3 — Pagos renders two pagination bars (LOW)

_Requirements: 1.3 / 2.3 / preserve 3.4_

**Bug Condition**

```
FUNCTION isBugCondition_pagos(X)
  INPUT: X = rendered /pagos view (desktop viewport)
  OUTPUT: boolean
  RETURN countPaginationBars(X) > 1
END FUNCTION
```

**Examples**

- Open `/pagos` on desktop → two identical pagination bars ("Mostrando 1–6 de 6"), one just below
  the table, one at the page bottom. **Expected:** exactly one.

**Root Cause (confirmed)**

`frontend/src/pages/pagos.rs` renders pagination twice:
1. Inside the `PagoList` component (`<Pagination …/>` after `</table>`), which is mounted inside the
   desktop-only wrapper `<div class="gi-mobile-hidden"><PagoList …/></div>`.
2. A second page-level `<Pagination …/>` after `MobileCardList`, which is **not** wrapped in any
   responsive class, so it shows on every viewport.

On desktop both are visible → two bars. On mobile only the page-level one shows.

**Fix** — `frontend/src/pages/pagos.rs`

Remove the `<Pagination>` rendered **inside `PagoList`** and keep the single page-level
`<Pagination>` (it already serves both desktop and the mobile card list with identical
`total/page/per_page` and the same `on_page_change`/`on_per_page_change` callbacks). Then remove the
now-unused `total`, `page`, `per_page`, `on_page_change`, and `on_per_page_change` from
`PagoListProps` and its call site (clean up orphans created by this change).

**Preservation (¬C):** pagination range text, page navigation, and per-page selection are unchanged
because the retained bar uses the same state and callbacks (3.4).

---

### Bug 4 — Mobile hamburger intercepted by an SVG in `.gi-navbar-right` (HIGH)

_Requirements: 1.4 / 2.4 / preserve 3.5_

**Bug Condition**

```
FUNCTION isBugCondition_hamburger(X)
  INPUT: X = { viewportWidth, authenticated: boolean }
  OUTPUT: boolean
  RETURN X.viewportWidth <= 375 AND X.authenticated = true
         AND pointerInterceptedBy(clickHamburger(X)) != "hamburger-button"
END FUNCTION
```

**Examples**

- 375px viewport, authenticated, tap "Abrir menú" → an SVG inside `.gi-navbar-right` receives the
  event; the sidebar never opens. **Expected:** the hamburger receives the click and the menu opens.

**Root Cause (confirmed)**

In `frontend/styles/tailwind.css`, the navbar is a flex row with
`.gi-navbar { justify-content: space-between }`, `.gi-navbar-left { min-width: 0 }` (contains the
`.gi-hamburger`, the title is `display:none` on mobile), and `.gi-navbar-right { flex-shrink: 0 }`
(search button + ⌘K kbd + notification bell + theme toggle + logout). On a ≤375px viewport the right
group cannot shrink, so the negative free space under `space-between` causes the right group to
**overlap** the far-left hamburger. Because `.gi-navbar-right` comes later in DOM order, it paints on
top and its leading SVG intercepts the pointer over the hamburger.

**Fix** — `frontend/styles/tailwind.css`

1. Guarantee the hamburger always receives the pointer by raising its stacking context (it is
   `display:none` on desktop, so this is mobile-only in effect):

   ```css
   .gi-hamburger { position: relative; z-index: 50; }
   ```

2. Prevent the visual overlap on small screens inside the existing mobile media query: allow the
   right group to shrink and reduce its footprint (hide the low-priority `.gi-kbd` shortcut and the
   "Ir a…" search label at small widths):

   ```css
   @media (max-width: 768px) {
     .gi-navbar-right { min-width: 0; flex-wrap: wrap; }
     .gi-kbd { display: none; }
   }
   ```

**Preservation (¬C):** desktop/wider viewports are unaffected — the hamburger is `display:none`
there (z-index has no visible effect), and the right-group changes are scoped to the mobile media
query, so `.gi-navbar-right` renders and functions exactly as before on desktop (3.5).

---

### Bug 5 — NCF `GET /ncf/secuencias` returns 403 for admin (HIGH)

_Requirements: 1.5 / 2.5 / preserve 3.6_

**Bug Condition**

```
FUNCTION isBugCondition_ncf(X)
  INPUT: X = { role, endpoint, org.tipo_fiscal }
  OUTPUT: boolean
  RETURN X.role = "admin" AND X.endpoint = "GET /api/v1/ncf/secuencias"
         AND callNcf(X).status = 403
END FUNCTION
```

**Examples**

- Admin of a newly-registered org opens `/ncf` → `GET /api/v1/ncf/secuencias` returns 403, page
  shows "No tiene permisos para realizar esta acción." **Expected:** 200 with the (possibly empty)
  sequence list.

**Root Cause (confirmed)**

The 403 is **not** from RBAC. `handlers::ncf::listar_secuencias` correctly uses `AdminOnly`, which
admits `rol == "admin"`. The block comes from a **service-layer fiscal gate**:
`services::ncf::listar_secuencias` calls `obtener_org_con_acceso_fiscal(db, org_id)`, which calls
`verificar_acceso_fiscal` (`services/fiscal.rs`) and returns
`AppError::Forbidden("Funciones fiscales requieren registro en DGII")` when
`organizacion.tipo_fiscal == "informal"`. New organizations are created with
`tipo_fiscal: "informal"` (see `services::auth::register_new_org`), so an admin of such an org gets
403 on a read endpoint exposed by the UI.

**Fix** — `backend/src/services/ncf.rs`, `listar_secuencias`

Remove the fiscal-access gate from the **read** path so any admin can list sequences (an informal
org simply has none, and the page renders its empty state). RBAC (`AdminOnly`) and the multi-tenant
`organizacion_id` filter are retained.

```rust
pub async fn listar_secuencias(db: &DatabaseConnection, org_id: Uuid)
    -> Result<Vec<SecuenciaNcfResponse>, AppError> {
    // (removed) obtener_org_con_acceso_fiscal(db, org_id).await?;
    let secuencias = secuencia_ncf::Entity::find()
        .filter(secuencia_ncf::Column::OrganizacionId.eq(org_id))
        .all(db).await?;
    Ok(secuencias.iter().map(to_response).collect())
}
```

The fiscal gate **stays** on the write/assignment paths (`configurar_rango_con_acceso`,
`asignar_ncf`) — only reading the list is ungated. `obtener_alertas` is out of scope (the `/ncf`
page only calls `secuencias`).

**Preservation (¬C):** non-admin roles (`gerente`, `visualizador`) still receive 403 from the
`AdminOnly` extractor on all NCF endpoints (3.6); NCF *configuration* still requires fiscal access.

---

### Bug 6 — Invitaciones empty list fails to deserialize (MEDIUM)

_Requirements: 1.6 / 2.6 / preserve 3.7, 3.8_

**Bug Condition**

```
FUNCTION isBugCondition_invitaciones(X)
  INPUT: X = invitations dataset for the org
  OUTPUT: boolean
  RETURN responseShape(getInvitaciones(X)) != PaginatedResponse   // backend returns a bare array
END FUNCTION
```

(The empty dataset is where the frontend first observes the failure: `[]` cannot be read as a
4-field struct.)

**Examples**

- Open `/invitaciones` with no invitations → frontend alert: "Error al procesar respuesta: invalid
  length 0, expected struct PaginatedResponse with 4 elements at line 1 column 2". **Expected:** an
  empty, well-formed `PaginatedResponse` and an empty-state UI.

**Root Cause (confirmed)**

`services::invitaciones::listar` returns `Vec<InvitacionResponse>` and
`handlers::invitaciones::listar` serializes it directly (`HttpResponse::Ok().json(result)`), so the
body is a JSON **array**. The frontend calls
`api_get::<PaginatedResponse<Invitacion>>("/invitaciones?page=..&perPage=..")`, expecting the object
`{ data, total, page, perPage }`. Deserializing `[]` as a 4-field struct yields the reported serde
error. (Non-empty arrays would also fail; the empty case is simply what the E2E hit.)

**Fix** — backend, following the canonical paginated-list pattern (e.g. `services::inquilinos::list`)

1. `backend/src/models/invitacion.rs`: add a query DTO
   ```rust
   #[derive(serde::Deserialize)]
   #[serde(rename_all = "camelCase")]
   pub struct InvitacionListQuery { pub page: Option<u64>, pub per_page: Option<u64> }
   ```
2. `backend/src/services/invitaciones.rs`: change `listar` to return
   `PaginatedResponse<InvitacionResponse>`, paginating the existing filtered query
   (`OrganizacionId == org_id`, not used, not expired) with `.paginate(db, per_page)` /
   `num_items()` / `fetch_page(page-1)` — mirroring `inquilinos::list`.
3. `backend/src/handlers/invitaciones.rs`: accept `query: web::Query<InvitacionListQuery>` and pass
   `page`/`per_page` through; return the envelope. The empty case now returns
   `{ "data": [], "total": 0, "page": 1, "perPage": 20 }`.

**Preservation (¬C):** populated invitation lists now also deserialize/render correctly (3.7); no
other `PaginatedResponse`-backed endpoint is touched, so their behavior is unchanged (3.8).

---

### Bug 7 — Servicios Públicos called a non-existent property route (MEDIUM)

_Requirements: 1.7 / 2.7 / preserve 3.9_

**Bug Condition**

```
FUNCTION isBugCondition_servicios(X)
  INPUT: X = property-list request from /servicios-publicos
  OUTPUT: boolean
  RETURN requestedPath(X) = "GET /api/v1/propiedades/todas"   // non-existent route
END FUNCTION
```

**Examples**

- Deployed build: opening `/servicios-publicos` requested `GET /api/v1/propiedades/todas` → 404 →
  empty property dropdown. **Expected:** call an existing route and load properties.

**Root Cause (confirmed) & current state**

The deployed build requested `/propiedades/todas`, which has no route (`routes.rs::configure_propiedades`
exposes `GET /propiedades` → `propiedades::list`, but no `/todas`). **The current source is already
corrected**: `frontend/src/pages/servicios_publicos.rs` loads properties via
`api_get::<PaginatedResponse<Propiedad>>("/propiedades?perPage=200")`, which maps to the existing
`GET /api/v1/propiedades` handler. A repo-wide search finds no remaining caller of
`/propiedades/todas`.

**Fix**

No code change is required in the current source — the endpoint is already correct. The action is
to (a) verify no other caller exists (confirmed) and (b) add a regression guard so the dropdown
keeps calling an existing endpoint, and ensure the corrected frontend is deployed. If a stray caller
were found, it would be repointed to `/propiedades?perPage=200`.

**Preservation (¬C):** all other API calls on the page (units `/propiedades/{id}/unidades`,
servicios `/propiedades/{id}/unidades/{unit}/servicios`) are unchanged (3.9).

---

### Bug 8 — Property document thumbnail 404 (LOW)

_Requirements: 1.8 / 2.8 / preserve 3.10_

**Bug Condition**

```
FUNCTION isBugCondition_docimg(X)
  INPUT: X = { propiedadId, filename }  // a stored document exists on disk
  OUTPUT: boolean
  RETURN documentExists(X) AND serveDocument(X).status = 404
END FUNCTION
```

**Examples**

- Edit property "Higey" with an attached image → the gallery `<img>` requests
  `GET /api/v1/propiedad/{id}/{filename}` (e.g. `…/Eg3tKKlWsAA0v_w (1).jpg`) → 404, thumbnail
  broken. **Expected:** the image is served (200) and displays.

**Root Cause (confirmed)**

`frontend/src/components/common/document_gallery.rs::DocumentCard` builds:

```rust
let file_url = format!("{}/{}", BASE_URL.trim_end_matches("/api"), doc.file_path);
```

`BASE_URL` is `"/api/v1"`. `trim_end_matches("/api")` is a **no-op** because the string ends with
`"/v1"`, not `"/api"`. The result is `"/api/v1/" + doc.file_path`, and `doc.file_path` is
`propiedad/{entity_id}/{uuid}-{filename}` (see `services::documentos::upload`), producing
`GET /api/v1/propiedad/{id}/{file}` — a path with **no matching route**, hence 404.

The real file-serving route is `GET /uploads/{path:.*}` (`app.rs::serve_upload`), which streams
`{UPLOAD_DIR}/{path}`. Two issues must both be addressed for the thumbnail to display:
1. **Wrong URL base** (`/api/v1/…` instead of `/uploads/…`).
2. `serve_upload` requires a JWT (`Claims`) and sets `Content-Disposition: attachment`; a plain
   `<img src>` cannot send an `Authorization` header, so even the correct URL would 401.

**Fix** — `frontend/src/components/common/document_gallery.rs`

1. Build the URL against the real route: `let file_path = format!("/uploads/{}", doc.file_path);`
2. Because `/uploads` is authenticated, load protected files via an **authenticated blob fetch**
   (mirror the existing `services::api::api_download` pattern): fetch `file_path` with the
   `Authorization: Bearer {token}` header, build a `Blob`, create an object URL with
   `web_sys::Url::create_object_url_with_blob`, and use that as the `<img>`/`<embed>` `src`; revoke
   the object URL on cleanup. `DocumentCard` already receives `token`.

   This keeps the file-serving endpoint authenticated (no security relaxation) while letting the
   thumbnail render. URL-encoding of filenames with spaces/parens is handled by the fetch layer; the
   stored `file_path` is used verbatim end-to-end.

**Preservation (¬C):** missing files still return 404 (`serve_upload` → `NotFound`), and directory
traversal is still rejected (`serve_upload` rejects `..`), so well-formed/existing files continue to
serve while invalid requests behave as before (3.10).

> Security note (out of scope — flag for follow-up, do not expand here): `serve_upload` authenticates
> the caller but does **not** verify that the requested file's entity belongs to the caller's
> organization, so a knowledgeable authenticated user from another org could read files by path. This
> is a latent cross-tenant IDOR unrelated to the 9 E2E bugs; recommend a separate fix that scopes
> `serve_upload` by `organizacion_id`.

---

### Bug 9 — CSP blocks Cloudflare Insights site-wide (LOW)

_Requirements: 1.9 / 2.9 / preserve 3.11_

**Bug Condition**

```
FUNCTION isBugCondition_csp(X)
  INPUT: X = script asset request on any page
  OUTPUT: boolean
  RETURN X.src = "https://static.cloudflareinsights.com/beacon.min.js"
         AND cspBlocks(X) = true
END FUNCTION
```

**Examples**

- Any page load logs CSP violations for `https://static.cloudflareinsights.com/beacon.min.js` (and
  its inline bootstrap) under `script-src 'self' 'wasm-unsafe-eval'`. **Expected:** no CSP violation
  for legitimate assets (allow the beacon, or remove it).

**Root Cause (confirmed)**

The browser-facing CSP is set by the frontend Caddy server, not the backend (`index.html` has no
`<meta>` CSP; the backend `security_headers` middleware sets a separate `default-src 'none'` CSP
that applies only to API responses). The Caddy CSP lives in `infra/caddy/Caddyfile` (line ~16) and
`infra/k8s/app/overlays/prod/Caddyfile` (line ~19), delivered to the frontend pod via
`configMapGenerator: caddyfile` (prod `kustomization.yml`).

The **repo** Caddyfiles already contain the corrected CSP:

```
script-src 'self' 'wasm-unsafe-eval' https://static.cloudflareinsights.com;
… connect-src 'self' https://cloudflareinsights.com; …
```

The production CSP observed by the E2E (`script-src 'self' 'wasm-unsafe-eval'`, no Cloudflare)
predates this change — the deployed ConfigMap is **stale**. (The dev overlay Caddyfile has no CSP
header block at all; that is dev-only and out of scope.)

**Fix** — `infra/caddy/Caddyfile` + `infra/k8s/app/overlays/prod/Caddyfile`

1. Ensure the deployed `caddyfile` ConfigMap matches the repo CSP that already allows
   `https://static.cloudflareinsights.com` (`script-src`) and `https://cloudflareinsights.com`
   (`connect-src`), then roll the frontend pods so the new ConfigMap takes effect.
2. For the inline Cloudflare bootstrap: do **not** add `'unsafe-inline'` (that would weaken XSS
   protection and break Preservation 3.11). Cloudflare Web Analytics is injected at the Cloudflare
   edge, not by the app. Choose one:
   - **Recommended:** disable Cloudflare Web Analytics auto-injection for the domain (remove the
     beacon entirely) — eliminates both the external request and the inline snippet, so no CSP
     violation occurs. This matches requirement 2.9's "beacon SHALL be removed if analytics are not
     wanted."
   - **If analytics are wanted:** keep the external-origin allowance (already in the repo) and add a
     specific `'sha256-<hash>'` for the exact inline snippet to `script-src` — never
     `'unsafe-inline'`.

**Preservation (¬C):** first-party assets remain allowed (`default-src 'self'`, `script-src 'self'
'wasm-unsafe-eval'` retained for the WASM app), and all other disallowed third-party origins remain
blocked (no wildcards introduced) (3.11).

## Expected Behavior

### Preservation Requirements (consolidated — must remain unchanged)

- **3.1** Valid login authenticates and redirects to the authenticated landing area.
- **3.2** `/registro` with all valid fields creates the account.
- **3.3** Empty Nombre on `/registro` still shows "El nombre es obligatorio".
- **3.4** `/pagos` single pagination controls (range, page nav, per-page) operate correctly.
- **3.5** Desktop/wider navbar and `.gi-navbar-right` render and function as before.
- **3.6** `gerente`/`visualizador` remain restricted on NCF endpoints (RBAC).
- **3.7** Non-empty `/invitaciones` returns and renders the paginated list.
- **3.8** Other `PaginatedResponse`-backed endpoints (empty or populated) deserialize/render as before.
- **3.9** Other existing, valid backend calls from `/servicios-publicos` continue to succeed.
- **3.10** A document with a well-formed, existing path is served successfully.
- **3.11** Legitimate first-party assets stay allowed; disallowed third-party origins stay blocked.

**Scope:** every fix is gated by its Bug Condition. For all inputs where `¬C(X)` holds, `F'(X) =
F(X)`. The expected *correct* behavior for buggy inputs is captured per-bug in Correctness
Properties below.

## Hypothesized Root Cause

Summary of the root-cause analysis above (confirmed unless noted):

1. **Bug 1 (confirmed):** global 401 interceptor in `api.rs::handle_response` redirects on every
   401, including the unauthenticated login endpoint.
2. **Bug 2 (hypothesized):** submit-only name validation leaves a sticky `nombre_error`; confirm
   the exact trigger in the exploratory phase, fix via live revalidation.
3. **Bug 3 (confirmed):** two `<Pagination>` instances — one in `PagoList` (desktop wrapper) and one
   page-level.
4. **Bug 4 (confirmed):** `flex-shrink: 0` on `.gi-navbar-right` + `space-between` overlap on
   ≤375px; later DOM element paints over the hamburger.
5. **Bug 5 (confirmed):** service-layer fiscal gate (`verificar_acceso_fiscal`) returns 403 for
   `informal` orgs on a read endpoint; RBAC is fine.
6. **Bug 6 (confirmed):** handler/service return a bare `Vec` instead of `PaginatedResponse`.
7. **Bug 7 (confirmed/already fixed in source):** deployed build called non-existent
   `/propiedades/todas`; source already uses `/propiedades?perPage=200`.
8. **Bug 8 (confirmed):** no-op `trim_end_matches("/api")` builds `/api/v1/{file_path}` (no route);
   real route is authenticated `/uploads/{path}`.
9. **Bug 9 (confirmed):** browser CSP comes from the Caddy ConfigMap; repo already allows Cloudflare
   Insights but the deployed ConfigMap is stale; inline beacon needs removal or a hash (not
   `'unsafe-inline'`).

## Correctness Properties

This section is the single source of truth for correctness properties. Each bug contributes a
Bug-Condition property (the fix) and a Preservation property (no regression).

Property 1: Bug Condition - Login 401 surfaces an error and stays on /login

_For any_ login submission where the backend returns 401 (isBugCondition_login holds), the fixed
app SHALL remain on `/login` and display a credentials error, and SHALL NOT navigate to `/`.

**Validates: Requirements 2.1**

Property 2: Preservation - Authenticated flows and session expiry unchanged

_For any_ request where the login-401 condition does NOT hold (valid login, or a 401 on a request
that carries a token), the fixed app SHALL behave identically to the original: valid logins reach
the dashboard, and token-bearing 401s still clear the token and redirect to `/`.

**Validates: Requirements 3.1**

Property 3: Bug Condition - Filled Nombre clears the validation error

_For any_ `/registro` state where Nombre is non-empty after a prior empty submission
(isBugCondition_nombre holds), the fixed form SHALL report no `nombre` error.

**Validates: Requirements 2.2**

Property 4: Preservation - Empty-name error and valid submissions unchanged

_For any_ state where the condition does NOT hold, the fixed form SHALL match the original: an
empty/whitespace Nombre still errors with "El nombre es obligatorio", and a fully valid form still
submits and creates the account.

**Validates: Requirements 3.2, 3.3**

Property 5: Bug Condition - Pagos shows exactly one pagination bar

_For any_ rendered `/pagos` view (isBugCondition_pagos holds when >1 bar), the fixed page SHALL
render exactly one pagination bar.

**Validates: Requirements 2.3**

Property 6: Preservation - Pagination range and navigation unchanged

_For any_ `/pagos` dataset, the retained pagination bar's range text, page navigation, and per-page
selection SHALL be identical to the original behavior.

**Validates: Requirements 3.4**

Property 7: Bug Condition - Mobile hamburger receives the click

_For any_ authenticated view at viewport width ≤375px (isBugCondition_hamburger holds), the fixed
layout SHALL deliver the pointer event to the hamburger button and open the mobile navigation.

**Validates: Requirements 2.4**

Property 8: Preservation - Desktop navbar unchanged

_For any_ wider/desktop viewport, the navbar and `.gi-navbar-right` SHALL render and function
exactly as before.

**Validates: Requirements 3.5**

Property 9: Bug Condition - Admin can read NCF sequences

_For any_ admin request to `GET /api/v1/ncf/secuencias` (isBugCondition_ncf holds when it 403s), the
fixed backend SHALL return 200 with the organization's sequences (possibly empty).

**Validates: Requirements 2.5**

Property 10: Preservation - Non-admin NCF restrictions unchanged

_For any_ NCF request from a non-admin role (`gerente`, `visualizador`), the fixed backend SHALL
still return 403, and NCF configuration SHALL still require fiscal access.

**Validates: Requirements 3.6**

Property 11: Bug Condition - Empty Invitaciones returns a well-formed PaginatedResponse

_For any_ invitations dataset that is empty (isBugCondition_invitaciones holds), the fixed backend
SHALL return a well-formed `PaginatedResponse` with `data = []` and `total = 0`, and the frontend
SHALL render the empty state without a deserialization error.

**Validates: Requirements 2.6**

Property 12: Preservation - Populated invitations and other paginated endpoints unchanged

_For any_ non-empty invitations dataset, the fixed backend SHALL return and the frontend SHALL
render the paginated list correctly; all other `PaginatedResponse`-backed endpoints SHALL be
unaffected.

**Validates: Requirements 3.7, 3.8**

Property 13: Bug Condition - Servicios Públicos calls an existing property endpoint

_For any_ load of `/servicios-publicos` (isBugCondition_servicios holds when the missing
`/propiedades/todas` is requested), the fixed frontend SHALL request the existing
`GET /api/v1/propiedades` endpoint and the dropdown SHALL load (no 404).

**Validates: Requirements 2.7**

Property 14: Preservation - Other Servicios Públicos calls unchanged

_For any_ other API call on the page (units, servicios), the fixed frontend SHALL continue to call
the same existing endpoints successfully.

**Validates: Requirements 3.9**

Property 15: Bug Condition - Stored document image is served

_For any_ existing stored document (isBugCondition_docimg holds when it 404s), the fixed frontend
SHALL request the correct `/uploads/{file_path}` route with authentication and the thumbnail SHALL
display (HTTP 200).

**Validates: Requirements 2.8**

Property 16: Preservation - Missing files and traversal unchanged

_For any_ request where the document does not exist or the path is a traversal attempt, the fixed
system SHALL behave as before (404 for missing files, rejection for `..`).

**Validates: Requirements 3.10**

Property 17: Bug Condition - CSP no longer blocks Cloudflare Insights

_For any_ page load requesting the Cloudflare Insights beacon (isBugCondition_csp holds), the fixed
CSP SHALL not produce a violation — either by allowing the Cloudflare Insights origins or by
removing the beacon entirely.

**Validates: Requirements 2.9**

Property 18: Preservation - First-party allowed, third-party still blocked

_For any_ other asset, the fixed CSP SHALL still allow legitimate first-party scripts/assets
(`'self' 'wasm-unsafe-eval'`) and SHALL still block disallowed third-party origins.

**Validates: Requirements 3.11**

## Fix Implementation

Summary of concrete changes (full detail per bug above). New domain code follows the project layout
(backend: models → service → handler; frontend: types → services → components → pages).

| Bug | File(s) | Change |
|---|---|---|
| 1 | `frontend/src/services/api.rs` | Guard the 401 clear-and-redirect on `get_token().is_some()`; (optional) add explicit `401 =>` arm in `humanize_error`. |
| 2 | `frontend/src/components/auth/register_form.rs` | Live `nombre` revalidation on input; keep submit-time `validate_nombre`. |
| 3 | `frontend/src/pages/pagos.rs` | Remove the `<Pagination>` inside `PagoList`; drop now-unused pagination props from `PagoListProps` and its call site. |
| 4 | `frontend/styles/tailwind.css` | `.gi-hamburger { position: relative; z-index: 50 }`; in `@media (max-width:768px)` allow `.gi-navbar-right` to shrink/wrap and hide `.gi-kbd`. |
| 5 | `backend/src/services/ncf.rs` | Remove `obtener_org_con_acceso_fiscal` from `listar_secuencias` (read); keep it on write/assign paths. |
| 6 | `backend/src/models/invitacion.rs`, `backend/src/services/invitaciones.rs`, `backend/src/handlers/invitaciones.rs` | Add `InvitacionListQuery`; return `PaginatedResponse<InvitacionResponse>`; handler reads `page`/`perPage`. |
| 7 | (none — source already correct) | Verify `/propiedades?perPage=200`; add regression guard; ensure corrected build is deployed. |
| 8 | `frontend/src/components/common/document_gallery.rs` | Build `/uploads/{file_path}`; load protected files via authenticated blob fetch (mirror `api_download`); revoke object URLs on cleanup. |
| 9 | `infra/caddy/Caddyfile`, `infra/k8s/app/overlays/prod/Caddyfile` | Ensure deployed CSP matches repo (Cloudflare Insights origins allowed); redeploy ConfigMap; remove edge beacon or add a `sha256` hash for the inline snippet — never `'unsafe-inline'`. |

## Testing Strategy

### Validation Approach

Two phases. First, surface counterexamples on the **unfixed** code (exploratory) to confirm each
root cause. Then verify the fix (Fix Check) and verify no regression (Preservation Check). Backend
work uses Rust `#[cfg(test)]` unit tests and the existing integration-test harness
(`backend/tests/common`); property-based tests use the project's PBT setup (`*_pbt.rs`,
`proptest`/`quickcheck` as already used in `services/*_pbt.rs`). Frontend logic is tested with
`wasm-bindgen-test` / native unit tests for pure functions; layout and end-to-end behavior are
verified manually and via the Playwright E2E pass. CSP/infra changes are verified by inspecting
response headers and the browser console.

### Exploratory Bug Condition Checking

**Goal:** demonstrate each bug on unfixed code and confirm/refute the root cause.

- **Bug 1:** unit-test `handle_response`/login flow asserting a 401 with no token does NOT redirect
  (fails today); manual: wrong password on `/login` redirects to `/`.
- **Bug 2:** component/unit reproduction — empty submit then non-empty Nombre then submit; assert
  `nombre_error` is cleared. If the unfixed code already clears on re-submit, **re-hypothesize**
  (e.g., stale value capture) before finalizing the fix.
- **Bug 3:** render `/pagos` on a desktop viewport; assert exactly one pagination bar (fails today).
- **Bug 4:** Playwright at 375px — click "Abrir menú"; assert the hamburger (not an SVG in
  `.gi-navbar-right`) is the event target and the sidebar opens (fails today).
- **Bug 5:** integration test — admin of an `informal` org calls `GET /ncf/secuencias`; assert 200
  (returns 403 today). Confirms the fiscal gate, not RBAC, is the cause.
- **Bug 6:** integration test — empty invitations; assert the body is a `PaginatedResponse`
  (`data:[]`, `total:0`) (returns `[]` today).
- **Bug 7:** assert no caller of `/propiedades/todas` exists and the dropdown calls
  `/propiedades?perPage=200` (already true in source).
- **Bug 8:** assert the gallery image `src` resolves to `/uploads/{file_path}` and returns 200 with
  auth (today resolves to `/api/v1/{file_path}` → 404).
- **Bug 9:** load any page; assert no CSP violation for `static.cloudflareinsights.com` in the
  console (violations today against the deployed CSP).

### Fix Checking

**Goal:** for all inputs where the bug condition holds, the fixed function yields the expected
behavior.

```
FOR ALL X WHERE isBugCondition_b(X) DO        // b ∈ {1..9}
  ASSERT expectedBehavior_b(F'(X))
END FOR
```

Concretely, assert Properties 1, 3, 5, 7, 9, 11, 13, 15, 17 hold against the fixed system.

### Preservation Checking

**Goal:** for all inputs where the bug condition does NOT hold, the fixed function equals the
original.

```
FOR ALL X WHERE NOT isBugCondition_b(X) DO
  ASSERT F(X) = F'(X)
END FOR
```

Property-based testing is recommended for preservation because it samples broadly across the input
domain and catches edge cases manual tests miss. High-value PBT targets:

- **Bug 5:** generate `{role ∈ {admin,gerente,visualizador}, tipo_fiscal ∈ {informal, persona_fisica,
  persona_juridica}}`; assert non-admin always 403 and admin always 200 on the read endpoint.
- **Bug 6:** generate invitation counts `0..N`; assert the response is always a well-formed
  `PaginatedResponse` with `total == data.len()` for the page and correct `page`/`perPage` echo.
- **Bug 1:** generate `{has_token, status}`; assert redirect happens iff `status==401 && has_token`.

Assert Properties 2, 4, 6, 8, 10, 12, 14, 16, 18 hold.

### Unit Tests

- `api.rs`: 401-with-token redirects; 401-without-token surfaces an error; non-401 unchanged.
- `register_form.rs`: `validate_nombre` unchanged; live-revalidation clears error on non-empty input.
- `ncf.rs`: `listar_secuencias` returns sequences for an `informal` org (no fiscal gate); write paths
  still gated.
- `invitaciones`: service returns `PaginatedResponse`; handler maps `perPage`.
- `document_gallery.rs`: URL builder yields `/uploads/{file_path}` (regression test for the
  `trim_end_matches` bug).

### Property-Based Tests

- NCF role × tipo_fiscal matrix (Bug 5).
- Invitaciones pagination envelope invariants over arbitrary counts/pages (Bug 6).
- 401 redirect predicate over `{has_token, status}` (Bug 1).

### Integration Tests

- Auth: wrong-password login returns 401 and the client does not redirect (Bug 1).
- NCF: admin (informal org) end-to-end 200; non-admin 403 (Bug 5).
- Invitaciones: empty and populated lists round-trip as `PaginatedResponse` (Bug 6).
- Documentos: upload then fetch via `/uploads/{file_path}` returns 200; missing path 404; traversal
  rejected (Bug 8).

### Manual Re-Test (via the app / Playwright)

1. **Bug 1:** `/login` with a wrong password → stays on `/login`, shows "Credenciales inválidas".
2. **Bug 2:** `/registro` empty submit → error; type a name → error clears; submit → proceeds.
3. **Bug 3:** `/pagos` desktop → exactly one pagination bar; navigate pages.
4. **Bug 4:** 375px → tap "Abrir menú" → sidebar opens.
5. **Bug 5:** admin → `/ncf` → list renders (empty state if none), no permission alert.
6. **Bug 6:** `/invitaciones` with none → empty state, no deserialization alert.
7. **Bug 7:** `/servicios-publicos` → property dropdown populates (no 404).
8. **Bug 8:** `/propiedades` → edit a property with a document → thumbnail displays.
9. **Bug 9:** load any page → no CSP violations for Cloudflare Insights in the console.
