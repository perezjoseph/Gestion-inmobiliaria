#![allow(clippy::unwrap_used, clippy::expect_used, clippy::doc_markdown)]

//! Property-based tests for partial payment logic and informal receipt generation.
//!
//! These tests verify pure logic properties without requiring a database connection.

use proptest::prelude::*;
use rust_decimal::Decimal;
use std::collections::HashSet;

// ── Custom Strategies ──────────────────────────────────────────────────

/// Generate a sequence of partial payment amounts that sum to less than or equal to amount_due.
/// Returns (amount_due, payments) where each payment < amount_due.
fn arb_partial_payment_sequence() -> impl Strategy<Value = (Decimal, Vec<Decimal>)> {
    // amount_due in centavos: [100, 1_000_000] (i.e., 1.00 to 10,000.00)
    (100i64..=1_000_000i64).prop_flat_map(|due_cents| {
        let amount_due = Decimal::new(due_cents, 2);
        // Generate 1 to 10 payments, each between 1 centavo and (due_cents - 1) centavos
        let max_payment = due_cents.max(2) - 1; // ensure at least 1 centavo per payment
        let payment_strategy =
            prop::collection::vec(1i64..=max_payment, 1..=10usize).prop_map(move |raw_payments| {
                // Clamp the cumulative sum so it doesn't exceed amount_due
                let mut payments = Vec::new();
                let mut cumulative = 0i64;
                for p in raw_payments {
                    let remaining = due_cents - cumulative;
                    if remaining <= 0 {
                        break;
                    }
                    let clamped = p.min(remaining);
                    if clamped <= 0 {
                        break;
                    }
                    payments.push(Decimal::new(clamped, 2));
                    cumulative += clamped;
                }
                payments
            });
        (Just(amount_due), payment_strategy)
    })
}

/// Generate multiple unpaid periods with their amounts due (ordered by "age").
/// Returns Vec<(period_index, amount_due)> sorted oldest-first.
fn arb_unpaid_periods() -> impl Strategy<Value = Vec<Decimal>> {
    prop::collection::vec(100i64..=500_000i64, 2..=6usize)
        .prop_map(|cents_vec| cents_vec.into_iter().map(|c| Decimal::new(c, 2)).collect())
}

/// Generate a payment amount for FIFO allocation testing.
fn arb_fifo_payment() -> impl Strategy<Value = Decimal> {
    (1i64..=2_000_000i64).prop_map(|cents| Decimal::new(cents, 2))
}

/// Generate a starting sequence number for receipt references.
fn arb_receipt_sequence_start() -> impl Strategy<Value = u32> {
    0u32..=999_990u32
}

/// Generate count of receipts to produce.
fn arb_receipt_count() -> impl Strategy<Value = usize> {
    1usize..=100usize
}

// ── Pure Logic Under Test ──────────────────────────────────────────────

/// Computes the remaining balance for a billing period after payments.
/// Mirrors `balance_remaining` from `services/pagos.rs`.
fn compute_saldo_pendiente(amount_due: Decimal, payments: &[Decimal]) -> Decimal {
    let sum: Decimal = payments.iter().copied().sum();
    amount_due - sum
}

/// Determines whether a period should be marked as `pagado`.
/// The period is `pagado` iff sum(payments) >= amount_due.
fn is_pagado(amount_due: Decimal, payments: &[Decimal]) -> bool {
    let sum: Decimal = payments.iter().copied().sum();
    sum >= amount_due
}

/// Allocates a payment using FIFO logic across multiple unpaid periods.
/// Returns a Vec of (period_index, amount_applied, new_balance) tuples.
/// Periods are assumed sorted oldest-first.
fn allocate_fifo(
    periods: &[Decimal], // amount_due per period (oldest first)
    payment: Decimal,
) -> Vec<(usize, Decimal, Decimal)> {
    let mut remaining = payment;
    let mut allocations = Vec::new();

    for (idx, &amount_due) in periods.iter().enumerate() {
        if remaining <= Decimal::ZERO {
            break;
        }

        let applied = remaining.min(amount_due);
        let new_balance = amount_due - applied;
        remaining -= applied;

        allocations.push((idx, applied, new_balance));
    }

    allocations
}

/// Generates a sequence of referencia_interna strings starting from a given number.
/// Mirrors the logic in `services/recibos_informales.rs`.
fn generar_referencias(start: u32, count: usize) -> Vec<String> {
    (0..count)
        .map(|i| {
            let num = start + i as u32 + 1; // starts at 1 if start is 0
            format!("RI-{num:06}")
        })
        .collect()
}

// ── Property Tests ─────────────────────────────────────────────────────

proptest! {
    #![proptest_config(ProptestConfig { cases: crate::test_support::pbt_cases(), ..Default::default() })]

    // Feature: dr-landlord-compliance, Property 11: Partial Payment Balance Tracking
    /// For any sequence of partial payments against a billing period with `amount_due`,
    /// the `saldo_pendiente` after each payment equals `amount_due - sum(all_payments_so_far)`,
    /// and the period is marked `pagado` if and only if `sum(all_payments) >= amount_due`.
    ///
    /// **Validates: Requirements 3.1, 3.2, 3.3**
    #[test]
    fn partial_payment_balance_tracking(
        (amount_due, payments) in arb_partial_payment_sequence()
    ) {
        // Verify invariant at each step
        let mut cumulative_payments: Vec<Decimal> = Vec::new();

        for payment in &payments {
            cumulative_payments.push(*payment);

            let saldo = compute_saldo_pendiente(amount_due, &cumulative_payments);
            let expected_saldo = amount_due - cumulative_payments.iter().copied().sum::<Decimal>();

            // Property 11a: saldo_pendiente = amount_due - sum(payments_so_far)
            prop_assert_eq!(
                saldo, expected_saldo,
                "saldo_pendiente should equal amount_due - sum(payments). \
                 amount_due={}, payments={:?}, saldo={}, expected={}",
                amount_due, cumulative_payments, saldo, expected_saldo
            );

            // Property 11b: saldo_pendiente is never negative (payments are clamped)
            prop_assert!(
                saldo >= Decimal::ZERO,
                "saldo_pendiente must not be negative. Got {} for amount_due={}, payments={:?}",
                saldo, amount_due, cumulative_payments
            );
        }

        // Property 11c: period is `pagado` iff sum(payments) >= amount_due
        let total_paid: Decimal = payments.iter().copied().sum();
        let pagado = is_pagado(amount_due, &payments);

        if total_paid >= amount_due {
            prop_assert!(
                pagado,
                "Period should be marked pagado when total_paid ({}) >= amount_due ({})",
                total_paid, amount_due
            );
        } else {
            prop_assert!(
                !pagado,
                "Period should NOT be marked pagado when total_paid ({}) < amount_due ({})",
                total_paid, amount_due
            );
        }
    }

    // Feature: dr-landlord-compliance, Property 12: FIFO Payment Allocation
    /// For any payment without explicit fecha_vencimiento reference and a set of unpaid periods,
    /// the payment is allocated to the period with the earliest fecha_vencimiento (oldest first).
    /// If the payment exceeds that period's balance, the surplus cascades to the next oldest.
    ///
    /// **Validates: Requirements 3.4, 3.8**
    #[test]
    fn fifo_payment_allocation(
        periods in arb_unpaid_periods(),
        payment in arb_fifo_payment(),
    ) {
        let total_owed: Decimal = periods.iter().copied().sum();
        // Clamp payment to total owed (the service rejects overpayment)
        let clamped_payment = payment.min(total_owed);

        if clamped_payment <= Decimal::ZERO {
            return Ok(());
        }

        let allocations = allocate_fifo(&periods, clamped_payment);

        // Property 12a: Payment is allocated to oldest period first
        if !allocations.is_empty() {
            prop_assert_eq!(
                allocations[0].0, 0,
                "First allocation must go to the oldest period (index 0)"
            );
        }

        // Property 12b: Allocation indices are strictly increasing (cascade order)
        for window in allocations.windows(2) {
            prop_assert!(
                window[1].0 > window[0].0,
                "Allocation indices must be strictly increasing (FIFO order). \
                 Got idx {} after idx {}",
                window[1].0, window[0].0
            );
        }

        // Property 12c: Sum of all applied amounts equals the clamped payment
        let total_applied: Decimal = allocations.iter().map(|(_, applied, _)| applied).sum();
        prop_assert_eq!(
            total_applied, clamped_payment,
            "Sum of applied amounts ({}) must equal the payment ({}). allocations={:?}",
            total_applied, clamped_payment, allocations
        );

        // Property 12d: Each allocation does not exceed the period's amount_due
        for &(idx, applied, _) in &allocations {
            prop_assert!(
                applied <= periods[idx],
                "Applied amount ({}) must not exceed period amount_due ({}) at index {}",
                applied, periods[idx], idx
            );
        }

        // Property 12e: new_balance = amount_due - applied for each period
        for &(idx, applied, new_balance) in &allocations {
            let expected_balance = periods[idx] - applied;
            prop_assert_eq!(
                new_balance, expected_balance,
                "new_balance ({}) must equal amount_due ({}) - applied ({}) at index {}",
                new_balance, periods[idx], applied, idx
            );
        }

        // Property 12f: If a period is not fully paid, no subsequent period receives allocation
        for (i, &(idx, applied, _)) in allocations.iter().enumerate() {
            let period_fully_paid = applied == periods[idx];
            if !period_fully_paid {
                // This must be the last allocation
                prop_assert_eq!(
                    i, allocations.len() - 1,
                    "If period {} is not fully paid (applied={}, due={}), it must be the last allocation",
                    idx, applied, periods[idx]
                );
            }
        }
    }

    // Feature: dr-landlord-compliance, Property 13: Informal Receipt Uniqueness
    /// For any sequence of referencia_interna generations, each is unique across all
    /// recibos_informales for that organization.
    ///
    /// **Validates: Requirements 3.5**
    #[test]
    fn informal_receipt_uniqueness(
        start in arb_receipt_sequence_start(),
        count in arb_receipt_count(),
    ) {
        let referencias = generar_referencias(start, count);

        // Property 13a: All generated references are unique
        let unique_set: HashSet<&String> = referencias.iter().collect();
        prop_assert_eq!(
            unique_set.len(), referencias.len(),
            "All referencia_interna values must be unique. Got {} unique out of {}. start={}, count={}",
            unique_set.len(), referencias.len(), start, count
        );

        // Property 13b: All references follow the "RI-NNNNNN" format (6-digit zero-padded)
        for referencia in &referencias {
            prop_assert!(
                referencia.starts_with("RI-"),
                "referencia_interna must start with 'RI-', got: {}", referencia
            );
            let num_part = &referencia[3..];
            prop_assert_eq!(
                num_part.len(), 6,
                "Numeric portion must be exactly 6 digits, got '{}' (len={})",
                num_part, num_part.len()
            );
            prop_assert!(
                num_part.chars().all(|c| c.is_ascii_digit()),
                "Numeric portion must be all digits, got '{}'", num_part
            );
        }

        // Property 13c: References are sequential (each number is previous + 1)
        for i in 1..referencias.len() {
            let prev_num: u32 = referencias[i - 1][3..].parse().unwrap();
            let curr_num: u32 = referencias[i][3..].parse().unwrap();
            prop_assert_eq!(
                curr_num, prev_num + 1,
                "References must be sequential. prev={}, curr={}, index={}",
                prev_num, curr_num, i
            );
        }
    }
}
