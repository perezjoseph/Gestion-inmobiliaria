#![allow(clippy::unwrap_used, clippy::expect_used, clippy::doc_markdown)]

use proptest::prelude::*;
use rust_decimal::Decimal;
use std::collections::HashSet;

fn arb_partial_payment_sequence() -> impl Strategy<Value = (Decimal, Vec<Decimal>)> {
    (100i64..=1_000_000i64).prop_flat_map(|due_cents| {
        let amount_due = Decimal::new(due_cents, 2);
        let max_payment = due_cents.max(2) - 1;
        let payment_strategy =
            prop::collection::vec(1i64..=max_payment, 1..=10usize).prop_map(move |raw_payments| {
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

fn arb_unpaid_periods() -> impl Strategy<Value = Vec<Decimal>> {
    prop::collection::vec(100i64..=500_000i64, 2..=6usize)
        .prop_map(|cents_vec| cents_vec.into_iter().map(|c| Decimal::new(c, 2)).collect())
}

fn arb_fifo_payment() -> impl Strategy<Value = Decimal> {
    (1i64..=2_000_000i64).prop_map(|cents| Decimal::new(cents, 2))
}

fn arb_receipt_sequence_start() -> impl Strategy<Value = u32> {
    0u32..=999_990u32
}

fn arb_receipt_count() -> impl Strategy<Value = usize> {
    1usize..=100usize
}

fn compute_saldo_pendiente(amount_due: Decimal, payments: &[Decimal]) -> Decimal {
    let sum: Decimal = payments.iter().copied().sum();
    amount_due - sum
}

fn is_pagado(amount_due: Decimal, payments: &[Decimal]) -> bool {
    let sum: Decimal = payments.iter().copied().sum();
    sum >= amount_due
}

fn allocate_fifo(periods: &[Decimal], payment: Decimal) -> Vec<(usize, Decimal, Decimal)> {
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

fn generar_referencias(start: u32, count: usize) -> Vec<String> {
    (0..count)
        .map(|i| {
            let num = start + i as u32 + 1;
            format!("RI-{num:06}")
        })
        .collect()
}

proptest! {
    #![proptest_config(ProptestConfig { cases: crate::test_support::pbt_cases(), ..Default::default() })]

    #[test]
    fn partial_payment_balance_tracking(
        (amount_due, payments) in arb_partial_payment_sequence()
    ) {
        let mut cumulative_payments: Vec<Decimal> = Vec::new();

        for payment in &payments {
            cumulative_payments.push(*payment);

            let saldo = compute_saldo_pendiente(amount_due, &cumulative_payments);
            let expected_saldo = amount_due - cumulative_payments.iter().copied().sum::<Decimal>();

            prop_assert_eq!(
                saldo, expected_saldo,
                "saldo_pendiente should equal amount_due - sum(payments). \
                 amount_due={}, payments={:?}, saldo={}, expected={}",
                amount_due, cumulative_payments, saldo, expected_saldo
            );

            prop_assert!(
                saldo >= Decimal::ZERO,
                "saldo_pendiente must not be negative. Got {} for amount_due={}, payments={:?}",
                saldo, amount_due, cumulative_payments
            );
        }

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

    #[test]
    fn fifo_payment_allocation(
        periods in arb_unpaid_periods(),
        payment in arb_fifo_payment(),
    ) {
        let total_owed: Decimal = periods.iter().copied().sum();
        let clamped_payment = payment.min(total_owed);

        if clamped_payment <= Decimal::ZERO {
            return Ok(());
        }

        let allocations = allocate_fifo(&periods, clamped_payment);

        if !allocations.is_empty() {
            prop_assert_eq!(
                allocations[0].0, 0,
                "First allocation must go to the oldest period (index 0)"
            );
        }

        for window in allocations.windows(2) {
            prop_assert!(
                window[1].0 > window[0].0,
                "Allocation indices must be strictly increasing (FIFO order). \
                 Got idx {} after idx {}",
                window[1].0, window[0].0
            );
        }

        let total_applied: Decimal = allocations.iter().map(|(_, applied, _)| applied).sum();
        prop_assert_eq!(
            total_applied, clamped_payment,
            "Sum of applied amounts ({}) must equal the payment ({}). allocations={:?}",
            total_applied, clamped_payment, allocations
        );

        for &(idx, applied, _) in &allocations {
            prop_assert!(
                applied <= periods[idx],
                "Applied amount ({}) must not exceed period amount_due ({}) at index {}",
                applied, periods[idx], idx
            );
        }

        for &(idx, applied, new_balance) in &allocations {
            let expected_balance = periods[idx] - applied;
            prop_assert_eq!(
                new_balance, expected_balance,
                "new_balance ({}) must equal amount_due ({}) - applied ({}) at index {}",
                new_balance, periods[idx], applied, idx
            );
        }

        for (i, &(idx, applied, _)) in allocations.iter().enumerate() {
            let period_fully_paid = applied == periods[idx];
            if !period_fully_paid {
                prop_assert_eq!(
                    i, allocations.len() - 1,
                    "If period {} is not fully paid (applied={}, due={}), it must be the last allocation",
                    idx, applied, periods[idx]
                );
            }
        }
    }

    #[test]
    fn informal_receipt_uniqueness(
        start in arb_receipt_sequence_start(),
        count in arb_receipt_count(),
    ) {
        let referencias = generar_referencias(start, count);

        let unique_set: HashSet<&String> = referencias.iter().collect();
        prop_assert_eq!(
            unique_set.len(), referencias.len(),
            "All referencia_interna values must be unique. Got {} unique out of {}. start={}, count={}",
            unique_set.len(), referencias.len(), start, count
        );

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
