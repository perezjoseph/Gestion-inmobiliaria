# Implementation Plan

## Overview

This plan fixes the nine confirmed E2E defects using the bug-condition methodology. For each bug we
first write a **Bug Condition** exploration test that FAILS on the unfixed code (proving the bug),
then a **Preservation** test that PASSES on the unfixed code (capturing behavior to protect), then
apply the fix, then re-run both tests. Property numbers below match the Correctness Properties in
`design.md` (Property 2k-1 = Bug Condition for bug k, Property 2k = Preservation for bug k).

Test harness: backend uses Rust `#[cfg(test)]` + `backend/tests/common` integration harness +
`proptest` for PBT (`services/*_pbt.rs`); frontend pure logic uses `wasm-bindgen-test`/native unit
tests; layout, CSP, and deployment behavior are verified via the Playwright E2E pass and by
inspecting response headers / the browser console.

## Tasks

### Bug 1 — Login 401 redirects to landing instead of showing an error

- [x] 1. Write bug condition exploration test for login 401 handling
  - **Property 1: Bug Condition** - Login 401 surfaces an error and stays on /login
  - **CRITICAL**: This test MUST FAIL on unfixed code - failure confirms the bug exists
  - **DO NOT attempt to fix the test or the code when it fails**
  - **GOAL**: Surface counterexamples that demonstrate the bug exists
  - **Scoped PBT Approach**: Scope the property over `{has_token: false, status: 401}` inputs to the
    `handle_response`/login predicate (no token present = unauthenticated login attempt)
  - Assert: for a 401 response when no token is stored, `handle_response` does NOT clear-and-redirect
    and the login flow stays on `/login` with an error surfaced (from Bug Condition in design,
    `isBugCondition_login`)
  - Run test on UNFIXED `frontend/src/services/api.rs::handle_response`
  - **EXPECTED OUTCOME**: Test FAILS (a 401 triggers `clear_token_and_redirect()` → navigates to `/`)
  - Document counterexample (e.g., "wrong-password login redirects to `/` with no error")
  - Mark task complete when test is written, run, and failure is documented
  - _Requirements: 1.1_

- [x] 2. Write preservation property test for authenticated flows and session expiry
  - **Property 2: Preservation** - Authenticated flows and session expiry unchanged
  - **IMPORTANT**: Follow observation-first methodology
  - Observe on UNFIXED code: a 401 WITH a token present clears the token and redirects to `/`;
    a successful login navigates to the dashboard
  - Write a property-based test over `{has_token ∈ {true,false}, status ∈ {200,401,...}}`: redirect
    happens iff `status == 401 && has_token == true` (from Preservation Requirements in design)
  - Run test on UNFIXED code
  - **EXPECTED OUTCOME**: Test PASSES (confirms baseline: token-bearing 401s redirect, valid logins proceed)
  - Mark task complete when test is written, run, and passing on unfixed code
  - _Requirements: 3.1_

- [ ] 3. Fix login 401 handling so errors surface on /login

  - [x] 3.1 Guard the 401 clear-and-redirect on token presence
    - In `frontend/src/services/api.rs::handle_response`, only `clear_token_and_redirect()` when
      `get_token().is_some()`; otherwise fall through to error humanization
    - (Optional) add an explicit `401 =>` arm in `humanize_error` returning "Credenciales inválidas."
    - _Bug_Condition: isBugCondition_login(X) where X.authResult = HTTP_401 AND no token present_
    - _Expected_Behavior: stay on /login, errorShown = true; SHALL NOT navigate to /_
    - _Preservation: token-bearing 401s still clear token and redirect; valid logins still reach dashboard_
    - _Requirements: 1.1, 2.1, 3.1_

  - [~] 3.2 Verify bug condition exploration test now passes
    - **Property 1: Expected Behavior** - Login 401 surfaces an error and stays on /login
    - **IMPORTANT**: Re-run the SAME test from task 1 - do NOT write a new test
    - **EXPECTED OUTCOME**: Test PASSES (confirms 401 without token surfaces an error, no redirect)
    - _Requirements: 2.1_

  - [~] 3.3 Verify preservation test still passes
    - **Property 2: Preservation** - Authenticated flows and session expiry unchanged
    - **IMPORTANT**: Re-run the SAME test from task 2 - do NOT write a new test
    - **EXPECTED OUTCOME**: Test PASSES (no regression to session-expiry redirect or valid login)
    - _Requirements: 3.1_

---

### Bug 2 — Registration "El nombre es obligatorio" is sticky

- [~] 4. Write bug condition exploration test for sticky Nombre validation
  - **Property 3: Bug Condition** - Filled Nombre clears the validation error
  - **CRITICAL**: This test MUST FAIL on unfixed code - failure confirms the bug exists
  - **GOAL**: Reproduce the sticky `nombre_error` after a prior empty submission
  - **Scoped PBT Approach**: Scope over `{priorEmptySubmission: true, nombre: non-empty}` inputs
  - Assert: after an empty submit, entering a non-empty Nombre yields `nombre_error == none`
    (from Bug Condition in design, `isBugCondition_nombre`)
  - Run test on UNFIXED `frontend/src/components/auth/register_form.rs`
  - **EXPECTED OUTCOME**: Test FAILS (error persists / value not re-evaluated)
  - **NOTE (root cause hypothesized)**: if unfixed code already clears on re-submit, re-hypothesize
    (e.g., stale value capture) and document the actual mechanism before finalizing the fix
  - Document counterexample found
  - _Requirements: 1.2_

- [~] 5. Write preservation property test for empty-name error and valid submissions
  - **Property 4: Preservation** - Empty-name error and valid submissions unchanged
  - **IMPORTANT**: Follow observation-first methodology
  - Observe on UNFIXED code: empty/whitespace Nombre on submit shows "El nombre es obligatorio";
    a fully valid form submits and creates the account
  - Write property-based tests over Nombre values: empty/whitespace ⇒ error; non-empty ⇒ no error
    from `validate_nombre` (from Preservation Requirements in design)
  - Run tests on UNFIXED code
  - **EXPECTED OUTCOME**: Tests PASS (baseline `validate_nombre` behavior captured)
  - _Requirements: 3.2, 3.3_

- [ ] 6. Fix sticky Nombre validation

  - [~] 6.1 Add live revalidation for the Nombre field
    - In `frontend/src/components/auth/register_form.rs`, replace the generic `nombre` handler with
      one that calls `nombre_error.set(validate_nombre(&value))` on input (clears once non-empty),
      keeping the submit-time `validate_nombre` call intact
    - _Bug_Condition: isBugCondition_nombre(X) where nonEmpty(nombre) AND priorEmptySubmission = true_
    - _Expected_Behavior: result.nombreError = none for non-empty Nombre_
    - _Preservation: empty Nombre still errors on submit; valid submissions still create the account_
    - _Requirements: 1.2, 2.2, 3.2, 3.3_

  - [~] 6.2 Verify bug condition exploration test now passes
    - **Property 3: Expected Behavior** - Filled Nombre clears the validation error
    - **IMPORTANT**: Re-run the SAME test from task 4 - do NOT write a new test
    - **EXPECTED OUTCOME**: Test PASSES (non-empty Nombre after empty submit ⇒ no error)
    - _Requirements: 2.2_

  - [~] 6.3 Verify preservation tests still pass
    - **Property 4: Preservation** - Empty-name error and valid submissions unchanged
    - **IMPORTANT**: Re-run the SAME tests from task 5 - do NOT write new tests
    - **EXPECTED OUTCOME**: Tests PASS (empty Nombre still errors; valid form still submits)
    - _Requirements: 3.2, 3.3_

---

### Bug 3 — Pagos renders two pagination bars

- [~] 7. Write bug condition exploration test for duplicate pagination
  - **Property 5: Bug Condition** - Pagos shows exactly one pagination bar
  - **CRITICAL**: This test MUST FAIL on unfixed code - failure confirms the bug exists
  - **GOAL**: Demonstrate two pagination bars on a desktop `/pagos` render
  - **Scoped PBT Approach**: Scope to the concrete desktop-viewport render of `/pagos`
  - Assert: `countPaginationBars(renderPagos()) == 1` (from Bug Condition in design, `isBugCondition_pagos`)
  - Run on UNFIXED `frontend/src/pages/pagos.rs` (Playwright/desktop render)
  - **EXPECTED OUTCOME**: Test FAILS (two bars: one in `PagoList`, one page-level)
  - Document counterexample (both bars show "Mostrando 1–6 de 6")
  - _Requirements: 1.3_

- [~] 8. Write preservation property test for pagination range and navigation
  - **Property 6: Preservation** - Pagination range and navigation unchanged
  - **IMPORTANT**: Follow observation-first methodology
  - Observe on UNFIXED code: the retained (page-level) bar's range text, page nav, and per-page
    selection for a given dataset
  - Write a property-based test over datasets/pages asserting `paginationRange` and navigation
    callbacks match observed behavior (from Preservation Requirements in design)
  - Run on UNFIXED code
  - **EXPECTED OUTCOME**: Tests PASS (baseline pagination behavior captured)
  - _Requirements: 3.4_

- [ ] 9. Fix duplicate pagination on /pagos

  - [~] 9.1 Remove the redundant pagination bar
    - In `frontend/src/pages/pagos.rs`, remove the `<Pagination>` rendered inside `PagoList`; keep
      the single page-level `<Pagination>`
    - Drop now-unused `total`, `page`, `per_page`, `on_page_change`, `on_per_page_change` from
      `PagoListProps` and its call site (clean up orphans created by this change only)
    - _Bug_Condition: isBugCondition_pagos(X) where countPaginationBars(X) > 1_
    - _Expected_Behavior: countPaginationBars(result) = 1_
    - _Preservation: retained bar uses the same state/callbacks → range and navigation unchanged_
    - _Requirements: 1.3, 2.3, 3.4_

  - [~] 9.2 Verify bug condition exploration test now passes
    - **Property 5: Expected Behavior** - Pagos shows exactly one pagination bar
    - **IMPORTANT**: Re-run the SAME test from task 7 - do NOT write a new test
    - **EXPECTED OUTCOME**: Test PASSES (exactly one bar on desktop)
    - _Requirements: 2.3_

  - [~] 9.3 Verify preservation test still passes
    - **Property 6: Preservation** - Pagination range and navigation unchanged
    - **IMPORTANT**: Re-run the SAME test from task 8 - do NOT write a new test
    - **EXPECTED OUTCOME**: Tests PASS (range text, page nav, per-page selection unchanged)
    - _Requirements: 3.4_

---

### Bug 4 — Mobile hamburger intercepted by an SVG in `.gi-navbar-right`

- [~] 10. Write bug condition exploration test for the mobile hamburger
  - **Property 7: Bug Condition** - Mobile hamburger receives the click
  - **CRITICAL**: This test MUST FAIL on unfixed code - failure confirms the bug exists
  - **GOAL**: Demonstrate the pointer event is intercepted by an element other than the hamburger
  - **Scoped PBT Approach**: Scope to the concrete case `{viewportWidth: 375, authenticated: true}`
  - Assert (Playwright at 375px): the click target is the hamburger button (not an SVG in
    `.gi-navbar-right`) and the mobile menu opens (from Bug Condition in design, `isBugCondition_hamburger`)
  - Run on UNFIXED `frontend/styles/tailwind.css` layout
  - **EXPECTED OUTCOME**: Test FAILS (SVG in `.gi-navbar-right` intercepts the tap; menu never opens)
  - Document counterexample
  - _Requirements: 1.4_

- [~] 11. Write preservation property test for the desktop navbar
  - **Property 8: Preservation** - Desktop navbar unchanged
  - **IMPORTANT**: Follow observation-first methodology
  - Observe on UNFIXED code: desktop/wider viewport navbar and `.gi-navbar-right` layout/behavior
    (the hamburger is `display:none` on desktop)
  - Write a property-based test over viewport widths > 768px asserting navbar layout/behavior is
    unchanged (from Preservation Requirements in design)
  - Run on UNFIXED code
  - **EXPECTED OUTCOME**: Tests PASS (baseline desktop layout captured)
  - _Requirements: 3.5_

- [ ] 12. Fix mobile hamburger interception

  - [~] 12.1 Raise hamburger stacking and prevent right-group overlap on mobile
    - In `frontend/styles/tailwind.css`: add `.gi-hamburger { position: relative; z-index: 50; }`
    - Within `@media (max-width: 768px)`: allow `.gi-navbar-right { min-width: 0; flex-wrap: wrap; }`
      and hide low-priority controls (`.gi-kbd { display: none; }`)
    - _Bug_Condition: isBugCondition_hamburger(X) where viewportWidth <= 375 AND authenticated_
    - _Expected_Behavior: pointerInterceptedBy(result) = "hamburger-button" AND menuOpen = true_
    - _Preservation: desktop hamburger is display:none; right-group changes scoped to mobile media query_
    - _Requirements: 1.4, 2.4, 3.5_

  - [~] 12.2 Verify bug condition exploration test now passes
    - **Property 7: Expected Behavior** - Mobile hamburger receives the click
    - **IMPORTANT**: Re-run the SAME test from task 10 - do NOT write a new test
    - **EXPECTED OUTCOME**: Test PASSES (hamburger receives the click; menu opens at 375px)
    - _Requirements: 2.4_

  - [~] 12.3 Verify preservation test still passes
    - **Property 8: Preservation** - Desktop navbar unchanged
    - **IMPORTANT**: Re-run the SAME test from task 11 - do NOT write a new test
    - **EXPECTED OUTCOME**: Tests PASS (desktop navbar and `.gi-navbar-right` unchanged)
    - _Requirements: 3.5_

---

### Bug 5 — NCF `GET /ncf/secuencias` returns 403 for admin

- [~] 13. Write bug condition exploration test for admin NCF read
  - **Property 9: Bug Condition** - Admin can read NCF sequences
  - **CRITICAL**: This test MUST FAIL on unfixed code - failure confirms the bug exists
  - **GOAL**: Show an admin of an `informal` org is blocked from reading NCF sequences
  - **Scoped PBT Approach**: Scope to `{role: "admin", endpoint: GET /api/v1/ncf/secuencias, org.tipo_fiscal: "informal"}`
  - Assert (integration test): admin request returns 200 with the (possibly empty) sequence list
    (from Bug Condition in design, `isBugCondition_ncf`)
  - Run on UNFIXED `backend/src/services/ncf.rs::listar_secuencias`
  - **EXPECTED OUTCOME**: Test FAILS (returns 403 from the fiscal-access gate, not RBAC)
  - Document counterexample (admin of informal org → 403 "Funciones fiscales requieren registro en DGII")
  - _Requirements: 1.5_

- [~] 14. Write preservation property test for non-admin NCF restrictions
  - **Property 10: Preservation** - Non-admin NCF restrictions unchanged
  - **IMPORTANT**: Follow observation-first methodology
  - Observe on UNFIXED code: `gerente`/`visualizador` receive 403 (from `AdminOnly`) on NCF endpoints;
    NCF write/configuration paths still require fiscal access
  - Write a property-based test over the matrix `{role ∈ {admin,gerente,visualizador}} ×
    {tipo_fiscal ∈ {informal, persona_fisica, persona_juridica}}` asserting non-admin always 403 on
    the read endpoint, and write paths still gated (from Preservation Requirements in design)
  - Run on UNFIXED code
  - **EXPECTED OUTCOME**: Tests PASS (baseline RBAC + write-path fiscal gating captured)
  - _Requirements: 3.6_

- [ ] 15. Fix NCF read access for admin

  - [~] 15.1 Remove the fiscal-access gate from the read path
    - In `backend/src/services/ncf.rs::listar_secuencias`, remove the
      `obtener_org_con_acceso_fiscal(db, org_id)` call; retain the `AdminOnly` RBAC and the
      `organizacion_id` multi-tenant filter
    - Keep the fiscal gate on write/assignment paths (`configurar_rango_con_acceso`, `asignar_ncf`)
    - _Bug_Condition: isBugCondition_ncf(X) where role = "admin" AND read endpoint returns 403_
    - _Expected_Behavior: result.status = 200 with the org's sequences (possibly empty)_
    - _Preservation: non-admin still 403 via AdminOnly; write/config paths still fiscally gated_
    - _Requirements: 1.5, 2.5, 3.6_

  - [~] 15.2 Verify bug condition exploration test now passes
    - **Property 9: Expected Behavior** - Admin can read NCF sequences
    - **IMPORTANT**: Re-run the SAME test from task 13 - do NOT write a new test
    - **EXPECTED OUTCOME**: Test PASSES (admin of informal org → 200)
    - _Requirements: 2.5_

  - [~] 15.3 Verify preservation test still passes
    - **Property 10: Preservation** - Non-admin NCF restrictions unchanged
    - **IMPORTANT**: Re-run the SAME test from task 14 - do NOT write a new test
    - **EXPECTED OUTCOME**: Tests PASS (non-admin still 403; write paths still gated)
    - _Requirements: 3.6_

---

### Bug 6 — Invitaciones empty list fails to deserialize

- [~] 16. Write bug condition exploration test for empty Invitaciones response shape
  - **Property 11: Bug Condition** - Empty Invitaciones returns a well-formed PaginatedResponse
  - **CRITICAL**: This test MUST FAIL on unfixed code - failure confirms the bug exists
  - **GOAL**: Show the empty list serializes as a bare array, not a `PaginatedResponse`
  - **Scoped PBT Approach**: Scope to `{invitations dataset = empty}` for the org
  - Assert (integration test): `GET /invitaciones` body is a well-formed `PaginatedResponse` with
    `data = []` and `total = 0` (from Bug Condition in design, `isBugCondition_invitaciones`)
  - Run on UNFIXED `backend/src/handlers/invitaciones.rs` / `services/invitaciones.rs`
  - **EXPECTED OUTCOME**: Test FAILS (body is `[]`; frontend reports
    "invalid length 0, expected struct PaginatedResponse with 4 elements")
  - Document counterexample
  - _Requirements: 1.6_

- [~] 17. Write preservation property test for populated invitations and other paginated endpoints
  - **Property 12: Preservation** - Populated invitations and other paginated endpoints unchanged
  - **IMPORTANT**: Follow observation-first methodology
  - Observe on UNFIXED code: other `PaginatedResponse`-backed endpoints (e.g. `inquilinos::list`)
    return `{ data, total, page, perPage }` and deserialize/render correctly
  - Write a property-based test over invitation counts `0..N` asserting the response is always a
    well-formed `PaginatedResponse` with `total == count`, correct `page`/`perPage` echo, and that
    other paginated endpoints are untouched (from Preservation Requirements in design)
  - Run on UNFIXED code (the other-endpoints portion PASSES today; the invitations portion captures
    the target envelope for after the fix)
  - **EXPECTED OUTCOME**: Other-paginated-endpoint assertions PASS (baseline captured)
  - _Requirements: 3.7, 3.8_

- [ ] 18. Fix Invitaciones to return a PaginatedResponse

  - [~] 18.1 Add a list query DTO
    - In `backend/src/models/invitacion.rs`, add `InvitacionListQuery { page: Option<u64>,
      per_page: Option<u64> }` with `#[serde(rename_all = "camelCase")]`
    - _Requirements: 1.6, 2.6_

  - [~] 18.2 Return PaginatedResponse from the service
    - In `backend/src/services/invitaciones.rs`, change `listar` to return
      `PaginatedResponse<InvitacionResponse>`, paginating the existing filtered query
      (`OrganizacionId == org_id`, not used, not expired) with `.paginate(db, per_page)` /
      `num_items()` / `fetch_page(page-1)` — mirroring `services::inquilinos::list`
    - _Requirements: 1.6, 2.6_

  - [~] 18.3 Wire the handler to the envelope
    - In `backend/src/handlers/invitaciones.rs`, accept `query: web::Query<InvitacionListQuery>`,
      pass `page`/`per_page` through, and return the envelope; empty case returns
      `{ "data": [], "total": 0, "page": 1, "perPage": 20 }`
    - _Bug_Condition: isBugCondition_invitaciones(X) where responseShape != PaginatedResponse_
    - _Expected_Behavior: isWellFormedPaginatedResponse(result) AND result.items = [] AND result.total = 0_
    - _Preservation: populated lists round-trip correctly; other paginated endpoints untouched_
    - _Requirements: 1.6, 2.6, 3.7, 3.8_

  - [~] 18.4 Verify bug condition exploration test now passes
    - **Property 11: Expected Behavior** - Empty Invitaciones returns a well-formed PaginatedResponse
    - **IMPORTANT**: Re-run the SAME test from task 16 - do NOT write a new test
    - **EXPECTED OUTCOME**: Test PASSES (empty list → `{ data: [], total: 0, ... }`)
    - _Requirements: 2.6_

  - [~] 18.5 Verify preservation test still passes
    - **Property 12: Preservation** - Populated invitations and other paginated endpoints unchanged
    - **IMPORTANT**: Re-run the SAME test from task 17 - do NOT write a new test
    - **EXPECTED OUTCOME**: Tests PASS (populated invitations render; other endpoints unaffected)
    - _Requirements: 3.7, 3.8_

---

### Bug 7 — Servicios Públicos called a non-existent property route (already fixed in source; add regression guard)

> Note: the current source already calls `/propiedades?perPage=200`. The defect existed only in the
> deployed build. The exploration test here is expected to PASS on current source (acting as a
> regression guard); if it FAILS, a stray caller of `/propiedades/todas` was found and must be
> repointed.

- [~] 19. Write bug condition / regression-guard test for the property-list endpoint
  - **Property 13: Bug Condition** - Servicios Públicos calls an existing property endpoint
  - **GOAL**: Guard that no caller of the non-existent `/propiedades/todas` route exists
  - **Scoped PBT Approach**: Scope to the property-list request from `/servicios-publicos`
  - Assert: `frontend/src/pages/servicios_publicos.rs` requests
    `GET /api/v1/propiedades?perPage=200` (an existing route) and a repo-wide search finds no caller
    of `/propiedades/todas` (from Bug Condition in design, `isBugCondition_servicios`)
  - Run on CURRENT source
  - **EXPECTED OUTCOME**: Test PASSES on current source (regression guard). Record as
    `unexpected_pass` for a bug-condition test — the bug is already resolved in source; the value is
    preventing reintroduction and ensuring the corrected build is deployed
  - _Requirements: 1.7_

- [~] 20. Write preservation property test for other Servicios Públicos calls
  - **Property 14: Preservation** - Other Servicios Públicos calls unchanged
  - **IMPORTANT**: Follow observation-first methodology
  - Observe on current source: units (`/propiedades/{id}/unidades`) and servicios
    (`/propiedades/{id}/unidades/{unit}/servicios`) calls succeed against existing routes
  - Write a test asserting these calls target the same existing endpoints (from Preservation
    Requirements in design)
  - Run on current source
  - **EXPECTED OUTCOME**: Tests PASS (other page calls unchanged)
  - _Requirements: 3.9_

- [ ] 21. Confirm source correctness and ensure deployment

  - [~] 21.1 Verify the corrected endpoint and deploy the fixed frontend build
    - Confirm no remaining caller of `/propiedades/todas`; the dropdown loads via
      `/propiedades?perPage=200`. Ensure the corrected frontend build is deployed (the defect was in
      the stale deployed build, not current source). No code change required unless a stray caller is found
    - _Bug_Condition: isBugCondition_servicios(X) where requestedPath = GET /api/v1/propiedades/todas_
    - _Expected_Behavior: requestedPath(result) = existing route AND result.status = 200_
    - _Preservation: units/servicios calls unchanged_
    - _Requirements: 1.7, 2.7, 3.9_

  - [~] 21.2 Verify the regression-guard test passes
    - **Property 13: Expected Behavior** - Servicios Públicos calls an existing property endpoint
    - **IMPORTANT**: Re-run the SAME test from task 19 - do NOT write a new test
    - **EXPECTED OUTCOME**: Test PASSES (dropdown calls `/propiedades?perPage=200`)
    - _Requirements: 2.7_

  - [~] 21.3 Verify preservation test still passes
    - **Property 14: Preservation** - Other Servicios Públicos calls unchanged
    - **IMPORTANT**: Re-run the SAME test from task 20 - do NOT write a new test
    - **EXPECTED OUTCOME**: Tests PASS (units/servicios calls unchanged)
    - _Requirements: 3.9_

---

### Bug 8 — Property document thumbnail 404

- [~] 22. Write bug condition exploration test for the document URL builder
  - **Property 15: Bug Condition** - Stored document image is served
  - **CRITICAL**: This test MUST FAIL on unfixed code - failure confirms the bug exists
  - **GOAL**: Show the gallery builds an unroutable `/api/v1/{file_path}` URL (no-op `trim_end_matches`)
  - **Scoped PBT Approach**: Scope to `{propiedadId, filename}` where a stored document exists
  - Assert: the URL builder in `document_gallery.rs::DocumentCard` resolves to `/uploads/{file_path}`
    and the image returns 200 with authentication (from Bug Condition in design, `isBugCondition_docimg`)
  - Run on UNFIXED `frontend/src/components/common/document_gallery.rs`
  - **EXPECTED OUTCOME**: Test FAILS (builds `/api/v1/{file_path}` → 404; `trim_end_matches("/api")`
    is a no-op on `"/api/v1"`)
  - Document counterexample (e.g., `…/Eg3tKKlWsAA0v_w (1).jpg` → 404)
  - _Requirements: 1.8_

- [~] 23. Write preservation property test for missing files and traversal
  - **Property 16: Preservation** - Missing files and traversal unchanged
  - **IMPORTANT**: Follow observation-first methodology
  - Observe on UNFIXED code: `serve_upload` returns 404 for missing files and rejects `..` traversal
  - Write a property-based test over paths asserting missing files → 404 and traversal attempts →
    rejected (from Preservation Requirements in design)
  - Run on UNFIXED `backend/src/app.rs::serve_upload`
  - **EXPECTED OUTCOME**: Tests PASS (baseline 404/traversal behavior captured)
  - _Requirements: 3.10_

- [ ] 24. Fix the document thumbnail URL and authenticated loading

  - [~] 24.1 Build the correct route and load via authenticated blob fetch
    - In `frontend/src/components/common/document_gallery.rs`: build
      `let file_path = format!("/uploads/{}", doc.file_path);`
    - Load the protected file via an authenticated blob fetch (mirror `services::api::api_download`):
      fetch with `Authorization: Bearer {token}`, build a `Blob`, create an object URL with
      `web_sys::Url::create_object_url_with_blob`, use it as the `<img>`/`<embed>` `src`, and revoke
      the object URL on cleanup (`DocumentCard` already receives `token`)
    - Do NOT relax `serve_upload` authentication
    - _Bug_Condition: isBugCondition_docimg(X) where documentExists AND serveDocument(X).status = 404_
    - _Expected_Behavior: result.status = 200; thumbnail displays_
    - _Preservation: missing files still 404; traversal still rejected_
    - _Requirements: 1.8, 2.8, 3.10_

  - [~] 24.2 Verify bug condition exploration test now passes
    - **Property 15: Expected Behavior** - Stored document image is served
    - **IMPORTANT**: Re-run the SAME test from task 22 - do NOT write a new test
    - **EXPECTED OUTCOME**: Test PASSES (URL resolves to `/uploads/{file_path}`; image served 200)
    - _Requirements: 2.8_

  - [~] 24.3 Verify preservation test still passes
    - **Property 16: Preservation** - Missing files and traversal unchanged
    - **IMPORTANT**: Re-run the SAME test from task 23 - do NOT write a new test
    - **EXPECTED OUTCOME**: Tests PASS (missing files still 404; traversal still rejected)
    - _Requirements: 3.10_

---

### Bug 9 — CSP blocks Cloudflare Insights site-wide

- [~] 25. Write bug condition verification for the CSP and Cloudflare Insights beacon
  - **Property 17: Bug Condition** - CSP no longer blocks Cloudflare Insights
  - **CRITICAL**: This check MUST FAIL against the deployed (stale) CSP - failure confirms the bug
  - **GOAL**: Demonstrate the deployed `script-src 'self' 'wasm-unsafe-eval'` blocks
    `https://static.cloudflareinsights.com/beacon.min.js`
  - **Scoped PBT Approach**: Scope to the Cloudflare Insights beacon script request
  - Assert: no CSP violation is produced for the beacon — either the CSP allows the Cloudflare
    Insights origins (`script-src … https://static.cloudflareinsights.com`,
    `connect-src … https://cloudflareinsights.com`) or the beacon is removed (from Bug Condition in
    design, `isBugCondition_csp`). Verify by inspecting the served CSP header and the browser console
  - Run against the DEPLOYED CSP / a page load
  - **EXPECTED OUTCOME**: Check FAILS against the stale deployed ConfigMap (CSP violation logged).
    (The repo Caddyfiles already contain the corrected CSP.)
  - Document the observed violation
  - _Requirements: 1.9_

- [~] 26. Write preservation check for first-party allowed / third-party blocked
  - **Property 18: Preservation** - First-party allowed, third-party still blocked
  - **IMPORTANT**: Follow observation-first methodology
  - Observe: first-party assets are allowed (`default-src 'self'`, `script-src 'self'
    'wasm-unsafe-eval'`) and disallowed third-party origins are blocked
  - Assert the CSP introduces NO wildcards and does NOT add `'unsafe-inline'`; legitimate first-party
    scripts/assets remain allowed and other third-party origins remain blocked (from Preservation
    Requirements in design). Verify by inspecting the CSP header
  - **EXPECTED OUTCOME**: Check PASSES (baseline first-party/third-party policy captured)
  - _Requirements: 3.11_

- [ ] 27. Fix the deployed CSP for Cloudflare Insights

  - [~] 27.1 Align deployed CSP and handle the inline beacon without weakening protection
    - Ensure the deployed `caddyfile` ConfigMap matches the repo CSP in `infra/caddy/Caddyfile` and
      `infra/k8s/app/overlays/prod/Caddyfile` (allows `https://static.cloudflareinsights.com` in
      `script-src` and `https://cloudflareinsights.com` in `connect-src`), then roll the frontend pods
    - For the inline bootstrap: do NOT add `'unsafe-inline'`. Either disable Cloudflare Web Analytics
      auto-injection (remove the beacon — recommended, matches 2.9) or add a specific `'sha256-<hash>'`
      for the exact inline snippet
    - _Bug_Condition: isBugCondition_csp(X) where beacon src blocked by CSP_
    - _Expected_Behavior: cspBlocks(result) = false (or beacon removed); no CSP violation_
    - _Preservation: first-party assets allowed; third-party origins still blocked; no `'unsafe-inline'`/wildcards_
    - _Requirements: 1.9, 2.9, 3.11_

  - [~] 27.2 Verify bug condition check now passes
    - **Property 17: Expected Behavior** - CSP no longer blocks Cloudflare Insights
    - **IMPORTANT**: Re-run the SAME check from task 25 - do NOT write a new check
    - **EXPECTED OUTCOME**: Check PASSES (no CSP violation for Cloudflare Insights on page load)
    - _Requirements: 2.9_

  - [~] 27.3 Verify preservation check still passes
    - **Property 18: Preservation** - First-party allowed, third-party still blocked
    - **IMPORTANT**: Re-run the SAME check from task 26 - do NOT write a new check
    - **EXPECTED OUTCOME**: Check PASSES (first-party allowed; third-party blocked; no weakening)
    - _Requirements: 3.11_

---

### Final Validation

- [~] 28. Checkpoint - Ensure all tests pass
  - Run the full backend test suite (`cargo test`) and frontend tests; run the Playwright E2E pass
  - Confirm every Bug Condition exploration test/check now PASSES (Properties 1, 3, 5, 7, 9, 11, 13,
    15, 17) and every Preservation test/check still PASSES (Properties 2, 4, 6, 8, 10, 12, 14, 16, 18)
  - Run the Manual Re-Test checklist from the design (`/login`, `/registro`, `/pagos`, mobile menu,
    `/ncf`, `/invitaciones`, `/servicios-publicos`, property documents, CSP console)
  - Ensure all tests pass; ask the user if questions arise
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7, 2.8, 2.9, 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 3.7, 3.8, 3.9, 3.10, 3.11_

---

## Task Dependency Graph

The nine bugs are independent and can be worked in any order or in parallel. Within each bug, the
exploration test and preservation test must be written before the fix, and the verification
sub-tasks run after the fix. The final checkpoint depends on all bugs being complete.

```
Bug 1:  1 ─┐
        2 ─┼─> 3 (3.1 ─> 3.2, 3.3)
Bug 2:  4 ─┐
        5 ─┼─> 6 (6.1 ─> 6.2, 6.3)
Bug 3:  7 ─┐
        8 ─┼─> 9 (9.1 ─> 9.2, 9.3)
Bug 4: 10 ─┐
       11 ─┼─> 12 (12.1 ─> 12.2, 12.3)
Bug 5: 13 ─┐
       14 ─┼─> 15 (15.1 ─> 15.2, 15.3)
Bug 6: 16 ─┐
       17 ─┼─> 18 (18.1 ─> 18.2 ─> 18.3 ─> 18.4, 18.5)
Bug 7: 19 ─┐
       20 ─┼─> 21 (21.1 ─> 21.2, 21.3)
Bug 8: 22 ─┐
       23 ─┼─> 24 (24.1 ─> 24.2, 24.3)
Bug 9: 25 ─┐
       26 ─┼─> 27 (27.1 ─> 27.2, 27.3)

All bugs ─> 28 (Final Validation Checkpoint)
```

```json
{
  "waves": [
    {
      "wave": 1,
      "description": "Write Bug Condition exploration tests and Preservation tests on unfixed code (all bugs, parallelizable)",
      "tasks": ["1", "2", "4", "5", "7", "8", "10", "11", "13", "14", "16", "17", "19", "20", "22", "23", "25", "26"]
    },
    {
      "wave": 2,
      "description": "Implement fixes and verify each bug's exploration + preservation tests (parallelizable across bugs)",
      "tasks": ["3", "6", "9", "12", "15", "18", "21", "24", "27"]
    },
    {
      "wave": 3,
      "description": "Final validation checkpoint across all bugs",
      "tasks": ["28"]
    }
  ]
}
```

- **Within each bug**: write the Bug Condition exploration test and the Preservation test first
  (they establish the baseline on unfixed code), then implement the fix, then verify both tests.
- **Across bugs**: no cross-bug dependencies. Frontend (1–4, 7, 8), backend (5, 6, 8), and infra
  (9) work can proceed concurrently.
- **Task 28** depends on every preceding task.

## Notes

- **Property numbering** matches `design.md` exactly: Property 2k-1 is the Bug Condition for bug k,
  Property 2k is its Preservation property. Verification sub-tasks re-use the same Property number
  (re-running the same test) and re-label the Bug Condition as **Expected Behavior** once the fix
  is in place.
- **Exploration tests must FAIL first.** Do not "fix" a failing exploration test — its failure on
  unfixed code is the proof the bug exists. It validates the fix only after implementation.
- **Preservation tests must PASS on unfixed code** (observation-first): observe and record real
  behavior, then assert it, so the fix cannot silently regress untouched paths.
- **Bug 7 special case**: the defect lives only in the stale deployed build; the current source is
  already correct. Its exploration test is expected to PASS on current source as a regression guard
  (record as `unexpected_pass`). The remaining action is to ensure the corrected frontend build is
  deployed.
- **Bug 9 is infra/CSP**: "tests" are verifications of the served CSP header and the browser
  console rather than generated property-based tests; the Property-format labels are kept for
  consistent status tracking.
- **Security**: no fix weakens authentication, authorization, or input validation. Bug 5 removes
  only the fiscal gate on the NCF *read* path (RBAC and tenant filter retained); Bug 8 keeps
  `serve_upload` authenticated via an authenticated blob fetch; Bug 9 never adds `'unsafe-inline'`
  or wildcards.
- **Out of scope (flagged for follow-up)**: `serve_upload` does not scope files by
  `organizacion_id` — a latent cross-tenant IDOR unrelated to these nine bugs. Track separately.
