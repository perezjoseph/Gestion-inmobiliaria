# Implementation Plan

- [x] 1. Write bug condition exploration test
  - **Property 1: Bug Condition** — No Console Output During Initialization
  - **CRITICAL**: This test MUST FAIL on unfixed code — failure confirms the bug exists
  - **DO NOT attempt to fix the test or the code when it fails**
  - **NOTE**: This test encodes the expected behavior — it will validate the fix when it passes after implementation
  - **GOAL**: Surface counterexamples that demonstrate no `[INIT]` log messages are produced during the initialization pipeline
  - **Scoped PBT Approach**: Scope the property to the concrete initialization stages: `main()` entry point, App mount, context init, route resolution, and ProtectedRoute auth check
  - Write a property-based test in `frontend/tests/init_logging_tests.rs` using `proptest`
  - Generate arbitrary `Route` variants and verify that calling `switch(route)` produces a `console.log` message containing `[INIT] Route resolved:` with the route debug name (from Bug Condition: `consoleLogCount(input.stage) == 0`)
  - The test assertions should match the Expected Behavior: each initialization stage SHALL produce at least one `console.log` message with `[INIT]` prefix identifying that stage
  - Run test on UNFIXED code
  - **EXPECTED OUTCOME**: Test FAILS (this is correct — it proves the bug exists: no `[INIT]` messages are produced)
  - Document counterexamples found (e.g., "switch(Route::Dashboard) produces zero `[INIT]` log messages")
  - Mark task complete when test is written, run, and failure is documented
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 2.1, 2.2, 2.3, 2.4, 2.5_

- [x] 2. Write preservation property tests (BEFORE implementing fix)
  - **Property 2: Preservation** — Routing and Auth Behavior Unchanged
  - **IMPORTANT**: Follow observation-first methodology
  - Observe behavior on UNFIXED code: `switch(Route::Login)` returns Login Html, `switch(Route::Dashboard)` returns ProtectedRoute-wrapped Dashboard Html, `switch(Route::NotFound)` returns 404 Html
  - Observe: `ProtectedRoute` with no token redirects to `/login`; with a valid token renders Navbar, Sidebar, Footer, OfflineBanner, and children
  - Observe: `index.html` global `error` and `unhandledrejection` handlers display error overlay (unchanged file)
  - Observe: `TrunkApplicationStarted` event listener removes the `#loading` element (unchanged file)
  - Write property-based test in `frontend/tests/init_logging_tests.rs` using `proptest`: for all `Route` variants, `switch(route)` returns valid `Html` matching the expected component for that route (from Preservation Requirements in design)
  - Write property-based test: for all auth states (token present/absent), `ProtectedRoute` makes the correct render/redirect decision identically to unfixed code
  - Verify tests pass on UNFIXED code
  - **EXPECTED OUTCOME**: Tests PASS (this confirms baseline behavior to preserve)
  - Mark task complete when tests are written, run, and passing on unfixed code
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5_

- [x] 3. Add console logging to initialization pipeline

  - [x] 3.1 Add entry point and renderer start logs in `frontend/src/main.rs`
    - Add `use web_sys::console;` and `use wasm_bindgen::JsValue;` imports
    - Before `console_error_panic_hook::set_once()`, add `console::log_1(&JsValue::from_str("[INIT] main() reached — WASM entry point"));`
    - Before `yew::Renderer::<app::App>::new().render()`, add `console::log_1(&JsValue::from_str("[INIT] Starting Yew renderer"));`
    - _Bug_Condition: isBugCondition(input) where input.stage IN ['wasm_entry', 'renderer_start'] AND consoleLogCount(input.stage) == 0_
    - _Expected_Behavior: console contains "[INIT] main() reached" and "[INIT] Starting Yew renderer" after main() executes_
    - _Preservation: No control flow changes — only log statements added before existing calls_
    - _Requirements: 1.1, 2.1, 2.4_

  - [x] 3.2 Add App mount and context init logs in `frontend/src/app.rs` `App()`
    - Add `use web_sys::console;` and `use wasm_bindgen::JsValue;` imports to `app.rs`
    - At the top of the `App` function component body, add `console::log_1(&JsValue::from_str("[INIT] App component mounting"));`
    - After the `use_reducer` and `use_state` calls (before the `html!` block), add `console::log_1(&JsValue::from_str("[INIT] Context providers initialized (Auth, Theme, Toast)"));`
    - _Bug_Condition: isBugCondition(input) where input.stage IN ['app_mount', 'context_init'] AND consoleLogCount(input.stage) == 0_
    - _Expected_Behavior: console contains "[INIT] App component mounting" and "[INIT] Context providers initialized" after App renders_
    - _Preservation: No changes to use_reducer, use_state, or html! output — only log statements inserted_
    - _Requirements: 1.2, 2.2, 2.4_

  - [x] 3.3 Add route resolution log in `frontend/src/app.rs` `switch()`
    - At the top of the `switch` function, add `console::log_1(&JsValue::from_str(&format!("[INIT] Route resolved: {:?}", routes)));`
    - The `Route` enum already derives `Debug` so `{:?}` formatting works
    - _Bug_Condition: isBugCondition(input) where input.stage == 'route_resolution' AND consoleLogCount(input.stage) == 0_
    - _Expected_Behavior: console contains "[INIT] Route resolved: <variant>" when switch() is called_
    - _Preservation: switch() continues to return the same Html for each Route variant — log is added before the match_
    - _Requirements: 1.3, 2.3, 2.4_

  - [x] 3.4 Add auth check and redirect logs in `frontend/src/app.rs` `ProtectedRoute()`
    - After `let is_authed = ...`, add `console::log_1(&JsValue::from_str(&format!("[INIT] ProtectedRoute — authenticated: {}", is_authed)));`
    - Inside the `use_effect_with` closure, before `navigator.push(&Route::Login)`, add `console::log_1(&JsValue::from_str("[INIT] ProtectedRoute — redirecting to /login"));`
    - _Bug_Condition: isBugCondition(input) where input.stage IN ['protected_route_check'] AND consoleLogCount(input.stage) == 0_
    - _Expected_Behavior: console contains "[INIT] ProtectedRoute — authenticated: <bool>" and optionally "[INIT] ProtectedRoute — redirecting to /login"_
    - _Preservation: ProtectedRoute continues to redirect unauthenticated users and render layout for authenticated users — only log statements added_
    - _Requirements: 1.5, 2.5, 2.4_

  - [x] 3.5 Verify bug condition exploration test now passes
    - **Property 1: Expected Behavior** — Initialization Stages Produce Console Output
    - **IMPORTANT**: Re-run the SAME test from task 1 — do NOT write a new test
    - The test from task 1 encodes the expected behavior: each stage produces `[INIT]` log messages
    - When this test passes, it confirms the expected behavior is satisfied
    - Run bug condition exploration test from step 1
    - **EXPECTED OUTCOME**: Test PASSES (confirms bug is fixed)
    - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5_

  - [x] 3.6 Verify preservation tests still pass
    - **Property 2: Preservation** — Routing and Auth Behavior Unchanged
    - **IMPORTANT**: Re-run the SAME tests from task 2 — do NOT write new tests
    - Run preservation property tests from step 2
    - **EXPECTED OUTCOME**: Tests PASS (confirms no regressions)
    - Confirm all routing, auth redirect, error handler, and spinner removal behavior is unchanged
    - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5_

- [x] 4. Checkpoint — Ensure all tests pass
  - Run `cargo test --workspace` to verify all existing and new tests pass
  - Verify no compiler warnings or errors in `frontend/src/main.rs` and `frontend/src/app.rs`
  - Ensure all tests pass, ask the user if questions arise
