# Technical Design Document

## Overview

The landing page is a purely frontend feature that introduces new visitors to the Gestión Inmobiliaria platform. It adds a new public route at `/` with seven visual sections (Hero, Stats Bar, How It Works, Features Grid, Dashboard Preview, Open-Source Transparency, Footer), moves the existing login form to `/login`, and redirects authenticated users from `/` to `/dashboard`.

The page follows a progressive trust-building narrative: immediate value proposition (Hero), social proof (Stats), simplicity demonstration (How It Works), capability overview (Features), visual confidence (Preview), and transparency (Open-Source). A single primary CTA — "Registrarse gratis" — dominates throughout.

No backend changes are required. The implementation uses Yew 0.23 functional components, Tailwind CSS utilities, and the existing CSS custom property theming system.

## Architecture

### Component Hierarchy

```
LandingPage (pages/landing.rs)
├── LandingHero (components/landing/hero.rs)
├── LandingStats (components/landing/stats.rs)          — NEW
├── LandingHowItWorks (components/landing/how_it_works.rs) — NEW
├── LandingFeatures (components/landing/features.rs)    — UPDATED (6 items)
├── LandingPreview (components/landing/preview.rs)      — UPDATED (demo CTA)
├── LandingTransparency (components/landing/transparency.rs)
└── LandingFooter (components/landing/footer.rs)
```

Each section is an independent functional component with no props (all content is static). This keeps every `html!` block well under the 150-line limit.

### File Structure

```
frontend/src/
├── pages/
│   ├── landing.rs          (NEW — page-level component with auth redirect)
│   └── mod.rs              (MODIFIED — add `pub mod landing;`)
├── components/
│   ├── landing/
│   │   ├── mod.rs          (NEW — re-exports)
│   │   ├── hero.rs         (NEW)
│   │   ├── stats.rs        (NEW)
│   │   ├── how_it_works.rs (NEW)
│   │   ├── features.rs     (NEW)
│   │   ├── preview.rs      (NEW)
│   │   ├── transparency.rs (NEW)
│   │   └── footer.rs       (NEW)
│   └── mod.rs              (MODIFIED — add `pub mod landing;`)
├── app.rs                  (MODIFIED — route changes)
```

## Route Changes

### Current State

```rust
#[at("/")]
Login,
```

The `/` route currently renders the `Login` component directly.

### Target State

```rust
#[derive(Clone, Debug, Routable, PartialEq, Eq)]
pub enum Route {
    #[at("/")]
    Landing,            // NEW — public landing page
    #[at("/login")]
    Login,              // MOVED from "/" to "/login"
    #[at("/registro")]
    Registro,           // UNCHANGED
    #[at("/dashboard")]
    Dashboard,
    // ... rest unchanged
}
```

### Switch Function Changes

```rust
fn switch(routes: Route) -> Html {
    match routes {
        Route::Landing => html! { <Landing /> },
        Route::Login => html! { <Login /> },
        // ... rest unchanged
    }
}
```

### Impact on Existing Code

- The `ProtectedRoute` component currently redirects unauthenticated users to `Route::Login`. This reference remains correct since `Login` just changes its `#[at(...)]` attribute.
- The `NotFound` page links to `Route::Login` — this should remain pointing to `Route::Login` (the login page).
- The `logout()` function in `services/auth.rs` sets `href = "/"` — after this change, that lands on the landing page, which is acceptable behavior (visitor sees the landing page after logout).

## Component Design

### `pages/landing.rs` — Landing Page

**Responsibility:** Top-level page component that checks auth state and either redirects to dashboard or renders the landing sections in the prescribed order.

```rust
use yew::prelude::*;
use yew_router::prelude::*;

use crate::app::Route;
use crate::components::landing::{
    LandingFeatures, LandingFooter, LandingHero, LandingHowItWorks,
    LandingPreview, LandingStats, LandingTransparency,
};
use crate::services::auth::is_authenticated;

#[component]
pub fn Landing() -> Html {
    let navigator = use_navigator();

    // Redirect authenticated users to dashboard
    use_effect_with((), move |()| {
        if is_authenticated() {
            if let Some(nav) = navigator {
                nav.push(&Route::Dashboard);
            }
        }
    });

    // If authenticated, render nothing while redirect happens
    if is_authenticated() {
        return html! {};
    }

    html! {
        <div class="min-h-screen" style="background-color: var(--surface-base); color: var(--text-primary);">
            <LandingHero />
            <LandingStats />
            <LandingHowItWorks />
            <LandingFeatures />
            <LandingPreview />
            <LandingTransparency />
            <LandingFooter />
        </div>
    }
}
```

**Auth redirect approach:** Uses `is_authenticated()` (checks localStorage for JWT) rather than `AuthContext` to avoid timing issues with context restoration on fresh page loads. The `use_effect_with((), ...)` fires once on mount, matching the login page pattern.

### `components/landing/hero.rs` — Hero Section

**Responsibility:** Displays the main headline, supporting subtitle, and two CTA buttons. The Primary CTA ("Registrarse gratis") is visually dominant via filled teal background and larger size. The Secondary CTA ("Ya tengo cuenta") uses outline styling.

```rust
use yew::prelude::*;
use yew_router::prelude::*;

use crate::app::Route;

#[component]
pub fn LandingHero() -> Html {
    html! {
        <section class="px-4 py-16 md:py-24 text-center max-w-4xl mx-auto">
            <h1
                class="text-3xl md:text-5xl font-bold mb-4"
                style="font-family: var(--font-display); color: var(--text-primary);"
            >
                {"Gestiona tus propiedades en República Dominicana — simple y gratis"}
            </h1>
            <p
                class="text-lg md:text-xl mb-8 max-w-2xl mx-auto"
                style="font-family: var(--font-body); color: var(--text-secondary);"
            >
                {"Una herramienta hecha para administradores que quieren tener todo en orden: propiedades, inquilinos, contratos, pagos y más — sin complicaciones."}
            </p>
            <div class="flex flex-col sm:flex-row gap-3 justify-center">
                <Link<Route>
                    to={Route::Registro}
                    classes="w-full sm:w-auto px-8 py-3.5 rounded-lg font-bold text-white text-center text-lg"
                    style="background-color: #3d8b8b;"
                >
                    {"Registrarse gratis"}
                </Link<Route>>
                <Link<Route>
                    to={Route::Login}
                    classes="w-full sm:w-auto px-6 py-3 rounded-lg font-semibold text-center"
                    style="border: 1px solid var(--border-default); color: var(--text-primary);"
                >
                    {"Ya tengo cuenta"}
                </Link<Route>>
            </div>
        </section>
    }
}
```

**CTA hierarchy:** The Primary CTA uses `px-8 py-3.5 text-lg font-bold` with filled `#3d8b8b` background. The Secondary CTA uses `px-6 py-3 font-semibold` with outline border. This establishes clear visual dominance through size, weight, and color contrast.

### `components/landing/stats.rs` — Stats Bar

**Responsibility:** Displays social proof indicators (GitHub stars, active development badge) in a compact horizontal bar immediately after the hero. Uses muted styling to avoid competing with the Primary CTA.

```rust
use yew::prelude::*;

#[component]
pub fn LandingStats() -> Html {
    html! {
        <section
            class="px-4 py-4 border-y"
            style="border-color: var(--border-subtle); background-color: var(--surface-raised);"
        >
            <div class="max-w-4xl mx-auto flex flex-wrap items-center justify-center gap-6 text-sm"
                 style="color: var(--text-tertiary);">
                <div class="flex items-center gap-2">
                    <span>{"⭐"}</span>
                    <span>{"GitHub Stars"}</span>
                </div>
                <div class="flex items-center gap-2">
                    <span
                        class="w-2 h-2 rounded-full inline-block"
                        style="background-color: #22c55e;"
                    ></span>
                    <span>{"Proyecto en desarrollo activo"}</span>
                </div>
                <div class="flex items-center gap-2">
                    <span>{"🆓"}</span>
                    <span>{"100% gratis y código abierto"}</span>
                </div>
            </div>
        </section>
    }
}
```

**Design decisions:**
- Uses `var(--text-tertiary)` for muted text so the stats bar doesn't distract from the hero's CTA.
- Horizontal `flex-wrap` layout collapses gracefully on mobile.
- The green dot (active development indicator) uses a simple inline span rather than an external icon dependency.
- GitHub stars can later be fetched dynamically via the GitHub API, but the initial implementation uses a static indicator to avoid external API dependencies on page load.

### `components/landing/how_it_works.rs` — How It Works Section

**Responsibility:** Presents three sequential steps that explain the platform workflow, reducing perceived complexity. Uses a horizontal layout on desktop and stacked vertical layout on mobile.

```rust
use yew::prelude::*;

struct Step {
    number: &'static str,
    title: &'static str,
    description: &'static str,
}

const STEPS: &[Step] = &[
    Step {
        number: "1",
        title: "Registra tus propiedades",
        description: "Añade tus inmuebles con dirección, unidades y precio. Todo organizado desde el inicio.",
    },
    Step {
        number: "2",
        title: "Organiza inquilinos y contratos",
        description: "Asocia inquilinos a tus propiedades con contratos claros: fechas, montos y estado.",
    },
    Step {
        number: "3",
        title: "Controla pagos y gastos",
        description: "Registra cobros, da seguimiento a pagos atrasados y lleva el control de cada gasto.",
    },
];

#[component]
pub fn LandingHowItWorks() -> Html {
    html! {
        <section class="px-4 py-12 max-w-5xl mx-auto">
            <h2
                class="text-2xl md:text-3xl font-bold text-center mb-10"
                style="font-family: var(--font-display); color: var(--text-primary);"
            >
                {"Cómo funciona"}
            </h2>
            <div class="grid grid-cols-1 md:grid-cols-3 gap-8">
                { for STEPS.iter().map(|step| html! {
                    <div class="text-center">
                        <div
                            class="w-12 h-12 rounded-full flex items-center justify-center text-xl font-bold mx-auto mb-4 text-white"
                            style="background-color: #3d8b8b;"
                        >
                            { step.number }
                        </div>
                        <h3
                            class="text-lg font-semibold mb-2"
                            style="color: var(--text-primary);"
                        >
                            { step.title }
                        </h3>
                        <p class="text-sm" style="color: var(--text-secondary);">
                            { step.description }
                        </p>
                    </div>
                })}
            </div>
        </section>
    }
}
```

**Layout:** `grid-cols-1 md:grid-cols-3` gives vertical stacking on mobile and horizontal three-column on desktop. Each step uses a numbered circle with the accent color to create visual sequential flow.

### `components/landing/features.rs` — Features Grid

**Responsibility:** Displays six grouped platform capabilities with icons and descriptions. Reduced from the previous 11 individual features to 6 logical groupings for lower cognitive load.

```rust
use yew::prelude::*;

struct FeatureItem {
    icon: &'static str,
    title: &'static str,
    description: &'static str,
}

const FEATURES: &[FeatureItem] = &[
    FeatureItem {
        icon: "🏠",
        title: "Propiedades y Unidades",
        description: "Registra inmuebles, divide en unidades y controla el estado de cada uno.",
    },
    FeatureItem {
        icon: "👤",
        title: "Inquilinos y Contratos",
        description: "Gestiona inquilinos con sus contratos, fechas de vigencia y montos mensuales.",
    },
    FeatureItem {
        icon: "💰",
        title: "Pagos y Cobros",
        description: "Registra cobros en DOP o USD, identifica atrasos y genera recibos.",
    },
    FeatureItem {
        icon: "📊",
        title: "Gastos y Reportes",
        description: "Controla gastos por categoría y genera informes de ingresos y ocupación.",
    },
    FeatureItem {
        icon: "🔧",
        title: "Mantenimiento",
        description: "Solicitudes de reparación con seguimiento de estado y prioridad.",
    },
    FeatureItem {
        icon: "📈",
        title: "Dashboard en tiempo real",
        description: "Vista general de tu portafolio: ocupación, cobros pendientes y vencimientos.",
    },
];

#[component]
pub fn LandingFeatures() -> Html {
    html! {
        <section class="px-4 py-12 max-w-6xl mx-auto">
            <h2
                class="text-2xl md:text-3xl font-bold text-center mb-8"
                style="font-family: var(--font-display); color: var(--text-primary);"
            >
                {"Todo lo que necesitas para administrar"}
            </h2>
            <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
                { for FEATURES.iter().map(|f| html! {
                    <div
                        class="p-5 rounded-lg"
                        style="background-color: var(--surface-raised); border: 1px solid var(--border-subtle);"
                    >
                        <div class="text-2xl mb-2">{ f.icon }</div>
                        <h3
                            class="font-semibold mb-1"
                            style="color: var(--text-primary);"
                        >
                            { f.title }
                        </h3>
                        <p class="text-sm" style="color: var(--text-secondary);">
                            { f.description }
                        </p>
                    </div>
                })}
            </div>
        </section>
    }
}
```

**Grouping rationale:** The 6 capabilities map to the platform's core domain entities grouped logically: (1) Propiedades + Unidades, (2) Inquilinos + Contratos, (3) Pagos + Cobros, (4) Gastos + Reportes, (5) Mantenimiento, (6) Dashboard. This reduces cognitive load compared to listing 11 separate features.

### `components/landing/preview.rs` — Dashboard Preview

**Responsibility:** Shows a visual representation of the dashboard (animated GIF or screenshot) and includes a Secondary CTA linking to a live demo.

```rust
use yew::prelude::*;

#[component]
pub fn LandingPreview() -> Html {
    html! {
        <section class="px-4 py-12 max-w-5xl mx-auto">
            <h2
                class="text-2xl md:text-3xl font-bold text-center mb-6"
                style="font-family: var(--font-display); color: var(--text-primary);"
            >
                {"Así se ve por dentro"}
            </h2>
            <div
                class="rounded-xl overflow-hidden mb-6"
                style="box-shadow: var(--shadow-lg); border: 1px solid var(--border-subtle);"
            >
                <img
                    src="/assets/dashboard-preview.gif"
                    alt="Demo animada del dashboard de Gestión Inmobiliaria"
                    class="w-full h-auto"
                    loading="lazy"
                />
            </div>
            <div class="text-center">
                <a
                    href="https://demo.gestion-inmobiliaria.com"
                    target="_blank"
                    rel="noopener noreferrer"
                    class="inline-flex items-center gap-2 px-5 py-2.5 rounded-lg font-semibold"
                    style="border: 1px solid var(--border-default); color: var(--text-primary);"
                >
                    {"Ver demo en vivo"}
                </a>
            </div>
        </section>
    }
}
```

**Design decisions:**
- The image source defaults to `.gif` to support the animated walkthrough. Falls back gracefully if a static `.png` is used instead.
- The "Ver demo en vivo" link uses outline styling (Secondary CTA) to remain subordinate to the Primary CTA.
- `loading="lazy"` prevents the GIF from blocking initial page render.
- The demo URL is a placeholder — the actual live demo instance URL should be configured when deployed.

### `components/landing/transparency.rs` — Open-Source Section

**Responsibility:** Communicates the open-source nature and community motivation with a friendly tone.

```rust
use yew::prelude::*;

#[component]
pub fn LandingTransparency() -> Html {
    html! {
        <section
            class="px-4 py-12 text-center"
            style="background-color: var(--surface-raised);"
        >
            <div class="max-w-3xl mx-auto">
                <h2
                    class="text-2xl md:text-3xl font-bold mb-4"
                    style="font-family: var(--font-display); color: var(--text-primary);"
                >
                    {"Código abierto, hecho con cariño"}
                </h2>
                <p
                    class="text-lg mb-6"
                    style="font-family: var(--font-body); color: var(--text-secondary);"
                >
                    {"Este proyecto es de código abierto — lo construimos porque nos pareció útil y divertido. No hay letra pequeña, ni planes de pago escondidos. Si te sirve, úsalo."}
                </p>
                <a
                    href="https://github.com/jpilier/Gestion-inmobiliaria"
                    target="_blank"
                    rel="noopener noreferrer"
                    class="inline-flex items-center gap-2 px-5 py-2.5 rounded-lg font-semibold"
                    style="border: 1px solid var(--border-default); color: var(--text-primary);"
                >
                    {"Ver código en GitHub"}
                </a>
            </div>
        </section>
    }
}
```

### `components/landing/footer.rs` — Landing Footer

**Responsibility:** Displays project name and attribution. Distinct from the app's internal `layout/footer.rs`.

```rust
use yew::prelude::*;

#[component]
pub fn LandingFooter() -> Html {
    html! {
        <footer
            class="px-4 py-8 text-center"
            style="border-top: 1px solid var(--border-subtle); color: var(--text-tertiary);"
        >
            <p class="font-semibold" style="color: var(--text-secondary);">
                {"Gestión Inmobiliaria"}
            </p>
            <p class="text-sm mt-1">
                {"© 2025 — Proyecto de código abierto"}
            </p>
        </footer>
    }
}
```

### `components/landing/mod.rs`

```rust
pub mod footer;
pub mod features;
pub mod hero;
pub mod how_it_works;
pub mod preview;
pub mod stats;
pub mod transparency;

pub use footer::LandingFooter;
pub use features::LandingFeatures;
pub use hero::LandingHero;
pub use how_it_works::LandingHowItWorks;
pub use preview::LandingPreview;
pub use stats::LandingStats;
pub use transparency::LandingTransparency;
```

## Data Flow

This feature has no data fetching or backend interaction. The data flow is minimal:

```
localStorage (JWT check)
       │
       ▼
LandingPage ──[authenticated?]──► Navigator.push("/dashboard")
       │
       ▼ (not authenticated)
Static HTML render (Hero → Stats → How It Works → Features → Preview → Transparency → Footer)
       │
       ▼ (user clicks CTA)
Router navigation to /registro, /login, or external link (demo, GitHub)
```

**State dependencies:**
- `is_authenticated()` — reads `jwt_token` from localStorage (no context needed)
- `use_navigator()` — provided by `BrowserRouter` already in the tree
- No `AuthContext` consumption in the landing page itself (avoids coupling to context restoration timing)

## Responsive Design Strategy

### Tailwind Breakpoint Usage

| Breakpoint | Width | Layout behavior |
|---|---|---|
| Default (mobile) | < 640px | Single column, full-width CTAs, stacked content |
| `sm:` | ≥ 640px | CTAs become inline, 2-column features grid |
| `md:` | ≥ 768px | Larger headings, more generous padding, 3-column steps |
| `lg:` | ≥ 1024px | 3-column features grid |

### CSS Pattern

All components use mobile-first Tailwind utilities:

```
class="text-3xl md:text-5xl"              -- font size scaling
class="py-16 md:py-24"                    -- vertical rhythm
class="flex-col sm:flex-row"              -- CTA button layout
class="grid-cols-1 md:grid-cols-3"        -- steps layout
class="grid-cols-1 sm:grid-cols-2 lg:grid-cols-3" -- features grid
class="w-full sm:w-auto"                  -- CTA width
class="px-4"                              -- consistent horizontal padding
class="max-w-4xl mx-auto"                 -- content width constraint
```

### Theming

All color values use CSS custom properties (`var(--surface-base)`, `var(--text-primary)`, etc.) which automatically adapt between light and dark themes via the `[data-theme="dark"]` selector defined in `tailwind.css`. The accent color `#3d8b8b` is used directly for the primary CTA and step number circles since it's the brand color with sufficient contrast in both themes.

### Single Primary CTA Focus Pattern

The visual hierarchy is enforced structurally:

| Element | Style | Purpose |
|---|---|---|
| "Registrarse gratis" | `px-8 py-3.5 text-lg font-bold bg-[#3d8b8b] text-white` | Primary CTA — largest, filled, accent color |
| "Ya tengo cuenta" | `px-6 py-3 font-semibold border outline` | Secondary — smaller, outline, no fill |
| "Ver demo en vivo" | `px-5 py-2.5 font-semibold border outline` | Secondary — smaller still, outline |
| "Ver código en GitHub" | `px-5 py-2.5 font-semibold border outline` | Secondary — outline, same tier as demo |

## Components and Interfaces

### Public Interfaces

| Component | Props | Description |
|---|---|---|
| `Landing` | None | Page-level component at route `/`. Redirects authenticated users to `/dashboard`. |
| `LandingHero` | None | Hero section with headline, subtitle, and CTA buttons. |
| `LandingStats` | None | Trust bar with GitHub stars and active development indicator. |
| `LandingHowItWorks` | None | Three sequential steps explaining the workflow. |
| `LandingFeatures` | None | Responsive grid of 6 grouped platform capabilities. |
| `LandingPreview` | None | Dashboard GIF/screenshot and "Ver demo en vivo" link. |
| `LandingTransparency` | None | Open-source messaging section. |
| `LandingFooter` | None | Project name and attribution footer. |

All components are stateless and prop-less — they render static content. This means no `Properties` derive macro is needed and no `AttrValue` or `Rc` wrapping is required.

### Internal Interface: Route Enum Change

The `Route` enum in `app.rs` gains a `Landing` variant and the existing `Login` variant changes its `#[at(...)]` path:

```rust
// Before
#[at("/")]
Login,

// After
#[at("/")]
Landing,
#[at("/login")]
Login,
```

## Data Models

This feature introduces no new data models or types. All content is static and hardcoded in the component source files.

The internal data structures (not exported) used to organize static content for rendering:

```rust
// In how_it_works.rs
struct Step {
    number: &'static str,
    title: &'static str,
    description: &'static str,
}

// In features.rs
struct FeatureItem {
    icon: &'static str,
    title: &'static str,
    description: &'static str,
}
```

These structs exist solely to organize static arrays for iteration in `html!` blocks.

## Testing Strategy

### Unit Tests (Example-Based)

The landing page is primarily static content, so testing focuses on:

1. **Route mapping:** Verify `/` renders `Landing`, `/login` renders `Login`, `/registro` renders `Registro`.
2. **Auth redirect:** Verify that when a JWT is present in localStorage, navigating to `/` redirects to `/dashboard`.
3. **Section ordering:** Verify sections render in the correct order: Hero, Stats, How It Works, Features, Preview, Transparency, Footer.
4. **Content presence:** Verify each section renders its expected headings and CTA text.
5. **Feature count:** Verify exactly 6 features appear in the grid.
6. **Step count:** Verify exactly 3 steps appear in How It Works.
7. **CTA targets:** Verify "Registrarse gratis" links to `/registro`, "Ya tengo cuenta" links to `/login`, "Ver demo en vivo" links to demo instance.

### Property-Based Tests

Three properties are suitable for PBT in this feature (see Correctness Properties section):
1. Step card rendering completeness
2. Feature card rendering completeness
3. Secondary CTA styling consistency

### Not Tested (Visual/CSS)

- Responsive breakpoint behavior → manual browser testing
- Light/dark theme rendering → manual visual review
- Font family application → manual visual review
- Color contrast → verified statically against WCAG guidelines
- Visual dominance of Primary CTA → verified by structural style checks

## Error Handling

This feature has no error states since it renders static content with no network requests. The only edge case is the auth redirect:

- If `localStorage` is unavailable (private browsing in some browsers), `is_authenticated()` returns `false` and the landing page renders normally.
- If `use_navigator()` returns `None` (shouldn't happen inside `BrowserRouter`), the component returns empty HTML — matching the existing pattern in `login.rs`.
- If the dashboard preview image fails to load, the `<img>` element renders as empty space with alt text visible — acceptable degradation.

## Accessibility

- Semantic HTML: `<section>`, `<h1>`/`<h2>`/`<h3>`, `<footer>` elements
- Image alt text on the dashboard preview GIF
- Sufficient color contrast: `#3d8b8b` on white passes WCAG AA for large text (the CTA has white text on teal)
- Keyboard navigable: all interactive elements are `<a>` or `<Link>` (natively focusable)
- `loading="lazy"` on the preview image for performance
- Step numbers provide sequential context without relying solely on visual positioning

## Correctness Properties

*A property is a characteristic or behavior that should hold true across all valid executions of a system — essentially, a formal statement about what the system should do. Properties serve as the bridge between human-readable specifications and machine-verifiable correctness guarantees.*

### Property 1: Step card rendering completeness

*For any* step definition containing a number, title, and description, the rendered step card output SHALL include a visible number element, a non-empty title heading, and a non-empty description paragraph.

**Validates: Requirements 4.3**

### Property 2: Feature card rendering completeness

*For any* feature item definition containing an icon, title, and description, the rendered feature card output SHALL include the icon element, a non-empty title, and a non-empty description string.

**Validates: Requirements 5.2**

### Property 3: Secondary CTA styling consistency

*For any* action button rendered on the landing page that is not labeled "Registrarse gratis", it SHALL use secondary styling (outline border without filled accent background color) and SHALL NOT use the `#3d8b8b` background color.

**Validates: Requirements 9.1, 9.3**
