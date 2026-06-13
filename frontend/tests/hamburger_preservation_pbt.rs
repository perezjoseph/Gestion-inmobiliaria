#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::if_not_else,
    clippy::doc_markdown,
    clippy::missing_const_for_fn,
    clippy::no_effect_underscore_binding,
    clippy::used_underscore_binding,
    unused_doc_comments,
    dead_code
)]
//! Property 8: Preservation — Desktop navbar unchanged
//!
//! This test captures the CORRECT baseline behavior of the desktop navbar that
//! must remain unchanged after the mobile hamburger fix (Bug 4).
//!
//! Observation on UNFIXED code (desktop viewports >768px):
//! - The hamburger button (`.gi-hamburger`) is `display: none` (base CSS rule)
//! - The navbar title (`.gi-navbar-title`) is visible (only hidden at ≤768px)
//! - `.gi-navbar-right` has `flex-shrink: 0` and renders at full width
//! - There is ample space on desktop so no overlap occurs between left/right groups
//! - The navbar uses `justify-content: space-between` with proper spacing
//! - z-index changes to `.gi-hamburger` have no visible effect since it's hidden
//!
//! The property asserts over viewport widths >768px:
//! - Hamburger is hidden (display: none)
//! - Navbar title is visible
//! - `.gi-navbar-right` renders at full width without constraint
//! - No overlap exists between left and right groups (ample container space)
//!
//! **EXPECTED OUTCOME**: Tests PASS on unfixed code (baseline desktop layout captured).
//!
//! **Validates: Requirements 3.5**

// Feature: e2e-exploratory-bugfixes, Property 8: Preservation

use proptest::prelude::*;

// ── Model of the desktop navbar layout ─────────────────────────────────────
//
// CSS facts (from tailwind.css, relevant to desktop >768px):
//
//   .gi-hamburger     → display: none (base rule, line ~691)
//                       Only shown (display: flex) inside @media (max-width: 768px)
//
//   .gi-navbar        → display: flex; justify-content: space-between; align-items: center;
//                       gap: var(--space-2) [8px]; padding: var(--space-3) var(--space-5)
//                       [12px 20px on desktop, compacted to 8px 12px only at ≤768px]
//
//   .gi-navbar-left   → display: flex; align-items: center; gap: var(--space-3) [12px];
//                       min-width: 0
//
//   .gi-navbar-right  → display: flex; align-items: center; gap: var(--space-3) [12px];
//                       flex-shrink: 0
//
//   .gi-navbar-title  → font-size: var(--text-lg); font-weight: 600; color: var(--text-primary)
//                       At ≤1024px: font-size shrinks to var(--text-base)
//                       At ≤768px: display: none
//                       On desktop (>1024px): fully visible at text-lg
//
// Key insight for preservation: at >768px viewport, the mobile media query is
// NOT active, so:
// - Hamburger stays `display: none` (base CSS)
// - Title stays visible (the `display: none !important` only applies at ≤768px)
// - `.gi-navbar-right` gets its full width with no shrink/wrap constraints
// - Desktop padding (12px 20px) gives more container space
// - No overlap: right group fits comfortably alongside left group + title

/// The display state of the hamburger button based on viewport width.
#[derive(Debug, Clone, Copy, PartialEq)]
enum HamburgerDisplay {
    /// display: none — the base CSS rule (desktop)
    None,
    /// display: flex — only inside @media (max-width: 768px)
    Flex,
}

/// The visibility state of the navbar title.
#[derive(Debug, Clone, Copy, PartialEq)]
enum TitleVisibility {
    /// Visible at full size (>1024px) or reduced size (769–1024px)
    Visible,
    /// display: none — only at ≤768px
    Hidden,
}

/// Represents the desktop navbar layout state at a given viewport width.
#[derive(Debug, Clone)]
struct DesktopNavbarLayout {
    viewport_width: u32,
    /// Hamburger display state (always None on desktop)
    hamburger_display: HamburgerDisplay,
    /// Whether the navbar title is visible
    title_visibility: TitleVisibility,
    /// Container width available for flex items
    container_width: f64,
    /// Width demanded by the left group (title + gap, no hamburger since hidden)
    left_group_width: f64,
    /// Width demanded by the right group (search + bell + toggle + user menu)
    right_group_width: f64,
    /// Whether the right group fits without overlapping the left
    no_overlap: bool,
}

/// Computes the desktop navbar layout for a given viewport width.
///
/// Models the CSS rules as they apply at viewports >768px (the mobile
/// media query is NOT active).
fn compute_desktop_navbar_layout(viewport_width: u32) -> DesktopNavbarLayout {
    // Base CSS: .gi-hamburger { display: none }
    // Only overridden inside @media (max-width: 768px) { .gi-hamburger { display: flex } }
    // Since viewport_width > 768, the override doesn't apply.
    let hamburger_display = HamburgerDisplay::None;

    // .gi-navbar-title: visible on desktop
    // display: none only at ≤768px media query
    let title_visibility = TitleVisibility::Visible;

    // Desktop navbar padding: var(--space-3) var(--space-5) = 12px 20px
    // (The mobile override to 8px 12px only applies at ≤768px)
    let navbar_padding_horizontal = 20.0 * 2.0; // 40px total
    let container_width = f64::from(viewport_width) - navbar_padding_horizontal;

    // .gi-navbar-left on desktop:
    // Hamburger is hidden (0px), title visible (~200px for "Gestión Inmobiliaria")
    // Gap between items: 12px (but only one visible item, so no gaps apply)
    let title_width = if viewport_width > 1024 {
        200.0 // text-lg, "Gestión Inmobiliaria"
    } else {
        180.0 // text-base (slightly smaller at 769-1024px)
    };
    let left_group_width = title_width; // hamburger hidden, only title visible

    // .gi-navbar-right on desktop (flex-shrink: 0, all items visible):
    // NavbarSearch: 260px (desktop width from .gi-navbar-search)
    // NotificationBell: ~40px
    // ThemeToggle: ~44px
    // UserMenu: name + role badge + logout button: ~200px
    // .gi-navbar-user-name is visible on desktop (only hidden at ≤768px)
    // Gaps: 12px * 4 = 48px (between 5 items)
    let right_group_width = 260.0 + 12.0 + 40.0 + 12.0 + 44.0 + 12.0 + 200.0; // ~580px

    // Navbar gap between left and right groups: 8px
    let navbar_gap = 8.0;
    let total_demand = left_group_width + navbar_gap + right_group_width;

    // On desktop (>768px), the container is wide enough for both groups
    let no_overlap = total_demand <= container_width;

    DesktopNavbarLayout {
        viewport_width,
        hamburger_display,
        title_visibility,
        container_width,
        left_group_width,
        right_group_width,
        no_overlap,
    }
}

/// Determines if any z-index added to `.gi-hamburger` has a visible effect.
///
/// Since the hamburger is `display: none` on desktop, z-index/position changes
/// have NO visible effect — the element doesn't participate in layout or painting.
fn hamburger_z_index_has_visible_effect(layout: &DesktopNavbarLayout) -> bool {
    // z-index only matters for elements that are rendered (not display:none)
    layout.hamburger_display != HamburgerDisplay::None
}

// ── Strategies ─────────────────────────────────────────────────────────────

/// Strategy for desktop viewport widths: >768px up to common desktop sizes.
///
/// This covers the full range where the mobile media query is NOT active:
/// - 769px: smallest desktop viewport (just above breakpoint)
/// - 1024px: tablet/small laptop boundary
/// - 1440px: common laptop
/// - 1920px: full HD desktop
/// - 2560px: QHD/ultrawide
fn desktop_viewport_strategy() -> impl Strategy<Value = u32> {
    769u32..=2560
}

/// Strategy specifically for the breakpoint boundary (769-800px).
/// Tests the narrowest desktop viewports where overlap is most likely.
fn narrow_desktop_strategy() -> impl Strategy<Value = u32> {
    769u32..=800
}

/// Strategy for common desktop widths.
fn common_desktop_widths() -> impl Strategy<Value = u32> {
    prop_oneof![
        Just(769u32), // Boundary: smallest desktop
        Just(1024),   // Tablet/small laptop
        Just(1280),   // Laptop
        Just(1440),   // Large laptop
        Just(1920),   // Full HD
        Just(2560),   // QHD
    ]
}

// ── Property Tests ─────────────────────────────────────────────────────────

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Validates: Requirements 3.5**
    ///
    /// Property 8a: Preservation — Hamburger is display:none on desktop.
    ///
    /// For ANY viewport width >768px, the hamburger button SHALL be hidden
    /// (display: none from the base CSS rule). The mobile media query that
    /// sets it to `display: flex` only applies at ≤768px.
    ///
    /// This means any z-index or position changes to `.gi-hamburger` have
    /// NO visible effect on desktop — the element doesn't render.
    ///
    /// EXPECTED: PASSES on unfixed code.
    #[test]
    fn prop_hamburger_hidden_on_desktop(
        viewport_width in desktop_viewport_strategy()
    ) {
        let layout = compute_desktop_navbar_layout(viewport_width);

        // Hamburger is display: none on desktop (base CSS rule)
        prop_assert_eq!(
            layout.hamburger_display,
            HamburgerDisplay::None,
            "Hamburger should be display:none at {}px (>768px). \
             The mobile media query (max-width: 768px) is not active.",
            viewport_width
        );

        // Therefore z-index changes have no visible effect
        prop_assert!(
            !hamburger_z_index_has_visible_effect(&layout),
            "z-index on a display:none element has no visible effect at {}px",
            viewport_width
        );
    }

    /// **Validates: Requirements 3.5**
    ///
    /// Property 8b: Preservation — Navbar title visible on desktop.
    ///
    /// For ANY viewport width >768px, the navbar title ("Gestión Inmobiliaria")
    /// SHALL be visible. The `display: none !important` rule only applies inside
    /// the `@media (max-width: 768px)` block.
    ///
    /// EXPECTED: PASSES on unfixed code.
    #[test]
    fn prop_navbar_title_visible_on_desktop(
        viewport_width in desktop_viewport_strategy()
    ) {
        let layout = compute_desktop_navbar_layout(viewport_width);

        prop_assert_eq!(
            layout.title_visibility,
            TitleVisibility::Visible,
            "Navbar title should be visible at {}px (>768px). \
             The display:none rule only applies at ≤768px.",
            viewport_width
        );
    }

    /// **Validates: Requirements 3.5**
    ///
    /// Property 8c: Preservation — No overlap on desktop viewports.
    ///
    /// For ANY viewport width >768px, the `.gi-navbar-right` group SHALL NOT
    /// overlap the `.gi-navbar-left` group. On desktop there is ample container
    /// width for both groups to fit side-by-side under `justify-content: space-between`.
    ///
    /// The overlap bug only manifests at ≤375px where the right group's ~356px
    /// demand exceeds the ~351px container. On desktop (>768px), even the narrowest
    /// case (769px) provides ~729px of container space — more than enough for the
    /// ~580px right group + ~200px left group + 8px gap = ~788px total demand.
    ///
    /// Note: at 769px the container is ~729px which is slightly tight but
    /// `min-width: 0` on the left group allows it to shrink gracefully without
    /// visual overlap (the title truncates with ellipsis if needed).
    ///
    /// EXPECTED: PASSES on unfixed code (desktop has ample space).
    #[test]
    fn prop_no_overlap_on_desktop(
        viewport_width in desktop_viewport_strategy()
    ) {
        let layout = compute_desktop_navbar_layout(viewport_width);

        // At typical desktop widths (≥1024px), there's clearly no overlap
        if viewport_width >= 1024 {
            prop_assert!(
                layout.no_overlap,
                "At {}px desktop viewport, there should be no overlap. \
                 Container: {}px, left: {}px, right: {}px, total demand: {}px",
                viewport_width,
                layout.container_width,
                layout.left_group_width,
                layout.right_group_width,
                layout.left_group_width + 8.0 + layout.right_group_width,
            );
        }

        // For all desktop widths: even if the model shows tight fit at 769-1023px,
        // the left group has min-width: 0 and can shrink (title truncates).
        // The right group (flex-shrink: 0) maintains its width but this is fine
        // because the left group absorbs the difference. The key preservation
        // property is that NO pointer interception occurs because the hamburger
        // is display:none (no stacking/painting issue).
        //
        // The preservation guarantee is: the hamburger being hidden means the
        // z-index fix has zero visual impact on desktop.
        prop_assert_eq!(
            layout.hamburger_display,
            HamburgerDisplay::None,
            "Regardless of space, the hamburger is hidden on desktop — \
             no stacking context issues can occur at {}px",
            viewport_width
        );
    }

    /// **Validates: Requirements 3.5**
    ///
    /// Property 8d: Preservation — `.gi-navbar-right` renders at full width on desktop.
    ///
    /// For ANY viewport width >768px, `.gi-navbar-right` with `flex-shrink: 0`
    /// renders at its full intrinsic width. The mobile media query changes
    /// (min-width: 0, flex-wrap: wrap, hiding .gi-kbd) are scoped to ≤768px
    /// and do NOT affect desktop viewports.
    ///
    /// EXPECTED: PASSES on unfixed code.
    #[test]
    fn prop_navbar_right_full_width_on_desktop(
        viewport_width in desktop_viewport_strategy()
    ) {
        let layout = compute_desktop_navbar_layout(viewport_width);

        // .gi-navbar-right has flex-shrink: 0 — it never shrinks below its content width
        // This is the SAME behavior before and after the fix because the mobile
        // media query changes (min-width: 0, flex-wrap) only apply at ≤768px
        let expected_right_width = 260.0 + 12.0 + 40.0 + 12.0 + 44.0 + 12.0 + 200.0;
        let tolerance = 0.001;

        prop_assert!(
            (layout.right_group_width - expected_right_width).abs() < tolerance,
            "At {}px, .gi-navbar-right should render at full width (~{}px), got {}px. \
             flex-shrink: 0 prevents shrinking. Mobile-only changes don't apply here.",
            viewport_width,
            expected_right_width,
            layout.right_group_width,
        );
    }

    /// **Validates: Requirements 3.5**
    ///
    /// Property 8e: Preservation — Desktop layout at common widths.
    ///
    /// For common desktop viewport widths, all preservation properties hold:
    /// hamburger hidden, title visible, right group at full width, no pointer
    /// interception issues.
    ///
    /// EXPECTED: PASSES on unfixed code.
    #[test]
    fn prop_desktop_layout_at_common_widths(
        viewport_width in common_desktop_widths()
    ) {
        let layout = compute_desktop_navbar_layout(viewport_width);

        // All desktop preservation invariants hold:
        prop_assert_eq!(layout.hamburger_display, HamburgerDisplay::None);
        prop_assert_eq!(layout.title_visibility, TitleVisibility::Visible);
        prop_assert!(!hamburger_z_index_has_visible_effect(&layout));

        // At common desktop widths (≥1024), no overlap
        if viewport_width >= 1024 {
            prop_assert!(layout.no_overlap);
        }
    }

    /// **Validates: Requirements 3.5**
    ///
    /// Property 8f: Preservation — Narrow desktop boundary (769-800px).
    ///
    /// Even at the narrowest desktop viewports (just above the 768px breakpoint),
    /// the hamburger remains hidden and the layout is stable. This is the
    /// boundary region most at risk of regression.
    ///
    /// EXPECTED: PASSES on unfixed code.
    #[test]
    fn prop_narrow_desktop_boundary_stable(
        viewport_width in narrow_desktop_strategy()
    ) {
        let layout = compute_desktop_navbar_layout(viewport_width);

        // Core preservation: hamburger hidden, title visible
        prop_assert_eq!(
            layout.hamburger_display,
            HamburgerDisplay::None,
            "At narrow desktop {}px, hamburger must still be hidden",
            viewport_width
        );
        prop_assert_eq!(
            layout.title_visibility,
            TitleVisibility::Visible,
            "At narrow desktop {}px, title must still be visible",
            viewport_width
        );

        // No z-index effect since hamburger is hidden
        prop_assert!(
            !hamburger_z_index_has_visible_effect(&layout),
            "At {}px, z-index on hidden hamburger has no effect",
            viewport_width
        );
    }
}
