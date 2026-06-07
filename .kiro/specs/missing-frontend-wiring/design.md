# Missing Frontend Wiring Bugfix Design

## Overview

Seven backend domains (Desahucios, NCF, Tareas, Invitaciones, Organizacion, DGII, Servicios Públicos) have fully functional API endpoints but no corresponding frontend pages, routes, or sidebar navigation links. Users encounter 404 pages when navigating to these paths. The fix involves creating the complete frontend layer for each domain following the established pattern: types → API service calls → page component → Route enum variant → sidebar link.

## Glossary

- **Bug_Condition (C)**: Navigation to any of the 7 missing routes (`/desahucios`, `/ncf`, `/tareas`, `/invitaciones`, `/organizacion`, `/dgii`, `/servicios-publicos`) results in a 404 page
- **Property (P)**: Each route renders its corresponding CRUD/listing page with proper data loading, error handling, and role-based visibility
- **Preservation**: All existing routes, sidebar links, page components, and API behaviors remain unchanged after the fix
- **Route enum**: The `Route` derive enum in `frontend/src/app.rs` using `yew_router::Routable`
- **ProtectedRoute**: Wrapper component that checks auth context and redirects to login if unauthenticated
- **Sidebar**: The `gi-sidebar` component in `components/layout/sidebar.rs` that renders `Link<Route>` elements organized into `gi-sidebar-group` sections
- **WriteAccess/AdminOnly**: Backend RBAC extractors; frontend mirrors via `can_write` and `is_admin` utility checks on `user.rol`

## Bug Details

### Bug Condition

The bug manifests when an authenticated user navigates to any of the 7 unregistered paths. The Yew Router's `#[not_found]` handler catches these paths before any `ProtectedRoute` check can execute, rendering the "404 — Página no encontrada" page. Additionally, no sidebar links exist for these routes, so users have no UI affordance to discover these features.

**Formal Specification:**
```
FUNCTION isBugCondition(input)
  INPUT: input of type NavigationEvent { path: String, authenticated: bool }
  OUTPUT: boolean

  LET missing_paths = ["/desahucios", "/ncf", "/tareas", "/invitaciones",
                       "/organizacion", "/dgii", "/servicios-publicos"]

  RETURN input.path IN missing_paths
         AND Route::recognize(input.path) == None
         AND renders_404_page(input.path)
END FUNCTION
```

### Examples

- User navigates to `/desahucios` → sees "404 — Página no encontrada" instead of a desahucios listing table
- User navigates to `/ncf` → sees 404 instead of NCF sequence configuration page
- User navigates to `/tareas` → sees 404 instead of background job history
- User opens sidebar → no links for "Desahucios", "NCF", "Tareas", "Invitaciones", "Organización", "DGII", or "Servicios Públicos" are visible
- Unauthenticated user navigates to `/organizacion` → sees 404 instead of being redirected to `/` (login)

## Expected Behavior

### Preservation Requirements

**Unchanged Behaviors:**
- All existing Route enum variants (`Dashboard`, `Propiedades`, `Inquilinos`, `Contratos`, `Pagos`, `Gastos`, `Mantenimiento`, etc.) continue to resolve to their current pages
- All existing sidebar links in "Operaciones", "Herramientas", and "Sistema" groups remain intact
- Mouse clicks on all existing sidebar links continue to route without full-page reload
- The `ProtectedRoute` component continues redirecting unauthenticated users to `/`
- Backend API endpoints for existing domains return identical responses
- The `NotFound` route continues to handle truly unknown paths

**Scope:**
All navigation events that do NOT target the 7 new paths should be completely unaffected by this fix. This includes:
- Navigation to existing routes (`/dashboard`, `/propiedades`, `/contratos`, etc.)
- The login flow (`/` and `/registro`)
- Public routes (`/firmas/:token`)
- API calls from existing pages
- Theme toggling, command palette, offline detection

## Hypothesized Root Cause

Based on the bug description, the issue is straightforward missing implementation:

1. **Missing Route Variants**: The `Route` enum in `app.rs` has no variants with `#[at("/desahucios")]`, `#[at("/ncf")]`, `#[at("/tareas")]`, `#[at("/invitaciones")]`, `#[at("/organizacion")]`, `#[at("/dgii")]`, or `#[at("/servicios-publicos")]`

2. **Missing Page Modules**: No page component files exist in `frontend/src/pages/` for these domains (no `desahucios.rs`, `ncf.rs`, `tareas.rs`, `invitaciones.rs`, `organizacion.rs`, `dgii.rs`, `servicios_publicos.rs`)

3. **Missing Type Definitions**: No frontend type modules exist in `frontend/src/types/` mirroring the backend DTOs for these domains

4. **Missing Sidebar Links**: The sidebar component in `components/layout/sidebar.rs` has no `Link<Route>` elements referencing these route variants

5. **Missing Switch Arms**: The `switch()` function in `app.rs` has no match arms for the new route variants

## Correctness Properties

Property 1: Bug Condition - Routes Render Correct Pages

_For any_ navigation event where the path is one of the 7 missing routes (`/desahucios`, `/ncf`, `/tareas`, `/invitaciones`, `/organizacion`, `/dgii`, `/servicios-publicos`) and the user is authenticated, the fixed app SHALL render the corresponding domain page component wrapped in `ProtectedRoute`, displaying either a loading spinner (while fetching), data content (on success), or an error toast (on API failure) — never a 404 page.

**Validates: Requirements 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7, 2.8, 2.10, 2.11**

Property 2: Preservation - Existing Routes Unchanged

_For any_ navigation event where the path is NOT one of the 7 new routes, the fixed app SHALL produce exactly the same routing behavior, page rendering, and sidebar display as the original code, preserving all existing functionality for previously registered routes.

**Validates: Requirements 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 3.7, 3.8, 3.9, 3.10, 3.11, 3.12**

Property 3: Role-Based Visibility

_For any_ authenticated user with role `visualizador` navigating to any of the 7 new pages, the page SHALL render in read-only mode with data tables visible but all write-action buttons (Crear, Editar, Revocar, Ejecutar, Invalidar Caché, submit buttons) NOT rendered.

**Validates: Requirements 2.9**

## Fix Implementation

### Changes Required

**File**: `frontend/src/types/` — New type modules

**New Files**:
1. `types/desahucio.rs` — `Desahucio`, `CreateDesahucio`, `UpdateDesahucio` structs matching backend camelCase DTOs
2. `types/ncf.rs` — `SecuenciaNcf`, `ConfigurarRango`, `AlertaRango` structs
3. `types/tarea.rs` — `EjecucionTarea` struct for job history display
4. `types/invitacion.rs` — `Invitacion`, `CrearInvitacion` structs
5. `types/organizacion.rs` — `Organizacion`, `UpdateOrganizacion` structs
6. `types/dgii.rs` — `DgiiConsulta`, `DgiiNombreResult` structs
7. `types/servicio_publico.rs` — `ResponsabilidadEfectiva`, `UpdateResponsabilidad` structs

**Specific Changes**:
- Each type uses `#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]` and `#[serde(rename_all = "camelCase")]`
- Monetary fields use `#[serde(deserialize_with = "deserialize_f64_from_any")]` where applicable
- String props wrapped in `AttrValue` or `Rc<str>` when passed as Yew props

---

**File**: `frontend/src/types/mod.rs`

**Change**: Add `pub mod` declarations for the 7 new type modules

---

**File**: `frontend/src/pages/` — New page modules

**New Files**:
1. `pages/desahucios.rs` — Listing table with create/edit modal, estado badge, pagination
2. `pages/ncf.rs` — Sequences table, configure range form, alerts section
3. `pages/tareas.rs` — Job history table with manual execute buttons (admin only)
4. `pages/invitaciones.rs` — Create invitation form, list table, revoke action (admin only)
5. `pages/organizacion.rs` — Display org info + editable form
6. `pages/dgii.rs` — RNC search input, name search input, cache invalidation button
7. `pages/servicios_publicos.rs` — Responsibility assignments per unit/contract

**Component Pattern** (each page follows):
```
#[function_component]
pub fn DomainPage() -> Html {
    // 1. Auth context for role checks
    let auth = use_context::<AuthContext>();
    let can_write = utils::can_write(&auth);

    // 2. State: data, loading, error, pagination, reload trigger
    let data = use_state(|| Vec::new());
    let loading = use_state(|| true);
    let error = use_state(|| None::<String>);
    let page = use_state(|| 1u64);
    let reload = use_state(|| 0u32);

    // 3. Fetch effect (depends on reload + page only, not filters)
    use_effect_with((reload, page), move |_| { spawn_local(async { ... }); });

    // 4. Render: loading skeleton | error banner | data table + actions
    html! { ... }
}
```

---

**File**: `frontend/src/pages/mod.rs`

**Change**: Add `pub mod` declarations for the 7 new page modules

---

**File**: `frontend/src/app.rs` — Route enum and switch function

**Specific Changes**:
1. Add 7 new variants to `Route` enum:
   ```rust
   #[at("/desahucios")]
   Desahucios,
   #[at("/ncf")]
   Ncf,
   #[at("/tareas")]
   Tareas,
   #[at("/invitaciones")]
   Invitaciones,
   #[at("/organizacion")]
   Organizacion,
   #[at("/dgii")]
   Dgii,
   #[at("/servicios-publicos")]
   ServiciosPublicos,
   ```

2. Add 7 `use` imports for the new page components

3. Add 7 match arms in `switch()`:
   ```rust
   Route::Desahucios => html! { <ProtectedRoute><Desahucios /></ProtectedRoute> },
   Route::Ncf => html! { <ProtectedRoute><Ncf /></ProtectedRoute> },
   Route::Tareas => html! { <ProtectedRoute><Tareas /></ProtectedRoute> },
   Route::Invitaciones => html! { <ProtectedRoute><Invitaciones /></ProtectedRoute> },
   Route::Organizacion => html! { <ProtectedRoute><Organizacion /></ProtectedRoute> },
   Route::Dgii => html! { <ProtectedRoute><Dgii /></ProtectedRoute> },
   Route::ServiciosPublicos => html! { <ProtectedRoute><ServiciosPublicos /></ProtectedRoute> },
   ```

---

**File**: `frontend/src/components/layout/sidebar.rs`

**Specific Changes**:
1. Add new sidebar links organized into appropriate groups:
   - **Operaciones group**: Add "Desahucios" link (visible to all roles, write-only actions gated in page)
   - **Herramientas group**: Add "DGII" and "Servicios Públicos" links (visible when `can_write`)
   - **Sistema group** (admin section): Add "NCF", "Tareas", "Invitaciones", "Organización" links (visible when `is_admin`)

2. Each link follows the existing pattern:
   ```rust
   <li onclick={make_click(on_nav_click.clone())}>
       <Link<Route> to={Route::Desahucios}
           classes={classes!(link_class(&Route::Desahucios))}>
           {icon_desahucios()}
           {"Desahucios"}
       </Link<Route>>
   </li>
   ```

3. Add SVG icon helper functions for each new link

### Data Flow Per Domain

| Domain | API Endpoints | Page Behavior |
|--------|--------------|---------------|
| Desahucios | `GET /api/v1/desahucios` (list), `POST` (create), `PUT /{id}` (update) | Paginated table, create/edit modal, estado filter |
| NCF | `GET /api/v1/ncf/secuencias`, `POST /configurar-rango`, `GET /alertas` | Sequences table, range config form, alerts badges |
| Tareas | `GET /api/v1/tareas/historial`, `POST /{nombre}/ejecutar` | History table with filters, execute buttons per task |
| Invitaciones | `GET /api/v1/invitaciones`, `POST` (create), `DELETE /{id}` (revoke) | Table with status, create form (email+rol), revoke button |
| Organizacion | `GET /api/v1/organizacion`, `PUT` (update) | Display card + edit form for mutable fields |
| DGII | `GET /api/v1/dgii/consulta?rnc=`, `GET /consulta/nombre?buscar=`, `DELETE /cache/{rnc}` | Two search inputs, results display, cache invalidation |
| Servicios Públicos | `GET /propiedades/{id}/unidades/{id}/servicios`, `PUT` (update unit), `PUT /contratos/{id}/servicios` | Property/unit selector, responsibility table, assignment controls |

### Role-Based Rendering Approach

The existing pattern uses utility functions `can_write(&auth)` and `is_admin(&auth)` (checking `user.rol`):

- **Sidebar visibility**: `is_admin` gates NCF, Tareas, Invitaciones, Organización links. `can_write` gates DGII and Servicios Públicos. Desahucios is visible to all (write actions hidden in-page).
- **In-page write controls**: Each page conditionally renders create/edit/delete buttons using `if can_write { html! { <button ...> } }` blocks
- **Admin-only pages**: Tareas and Invitaciones check `is_admin` and display a "sin permisos" message for non-admin users
- **Read-only mode for `visualizador`**: Tables and data are always rendered; action buttons are conditionally omitted

## Testing Strategy

### Validation Approach

The testing strategy follows a two-phase approach: first, surface counterexamples that demonstrate the bug on unfixed code (routes resolving to 404), then verify the fix renders correct pages and preserves existing behavior.

### Exploratory Bug Condition Checking

**Goal**: Surface counterexamples that demonstrate the bug BEFORE implementing the fix. Confirm that navigating to the 7 paths produces 404 responses and that the sidebar has no links for these routes.

**Test Plan**: Write Playwright or wasm-bindgen-test scenarios that navigate to each of the 7 paths and assert that the 404 page content is displayed. Run these on UNFIXED code to confirm the bug exists.

**Test Cases**:
1. **Desahucios 404 Test**: Navigate to `/desahucios`, assert page contains "404 — Página no encontrada" (will fail on unfixed code)
2. **NCF 404 Test**: Navigate to `/ncf`, assert 404 page renders (will fail on unfixed code)
3. **Tareas 404 Test**: Navigate to `/tareas`, assert 404 page renders (will fail on unfixed code)
4. **Sidebar Missing Links Test**: Render sidebar component, assert no link elements with text "Desahucios", "NCF", etc. (will pass on unfixed code — confirming absence)

**Expected Counterexamples**:
- All 7 routes display the 404 page instead of domain-specific content
- Sidebar renders with no links to the missing pages

### Fix Checking

**Goal**: Verify that for all inputs where the bug condition holds, the fixed app produces the expected behavior.

**Pseudocode:**
```
FOR ALL path IN ["/desahucios", "/ncf", "/tareas", "/invitaciones",
                 "/organizacion", "/dgii", "/servicios-publicos"] DO
  result := navigate_authenticated(path)
  ASSERT NOT contains_404(result)
  ASSERT contains_page_component(result, path)
  ASSERT contains_loading_or_data(result)
END FOR
```

### Preservation Checking

**Goal**: Verify that for all inputs where the bug condition does NOT hold, the fixed app produces the same result as the original.

**Pseudocode:**
```
FOR ALL path IN existing_routes DO
  ASSERT render_fixed(path) == render_original(path)
END FOR

FOR ALL link IN sidebar_links_original DO
  ASSERT link IN sidebar_links_fixed
END FOR
```

**Testing Approach**: Property-based testing is recommended for preservation checking because:
- It generates many navigation scenarios across existing routes automatically
- It catches edge cases like route conflicts or sidebar ordering regressions
- It provides strong guarantees that existing behavior is unchanged

**Test Plan**: Observe behavior on UNFIXED code first for all existing routes, then write tests capturing that behavior continues after the fix.

**Test Cases**:
1. **Existing Routes Preservation**: For each of the 12+ existing protected routes, verify navigation renders the correct page component (not 404, not wrong page)
2. **Sidebar Link Preservation**: Verify all original sidebar links remain present with correct Route targets and correct visibility rules
3. **Auth Redirect Preservation**: Verify unauthenticated navigation to existing protected routes still redirects to `/`
4. **NotFound Preservation**: Verify truly unknown paths like `/foobar` still render 404

### Unit Tests

- Test each new page component renders loading state initially
- Test each page handles API error responses (displays toast, not crash)
- Test role-based button visibility (`can_write` false → no create button)
- Test Route enum parsing: `Route::recognize("/desahucios")` returns `Some(Route::Desahucios)`

### Property-Based Tests

- Generate random authenticated navigation sequences across all routes (old + new) and verify no 404 for registered paths
- Generate random role assignments and verify write buttons appear only for `admin`/`gerente`
- Generate random API error codes and verify graceful handling (toast, no panic)

### Integration Tests

- Full flow: login → sidebar link click → page render → data fetch → display
- Sidebar navigation: click each new link, verify URL changes and correct page renders
- Role switching: admin sees all 7 links; visualizador sees subset; gerente sees subset with write controls
