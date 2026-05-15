# Frontend Loading Fix — Bugfix Design

## Overview

The Yew/WASM frontend renders a blank page with no console output, making it impossible to diagnose where the initialization pipeline stalls. The fix adds `web_sys::console::log_1` calls at each stage of the startup sequence: WASM entry point (`main.rs`), App component mount (`app.rs`), route resolution (`switch` function), context provider initialization, and `ProtectedRoute` auth checks. This is a pure observability fix — no behavioral logic changes.

## Glossary

- **Bug_Condition (C)**: The condition that triggers the bug — the initialization pipeline produces no diagnostic output, so a blank page provides zero information about where the process stalled
- **Property (P)**: The desired behavior — each initialization stage logs a message to the browser console before and/or after executing, creating a breadcrumb trail
- **Preservation**: All existing rendering, routing, authentication, and error-handling behavior must remain unchanged by the addition of logging
- **Initialization Pipeline**: The sequence `WASM load → main() → Renderer::new().render() → App mount → ContextProviders → BrowserRouter → switch() → ProtectedRoute`
- **`main()`**: The WASM entry point in `frontend/src/main.rs` that sets the panic hook and starts the Yew renderer
- **`App`**: The root `#[function_component]` in `frontend/src/app.rs` that initializes AuthContext, ThemeContext, ToastContext, and BrowserRouter
- **`switch`**: The function in `frontend/src/app.rs` that maps a `Route` enum variant to the corresponding page `Html`
- **`ProtectedRoute`**: The `#[function_component]` in `frontend/src/app.rs` that checks `AuthContext` for a valid token and redirects unauthenticated users to `/login`

## Bug Details

### Bug Condition

The bug manifests when the WASM module loads and the initialization pipeline executes without producing any diagnostic console output. When any stage silently fails or stalls, the user sees a blank page (or a perpetual loading spinner) with no way to determine the failure point.

**Formal Specification:**
```
FUNCTION isBugCondition(input)
  INPUT: input of type InitializationEvent (any stage of the startup pipeline executing)
  OUTPUT: boolean

  RETURN input.stage IN ['wasm_entry', 'app_mount', 'context_init', 'route_resolution', 'protected_route_check']
         AND consoleLogCount(input.stage) == 0
END FUNCTION
```

### Examples

- User navigates to `/dashboard` → WASM loads, `main()` runs, App mounts, switch resolves `Route::Dashboard`, ProtectedRoute checks auth — **no console output at any stage**. If the page is blank, the developer cannot tell whether `main()` even executed.
- User navigates to `/` (login) → WASM loads, `main()` runs, App mounts, switch resolves `Route::Login` — **no log indicating which route was matched**. Developer cannot confirm the router is working.
- User navigates to `/propiedades` without a token → ProtectedRoute redirects to login — **no log indicating the auth check result or redirect decision**.
- WASM fails to compile or load → the `index.html` error/rejection handlers fire — this case is already handled and is NOT part of the bug condition.

## Expected Behavior

### Preservation Requirements

**Unchanged Behaviors:**
- The `switch` function must continue to return the correct `Html` for each `Route` variant
- `ProtectedRoute` must continue to redirect unauthenticated users to `/login` via `navigator.push(&Route::Login)`
- `ProtectedRoute` must continue to render `Navbar`, `Sidebar`, `Footer`, `OfflineBanner`, and children for authenticated users
- `AuthState::default()` must continue to read the token from `localStorage` via `get_token()`
- The `TrunkApplicationStarted` event listener in `index.html` must continue to remove the loading spinner
- The global `error` and `unhandledrejection` handlers in `index.html` must continue to display the error overlay
- Mouse clicks, form submissions, and all other user interactions must remain unaffected

**Scope:**
The fix adds only `web_sys::console::log_1` calls. No control flow, state management, routing logic, or rendering output is modified. All existing behavior for every input type is preserved.

## Hypothesized Root Cause

This is not a logic bug — it is a missing observability feature. The root cause of the *symptom* (inability to diagnose blank pages) is:

1. **No logging in `main()`**: `frontend/src/main.rs` calls `console_error_panic_hook::set_once()` and `yew::Renderer::<app::App>::new().render()` with no `console::log_1` calls, so there is no confirmation the entry point was reached or the renderer started.

2. **No logging in `App` component**: The `App` function component in `frontend/src/app.rs` initializes `use_reducer(AuthState::default)`, `use_reducer(ToastState::default)`, and `use_state` for theme detection, then renders the context provider tree and `BrowserRouter` — all without any console output.

3. **No logging in `switch`**: The `switch` function pattern-matches on `Route` variants and returns `Html` without logging which route was resolved.

4. **No logging in `ProtectedRoute`**: The `ProtectedRoute` component reads `AuthContext`, checks `is_authed`, and either redirects or renders the layout — without logging the auth check result or redirect decision.

## Correctness Properties

Property 1: Bug Condition — Each initialization stage produces console output

_For any_ execution of the initialization pipeline where a stage (WASM entry, App mount, route resolution, ProtectedRoute auth check) is reached, the fixed code SHALL produce at least one `console.log` message identifying that stage, so that the last visible log message pinpoints where a stall or failure occurred.

**Validates: Requirements 2.1, 2.2, 2.3, 2.4, 2.5**

Property 2: Preservation — All existing behavior unchanged

_For any_ user interaction (navigation, clicks, form submissions, auth flows, error handling), the fixed code SHALL produce exactly the same rendered output, routing decisions, and state transitions as the original code, preserving all functional behavior.

**Validates: Requirements 3.1, 3.2, 3.3, 3.4, 3.5**

## Fix Implementation

### Changes Required

**File**: `frontend/src/main.rs`

**Function**: `main()`

**Specific Changes**:
1. **Add entry point log**: Before `console_error_panic_hook::set_once()`, log `"[INIT] main() reached — WASM entry point"` using `web_sys::console::log_1`.
2. **Add renderer start log**: Before `yew::Renderer::<app::App>::new().render()`, log `"[INIT] Starting Yew renderer"`.

---

**File**: `frontend/src/app.rs`

**Function**: `App()`

**Specific Changes**:
3. **Add App mount log**: At the top of the `App` function component body, log `"[INIT] App component mounting"`.
4. **Add context init log**: After the `use_reducer` and `use_state` calls, log `"[INIT] Context providers initialized (Auth, Theme, Toast)"`.

**Function**: `switch()`

**Specific Changes**:
5. **Add route resolution log**: At the top of the `switch` function, log the matched route using `format!("[INIT] Route resolved: {:?}", routes)` (the `Route` enum already derives `Debug`).

**Function**: `ProtectedRoute()`

**Specific Changes**:
6. **Add auth check log**: After computing `is_authed`, log `format!("[INIT] ProtectedRoute — authenticated: {}", is_authed)`.
7. **Add redirect log**: Inside the `use_effect_with` closure, when `!*authed`, log `"[INIT] ProtectedRoute — redirecting to /login"` before `navigator.push`.

## Testing Strategy

### Validation Approach

The testing strategy follows a two-phase approach: first, surface counterexamples that demonstrate the absence of logging on unfixed code, then verify the fix adds the expected log messages and preserves all existing behavior.

### Exploratory Bug Condition Checking

**Goal**: Surface counterexamples that demonstrate the bug BEFORE implementing the fix. Confirm that no console output is produced during initialization.

**Test Plan**: Build the unfixed frontend with `trunk build`, load it in a browser, and observe the console. Alternatively, write a unit test that calls `main()` or renders `App` in a WASM test harness and asserts no log output is produced.

**Test Cases**:
1. **Entry Point Test**: Load the WASM module and check console — no `[INIT]` messages appear (will confirm bug on unfixed code)
2. **Route Resolution Test**: Navigate to `/dashboard` and check console — no route resolution message appears (will confirm bug on unfixed code)
3. **Auth Check Test**: Navigate to a protected route without a token and check console — no auth check or redirect message appears (will confirm bug on unfixed code)

**Expected Counterexamples**:
- Zero `[INIT]` prefixed messages in the browser console after full page load
- Developer cannot determine whether `main()` executed, which route was matched, or why a redirect occurred

### Fix Checking

**Goal**: Verify that for all inputs where the bug condition holds (any initialization stage is reached), the fixed code produces the expected console log message.

**Pseudocode:**
```
FOR ALL stage IN ['wasm_entry', 'renderer_start', 'app_mount', 'context_init', 'route_resolution', 'protected_route_check'] DO
  result := executeStage_fixed(stage)
  ASSERT consoleContains("[INIT]", stage)
END FOR
```

### Preservation Checking

**Goal**: Verify that for all inputs where the bug condition does NOT hold (all non-logging behavior), the fixed code produces the same result as the original code.

**Pseudocode:**
```
FOR ALL input WHERE NOT isBugCondition(input) DO
  ASSERT originalBehavior(input) = fixedBehavior(input)
END FOR
```

**Testing Approach**: Property-based testing is recommended for preservation checking because:
- It generates many route/auth-state combinations automatically
- It catches edge cases in routing or auth logic that manual tests might miss
- It provides strong guarantees that adding log statements did not alter control flow

**Test Plan**: Observe behavior on UNFIXED code first for routing, auth redirects, and rendering, then write property-based tests capturing that behavior.

**Test Cases**:
1. **Routing Preservation**: Verify that `switch()` returns the same `Html` variant for every `Route` enum value before and after the fix
2. **Auth Redirect Preservation**: Verify that `ProtectedRoute` redirects to `/login` when `token` is `None` and renders children when `token` is `Some(_)`, identically before and after the fix
3. **Error Handler Preservation**: Verify that the `index.html` global error/rejection handlers still display the error overlay (unchanged file)
4. **Spinner Removal Preservation**: Verify that the `TrunkApplicationStarted` listener still removes the loading element (unchanged file)

### Unit Tests

- Test that `main()` executes without panic and the renderer is invoked
- Test that `App` component renders the context provider tree and `BrowserRouter`
- Test that `switch()` returns correct `Html` for each `Route` variant
- Test that `ProtectedRoute` redirects when unauthenticated and renders layout when authenticated

### Property-Based Tests

- Generate random `Route` enum variants and verify `switch()` produces valid `Html` with a log message containing the route name
- Generate random `AuthState` values (token present/absent, user present/absent) and verify `ProtectedRoute` makes the correct render/redirect decision while logging the auth check result
- Generate random sequences of route navigations and verify the console log trail matches the expected stage sequence

### Integration Tests

- Build the frontend with `trunk build` and load in a headless browser; verify `[INIT]` messages appear in console in the correct order
- Navigate to a protected route without a token and verify both the redirect AND the `[INIT] ProtectedRoute — redirecting to /login` log message
- Navigate to `/dashboard` with a valid token and verify the full log trail: `main() reached → Starting Yew renderer → App component mounting → Context providers initialized → Route resolved: Dashboard → ProtectedRoute — authenticated: true`
