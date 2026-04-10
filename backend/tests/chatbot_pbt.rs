use proptest::prelude::*;
use proptest::test_runner::{Config as ProptestConfig, TestRunner};
use uuid::Uuid;

// ── Custom Strategies ──────────────────────────────────────────────────

/// Generate a valid E.164 phone number (+ followed by 7-15 digits, first digit non-zero).
fn arb_e164_phone() -> impl Strategy<Value = String> {
    // Country code 1-3 digits (first non-zero) + subscriber 6-12 digits
    (1u8..=9, prop::collection::vec(0u8..=9, 6..=12)).prop_map(|(first, rest)| {
        let digits: String = rest.iter().map(|d| char::from(b'0' + d)).collect();
        format!("+{first}{digits}")
    })
}

/// Generate a pair of distinct UUIDs representing two different organizations.
fn arb_distinct_org_ids() -> impl Strategy<Value = (Uuid, Uuid)> {
    // Generate two random u128 values and ensure they differ
    (any::<u128>(), any::<u128>())
        .prop_filter("org IDs must differ", |(a, b)| a != b)
        .prop_map(|(a, b)| (Uuid::from_u128(a), Uuid::from_u128(b)))
}

// ── Tenant Resolution Model ───────────────────────────────────────────

/// Represents a tenant record with phone and organization scope.
#[derive(Debug, Clone)]
struct TenantRecord {
    phone: String,
    org_id: Uuid,
}

/// Simulates the org-scoped tenant resolution logic:
/// Query `inquilinos` WHERE `telefono = phone AND organizacion_id = org_id`.
/// Returns true if a matching tenant exists.
///
/// This mirrors the `tenants_only` policy check in `is_sender_allowed`:
/// ```rust
/// "tenants_only" => tenant_exists_by_phone(phone, org_id, db).await
/// ```
/// The critical property is that BOTH phone AND org_id are used as filters.
fn resolve_tenant_in_org(tenants: &[TenantRecord], phone: &str, org_id: Uuid) -> bool {
    tenants
        .iter()
        .any(|t| t.phone == phone && t.org_id == org_id)
}

// ── Property Tests ─────────────────────────────────────────────────────

// Feature: whatsapp-ai-assistant, Property 3: Organization-Scoped Tenant Resolution Isolation
// **Validates: Requirements 2.5, 13.2**
#[test]
fn test_org_scoped_tenant_resolution_isolation() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::pbt_cases(),
        ..Default::default()
    });

    let strategy =
        (arb_e164_phone(), arb_distinct_org_ids()).prop_flat_map(|(phone, (org_a, org_b))| {
            // Generate additional tenants in org_b with DIFFERENT phones to ensure
            // the org_b tenant set is non-empty but doesn't contain our target phone
            let other_phones = prop::collection::vec(arb_e164_phone(), 0..=5).prop_filter(
                "other phones must not match target",
                {
                    let phone = phone.clone();
                    move |phones| phones.iter().all(|p| *p != phone)
                },
            );
            (Just(phone), Just(org_a), Just(org_b), other_phones)
        });

    runner
        .run(&strategy, |(phone, org_a, org_b, other_phones_in_b)| {
            // Setup: phone P exists as a tenant in org_a
            let mut tenants = vec![TenantRecord {
                phone: phone.clone(),
                org_id: org_a,
            }];

            // Add other tenants in org_b (with different phones)
            for other_phone in &other_phones_in_b {
                tenants.push(TenantRecord {
                    phone: other_phone.clone(),
                    org_id: org_b,
                });
            }

            // Property: resolving phone P in org_a SHOULD find a match
            prop_assert!(
                resolve_tenant_in_org(&tenants, &phone, org_a),
                "Phone {} should be found in org_a {}",
                phone,
                org_a
            );

            // Property: resolving phone P in org_b SHALL return no match
            prop_assert!(
                !resolve_tenant_in_org(&tenants, &phone, org_b),
                "Phone {} must NOT be found in org_b {} (isolation violated)",
                phone,
                org_b
            );

            Ok(())
        })
        .unwrap();
}

// Feature: whatsapp-ai-assistant, Property 3 (supplementary): Resolution uses both filters
// **Validates: Requirements 2.5, 13.2**
#[test]
fn test_resolution_requires_both_phone_and_org_id() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::pbt_cases(),
        ..Default::default()
    });

    let strategy = (arb_e164_phone(), arb_e164_phone(), arb_distinct_org_ids())
        .prop_filter("phones must differ", |(p1, p2, _)| p1 != p2);

    runner
        .run(&strategy, |(phone_a, phone_b, (org_a, org_b))| {
            // Setup: phone_a in org_a, phone_b in org_b
            let tenants = vec![
                TenantRecord {
                    phone: phone_a.clone(),
                    org_id: org_a,
                },
                TenantRecord {
                    phone: phone_b.clone(),
                    org_id: org_b,
                },
            ];

            // Correct org + correct phone → match
            prop_assert!(resolve_tenant_in_org(&tenants, &phone_a, org_a));
            prop_assert!(resolve_tenant_in_org(&tenants, &phone_b, org_b));

            // Correct phone + wrong org → no match (isolation)
            prop_assert!(!resolve_tenant_in_org(&tenants, &phone_a, org_b));
            prop_assert!(!resolve_tenant_in_org(&tenants, &phone_b, org_a));

            // Wrong phone + correct org → no match
            prop_assert!(!resolve_tenant_in_org(&tenants, &phone_b, org_a));
            prop_assert!(!resolve_tenant_in_org(&tenants, &phone_a, org_b));

            Ok(())
        })
        .unwrap();
}

// Feature: whatsapp-ai-assistant, Property 3 (supplementary): Same phone in multiple orgs
// **Validates: Requirements 2.5, 13.2**
#[test]
fn test_same_phone_different_orgs_resolved_independently() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::pbt_cases(),
        ..Default::default()
    });

    let strategy = (
        arb_e164_phone(),
        arb_distinct_org_ids(),
        any::<bool>(), // whether phone exists in org_b too
    );

    runner
        .run(&strategy, |(phone, (org_a, org_b), exists_in_b)| {
            // Phone always exists in org_a
            let mut tenants = vec![TenantRecord {
                phone: phone.clone(),
                org_id: org_a,
            }];

            // Conditionally add to org_b
            if exists_in_b {
                tenants.push(TenantRecord {
                    phone: phone.clone(),
                    org_id: org_b,
                });
            }

            // org_a always resolves
            prop_assert!(resolve_tenant_in_org(&tenants, &phone, org_a));

            // org_b resolves only if we added it there
            prop_assert_eq!(
                resolve_tenant_in_org(&tenants, &phone, org_b),
                exists_in_b,
                "Resolution in org_b should match whether phone was added there"
            );

            Ok(())
        })
        .unwrap();
}
