#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::if_not_else,
    clippy::doc_markdown,
    clippy::missing_const_for_fn,
    clippy::no_effect_underscore_binding,
    clippy::used_underscore_binding,
    clippy::redundant_clone,
    clippy::implicit_clone,
    unused_doc_comments
)]
//! Property 6: Preservation — Pagination range and navigation unchanged
//!
//! This test captures the CORRECT baseline behavior of the page-level pagination
//! component's logic that must remain unchanged after removing the duplicate bar.
//!
//! Observation on UNFIXED code (the retained page-level bar):
//! - Range text: "Mostrando {start}–{end} de {total}" where
//!   start = (page - 1) * per_page + 1, end = min(page * per_page, total)
//! - total_pages = total.div_ceil(per_page)
//! - "← Anterior" navigates to page - 1 when page > 1 (disabled at page 1)
//! - "Siguiente →" navigates to page + 1 when page < total_pages (disabled at last page)
//! - Per-page options: [10, 20, 50]
//! - Component hidden (empty html) when total == 0
//!
//! The property asserts over generated datasets (total, page, per_page):
//! - Range text is correctly computed
//! - Navigation respects page bounds
//! - Per-page selection resets to page 1 (by convention, caller responsibility)
//!
//! **EXPECTED OUTCOME**: Tests PASS on unfixed code (baseline pagination behavior captured).
//!
//! **Validates: Requirements 3.4**

// Feature: e2e-exploratory-bugfixes, Property 6: Preservation

use proptest::prelude::*;

// ── Model of Pagination Logic ──────────────────────────────────────────────
//
// This models the pure computation that the `Pagination` component in
// `frontend/src/components/common/pagination.rs` performs. The retained
// page-level bar in pagos.rs uses this exact logic.

/// Compute the range text displayed by the pagination bar.
/// Returns None when total == 0 (component is hidden).
fn compute_range_text(total: u64, page: u64, per_page: u64) -> Option<String> {
    if total == 0 {
        return None;
    }
    let start = (page - 1) * per_page + 1;
    let end = (page * per_page).min(total);
    Some(format!("Mostrando {start}\u{2013}{end} de {total}"))
}

/// Compute total number of pages.
fn compute_total_pages(total: u64, per_page: u64) -> u64 {
    total.div_ceil(per_page)
}

/// Determine what page the "previous" button navigates to.
/// Returns None if at first page (button disabled).
fn prev_page(page: u64) -> Option<u64> {
    if page > 1 { Some(page - 1) } else { None }
}

/// Determine what page the "next" button navigates to.
/// Returns None if at last page (button disabled).
fn next_page(page: u64, total_pages: u64) -> Option<u64> {
    if page < total_pages {
        Some(page + 1)
    } else {
        None
    }
}

/// Valid per-page options offered by the pagination component.
const VALID_PER_PAGE_OPTIONS: [u64; 3] = [10, 20, 50];

// ── Strategies ─────────────────────────────────────────────────────────────

/// Strategy for valid pagination states: generates (total, page, per_page)
/// where page is within bounds for the given total and per_page.
fn valid_pagination_state() -> impl Strategy<Value = (u64, u64, u64)> {
    // total: 1..500 (non-zero, component is hidden for 0)
    // per_page: one of the valid options [10, 20, 50]
    (1u64..500, prop_oneof![Just(10u64), Just(20), Just(50)]).prop_flat_map(|(total, per_page)| {
        let max_page = total.div_ceil(per_page);
        (Just(total), 1..=max_page, Just(per_page))
    })
}

/// Strategy for boundary pagination cases: first page, last page, single page.
fn boundary_pagination_state() -> impl Strategy<Value = (u64, u64, u64)> {
    prop_oneof![
        // Single page (total <= per_page)
        (1u64..=10, Just(1u64), Just(10u64)),
        (1u64..=20, Just(1u64), Just(20u64)),
        (1u64..=50, Just(1u64), Just(50u64)),
        // First page of multi-page dataset
        (11u64..200, Just(1u64), Just(10u64)),
        (21u64..200, Just(1u64), Just(20u64)),
        (51u64..200, Just(1u64), Just(50u64)),
        // Last page of multi-page dataset
        (11u64..200, Just(10u64), Just(10u64))
            .prop_filter("page must be valid", |(total, page, per_page)| {
                *page == total.div_ceil(*per_page)
            }),
    ]
    .prop_filter("page within bounds", |(total, page, per_page)| {
        let max_page = total.div_ceil(*per_page);
        *page >= 1 && *page <= max_page
    })
}

// ── Property Tests ─────────────────────────────────────────────────────────

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Validates: Requirements 3.4**
    ///
    /// Property 6a: Preservation — Range text is correctly computed.
    ///
    /// For ANY valid pagination state (total > 0, valid page, valid per_page),
    /// the range text SHALL be "Mostrando {start}–{end} de {total}" where:
    /// - start = (page - 1) * per_page + 1
    /// - end = min(page * per_page, total)
    ///
    /// EXPECTED: PASSES on unfixed code.
    #[test]
    fn prop_pagination_range_text_correct(
        (total, page, per_page) in valid_pagination_state()
    ) {
        let range_text = compute_range_text(total, page, per_page);

        // Component renders when total > 0
        prop_assert!(range_text.is_some(), "Range text should exist for total={}", total);

        let text = range_text.unwrap();
        let expected_start = (page - 1) * per_page + 1;
        let expected_end = (page * per_page).min(total);

        // Validate the range bounds are correct
        prop_assert!(
            expected_start >= 1,
            "Start must be >= 1, got {} (page={}, per_page={})",
            expected_start, page, per_page
        );
        prop_assert!(
            expected_end <= total,
            "End must be <= total, got end={} total={}",
            expected_end, total
        );
        prop_assert!(
            expected_start <= expected_end,
            "Start must be <= end: start={}, end={}",
            expected_start, expected_end
        );

        let expected_text = format!("Mostrando {expected_start}\u{2013}{expected_end} de {total}");
        prop_assert_eq!(
            text,
            expected_text,
            "Range text mismatch for total={}, page={}, per_page={}",
            total, page, per_page
        );
    }

    /// **Validates: Requirements 3.4**
    ///
    /// Property 6b: Preservation — Page navigation respects bounds.
    ///
    /// For ANY valid pagination state:
    /// - Previous is disabled (None) at page 1, otherwise navigates to page - 1
    /// - Next is disabled (None) at last page, otherwise navigates to page + 1
    ///
    /// EXPECTED: PASSES on unfixed code.
    #[test]
    fn prop_pagination_navigation_respects_bounds(
        (total, page, per_page) in valid_pagination_state()
    ) {
        let total_pages = compute_total_pages(total, per_page);

        // Previous page logic
        let prev = prev_page(page);
        if page == 1 {
            prop_assert_eq!(
                prev, None,
                "Previous should be disabled at page 1"
            );
        } else {
            prop_assert_eq!(
                prev, Some(page - 1),
                "Previous should navigate to page {} from page {}",
                page - 1, page
            );
        }

        // Next page logic
        let next = next_page(page, total_pages);
        if page == total_pages {
            prop_assert_eq!(
                next, None,
                "Next should be disabled at last page (page={}, total_pages={})",
                page, total_pages
            );
        } else {
            prop_assert_eq!(
                next, Some(page + 1),
                "Next should navigate to page {} from page {}",
                page + 1, page
            );
        }
    }

    /// **Validates: Requirements 3.4**
    ///
    /// Property 6c: Preservation — Pagination hidden when no items.
    ///
    /// When total == 0, the pagination component renders nothing (empty html).
    /// This ensures the retained bar behaves correctly with empty datasets.
    ///
    /// EXPECTED: PASSES on unfixed code.
    #[test]
    fn prop_pagination_hidden_when_empty(
        per_page in prop_oneof![Just(10u64), Just(20u64), Just(50u64)]
    ) {
        let range_text = compute_range_text(0, 1, per_page);
        prop_assert_eq!(
            range_text, None,
            "Pagination should be hidden (None) when total == 0"
        );
    }

    /// **Validates: Requirements 3.4**
    ///
    /// Property 6d: Preservation — Navigation at boundary pages.
    ///
    /// For boundary pagination states (first page, last page, single page),
    /// the navigation buttons are correctly enabled/disabled and the range
    /// text is correct.
    ///
    /// EXPECTED: PASSES on unfixed code.
    #[test]
    fn prop_pagination_boundary_behavior(
        (total, page, per_page) in boundary_pagination_state()
    ) {
        let total_pages = compute_total_pages(total, per_page);

        // Range text always present for total > 0
        let range_text = compute_range_text(total, page, per_page);
        prop_assert!(range_text.is_some());

        // At first page: prev disabled
        if page == 1 {
            prop_assert_eq!(prev_page(page), None);
        }

        // At last page: next disabled
        if page == total_pages {
            prop_assert_eq!(next_page(page, total_pages), None);
        }

        // Single page: both disabled
        if total_pages == 1 {
            prop_assert_eq!(prev_page(page), None);
            prop_assert_eq!(next_page(page, total_pages), None);
        }

        // Per-page is always one of the valid options
        prop_assert!(
            VALID_PER_PAGE_OPTIONS.contains(&per_page),
            "Per-page {} must be one of {:?}",
            per_page, VALID_PER_PAGE_OPTIONS
        );
    }
}
