# Bugfix Requirements Document

## Introduction

Several backend API domains have fully implemented endpoints but no corresponding frontend pages, routes, or UI components. Users cannot access these features from the application interface despite the backend being ready. This affects 7 domains: Desahucios, NCF, Tareas, Invitaciones, Organizacion, DGII, and Servicios Públicos.

## Bug Analysis

### Current Behavior (Defect)

**User Story:** As a property manager, I want to access evictions, NCF, background jobs, invitations, organization settings, DGII lookup, and utility services pages from the frontend, so that I can use these features whose backend APIs are already functional.

#### Acceptance Criteria

1.1 WHEN an authenticated user navigates to `/desahucios`, THEN the system displays a 404 page because no `Route` variant with `#[at("/desahucios")]` exists in the `Route` enum in `app.rs`, and no corresponding page component exists in `pages/`

1.2 WHEN an authenticated user navigates to `/ncf`, THEN the system displays a 404 page because no `Route` variant with `#[at("/ncf")]` exists in the `Route` enum in `app.rs`, and no corresponding page component exists in `pages/`

1.3 WHEN an authenticated user navigates to `/tareas`, THEN the system displays a 404 page because no `Route` variant with `#[at("/tareas")]` exists in the `Route` enum in `app.rs`, and no corresponding page component exists in `pages/`

1.4 WHEN an authenticated user navigates to `/invitaciones`, THEN the system displays a 404 page because no `Route` variant with `#[at("/invitaciones")]` exists in the `Route` enum in `app.rs`, and no corresponding page component exists in `pages/`

1.5 WHEN an authenticated user navigates to `/organizacion`, THEN the system displays a 404 page because no `Route` variant with `#[at("/organizacion")]` exists in the `Route` enum in `app.rs`, and no corresponding page component exists in `pages/`

1.6 WHEN an authenticated user navigates to `/dgii`, THEN the system displays a 404 page because no `Route` variant with `#[at("/dgii")]` exists in the `Route` enum in `app.rs`, and no corresponding page component exists in `pages/`

1.7 WHEN an authenticated user navigates to `/servicios-publicos`, THEN the system displays a 404 page because no `Route` variant with `#[at("/servicios-publicos")]` exists in the `Route` enum in `app.rs`, and no corresponding page component exists in `pages/`

1.8 WHEN an authenticated user with role `admin` opens the sidebar, THEN the sidebar does not render navigation links for "Desahucios", "NCF", "Tareas", "Invitaciones", "Organización", "DGII", or "Servicios Públicos" in any sidebar group (`gi-sidebar-group`), because no `Link<Route>` elements referencing these routes exist in `sidebar.rs`

1.9 WHEN an unauthenticated user navigates to any of `/desahucios`, `/ncf`, `/tareas`, `/invitaciones`, `/organizacion`, `/dgii`, or `/servicios-publicos`, THEN the system displays the custom 404 page (the `NotFound` route variant rendering the "404 — Página no encontrada" message) instead of redirecting to `/` login, because the Yew Router `#[not_found]` handler catches unregistered paths before the `ProtectedRoute` authentication check can execute

### Expected Behavior (Correct)

**User Story:** As a property manager, I want to access Desahucios, NCF, Tareas, Invitaciones, Organizacion, DGII, and Servicios Públicos pages from the sidebar, so that I can manage these features through the frontend.

#### Acceptance Criteria

2.1 WHEN an authenticated user navigates to `/desahucios`, THEN the system SHALL render a page containing a table listing desahucios records fetched from `/api/v1/desahucios`, a "Crear" button that opens a creation form, and an edit action per row that opens an update form.

2.2 WHEN an authenticated user navigates to `/ncf`, THEN the system SHALL render a page displaying a table of NCF sequences fetched from `/api/v1/ncf/secuencias`, a form or modal for configuring sequence ranges, and a section showing active alerts.

2.3 WHEN an authenticated user navigates to `/tareas`, THEN the system SHALL render a page displaying a table of job execution history fetched from `/api/v1/tareas/historial` and a button per task type to trigger manual execution.

2.4 WHEN an authenticated user navigates to `/invitaciones`, THEN the system SHALL render a page containing a "Crear" button that opens an invitation creation form, a table listing existing invitations, and a "Revocar" action per active invitation row.

2.5 WHEN an authenticated user navigates to `/organizacion`, THEN the system SHALL render a page displaying the current organization information fetched from `/api/v1/organizacion` and a form to update editable organization fields.

2.6 WHEN an authenticated user navigates to `/dgii`, THEN the system SHALL render a page containing a search input for RNC lookup by number, a search input for lookup by name, and an "Invalidar Caché" button that triggers cache invalidation via `/api/v1/dgii`.

2.7 WHEN an authenticated user navigates to `/servicios-publicos`, THEN the system SHALL render a page displaying utility service responsibility assignments for units and contracts, with controls to assign and remove services via the `/api/v1/propiedades/.../servicios` and `/api/v1/contratos/.../servicios` endpoints.

2.8 WHEN an authenticated user opens the sidebar, THEN the system SHALL display navigation links labeled "Desahucios", "NCF", "Tareas", "Invitaciones", "Organizacion", "DGII", and "Servicios Públicos" within appropriate sidebar group sections.

2.9 IF the user has role `visualizador` and navigates to any of the new pages, THEN the system SHALL render the page in read-only mode: the data table is visible but "Crear", "Editar", "Revocar", "Ejecutar", "Invalidar Caché", and update form submit buttons SHALL NOT be rendered.

2.10 IF an API request to any of the new page endpoints fails with a non-401 error, THEN the system SHALL display a toast notification with an error message indicating the failure and keep any previously loaded data visible on the page.

2.11 WHILE data is being fetched for any of the new pages, THEN the system SHALL display a loading indicator (spinner) in the page content area until the response is received or the request times out after 15 seconds.

### Unchanged Behavior (Regression Prevention)

**User Story:** As a developer, I want assurance that existing pages and API endpoints remain functional after the bugfix, so that no regressions are introduced.

#### Acceptance Criteria

3.1 WHEN an authenticated user navigates to `/propiedades`, THEN the system SHALL CONTINUE TO render the Propiedades page component within 3 seconds, displaying the property list table or an empty-state placeholder if no data exists.

3.2 WHEN an authenticated user navigates to `/inquilinos`, THEN the system SHALL CONTINUE TO render the Inquilinos page component within 3 seconds, displaying the tenant list table or an empty-state placeholder if no data exists.

3.3 WHEN an authenticated user navigates to `/contratos`, THEN the system SHALL CONTINUE TO render the Contratos page component within 3 seconds, displaying the contracts list table or an empty-state placeholder if no data exists.

3.4 WHEN an authenticated user navigates to `/pagos`, THEN the system SHALL CONTINUE TO render the Pagos page component within 3 seconds, displaying the payments list table or an empty-state placeholder if no data exists.

3.5 WHEN an authenticated user navigates to `/gastos`, THEN the system SHALL CONTINUE TO render the Gastos page component within 3 seconds, displaying the expenses list table or an empty-state placeholder if no data exists.

3.6 WHEN an authenticated user navigates to `/mantenimiento`, THEN the system SHALL CONTINUE TO render the Mantenimiento page component within 3 seconds, displaying the maintenance requests list or an empty-state placeholder if no data exists.

3.7 WHEN an authenticated user navigates to `/dashboard`, THEN the system SHALL CONTINUE TO render the Dashboard page component within 3 seconds, displaying at least one statistics widget or chart container.

3.8 WHEN an authenticated user navigates to `/configuracion`, THEN the system SHALL CONTINUE TO render the Configuracion page component within 3 seconds, displaying the settings form.

3.9 WHEN an authenticated user navigates to any route defined in the Route enum, THEN the system SHALL render the Sidebar component containing navigation links to all existing routes, and the Sidebar SHALL remain visible with no missing links compared to the state prior to this change.

3.10 WHEN the backend receives a GET request to `/api/v1/propiedades`, `/api/v1/inquilinos`, `/api/v1/contratos`, `/api/v1/pagos`, `/api/v1/gastos`, or `/api/v1/mantenimiento` with a valid JWT, THEN the system SHALL respond with HTTP 200 and a JSON array body within 5 seconds, with no change to the response schema compared to the state prior to this change.

3.11 WHEN an authenticated user clicks any navigation link in the Sidebar, THEN the system SHALL route to the corresponding page without a full-page reload and without producing a JavaScript console error.

3.12 IF the Route enum in `app.rs` is modified as part of this change, THEN all previously existing route variants SHALL remain present and map to the same URL paths as before.
