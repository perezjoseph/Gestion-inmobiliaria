# Bugfix Requirements Document

## Introduction

A comprehensive exploratory end-to-end test of the production web app at https://gestion.myhomeva.us (Dominican Republic property management platform — Rust/Actix-web + SeaORM backend, Rust/Leptos frontend) uncovered nine confirmed defects. They fall into three groups:

- **Frontend error-handling, validation, and layout** (Bugs 1–4): login does not surface auth errors, registration name validation misfires, the Pagos page renders duplicate pagination, and the mobile hamburger menu is unclickable.
- **Backend RBAC, serialization, and routing** (Bugs 5–8): NCF denies an admin, Invitaciones fails to deserialize an empty page, Servicios Públicos calls a missing endpoint, and a stored property document image 404s.
- **Configuration / CSP** (Bug 9): the Content Security Policy blocks Cloudflare Insights site-wide.

Each defect below pairs a Current Behavior clause (1.x) with the corresponding Expected Behavior clause (2.x). Section 3 captures behavior that must remain unchanged so the fixes do not introduce regressions.

## Bug Analysis

### Current Behavior (Defect)

These describe the inputs/conditions that trigger each bug and the incorrect behavior observed (this is `F`, the unfixed system).

1.1 WHEN a user submits the /login form with a valid-format email and an incorrect password THEN the backend returns 401 for POST /api/v1/auth/login, but the frontend navigates to / (landing page) and shows NO error message to the user.

1.2 WHEN a user fills the "Nombre" field on /registro with a non-empty value (e.g., "Test User") after a prior submission with the field empty, and clicks "Registrarse" THEN the system still displays the validation error "El nombre es obligatorio".

1.3 WHEN a user views the /pagos page THEN the system renders two pagination bars (one inside the table container and one outside), both showing the same range (e.g., "Mostrando 1–6 de 6").

1.4 WHEN a user on a 375px-wide (mobile) viewport on /dashboard (or any authenticated page) clicks the "Abrir menú" hamburger button THEN an SVG icon within .gi-navbar-right intercepts the pointer event, so the button cannot be activated and the mobile navigation never opens.

1.5 WHEN a user authenticated as `admin` navigates to /ncf THEN GET /api/v1/ncf/secuencias returns 403 and the system shows the alert "No tiene permisos para realizar esta acción."

1.6 WHEN a user opens /invitaciones and there are no invitations THEN the backend returns a malformed/empty body and the frontend shows the alert "Error al procesar respuesta: invalid length 0, expected struct PaginatedResponse with 4 elements at line 1 column 2".

1.7 WHEN a user opens /servicios-publicos THEN the frontend requests GET /api/v1/propiedades/todas, which does not exist, producing a 404 and a property dropdown that fails to load.

1.8 WHEN a user opens /propiedades and clicks "Editar" on the property "Higey" THEN GET /api/v1/propiedad/{id}/{filename} returns 404 and the stored document thumbnail fails to load.

1.9 WHEN any page loads THEN the Content Security Policy (script-src 'self' 'wasm-unsafe-eval') blocks https://static.cloudflareinsights.com/beacon.min.js and its associated inline script, generating CSP violation errors on every page.

### Expected Behavior (Correct)

These describe the correct behavior for the same triggering conditions (this is `F'`, the fixed system).

2.1 WHEN a user submits the /login form with a valid-format email and an incorrect password (POST /api/v1/auth/login returns 401) THEN the system SHALL remain on /login and display a clear error message (e.g., "Credenciales inválidas") on the form, and SHALL NOT navigate to /.

2.2 WHEN a user fills the "Nombre" field on /registro with a non-empty value and clicks "Registrarse" THEN the system SHALL re-evaluate the field, treat name validation as passing, and SHALL NOT display "El nombre es obligatorio".

2.3 WHEN a user views the /pagos page THEN the system SHALL render exactly one pagination bar below the table.

2.4 WHEN a user on a 375px-wide viewport clicks the "Abrir menú" hamburger button THEN the button SHALL receive the click (no other element intercepts the pointer event) and the mobile navigation SHALL open.

2.5 WHEN a user authenticated as `admin` navigates to /ncf THEN GET /api/v1/ncf/secuencias SHALL return 200 with the NCF sequences and the page SHALL render without a permission error.

2.6 WHEN a user opens /invitaciones and there are no invitations THEN the backend SHALL return a well-formed PaginatedResponse representing an empty page and the frontend SHALL render an empty state without any deserialization error.

2.7 WHEN a user opens /servicios-publicos THEN the frontend SHALL call the correct, existing backend endpoint for the property list and the property dropdown SHALL load successfully (no 404).

2.8 WHEN a user opens /propiedades and clicks "Editar" on a property that has a stored document THEN the document image SHALL be served successfully (HTTP 200) and the thumbnail SHALL display.

2.9 WHEN any page loads THEN the Content Security Policy SHALL allow the intended Cloudflare Insights script (or the beacon SHALL be removed if analytics are not wanted) so that no CSP violation occurs for legitimate site assets.

### Unchanged Behavior (Regression Prevention)

These describe inputs/conditions that do NOT trigger the bugs and whose behavior must be preserved: for all such inputs, `F(X) = F'(X)`.

3.1 WHEN a user submits /login with valid, correct credentials THEN the system SHALL CONTINUE TO authenticate and redirect the user to their authenticated landing area.

3.2 WHEN a user submits /registro with all required fields validly filled THEN the system SHALL CONTINUE TO accept the submission and create the account.

3.3 WHEN a user leaves the "Nombre" field empty on /registro and submits THEN the system SHALL CONTINUE TO display the "El nombre es obligatorio" validation error.

3.4 WHEN the /pagos table data and its single pagination controls operate (page navigation, range display) THEN the system SHALL CONTINUE TO paginate correctly.

3.5 WHEN a user is on a desktop/wider viewport THEN the navigation and the .gi-navbar-right area SHALL CONTINUE TO render and function as before.

3.6 WHEN a non-admin role (`gerente`, `visualizador`) accesses NCF endpoints THEN the system SHALL CONTINUE TO enforce the existing role restrictions for those roles.

3.7 WHEN /invitaciones contains one or more invitations THEN the system SHALL CONTINUE TO return and render the paginated list correctly.

3.8 WHEN any other PaginatedResponse-backed endpoint returns data (empty or populated) THEN the frontend SHALL CONTINUE TO deserialize and render it as before.

3.9 WHEN /servicios-publicos and other pages call their existing, valid backend endpoints THEN those calls SHALL CONTINUE TO succeed unchanged.

3.10 WHEN a property document with a well-formed, existing file path is requested THEN the system SHALL CONTINUE TO serve it successfully.

3.11 WHEN any legitimate first-party script or asset loads under the CSP THEN it SHALL CONTINUE TO be allowed, and disallowed third-party origins SHALL CONTINUE TO be blocked.

## Bug Conditions and Properties

The following structured definitions formalize each defect for fix checking and preservation checking. `F` is the original (unfixed) behavior; `F'` is the fixed behavior.

### Bug 1 — Login error not surfaced

```pascal
FUNCTION isBugCondition_login(X)
  INPUT: X = { email: valid-format, password, authResult }
  OUTPUT: boolean
  RETURN X.authResult = HTTP_401
END FUNCTION

// Fix Checking
FOR ALL X WHERE isBugCondition_login(X) DO
  result ← submitLogin'(X)
  ASSERT result.route = "/login" AND result.errorShown = true
END FOR

// Preservation Checking
FOR ALL X WHERE NOT isBugCondition_login(X) DO
  ASSERT submitLogin(X) = submitLogin'(X)
END FOR
```

### Bug 2 — Registration name validation false positive

```pascal
FUNCTION isBugCondition_nombre(X)
  INPUT: X = { nombre, priorEmptySubmission: boolean }
  OUTPUT: boolean
  RETURN nonEmpty(X.nombre) AND X.priorEmptySubmission = true
END FUNCTION

// Fix Checking
FOR ALL X WHERE isBugCondition_nombre(X) DO
  result ← validateRegistro'(X)
  ASSERT result.nombreError = none
END FOR

// Preservation Checking — empty nombre still errors, valid submissions unaffected
FOR ALL X WHERE NOT isBugCondition_nombre(X) DO
  ASSERT validateRegistro(X) = validateRegistro'(X)
END FOR
```

### Bug 3 — Duplicate pagination

```pascal
FUNCTION isBugCondition_pagos(X)
  INPUT: X = rendered /pagos view
  OUTPUT: boolean
  RETURN countPaginationBars(X) > 1
END FUNCTION

// Fix Checking
FOR ALL X WHERE isBugCondition_pagos(X) DO
  result ← renderPagos'(X)
  ASSERT countPaginationBars(result) = 1
END FOR

// Preservation Checking — pagination logic/range unchanged
FOR ALL X DO
  ASSERT paginationRange(renderPagos(X)) = paginationRange(renderPagos'(X))
END FOR
```

### Bug 4 — Hamburger menu intercepted on mobile

```pascal
FUNCTION isBugCondition_hamburger(X)
  INPUT: X = { viewportWidth, authenticated: boolean }
  OUTPUT: boolean
  RETURN X.viewportWidth <= 375 AND X.authenticated = true
END FUNCTION

// Fix Checking
FOR ALL X WHERE isBugCondition_hamburger(X) DO
  result ← clickHamburger'(X)
  ASSERT pointerInterceptedBy(result) = "hamburger-button" AND result.menuOpen = true
END FOR

// Preservation Checking — desktop layout unchanged
FOR ALL X WHERE NOT isBugCondition_hamburger(X) DO
  ASSERT navbarLayout(X) = navbarLayout'(X)
END FOR
```

### Bug 5 — NCF 403 for admin

```pascal
FUNCTION isBugCondition_ncf(X)
  INPUT: X = { role, endpoint }
  OUTPUT: boolean
  RETURN X.role = "admin" AND X.endpoint = "GET /api/v1/ncf/secuencias"
END FUNCTION

// Fix Checking
FOR ALL X WHERE isBugCondition_ncf(X) DO
  result ← callNcf'(X)
  ASSERT result.status = 200
END FOR

// Preservation Checking — non-admin RBAC preserved
FOR ALL X WHERE NOT isBugCondition_ncf(X) DO
  ASSERT callNcf(X) = callNcf'(X)
END FOR
```

### Bug 6 — Invitaciones empty deserialization

```pascal
FUNCTION isBugCondition_invitaciones(X)
  INPUT: X = invitations dataset
  OUTPUT: boolean
  RETURN count(X) = 0
END FUNCTION

// Fix Checking
FOR ALL X WHERE isBugCondition_invitaciones(X) DO
  result ← getInvitaciones'(X)
  ASSERT isWellFormedPaginatedResponse(result) AND result.items = [] AND result.total = 0
END FOR

// Preservation Checking — non-empty responses unchanged
FOR ALL X WHERE NOT isBugCondition_invitaciones(X) DO
  ASSERT getInvitaciones(X) = getInvitaciones'(X)
END FOR
```

### Bug 7 — Servicios Públicos wrong endpoint

```pascal
FUNCTION isBugCondition_servicios(X)
  INPUT: X = property-list request from /servicios-publicos
  OUTPUT: boolean
  RETURN requestedPath(X) = "GET /api/v1/propiedades/todas"  // non-existent
END FUNCTION

// Fix Checking
FOR ALL X WHERE isBugCondition_servicios(X) DO
  result ← loadProperties'(X)
  ASSERT requestedPath(result) = existing_backend_route AND result.status = 200
END FOR

// Preservation Checking — other API calls on the page unchanged
FOR ALL X WHERE NOT isBugCondition_servicios(X) DO
  ASSERT loadProperties(X) = loadProperties'(X)
END FOR
```

### Bug 8 — Property document image 404

```pascal
FUNCTION isBugCondition_docimg(X)
  INPUT: X = { propiedadId, filename }  // stored document exists
  OUTPUT: boolean
  RETURN documentExists(X) AND serveDocument(X).status = 404
END FUNCTION

// Fix Checking
FOR ALL X WHERE isBugCondition_docimg(X) DO
  result ← serveDocument'(X)
  ASSERT result.status = 200
END FOR

// Preservation Checking — missing files still 404
FOR ALL X WHERE NOT isBugCondition_docimg(X) DO
  ASSERT serveDocument(X) = serveDocument'(X)
END FOR
```

### Bug 9 — CSP blocks Cloudflare Insights

```pascal
FUNCTION isBugCondition_csp(X)
  INPUT: X = script asset request
  OUTPUT: boolean
  RETURN X.src = "https://static.cloudflareinsights.com/beacon.min.js"
         AND cspBlocks(X) = true
END FUNCTION

// Fix Checking
FOR ALL X WHERE isBugCondition_csp(X) DO
  result ← applyCsp'(X)
  ASSERT cspBlocks(result) = false   // OR beacon removed entirely
END FOR

// Preservation Checking — all other CSP decisions unchanged
FOR ALL X WHERE NOT isBugCondition_csp(X) DO
  ASSERT applyCsp(X) = applyCsp'(X)
END FOR
```
