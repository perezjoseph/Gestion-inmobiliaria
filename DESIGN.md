---
name: Gestión Inmobiliaria RD
description: Centralized property management for Dominican Republic rental operators
colors:
  primary-500: "#3d8b8b"
  primary-600: "#2d7373"
  primary-700: "#245e5e"
  primary-800: "#1c4a4a"
  accent-500: "#b8422e"
  accent-600: "#9c3826"
  sand-50: "#faf9f7"
  sand-100: "#f3f0ec"
  sand-200: "#e5e0d9"
  sand-300: "#c9c2b8"
  sand-600: "#6d6862"
  sand-900: "#2a2725"
  success: "#3a8a5c"
  warning: "#a8842e"
  error: "#b83e2e"
  info: "#3a6e9c"
typography:
  display:
    fontFamily: "'Bitter', Georgia, 'Times New Roman', serif"
    fontSize: "2rem"
    fontWeight: 700
    lineHeight: 1.2
  title:
    fontFamily: "'Bitter', Georgia, 'Times New Roman', serif"
    fontSize: "1.5rem"
    fontWeight: 600
    lineHeight: 1.3
  body:
    fontFamily: "'Source Sans 3', 'Segoe UI', system-ui, sans-serif"
    fontSize: "1rem"
    fontWeight: 400
    lineHeight: 1.5
  label:
    fontFamily: "'Source Sans 3', 'Segoe UI', system-ui, sans-serif"
    fontSize: "0.75rem"
    fontWeight: 600
    lineHeight: 1.4
    letterSpacing: "0.05em"
rounded:
  sm: "6px"
  md: "8px"
  lg: "12px"
  full: "9999px"
spacing:
  xs: "4px"
  sm: "8px"
  md: "16px"
  lg: "24px"
  xl: "32px"
  2xl: "48px"
components:
  button-primary:
    backgroundColor: "{colors.primary-500}"
    textColor: "#f8f7f5"
    rounded: "{rounded.md}"
    padding: "8px 16px"
  button-primary-hover:
    backgroundColor: "{colors.primary-600}"
  button-accent:
    backgroundColor: "{colors.accent-500}"
    textColor: "#f8f7f5"
    rounded: "{rounded.md}"
    padding: "8px 16px"
  button-ghost:
    backgroundColor: "transparent"
    textColor: "{colors.sand-600}"
    rounded: "{rounded.md}"
    padding: "8px 16px"
  badge-success:
    backgroundColor: "#e8f5ed"
    textColor: "#2d6844"
    rounded: "{rounded.full}"
    padding: "2px 12px"
  badge-error:
    backgroundColor: "#fce8e6"
    textColor: "#8c2e22"
    rounded: "{rounded.full}"
    padding: "2px 12px"
  input-default:
    backgroundColor: "{colors.sand-50}"
    textColor: "{colors.sand-900}"
    rounded: "{rounded.md}"
    padding: "8px 12px"
---

# Design System: Gestión Inmobiliaria RD

## 1. Overview

**Creative North Star: "La Oficina Costera"**

The grounded efficiency of a well-run office where you can hear the sea through open louvers. Business first, warmth always. This system serves property managers who spend hours in data-heavy screens; it must be dense without being cramped, warm without being decorative, and local without being kitsch.

The visual language draws from Caribbean professional spaces: sandy warm surfaces instead of cold grays, deep teal as the working accent (ocean depth, not tourism blue), and terracotta reserved for moments that demand attention. Every surface is tinted toward warmth. Every interaction is deliberate.

This is free, open-source software built for independent Dominican landlords. It rejects the aesthetic baggage of enterprise platforms and Silicon Valley SaaS alike. No purple gradients, no glassmorphism, no navy-and-gold pretension, no identical card grids with colored left borders. It looks like it belongs in Santo Domingo, not San Francisco.

**Key Characteristics:**
- Warm-tinted neutrals everywhere (sand, stone, never steel)
- Deep teal as the single working primary; terracotta/coral reserved for emphasis and alerts
- Serif headings (Bitter) for Caribbean confidence; clean sans body (Source Sans 3) for data density
- Flat by default; shadows appear only as state feedback (hover, elevation)
- Status communicated through color AND icon/text, never color alone
- Generous touch targets for accessibility across all ages

## 2. Colors

A restrained tropical palette: tinted neutrals dominate, deep teal carries the primary role on less than 10% of any screen, and terracotta appears only for destructive actions and urgent alerts.

### Primary

- **Deep Coastal Teal** (oklch(48% 0.10 185)): The working accent. Navigation active states, primary buttons, focus rings, links. Carries authority without coldness because the hue sits at 185 (green-teal), not 220 (corporate blue).

### Secondary

- **Warm Terracotta** (oklch(55% 0.13 30)): Accent for alerts, destructive actions, and moments needing urgency. Never decorative. Its rarity is what makes it effective.

### Neutral

- **Sand 50** (oklch(98% 0.008 80)): Base surface in light mode. The paper of the office.
- **Sand 200** (oklch(90% 0.012 80)): Borders, dividers. Warm enough to never feel like wire.
- **Sand 600** (oklch(48% 0.008 80)): Secondary text. The pencil color.
- **Sand 900** (oklch(20% 0.006 80)): Primary text. Deep and warm, never pure black.

### Semantic

- **Success** (oklch(55% 0.12 155)): Paid, active, available. Green with enough warmth to avoid clinical.
- **Warning** (oklch(65% 0.14 85)): Approaching due, expiring soon. Amber, not yellow.
- **Error** (oklch(55% 0.14 25)): Overdue, failed, rejected. Close to terracotta but distinct.
- **Info** (oklch(55% 0.10 230)): Informational badges, neutral status.

### Named Rules

**The Sand Foundation Rule.** Every neutral in the system carries hue 80 with chroma 0.005 to 0.012. Cold gray (chroma 0, hue 0) is prohibited. If a surface doesn't feel like sandstone, it's wrong.

**The Teal Budget Rule.** Primary teal occupies no more than 10% of any given screen's surface area. Its scarcity is its authority. When everything is teal, nothing is.

## 3. Typography

**Display Font:** Bitter (with Georgia, Times New Roman fallback)
**Body Font:** Source Sans 3 (with Segoe UI, system-ui fallback)

**Character:** A Caribbean editorial pairing. Bitter brings the confidence of printed Dominican newspapers; Source Sans 3 brings the clarity of modern data interfaces. The contrast between serif headings and sans body creates hierarchy without needing size extremes.

### Hierarchy

- **Display** (700, 2rem, line-height 1.2): Page titles. Appears once per screen, never repeated.
- **Title** (600, 1.5rem, line-height 1.3): Section headings, card titles. The workhorse heading.
- **Body** (400, 1rem, line-height 1.5): Running text, form content, table cells. Capped at 65ch for readability.
- **Label** (600, 0.75rem, line-height 1.4, letter-spacing 0.05em, uppercase): Table headers, sidebar group labels, field labels. The smallest text in the system; never below this.

### Named Rules

**The Bitter Reserve Rule.** Bitter appears only on h1 through h6 and elements with `.font-display`. It never appears in buttons, labels, badges, or body copy. Its exclusivity gives headings weight.

**The Tabular Data Rule.** All numeric data in tables uses `font-variant-numeric: tabular-nums` so columns align. Numbers are never proportional in data contexts.

## 4. Elevation

Flat by default. Shadows appear as state responses, not as resting decoration. The philosophy: surfaces at rest are paper on a desk; surfaces in motion have just been picked up.

### Shadow Vocabulary

- **Shadow SM** (`0 1px 2px oklch(20% 0.01 80 / 0.08)`): Resting state for navbar. Barely perceptible; just enough to separate header from content.
- **Shadow MD** (`0 2px 8px oklch(20% 0.01 80 / 0.10)`): Hover state for cards and buttons. The "just picked up" elevation.
- **Shadow LG** (`0 8px 24px oklch(20% 0.01 80 / 0.12)`): Modals, toasts, overlays. Reserved for floating elements that obscure content beneath.

All shadows use warm-tinted oklch (hue 80) instead of neutral black. In dark mode, shadows darken and shift to hue 185 (teal-tinted darkness) with higher opacity (0.3 to 0.5).

### Named Rules

**The Flat-By-Default Rule.** Surfaces are flat at rest. Shadows appear only as a response to state (hover, focus, elevation). If a card has a shadow before interaction, the shadow is wrong.

## 5. Components

### Buttons

- **Shape:** Gently rounded (8px radius). Not pill-shaped, not square.
- **Primary:** Deep Coastal Teal background, near-white text. Padding 8px vertical, 16px horizontal. Weight 600.
- **Hover:** Darkens one step (primary-600), gains Shadow MD. Feels like pressing into the desk.
- **Active:** Scale 0.98. Quick, tactile.
- **Focus:** 2px solid teal outline, 2px offset. Visible from across the room.
- **Ghost:** Transparent background, sand-600 text, 1px border. Hover fills with sand-100.
- **Danger:** Error red background, white text. Reserved for destructive confirmations.
- **Text:** No background, link-colored text, underline on hover. Minimal footprint.
- **Mobile:** All buttons gain min-height 44px for touch accessibility.

### Status Badges

- **Shape:** Full pill (9999px radius). Small, scannable.
- **Color coding:** Success (green bg/text), Warning (amber), Error (red), Info (blue), Neutral (sand).
- **Always paired:** Every badge sits next to text or an icon. Color alone never carries the meaning.
- **Dark mode:** Inverted approach; dark tinted backgrounds with lighter saturated text.

### Cards / Raised Surfaces

- **Corner style:** Generous rounding (12px radius). Softens the data density.
- **Background:** Surface-raised (slightly lighter than base in light mode).
- **Border:** 1px solid border-subtle. Present but quiet.
- **Shadow:** None at rest. Shadow MD on hover.
- **Internal padding:** space-5 (24px) default. space-4 (16px) on mobile.
- **Nested cards:** Prohibited. If you need hierarchy, use spacing and typography, not nesting.

### Inputs / Fields

- **Style:** 1px border (border-default), 8px radius, surface-raised background.
- **Focus:** Border shifts to teal (border-focus), gains a 3px teal ring at 15% opacity.
- **Error:** Border turns error red; ring turns red at 15% opacity. Error message below in text-xs.
- **Disabled:** 60% opacity, not-allowed cursor, slightly darker background.
- **Mobile:** Min-height 44px, font-size bumps to text-base for readability.
- **Labels:** Above the field. text-sm, weight 500, sand-600 color. Margin-bottom space-1.

### Tables

- **Headers:** Uppercase, letter-spaced, text-xs, weight 600. Background matches surface-base (no stripe).
- **Rows:** Alternating-free (no zebra). Hover highlights with primary-50 tint.
- **Cell padding:** space-3 vertical, space-5 horizontal. Comfortable density.
- **Mobile:** Horizontal scroll with sticky first column. First column gets surface-raised background for anchoring.
- **Touch:** Hover disabled; active state substitutes.

### Navigation (Sidebar)

- **Background:** Primary-800 (deep teal). The darkest element in the system.
- **Links:** 8px radius, space-3 vertical padding, weight 500. Teal-tinted white text.
- **Active:** Primary-600 background, full white text, weight 600.
- **Hover:** Primary-700 background, brightened text.
- **Group labels:** Tiny (0.65rem), uppercase, letter-spaced. Quiet organizational cue.
- **Mobile:** Off-canvas drawer, 280px wide, slides from left with ease-out transition.

### Toasts

- **Position:** Top-right (desktop), bottom full-width (mobile).
- **Shape:** 10px radius, Shadow LG.
- **Variants:** Success, Error, Info. Each uses its semantic light-bg/dark-text pair with a 1px border.
- **Entry:** Slides in from right (translateX 20px to 0), 300ms ease-out.
- **Icon:** 22px circle with semantic color, white checkmark/X/i inside.

## 6. Do's and Don'ts

### Do:

- **Do** tint every neutral toward hue 80 (sand/warm). Even borders, even shadows.
- **Do** use tabular-nums in any column showing currency amounts or dates.
- **Do** pair every status color with a text label or icon. A green badge alone is not accessible.
- **Do** use Bitter exclusively for headings. Its weight comes from scarcity.
- **Do** test all interactive elements at 44px minimum touch target on mobile.
- **Do** use the 4px spacing scale consistently. Gaps should be multiples of 4.
- **Do** keep button text at weight 600 and size text-sm. Consistency builds trust.
- **Do** respect `prefers-reduced-motion` by disabling transform/animation transitions.

### Don't:

- **Don't** use cold gray (oklch with hue 0 or chroma 0). Sand Foundation Rule: every neutral has warmth.
- **Don't** use border-left or border-right greater than 1px as a colored accent stripe on any element.
- **Don't** apply gradient backgrounds to text (`background-clip: text`). Emphasis comes from weight and size.
- **Don't** use glassmorphism (backdrop-filter blur + translucent backgrounds) decoratively.
- **Don't** create identical card grids with icon + heading + text repeated in a row. Vary the pattern.
- **Don't** use purple gradients, neon accents, or any SaaS-default aesthetic.
- **Don't** use navy-and-gold combinations. This is Caribbean professional, not banking.
- **Don't** add sunset gradients or palm tree illustrations. Business, not tourism.
- **Don't** make it look like enterprise software that costs $50k/year. This is free, GNU-licensed, community-built.
- **Don't** nest cards inside cards. If you need hierarchy, use spacing and typography.
- **Don't** use shadows at rest. Flat surfaces only; elevation is a state response.
- **Don't** use modals as a first resort. Exhaust inline alternatives (expand-in-place, slide panels, inline forms) before reaching for an overlay.
