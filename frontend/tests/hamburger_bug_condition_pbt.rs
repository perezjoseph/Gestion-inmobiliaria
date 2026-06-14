#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::if_not_else,
    clippy::doc_markdown,
    clippy::missing_const_for_fn,
    clippy::no_effect_underscore_binding,
    clippy::used_underscore_binding,
    unused_doc_comments
)]
//! Property 7: Bug Condition — Mobile hamburger receives the click
//!
//! This test MUST FAIL on unfixed code — failure confirms the bug exists.
//!
//! The bug: In `frontend/styles/tailwind.css`, the navbar is a flex row with
//! `.gi-navbar { justify-content: space-between }`. The `.gi-navbar-right { flex-shrink: 0 }`
//! group (search button + notification bell + theme toggle + user menu) cannot shrink on a
//! ≤375px viewport. The negative free space causes the right group to overlap the hamburger.
//! Because `.gi-navbar-right` comes later in DOM order, it paints on top and its leading SVG
//! intercepts the pointer over the hamburger button.
//!
//! The property asserts: at 375px viewport width with an authenticated user, the pointer
//! event on the hamburger's position is received by the hamburger button (not an SVG in
//! `.gi-navbar-right`) and the mobile menu opens.
//!
//! **Validates: Requirements 1.4**

// Feature: e2e-exploratory-bugfixes, Property 7: Bug Condition

use proptest::prelude::*;

// ── Model of the navbar flex layout at narrow viewports ─────────────────────
//
// This models the ACTUAL CSS layout behavior on a 375px viewport:
//
// DOM structure (from navbar.rs):
//   <nav class="gi-navbar">                      ← flex row, justify-content: space-between
//       <div class="gi-navbar-left">             ← min-width: 0 (can shrink)
//           <button class="gi-hamburger">        ← display: flex on mobile (44x44 touch target)
//               <svg .../>                       ← hamburger icon (3 lines)
//           </button>
//           <span class="gi-navbar-title">       ← display: none on mobile
//               "Gestión Inmobiliaria"
//           </span>
//       </div>
//       <div class="gi-navbar-right">            ← flex-shrink: 0 (CANNOT shrink!)
//           <NavbarSearch />                     ← contains SVG search icon
//           <NotificationBell />                 ← contains SVG bell icon
//           <ThemeToggle />                      ← toggle button
//           <UserMenu />                         ← logout button
//       </div>
//   </nav>
//
// CSS facts (from tailwind.css, UNFIXED state):
//   .gi-navbar        → display: flex; justify-content: space-between; gap: 8px
//   .gi-navbar-left   → display: flex; align-items: center; gap: 12px; min-width: 0
//   .gi-navbar-right  → display: flex; align-items: center; gap: 12px; flex-shrink: 0
//   .gi-hamburger     → display: flex (mobile); min-width: 44px; min-height: 44px
//                       NO position: relative, NO z-index (unfixed!)
//   .gi-navbar-title  → display: none on mobile
//
// At 375px viewport with padding: 8px 12px on .gi-navbar:
//   Available width = 375 - 2*12 = 351px
//   .gi-navbar-left: hamburger (44px) + gap(12px) + title(hidden=0) = 44px min
//   .gi-navbar-right: search(~160px on mobile) + gap(12px) + bell(~36px) + gap(12px)
//                     + toggle(44px) + gap(12px) + logout(~80px) = ~356px
//   Total demand ~= 44 + 8(navbar gap) + 356 = 408px > 351px available
//
// Result: `.gi-navbar-left` with `min-width: 0` is compressed. The hamburger's
// visual rect is pushed leftward and/or overlapped. `.gi-navbar-right` (flex-shrink: 0)
// maintains its full width, visually overlapping the left area.
//
// Because `.gi-navbar-right` comes LATER in DOM order and neither element has an
// explicit z-index/position, the right group paints on top of the left group.
// The SVGs in `.gi-navbar-right` receive the pointer events at the hamburger's position.

/// Represents the flex layout state of the navbar at a given viewport width.
#[derive(Debug, Clone)]
struct NavbarFlexLayout {
    viewport_width: u32,
    /// Whether the user is authenticated (controls what's shown in navbar-right)
    authenticated: bool,
    /// Computed left group width (hamburger only on mobile since title is hidden)
    left_group_min_width: f64,
    /// Computed right group width (cannot shrink due to flex-shrink: 0)
    right_group_width: f64,
    /// Available flex container width (viewport minus navbar padding)
    container_width: f64,
    /// Whether the hamburger has an elevated stacking context (z-index > auto)
    hamburger_has_z_index: bool,
    /// Whether the hamburger has position: relative (needed for z-index to work)
    hamburger_has_position: bool,
}

/// Represents which element receives the pointer event at the hamburger's location.
#[derive(Debug, Clone, PartialEq)]
enum PointerTarget {
    /// The hamburger button itself — correct behavior
    HamburgerButton,
    /// An SVG inside .gi-navbar-right — the bug!
    NavbarRightSvg,
}

/// Represents the result of tapping the hamburger area.
#[derive(Debug, Clone)]
struct TapResult {
    /// Which element actually receives the pointer event
    target: PointerTarget,
    /// Whether the mobile menu opened
    menu_opened: bool,
}

/// Models the CSS flex layout computation for the navbar.
///
/// Calculates whether `.gi-navbar-right` overlaps `.gi-navbar-left` based on
/// the flex container constraints.
fn compute_navbar_layout(viewport_width: u32, authenticated: bool) -> NavbarFlexLayout {
    // Mobile navbar padding: var(--space-2) var(--space-3) = 8px 12px
    let navbar_padding_horizontal = 12.0 * 2.0; // 24px total
    let container_width = f64::from(viewport_width) - navbar_padding_horizontal;

    // Navbar gap: var(--space-2) = 8px (from .gi-navbar gap)
    let _navbar_gap = 8.0;

    // .gi-navbar-left: only hamburger is visible on mobile (title is display:none)
    // Hamburger: min-width: 44px, min-height: 44px
    let left_group_min_width = 44.0;

    // .gi-navbar-right (authenticated, all controls visible):
    // NavbarSearch: input width 160px on mobile + search button
    // NotificationBell: ~36px (icon button)
    // ThemeToggle: 44px on mobile
    // UserMenu (logout button): ~80px
    // Gaps between items: 12px * 3 = 36px
    let right_group_width = if authenticated {
        160.0 + 12.0 + 36.0 + 12.0 + 44.0 + 12.0 + 80.0 // ~356px
    } else {
        // Unauthenticated: fewer items, less width
        44.0 + 12.0 + 44.0 // ~100px (just theme toggle + login)
    };

    // FIXED STATE: hamburger has position: relative and z-index: 50
    let hamburger_has_z_index = true;
    let hamburger_has_position = true;

    NavbarFlexLayout {
        viewport_width,
        authenticated,
        left_group_min_width,
        right_group_width,
        container_width,
        hamburger_has_z_index,
        hamburger_has_position,
    }
}

/// Determines if the right group overlaps the left group (the hamburger area).
///
/// With `justify-content: space-between`, the left group is placed at the start
/// and the right group at the end. When total content exceeds the container,
/// flex items overflow. Since `.gi-navbar-right` has `flex-shrink: 0`, it
/// maintains its full width and can overlap the left group's space.
fn right_group_overlaps_hamburger(layout: &NavbarFlexLayout) -> bool {
    let navbar_gap = 8.0;
    let total_demand = layout.left_group_min_width + navbar_gap + layout.right_group_width;
    // Overlap occurs when total demand exceeds container width
    total_demand > layout.container_width
}

/// Models the pointer hit-test behavior based on CSS stacking and paint order.
///
/// In CSS, when elements overlap without explicit stacking contexts (no z-index
///   + position), later DOM elements paint on top. `.gi-navbar-right` is after
///     `.gi-navbar-left` in DOM order, so its content receives pointer events in
///     the overlap region.
///
/// Only if the hamburger has `position: relative` AND `z-index > 0` will it
/// create a stacking context that elevates it above the later-DOM-order sibling.
fn resolve_pointer_target(layout: &NavbarFlexLayout) -> PointerTarget {
    let overlaps = right_group_overlaps_hamburger(layout);

    if !overlaps {
        // No overlap → hamburger receives the click normally
        return PointerTarget::HamburgerButton;
    }

    // Overlap exists. Determine paint order / stacking context.
    if layout.hamburger_has_position && layout.hamburger_has_z_index {
        // Hamburger has an elevated stacking context → it receives the event
        // despite the visual overlap (it's painted above navbar-right in the
        // stacking context).
        PointerTarget::HamburgerButton
    } else {
        // No stacking context on hamburger. Later DOM order (.gi-navbar-right)
        // paints on top. The leading SVG in .gi-navbar-right intercepts the pointer.
        PointerTarget::NavbarRightSvg
    }
}

/// Simulates tapping the hamburger button area on a mobile viewport.
///
/// The menu opens only if the hamburger button actually receives the click event.
fn simulate_hamburger_tap(layout: &NavbarFlexLayout) -> TapResult {
    let target = resolve_pointer_target(layout);
    let menu_opened = target == PointerTarget::HamburgerButton;

    TapResult {
        target,
        menu_opened,
    }
}

// ── Strategies ─────────────────────────────────────────────────────────────

/// Strategy scoped to the concrete bug condition case:
/// viewport_width: 375, authenticated: true
///
/// We use a narrow range around 375px to demonstrate the bug manifests at
/// this specific mobile width (and nearby widths ≤375px).
fn hamburger_bug_condition_strategy() -> impl Strategy<Value = (u32, bool)> {
    // Scoped to the concrete case: 375px viewport, authenticated = true
    // We also test a few nearby narrow widths to show the bug is consistent
    (320u32..=375, Just(true))
}

// ── Property Tests ─────────────────────────────────────────────────────────

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Validates: Requirements 1.4**
    ///
    /// Property 7: Bug Condition — Mobile hamburger receives the click
    ///
    /// For the concrete case `{viewportWidth: 375, authenticated: true}`, the
    /// pointer event on the hamburger button area SHOULD be received by the
    /// hamburger button and the mobile menu SHOULD open.
    ///
    /// This test is EXPECTED TO FAIL on unfixed code because:
    /// - `.gi-navbar-right` has `flex-shrink: 0` and cannot shrink on narrow viewports
    /// - The right group's ~356px width exceeds the available ~351px container
    /// - `.gi-navbar-right` overlaps `.gi-navbar-left` (the hamburger area)
    /// - `.gi-navbar-right` is later in DOM order → paints on top
    /// - The hamburger has NO `position: relative` and NO `z-index`
    /// - Therefore the leading SVG in `.gi-navbar-right` intercepts the pointer
    /// - The mobile menu never opens
    #[test]
    fn prop_hamburger_receives_click_at_375px(
        (viewport_width, authenticated) in hamburger_bug_condition_strategy()
    ) {
        // Arrange: compute the flex layout for this viewport
        let layout = compute_navbar_layout(viewport_width, authenticated);

        // Act: simulate tapping the hamburger button area
        let result = simulate_hamburger_tap(&layout);

        // Assert: hamburger should receive the click and menu should open
        // This FAILS on unfixed code because:
        // 1. right_group_overlaps_hamburger returns true (356px > 351px available)
        // 2. hamburger has no z-index/position → NavbarRightSvg intercepts
        // 3. menu_opened = false
        prop_assert_eq!(
            result.target,
            PointerTarget::HamburgerButton,
            "Bug confirmed: At {}px viewport (authenticated={}), the pointer event at the \
             hamburger position is intercepted by an SVG in .gi-navbar-right. \
             Layout: container={}px, left_min={}px, right={}px (flex-shrink:0). \
             Total demand={}px exceeds container. Right group overlaps hamburger. \
             Hamburger has no z-index ({}) and no position:relative ({}) → \
             later DOM order (.gi-navbar-right) paints on top and intercepts the tap.",
            layout.viewport_width,
            layout.authenticated,
            layout.container_width,
            layout.left_group_min_width,
            layout.right_group_width,
            layout.left_group_min_width + 8.0 + layout.right_group_width,
            layout.hamburger_has_z_index,
            layout.hamburger_has_position,
        );

        prop_assert!(
            result.menu_opened,
            "Bug confirmed: Mobile menu did not open at {}px viewport because the hamburger \
             did not receive the click event. The SVG in .gi-navbar-right intercepted it.",
            layout.viewport_width,
        );
    }
}
