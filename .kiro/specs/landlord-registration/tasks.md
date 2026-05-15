# Implementation Plan: Landlord Registration

## Overview

Frontend-only feature adding a self-registration page for landlords. Implements a new `/registro` route, registration page, registration form component with client-side validation, and a registration link on the login page. All code is Rust (Yew + Tailwind CSS) with Spanish UI text. No backend changes required.

## Tasks

- [x] 1. Create validation module and RegisterForm component
  - [x] 1.1 Create `frontend/src/components/auth/register_form.rs` with pure validation functions
    - Implement `validate_nombre(input: &str) -> Option<String>` — returns error if trimmed input is empty
    - Implement `validate_email(input: &str) -> Option<String>` — returns error if trimmed input is empty or missing "@"
    - Implement `validate_password(input: &str) -> Option<String>` — returns error if length < 8
    - Implement `validate_confirm_password(password: &str, confirm: &str) -> Option<String>` — returns error if not equal
    - Implement `validate_form(nombre, email, password, confirm) -> Option<RegisterRequest>` — returns `RegisterRequest` with `rol: "gerente"` if all valid, `None` otherwise
    - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 7.1_

  - [x] 1.2 Implement the `RegisterForm` functional component in the same file
    - Define `RegisterFormProps` with `on_success: Callback<LoginResponse>`
    - Add state hooks for nombre, email, password, confirm_password, per-field errors, server_error, and loading
    - Wire input fields with `oninput` callbacks using controlled component pattern
    - On submit: run validation functions, abort if errors, set `loading = true`, call `auth::register()` then `auth::login()`, emit `on_success` on login success
    - Map errors: 409/duplicate → "Este correo electrónico ya está registrado", network error → "Error de conexión. Intente nuevamente.", other → display server message verbatim
    - Re-enable button and revert text to "Registrarse" on any error
    - Render form with Tailwind classes matching login form style: nombre, correo electrónico, contraseña, confirmar contraseña fields with Spanish labels
    - Render submit button with text "Registrarse" / "Registrando..." when loading
    - Render inline errors below each field using `<p class="text-red-500 text-xs mt-1">`
    - Render server error using `ErrorBanner` component above form fields
    - Render link to login page: "¿Ya tienes cuenta? Inicia sesión"
    - _Requirements: 2.1, 2.2, 2.3, 2.4, 3.1, 3.2, 3.3, 3.4, 3.5, 4.1, 4.2, 4.3, 4.4, 5.1, 5.2, 5.3, 5.4, 7.1, 7.2_

  - [x] 1.3 Export the new module in `frontend/src/components/auth/mod.rs`
    - Add `pub mod register_form;`
    - _Requirements: 2.1_

  - [x] 1.4 Write property test: nombre validation rejects empty input
    - **Property 1: Nombre validation rejects empty input**
    - Generate arbitrary strings with `proptest`; verify `validate_nombre` returns error iff trimmed input is empty
    - **Validates: Requirements 3.1**

  - [x] 1.5 Write property test: email validation rejects missing "@"
    - **Property 2: Email validation rejects missing "@"**
    - Generate arbitrary strings; verify `validate_email` returns error iff trimmed input is empty or missing "@"
    - **Validates: Requirements 3.2**

  - [x] 1.6 Write property test: password validation enforces minimum length
    - **Property 3: Password validation enforces minimum length**
    - Generate arbitrary strings; verify `validate_password` returns error iff length < 8
    - **Validates: Requirements 3.3**

  - [x] 1.7 Write property test: confirm password validation rejects mismatches
    - **Property 4: Confirm password validation rejects mismatches**
    - Generate arbitrary string pairs; verify `validate_confirm_password` returns error iff strings differ
    - **Validates: Requirements 3.4**

  - [x] 1.8 Write property test: valid inputs produce correct request, invalid inputs block submission
    - **Property 5: Valid inputs produce correct request; invalid inputs block submission**
    - Generate arbitrary form inputs; verify `validate_form` returns `Some(RegisterRequest)` with `rol="gerente"` and correct fields when all valid, `None` otherwise
    - **Validates: Requirements 3.5, 4.1, 7.1**

- [x] 2. Checkpoint — Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [x] 3. Create registration page and wire routing
  - [x] 3.1 Create `frontend/src/pages/registro.rs`
    - Implement `Registro` functional component
    - Check `is_authenticated()` on mount via `use_effect_with`; redirect to `/dashboard` if true
    - Get `AuthContext` and `use_navigator`
    - Define `on_success` callback that dispatches `AuthAction::Login { token, user }` and navigates to `Route::Dashboard`
    - Render centered card layout matching login page style (same Tailwind classes)
    - Render heading "Gestión Inmobiliaria" and subtitle "Cree su cuenta para comenzar"
    - Render `RegisterForm` with `on_success` callback
    - _Requirements: 1.2, 1.3, 4.4_

  - [x] 3.2 Export the new page in `frontend/src/pages/mod.rs`
    - Add `pub mod registro;`
    - _Requirements: 1.1_

  - [x] 3.3 Add `Route::Registro` variant and switch arm in `frontend/src/app.rs`
    - Add `#[at("/registro")] Registro` variant to `Route` enum
    - Add `Route::Registro => html! { <Registro /> }` in `switch` function (no `ProtectedRoute` wrapping)
    - Add `use crate::pages::registro::Registro;` import
    - _Requirements: 1.1, 1.2_

- [x] 4. Add registration link to login page
  - [x] 4.1 Modify `frontend/src/pages/login.rs`
    - Add a `<Link<Route>>` to `Route::Registro` below the `LoginForm` component
    - Link text: "¿No tienes cuenta? Regístrate"
    - Style with Tailwind: centered text, blue link color, margin top
    - _Requirements: 6.1, 6.2_

- [x] 5. Final checkpoint — Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

## Notes

- Tasks marked with `*` are optional and can be skipped for faster MVP
- Each task references specific requirements for traceability
- Checkpoints ensure incremental validation
- Property tests validate universal correctness properties from the design document
- All validation logic is extracted into pure functions to enable direct property-based testing without UI rendering
- The `rol` field is hardcoded to `"gerente"` — never exposed to the user
