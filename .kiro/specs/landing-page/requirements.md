# Requirements Document

## Introduction

Public landing page for the Gestión Inmobiliaria platform — a free, open-source property management tool built for Dominican Republic property managers. The landing page introduces the product's value proposition, showcases its capabilities through a streamlined narrative (Hero → Trust → How It Works → Features → Preview → Transparency → Footer), and provides clear paths to registration and login. The page lives at route "/" while the login form lives at "/login". All content is in Spanish with a friendly, non-aggressive tone aimed at non-technical property managers. The design follows best practices from successful open-source SaaS landing pages: a single primary CTA focus, progressive trust-building, and reduced cognitive load through grouped features and a clear 3-step process.

## Glossary

- **Landing_Page**: The public-facing page rendered at route "/" that presents the platform's value proposition and directs visitors to registration or login.
- **Hero_Section**: The top section of the Landing_Page containing the main headline, supporting subtitle, and call-to-action buttons.
- **Stats_Bar**: A horizontal bar displayed after the Hero_Section showing social proof indicators such as GitHub stars count and active development status.
- **How_It_Works_Section**: A section displaying three sequential steps that explain how the platform works, reducing perceived complexity for visitors.
- **Features_Grid**: A grid layout displaying six focused platform capabilities with icons and short descriptions.
- **Dashboard_Preview**: A visual section showing an animated GIF walkthrough or screenshot of the application dashboard, with a secondary CTA linking to a live demo.
- **Transparency_Section**: A section communicating that the project is open-source and built as a useful community tool.
- **Landing_Footer**: The bottom section of the Landing_Page with project links and credits.
- **Primary_CTA**: The "Registrarse gratis" button that navigates to /registro — the single most prominent action on the page.
- **Secondary_CTA**: Any visually subordinate action button (e.g., "Ya tengo cuenta", "Ver demo en vivo", GitHub link) that does not compete with the Primary_CTA for attention.
- **Router**: The Yew frontend router that maps URL paths to page components.
- **Authenticated_User**: A user who has a valid JWT token stored in localStorage.

## Requirements

### Requirement 1: Route restructuring

**User Story:** As a visitor, I want to see a landing page at "/" so that I can learn about the platform before deciding to register or log in.

#### Acceptance Criteria

1. THE Router SHALL render the Landing_Page component at route "/".
2. THE Router SHALL render the Login component at route "/login".
3. THE Router SHALL continue to render the Registro component at route "/registro".
4. WHEN an Authenticated_User navigates to route "/", THE Landing_Page SHALL redirect the Authenticated_User to "/dashboard".

### Requirement 2: Hero section

**User Story:** As a visitor, I want to immediately understand what this platform does so that I can decide if it solves my property management needs.

#### Acceptance Criteria

1. THE Hero_Section SHALL display a headline communicating the platform's value proposition for Dominican Republic property managers.
2. THE Hero_Section SHALL display a supporting subtitle with a friendly, non-technical description of the platform's purpose.
3. THE Hero_Section SHALL display the Primary_CTA labeled "Registrarse gratis" that navigates to "/registro".
4. THE Hero_Section SHALL display a Secondary_CTA labeled "Ya tengo cuenta" that navigates to "/login".
5. THE Primary_CTA SHALL be visually dominant over the Secondary_CTA through size, color contrast, and positioning.
6. THE Hero_Section SHALL use the font family Bitter for the headline and Source Sans 3 for the subtitle.
7. THE Hero_Section SHALL use the accent color teal (#3d8b8b) for the Primary_CTA background.

### Requirement 3: Stats and trust bar

**User Story:** As a visitor, I want to see evidence that this project is actively developed and used so that I can trust it before investing time in registration.

#### Acceptance Criteria

1. THE Stats_Bar SHALL be rendered immediately after the Hero_Section in the page layout.
2. THE Stats_Bar SHALL display social proof indicators including GitHub stars count and an "active project" badge or similar development activity indicator.
3. THE Stats_Bar SHALL present indicators in a compact horizontal layout that does not distract from the primary page flow.
4. THE Stats_Bar SHALL use muted visual styling so that the indicators remain subordinate to the Primary_CTA.

### Requirement 4: How it works section

**User Story:** As a visitor, I want to understand how to use the platform in simple terms so that the product feels approachable rather than complex.

#### Acceptance Criteria

1. THE How_It_Works_Section SHALL display exactly three sequential steps explaining the platform workflow.
2. THE How_It_Works_Section SHALL present the steps as: (1) register properties, (2) organize tenants and contracts, (3) track payments and expenses.
3. THE How_It_Works_Section SHALL display each step with a number or icon, a short title, and a brief description.
4. THE How_It_Works_Section SHALL use a horizontal layout on desktop viewports and a vertical stacked layout on mobile viewports.
5. THE How_It_Works_Section SHALL include a heading such as "Cómo funciona" to introduce the steps.

### Requirement 5: Features grid

**User Story:** As a visitor, I want to see what capabilities the platform offers so that I can evaluate whether it covers my property management workflow.

#### Acceptance Criteria

1. THE Features_Grid SHALL display exactly six grouped capabilities: Propiedades y Unidades, Inquilinos y Contratos, Pagos y Cobros, Gastos y Reportes, Mantenimiento, and Dashboard en tiempo real.
2. THE Features_Grid SHALL display each capability with an icon and a short Spanish-language description.
3. THE Features_Grid SHALL use a responsive grid layout that adapts from a single column on mobile to a two-column or three-column layout on larger screens.
4. THE Features_Grid SHALL include a heading introducing the capabilities section.

### Requirement 6: Dashboard preview

**User Story:** As a visitor, I want to see what the application looks like so that I can build confidence in the product before registering.

#### Acceptance Criteria

1. THE Dashboard_Preview SHALL display a visual representation of the application dashboard interface using either an animated GIF walkthrough or a static screenshot.
2. THE Dashboard_Preview SHALL include a contextual heading that introduces the visual.
3. THE Dashboard_Preview SHALL display a Secondary_CTA labeled "Ver demo en vivo" that links to a live demo instance.
4. THE Dashboard_Preview SHALL render the visual with rounded corners and a subtle shadow consistent with the platform's design system.
5. THE Secondary_CTA in the Dashboard_Preview SHALL be visually subordinate to the Primary_CTA through reduced size, outline style, or muted color.

### Requirement 7: Open-source transparency section

**User Story:** As a visitor, I want to know that this tool is open-source and community-driven so that I can trust it as a free, non-commercial product.

#### Acceptance Criteria

1. THE Transparency_Section SHALL communicate that the project is open-source and created as a useful community tool.
2. THE Transparency_Section SHALL include a link to the project's source code repository styled as a Secondary_CTA.
3. THE Transparency_Section SHALL use a friendly tone that emphasizes the project was built for fun and utility rather than commercial gain.

### Requirement 8: Landing page footer

**User Story:** As a visitor, I want to see project credits and relevant links at the bottom of the page.

#### Acceptance Criteria

1. THE Landing_Footer SHALL display the project name "Gestión Inmobiliaria".
2. THE Landing_Footer SHALL display a copyright or attribution line.
3. THE Landing_Footer SHALL be visually distinct from the main content area.

### Requirement 9: Single primary CTA focus

**User Story:** As a visitor, I want one clear action to take so that I am not confused by competing calls-to-action.

#### Acceptance Criteria

1. THE Landing_Page SHALL maintain "Registrarse gratis" as the single Primary_CTA across all sections.
2. WHEN multiple call-to-action buttons are visible in the same viewport, THE Landing_Page SHALL ensure the Primary_CTA is visually dominant through accent color, larger size, or filled style.
3. THE Landing_Page SHALL style all other action buttons (login, demo link, GitHub link) as Secondary_CTAs using outline, ghost, or muted styling.

### Requirement 10: Theming and visual consistency

**User Story:** As a visitor, I want the landing page to respect the system's light and dark theme so that the experience is visually consistent.

#### Acceptance Criteria

1. THE Landing_Page SHALL render correctly in both light and dark themes using existing CSS custom properties (var(--surface-base), var(--text-primary), etc.).
2. THE Landing_Page SHALL use Tailwind CSS utility classes consistent with the existing frontend codebase.
3. THE Landing_Page SHALL use the accent color teal (#3d8b8b) for primary interactive elements.

### Requirement 11: Component architecture

**User Story:** As a developer, I want the landing page to follow the project's component patterns so that the code remains maintainable and avoids known anti-patterns.

#### Acceptance Criteria

1. THE Landing_Page SHALL be composed of sub-components where no single html! macro block exceeds 150 lines.
2. THE Landing_Page SHALL pass string props using AttrValue and complex data using Rc wrappers.
3. THE Landing_Page SHALL be implemented as Yew 0.23 functional components with hooks.

### Requirement 12: Responsive layout

**User Story:** As a visitor on a mobile device, I want the landing page to be readable and navigable so that I can learn about the platform regardless of my screen size.

#### Acceptance Criteria

1. THE Landing_Page SHALL display content in a single-column layout on viewports narrower than 768px.
2. THE Landing_Page SHALL use appropriate spacing and font sizes for mobile readability.
3. THE Primary_CTA and Secondary_CTA buttons SHALL be full-width on mobile viewports and inline on desktop viewports.

### Requirement 13: Spanish-language content

**User Story:** As a Dominican Republic property manager, I want all landing page content to be in Spanish so that I can understand everything without language barriers.

#### Acceptance Criteria

1. THE Landing_Page SHALL render all visible text content in Spanish.
2. THE Landing_Page SHALL use terminology familiar to Dominican Republic property managers (propiedades, inquilinos, contratos, pagos, gastos, mantenimiento).
3. THE Landing_Page SHALL use a friendly, non-aggressive tone that positions the tool as helpful rather than using hard-sell marketing language.

### Requirement 14: Page section ordering

**User Story:** As a visitor, I want the page to guide me through a logical narrative so that I progressively build understanding and trust before taking action.

#### Acceptance Criteria

1. THE Landing_Page SHALL render sections in the following order from top to bottom: Hero_Section, Stats_Bar, How_It_Works_Section, Features_Grid, Dashboard_Preview, Transparency_Section, Landing_Footer.
2. THE Landing_Page SHALL maintain the specified section order on all viewport sizes.
