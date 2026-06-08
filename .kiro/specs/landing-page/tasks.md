# Implementation Plan: Landing Page

## Overview

Add a public landing page at route `/` that introduces the Gestión Inmobiliaria platform to visitors. The implementation creates a `components/landing/` module with seven section components (Hero, Stats, How It Works, Features, Preview, Transparency, Footer), a `pages/landing.rs` page component with auth redirect, and modifies the router in `app.rs` to move Login to `/login`. Section order follows a progressive trust-building narrative: Hero → Stats → How It Works → Features → Preview → Transparency → Footer. All content is static and in Spanish. No backend changes.

## Tasks

- [x] 1. Create landing component module structure
  - [x] 1.1 Create `frontend/src/components/landing/mod.rs` with module declarations and re-exports
    - Declare `pub mod hero; pub mod stats; pub mod how_it_works; pub mod features; pub mod preview; pub mod transparency; pub mod footer;`
    - Re-export all 7 component names: `LandingHero`, `LandingStats`, `LandingHowItWorks`, `LandingFeatures`, `LandingPreview`, `LandingTransparency`, `LandingFooter`
    - _Requirements: 11.1, 11.3_

  - [x] 1.2 Register the landing module in `frontend/src/components/mod.rs`
    - Add `pub mod landing;` to the existing module declarations
    - _Requirements: 11.1_

- [ ] 2. Implement landing section components (part 1: Hero, Stats, How It Works)
  - [x] 2.1 Create `frontend/src/components/landing/hero.rs` — Hero section component
    - Implement `LandingHero` functional component with headline, supporting paragraph, and two CTA buttons
    - Headline uses `var(--font-display)` (Bitter), body uses `var(--font-body)` (Source Sans 3)
    - Primary CTA "Registrarse gratis" links to `Route::Registro` with `background-color: #3d8b8b`, `px-8 py-3.5 text-lg font-bold text-white`
    - Secondary CTA "Ya tengo cuenta" links to `Route::Login` with outline border styling, `px-6 py-3 font-semibold`
    - Responsive: full-width buttons on mobile (`w-full sm:w-auto`), stacked to inline (`flex-col sm:flex-row`)
    - Section centered with `max-w-4xl mx-auto`, padding `px-4 py-16 md:py-24`
    - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7, 9.1, 9.2, 9.3, 12.3_

  - [-] 2.2 Create `frontend/src/components/landing/stats.rs` — Stats bar component
    - Implement `LandingStats` functional component with horizontal trust indicators
    - Display: GitHub stars indicator (⭐ emoji + text), active development badge (green dot + "Proyecto en desarrollo activo"), free & open-source indicator (🆓 + "100% gratis y código abierto")
    - Use muted styling: `var(--text-tertiary)` text color, `var(--surface-raised)` background, `border-y` with `var(--border-subtle)`
    - Layout: `flex flex-wrap items-center justify-center gap-6 text-sm` within `max-w-4xl mx-auto`
    - Green dot for active development: `w-2 h-2 rounded-full` with `background-color: #22c55e`
    - _Requirements: 3.1, 3.2, 3.3, 3.4, 10.1_

  - [-] 2.3 Create `frontend/src/components/landing/how_it_works.rs` — How It Works section component
    - Implement `LandingHowItWorks` functional component with 3 sequential steps
    - Define internal `Step` struct with `number`, `title`, `description` fields and static `STEPS` array
    - Steps: (1) "Registra tus propiedades", (2) "Organiza inquilinos y contratos", (3) "Controla pagos y gastos"
    - Each step renders a numbered circle (`w-12 h-12 rounded-full` with `background-color: #3d8b8b`, white text), title, and description
    - Section heading: "Cómo funciona" centered
    - Layout: `grid-cols-1 md:grid-cols-3 gap-8` within `max-w-5xl mx-auto`
    - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5, 10.1, 12.1_

- [ ] 3. Implement landing section components (part 2: Features, Preview, Transparency, Footer)
  - [-] 3.1 Create `frontend/src/components/landing/features.rs` — Features grid component
    - Implement `LandingFeatures` functional component with a static `FEATURES` array of exactly 6 items
    - Define internal `FeatureItem` struct with `icon`, `title`, `description` fields
    - 6 grouped capabilities: Propiedades y Unidades, Inquilinos y Contratos, Pagos y Cobros, Gastos y Reportes, Mantenimiento, Dashboard en tiempo real
    - Each card: emoji icon, bold title, short Spanish description in a rounded card with `var(--surface-raised)` background and `var(--border-subtle)` border
    - Section heading: "Todo lo que necesitas para administrar"
    - Responsive grid: `grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4` within `max-w-6xl mx-auto`
    - _Requirements: 5.1, 5.2, 5.3, 5.4, 10.1, 10.2, 12.1_

  - [-] 3.2 Create `frontend/src/components/landing/preview.rs` — Dashboard preview component
    - Implement `LandingPreview` functional component with heading, GIF image, and demo CTA
    - Section heading: "Así se ve por dentro"
    - Image source: `/assets/dashboard-preview.gif` with descriptive `alt` text in Spanish
    - Image container: `rounded-xl overflow-hidden` with `var(--shadow-lg)` shadow and `var(--border-subtle)` border
    - Use `loading="lazy"` on the image for performance
    - Secondary CTA: "Ver demo en vivo" — external link with `target="_blank"`, `rel="noopener noreferrer"`, outline styling (`px-5 py-2.5 font-semibold`, border, no fill)
    - Layout: `max-w-5xl mx-auto`, CTA centered below image
    - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5, 9.3, 10.1_

  - [~] 3.3 Create `frontend/src/components/landing/transparency.rs` — Open-source section component
    - Implement `LandingTransparency` functional component with heading, paragraph, and GitHub link
    - Heading: "Código abierto, hecho con cariño"
    - Friendly tone emphasizing community and fun, not commercial
    - Link to `https://github.com/jpilier/Gestion-inmobiliaria` with `target="_blank"` and `rel="noopener noreferrer"`
    - Link styled as Secondary CTA: outline border, `px-5 py-2.5 font-semibold`
    - Section uses `var(--surface-raised)` background for visual distinction
    - _Requirements: 7.1, 7.2, 7.3, 9.3, 10.1_

  - [~] 3.4 Create `frontend/src/components/landing/footer.rs` — Landing footer component
    - Implement `LandingFooter` functional component with project name and attribution
    - Display "Gestión Inmobiliaria" and "© 2025 — Proyecto de código abierto"
    - Visually distinct with `border-top` using `var(--border-subtle)` and muted text color `var(--text-tertiary)`
    - _Requirements: 8.1, 8.2, 8.3, 10.1_

- [ ] 4. Implement landing page and route integration
  - [~] 4.1 Create `frontend/src/pages/landing.rs` — Page-level component with auth redirect
    - Implement `Landing` functional component
    - Use `is_authenticated()` to check JWT in localStorage on mount
    - Redirect authenticated users to `Route::Dashboard` via `use_navigator()` + `use_effect_with((), ...)`
    - Render empty HTML while redirect happens if authenticated
    - Compose all 7 section components in prescribed order: `LandingHero`, `LandingStats`, `LandingHowItWorks`, `LandingFeatures`, `LandingPreview`, `LandingTransparency`, `LandingFooter`
    - Wrap in `<div class="min-h-screen">` with `var(--surface-base)` background and `var(--text-primary)` color
    - _Requirements: 1.4, 11.1, 11.3, 14.1, 14.2_

  - [~] 4.2 Register the landing page in `frontend/src/pages/mod.rs`
    - Add `pub mod landing;` to the existing module declarations
    - _Requirements: 11.1_

  - [~] 4.3 Modify `frontend/src/app.rs` — Update Route enum and switch function
    - Add `Landing` variant with `#[at("/")]` attribute
    - Change `Login` variant from `#[at("/")]` to `#[at("/login")]`
    - Add `use crate::pages::landing::Landing;` import
    - Add `Route::Landing => html! { <Landing /> }` arm in the `switch` function (before Login)
    - Verify `Route::Registro` remains at `/registro`
    - _Requirements: 1.1, 1.2, 1.3_

- [~] 5. Checkpoint — Verify compilation and routing
  - Ensure all tests pass, ask the user if questions arise.

- [ ] 6. Add placeholder asset and tests
  - [~] 6.1 Create placeholder dashboard preview GIF
    - Add a placeholder GIF at `frontend/assets/dashboard-preview.gif` (can be a minimal placeholder until a real recording is produced)
    - _Requirements: 6.1_

  - [~] 6.2 Write property test for step card rendering completeness
    - **Property 1: Step card rendering completeness**
    - For any Step definition containing a number, title, and description, the rendered step card output SHALL include a visible number element, a non-empty title heading, and a non-empty description paragraph
    - **Validates: Requirements 4.3**

  - [~] 6.3 Write property test for feature card rendering completeness
    - **Property 2: Feature card rendering completeness**
    - For any FeatureItem definition containing an icon, title, and description, the rendered feature card output SHALL include the icon element, a non-empty title, and a non-empty description string
    - **Validates: Requirements 5.2**

  - [~] 6.4 Write property test for secondary CTA styling consistency
    - **Property 3: Secondary CTA styling consistency**
    - For any action button rendered on the landing page that is not labeled "Registrarse gratis", it SHALL use secondary styling (outline border without filled accent background) and SHALL NOT use the `#3d8b8b` background color
    - **Validates: Requirements 9.1, 9.3**

  - [~] 6.5 Write unit tests for route mapping and auth redirect
    - Verify `/` renders `Landing` component
    - Verify `/login` renders `Login` component
    - Verify authenticated user at `/` redirects to `/dashboard`
    - _Requirements: 1.1, 1.2, 1.4_

- [~] 7. Final checkpoint — Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

## Notes

- Tasks marked with `*` are optional and can be skipped for faster MVP
- Each task references specific requirements for traceability
- Checkpoints ensure incremental validation
- Property tests validate universal correctness properties from the design document
- Unit tests validate specific examples and edge cases
- All static text is in Spanish with Dominican Republic property management terminology
- No backend changes are required — this is purely a frontend feature
- The dashboard preview GIF is a placeholder; replace with a real animated recording when available
- Section order is enforced: Hero → Stats → How It Works → Features → Preview → Transparency → Footer
- The features grid uses exactly 6 grouped capabilities (reduced from 11 individual features for lower cognitive load)
- The `mod.rs` declares and re-exports all 7 components

## Task Dependency Graph

```json
{
  "waves": [
    { "id": 0, "tasks": ["1.1", "1.2"] },
    { "id": 1, "tasks": ["2.1", "2.2", "2.3", "3.1", "3.2", "3.3", "3.4", "4.2"] },
    { "id": 2, "tasks": ["4.1"] },
    { "id": 3, "tasks": ["4.3"] },
    { "id": 4, "tasks": ["6.1", "6.2", "6.3", "6.4", "6.5"] }
  ]
}
```
