---
inclusion: fileMatch
fileMatchPattern: ["frontend/**/*.rs", "frontend/Cargo.toml", "frontend/index.html", "frontend/Trunk.toml", "frontend/styles/**/*"]
---

# Frontend Rules

## Framework
> Why Yew + WASM: Keeps the entire stack in Rust for shared types between frontend and backend. Trunk provides simple build tooling with hot reload.

- Always use Yew compiled to WebAssembly via Trunk.
- Always use Yew's functional component model with hooks.
- Always use `frontend/src/main.rs` as entry point mounting `frontend/src/app.rs`.

## Routing
- Always use Yew Router for SPA navigation.
- Always define routes in `frontend/src/app.rs`.
- Always redirect protected routes to `/login` if no valid JWT is present.

## State Management
- Always use Yew contexts and reducers for shared state.
- Always store JWT token in `localStorage`.
- Always clear auth state on token expiry or logout.

## Styling
- Always use Tailwind CSS for styling.
- Always use utility classes directly.
- Never write custom CSS unless necessary.

## API Communication
- Always use `gloo-net` for HTTP requests from the browser.
- Always centralize API calls in `frontend/src/services/api.rs`.
- Always attach JWT token as `Authorization: Bearer <token>` to authenticated requests.
- Always handle 401 responses by redirecting to login.

## Components
- Always place layout components in `frontend/src/components/layout/`.
- Always place common/shared components in `frontend/src/components/common/`.
- Always place feature components in `frontend/src/components/<feature>/`.
- Always place pages in `frontend/src/pages/`.

## Language
- Always write all visible UI text in Spanish.
- Never use English strings in user-facing components.
- Always use Dominican Republic locale conventions for dates (DD/MM/YYYY) and currency (DOP/USD).

## Forms
- Always validate input client-side before submission.
- Always display inline validation errors in Spanish.
- Always use controlled components pattern with Yew state hooks.

## Build
- Always build with `trunk build --release` for production.
- Always use `trunk serve` for development.
