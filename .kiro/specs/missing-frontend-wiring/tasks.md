# Implementation Plan: Missing Frontend Wiring

## Overview

Fix 7 backend domains (Desahucios, NCF, Tareas, Invitaciones, Organizacion, DGII, Servicios Públicos) that have fully implemented endpoints but no corresponding frontend pages, routes, or UI components.

## Tasks

- [ ] 1. Write bug condition exploration test

  - [ ] 1.1 Create test file `frontend/tests/missing_routes_pbt.rs` that tests `Route::recognize("/desahucios")` returns `Some(Route::Desahucios)`, `Route::recognize("/ncf")` returns `Some(Route::Ncf)`, `Route::recognize("/tareas")` returns `Some(Route::Tareas)`, `Route::recognize("/invitaciones")` returns `Some(Route::Invitaciones)`, `Route::recognize("/organizacion")` returns `Some(Route::Organizacion)`, `Route::recognize("/dgii")` returns `Some(Route::Dgii)`, `Route::recognize("/servicios-publicos")` returns `Some(Route::ServiciosPublicos)`, and that sidebar HTML output contains link elements for all 7 new routes. **Property 1: Bug Condition** - Missing Routes Render 404. **CRITICAL**: This test MUST FAIL on unfixed code - failure confirms the bug exists. **DO NOT attempt to fix the test or the code when it fails**. Run test on UNFIXED code. **EXPECTED OUTCOME**: Test FAILS (proves the bug exists). Document counterexamples. Mark task complete when test is written, run, and failure is documented. _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 1.8_

- [ ] 2. Write preservation property tests (BEFORE implementing fix)

  - [ ] 2.1 Create test file `frontend/tests/existing_routes_preservation_pbt.rs`. **Property 2: Preservation** - Existing Routes and Sidebar Links Unchanged. Follow observation-first methodology. Observe existing route recognition and sidebar links on unfixed code. Write property-based tests verifying all existing route paths continue to return expected variants, existing sidebar links remain present, and `Route::recognize("/foobar")` returns `None`. Verify tests pass on UNFIXED code. **EXPECTED OUTCOME**: Tests PASS. _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 3.7, 3.8, 3.9, 3.11, 3.12_

- [ ] 3. Create type modules for all 7 domains

  - [ ] 3.1 Create `frontend/src/types/desahucio.rs` with `Desahucio`, `CreateDesahucio`, `UpdateDesahucio` structs using serde camelCase and deserialize_f64_from_any for monetary fields. _Requirements: 2.1_
  - [ ] 3.2 Create `frontend/src/types/ncf.rs` with `SecuenciaNcf`, `ConfigurarRango`, `AlertaRango` structs. _Requirements: 2.2_
  - [ ] 3.3 Create `frontend/src/types/tarea.rs` with `EjecucionTarea` struct. _Requirements: 2.3_
  - [ ] 3.4 Create `frontend/src/types/invitacion.rs` with `Invitacion`, `CrearInvitacion` structs. _Requirements: 2.4_
  - [ ] 3.5 Create `frontend/src/types/organizacion.rs` with `Organizacion`, `UpdateOrganizacion` structs. _Requirements: 2.5_
  - [ ] 3.6 Create `frontend/src/types/dgii.rs` with `DgiiConsulta`, `DgiiNombreResult` structs. _Requirements: 2.6_
  - [ ] 3.7 Create `frontend/src/types/servicio_publico.rs` with `ResponsabilidadEfectiva`, `UpdateResponsabilidad` structs. _Requirements: 2.7_
  - [ ] 3.8 Register all 7 new type modules in `frontend/src/types/mod.rs`. _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7_

- [ ] 4. Create page components for all 7 domains

  - [ ] 4.1 Create `frontend/src/pages/desahucios.rs` with `Desahucios` function component. _Requirements: 2.1, 2.9, 2.10, 2.11_
  - [ ] 4.2 Create `frontend/src/pages/ncf.rs` with `Ncf` function component. _Requirements: 2.2, 2.9, 2.10, 2.11_
  - [ ] 4.3 Create `frontend/src/pages/tareas.rs` with `Tareas` function component. _Requirements: 2.3, 2.9, 2.10, 2.11_
  - [ ] 4.4 Create `frontend/src/pages/invitaciones.rs` with `Invitaciones` function component. _Requirements: 2.4, 2.9, 2.10, 2.11_
  - [ ] 4.5 Create `frontend/src/pages/organizacion.rs` with `OrganizacionPage` function component. _Requirements: 2.5, 2.9, 2.10, 2.11_
  - [ ] 4.6 Create `frontend/src/pages/dgii.rs` with `Dgii` function component. _Requirements: 2.6, 2.9, 2.10, 2.11_
  - [ ] 4.7 Create `frontend/src/pages/servicios_publicos.rs` with `ServiciosPublicos` function component. _Requirements: 2.7, 2.9, 2.10, 2.11_
  - [ ] 4.8 Register all 7 new page modules in `frontend/src/pages/mod.rs`. _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7_

- [ ] 5. Wire routes in frontend/src/app.rs

  - [ ] 5.1 Add 7 new Route enum variants. _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7, 3.12_
  - [ ] 5.2 Add use imports for the 7 new page components. _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7_
  - [ ] 5.3 Add 7 match arms in the switch function. _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7_

- [ ] 6. Add sidebar navigation links

  - [ ] 6.1 Add Desahucios link to Operaciones group in sidebar.rs. _Requirements: 2.8_
  - [ ] 6.2 Add DGII and Servicios Publicos links to Herramientas group in sidebar.rs. _Requirements: 2.8_
  - [ ] 6.3 Add NCF, Tareas, Invitaciones, Organizacion links to Sistema group in sidebar.rs. _Requirements: 2.8_
  - [ ] 6.4 Verify all existing sidebar links remain unchanged. _Requirements: 3.9, 3.11_

- [ ] 7. Verify bug condition exploration test now passes

  - [ ] 7.1 Re-run the SAME test from task 1. **EXPECTED OUTCOME**: Test PASSES (confirms bug is fixed). _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7, 2.8_

- [ ] 8. Verify preservation tests still pass

  - [ ] 8.1 Re-run the SAME tests from task 2. **EXPECTED OUTCOME**: Tests PASS (confirms no regressions). _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 3.7, 3.8, 3.9, 3.11, 3.12_

- [ ] 9. Checkpoint - Ensure all tests pass

  - [ ] 9.1 Run `cargo build -p frontend` and `cargo test -p frontend`. Ensure all tests pass, ask the user if questions arise.

## Notes

- All new page components follow the established pattern in existing pages
- Role-based visibility uses existing `can_write` and `is_admin` utility functions
- Type modules mirror backend DTOs with camelCase serde renaming

## Task Dependency Graph

```json
{
  "waves": [
    ["1", "2"],
    ["3"],
    ["4"],
    ["5"],
    ["6"],
    ["7", "8"],
    ["9"]
  ]
}
```
