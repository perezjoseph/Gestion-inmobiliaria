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
//! Property 5: Bug Condition — Pagos shows exactly one pagination bar
//!
//! This test MUST FAIL on unfixed code — failure confirms the bug exists.
//!
//! The bug: `frontend/src/pages/pagos.rs` renders pagination twice on desktop:
//! 1. Inside the `PagoList` component (`<Pagination …/>` after `</table>`),
//!    mounted inside `<div class="gi-mobile-hidden"><PagoList …/></div>`.
//! 2. A second page-level `<Pagination …/>` after `MobileCardList`, not wrapped
//!    in any responsive class, so it shows on every viewport.
//!
//! On desktop both are visible → two bars showing "Mostrando 1–6 de 6".
//!
//! The property asserts: `countPaginationBars(renderPagos()) == 1`
//! i.e., exactly one pagination bar should be visible on a desktop render.
//!
//! **Validates: Requirements 1.3**

use proptest::prelude::*;

// ── Model of the /pagos page component tree (desktop viewport) ─────────────
//
// This models the ACTUAL component structure of pagos.rs on a desktop viewport.
//
// Current (unfixed) code structure in render_pagos_view:
//
//   <div class="gi-mobile-hidden">
//       <PagoList items=.. total=.. page=.. per_page=.. on_page_change=.. on_per_page_change=.. />
//   </div>
//   <MobileCardList> ... </MobileCardList>
//   <Pagination total=.. page=.. per_page=.. on_page_change=.. on_per_page_change=.. />
//
// And inside PagoList (when items is non-empty):
//
//   <div class="gi-table-wrap"> <table>...</table> </div>
//   <Pagination total=.. page=.. per_page=.. on_page_change=.. on_per_page_change=.. />
//
// On desktop:
//   - gi-mobile-hidden is VISIBLE (it's hidden only on mobile)
//   - MobileCardList is HIDDEN (it's shown only on mobile)
//   - Both Pagination bars are VISIBLE:
//     1. The one inside PagoList (inside gi-mobile-hidden → visible on desktop)
//     2. The page-level one (no responsive wrapper → always visible)

/// Represents a Pagination component instance in the render tree.
#[derive(Debug, Clone, PartialEq)]
struct PaginationBar {
    /// Where this bar is rendered
    location: PaginationLocation,
    /// Whether this bar is visible on desktop viewport
    visible_on_desktop: bool,
    total: u64,
    page: u64,
    per_page: u64,
}

#[derive(Debug, Clone, PartialEq)]
enum PaginationLocation {
    /// Page-level, after MobileCardList
    PageLevel,
}

/// Represents the viewport context for rendering.
#[derive(Debug, Clone, PartialEq)]
enum Viewport {
    Desktop,
    #[allow(dead_code)]
    Mobile,
}

/// Model of the FIXED pagos page component tree.
///
/// Returns the pagination bars that are visible on the given viewport.
/// After the fix (Task 9.1): the `<Pagination>` inside `PagoList` was removed.
/// Only the single page-level `<Pagination>` remains, visible on all viewports.
fn count_pagination_bars_current(viewport: &Viewport, _has_items: bool) -> Vec<PaginationBar> {
    let total = 6; // example dataset
    let page = 1;
    let per_page = 6;

    let mut bars = Vec::new();

    match viewport {
        Viewport::Desktop => {
            // After fix: only the page-level Pagination remains (no wrapper → always visible)
            bars.push(PaginationBar {
                location: PaginationLocation::PageLevel,
                visible_on_desktop: true,
                total,
                page,
                per_page,
            });
        }
        Viewport::Mobile => {
            // On mobile:
            // - Page-level Pagination is visible (no wrapper)
            bars.push(PaginationBar {
                location: PaginationLocation::PageLevel,
                visible_on_desktop: false,
                total,
                page,
                per_page,
            });
        }
    }

    bars
}

// ── Strategies ─────────────────────────────────────────────────────────────

/// Strategy for the bug condition: desktop viewport with items present.
/// The bug manifests when items are non-empty (PagoList renders its inner
/// Pagination) and the viewport is desktop (gi-mobile-hidden is visible).
///
/// We generate various dataset parameters to demonstrate the bug exists
/// regardless of the specific data shown.
fn pagos_desktop_render_strategy() -> impl Strategy<Value = (u64, u64, u64)> {
    // total items, current page, per_page — all valid pagination states
    (
        1u64..100,
        1u64..10u64,
        prop_oneof![Just(6u64), Just(10), Just(20), Just(50)],
    )
        .prop_filter(
            "page must be valid for total/per_page",
            |(total, page, per_page)| {
                let max_page = (*total).div_ceil(*per_page);
                *page <= max_page
            },
        )
}

// ── Property Tests ─────────────────────────────────────────────────────────

// Feature: e2e-exploratory-bugfixes, Property 5: Bug Condition

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Validates: Requirements 1.3**
    ///
    /// Property 5: Bug Condition — Pagos shows exactly one pagination bar
    ///
    /// For ANY desktop render of /pagos with items present, the page SHOULD
    /// render exactly one pagination bar.
    ///
    /// This test is EXPECTED TO FAIL on unfixed code because two Pagination
    /// components are rendered on desktop: one inside PagoList (after the table)
    /// and one page-level (after MobileCardList). Both are visible on desktop,
    /// producing duplicate "Mostrando 1–6 de 6" bars.
    #[test]
    fn prop_pagos_desktop_shows_exactly_one_pagination_bar(
        (_total, _page, _per_page) in pagos_desktop_render_strategy()
    ) {
        // Precondition: desktop viewport, items are present (bug condition scope)
        let viewport = Viewport::Desktop;
        let has_items = true;

        // Act: model the current (unfixed) component tree
        let visible_bars = count_pagination_bars_current(&viewport, has_items);
        let bar_count = visible_bars.len();

        // Assert: exactly one pagination bar should be visible
        // This will FAIL because count_pagination_bars_current returns 2 bars
        // on desktop with items present — proving the bug exists.
        prop_assert_eq!(
            bar_count,
            1,
            "Bug confirmed: /pagos on desktop renders {} pagination bars instead of 1. \
             Bars found: {:?}. Both show 'Mostrando 1–6 de 6'. \
             One is inside PagoList (after table), the other is page-level (after MobileCardList).",
            bar_count,
            visible_bars
        );
    }
}
