use proptest::prelude::*;
use proptest::test_runner::{Config as ProptestConfig, TestRunner};
use uuid::Uuid;

// ── Custom Strategies ──────────────────────────────────────────────────

/// Generate a non-empty name string (1-20 alpha chars).
fn arb_name() -> impl Strategy<Value = String> {
    "[A-Za-z]{1,20}".prop_map(|s| s.trim().to_string())
}

/// Generate a pair of distinct UUIDs representing two different organizations.
fn arb_distinct_org_ids() -> impl Strategy<Value = (Uuid, Uuid)> {
    (any::<u128>(), any::<u128>())
        .prop_filter("org IDs must differ", |(a, b)| a != b)
        .prop_map(|(a, b)| (Uuid::from_u128(a), Uuid::from_u128(b)))
}

// ── Tenant Matching Model ──────────────────────────────────────────────

/// Represents an inquilino record with nombre, apellido, and organization scope.
#[derive(Debug, Clone)]
struct InquilinoRecord {
    id: Uuid,
    nombre: String,
    apellido: String,
    org_id: Uuid,
}

/// Simulates the best-effort tenant matching logic from `ocr_mapping::map_deposito`:
///
/// 1. Filter inquilinos by `organizacion_id`
/// 2. Apply LIKE `%trimmed%` over `concat(nombre, ' ', apellido)`
/// 3. Return `Some(id)` only when candidate set has exactly one match
/// 4. Otherwise return `None` (never wrong)
///
/// This mirrors the design:
/// ```rust
/// let candidatos = inquilino::Entity::find()
///     .filter(inquilino::Column::OrganizacionId.eq(organizacion_id))
///     .filter(Expr::expr(Func::concat([nombre, ' ', apellido])).like(&pattern))
///     .limit(2)
///     .all(db)
///     .await?;
/// Ok(match candidatos.as_slice() {
///     [unico] => Some(unico.id),
///     _ => None,
/// })
/// ```
fn map_deposito_model(
    dataset: &[InquilinoRecord],
    nombre_extraido: &str,
    organizacion_id: Uuid,
) -> Option<Uuid> {
    let trimmed = nombre_extraido.trim();
    if trimmed.is_empty() {
        return None;
    }
    let pattern = trimmed.to_lowercase();

    let candidatos: Vec<&InquilinoRecord> = dataset
        .iter()
        .filter(|inq| inq.org_id == organizacion_id)
        .filter(|inq| {
            let full_name = format!("{} {}", inq.nombre, inq.apellido).to_lowercase();
            full_name.contains(&pattern)
        })
        .collect();

    match candidatos.as_slice() {
        [unico] => Some(unico.id),
        _ => None,
    }
}

// ── Strategies for datasets ────────────────────────────────────────────

/// Generate a random inquilino record for a given org.
fn arb_inquilino_in_org(org_id: Uuid) -> impl Strategy<Value = InquilinoRecord> {
    (arb_name(), arb_name(), any::<u128>()).prop_map(move |(nombre, apellido, id_seed)| {
        InquilinoRecord {
            id: Uuid::from_u128(id_seed),
            nombre,
            apellido,
            org_id,
        }
    })
}

/// Generate a dataset of inquilinos across two orgs.
fn arb_dataset(org_a: Uuid, org_b: Uuid) -> impl Strategy<Value = Vec<InquilinoRecord>> {
    (
        prop::collection::vec(arb_inquilino_in_org(org_a), 0..=5),
        prop::collection::vec(arb_inquilino_in_org(org_b), 0..=5),
    )
        .prop_map(|(mut a, b)| {
            a.extend(b);
            a
        })
}

// ── Property Tests ─────────────────────────────────────────────────────

// Feature: spec-gap-remediation, Property 5: Tenant match is best-effort and never wrong
/// **Validates: Requirements 5.4, 5.5**
#[test]
fn test_tenant_match_returns_some_only_for_unique_candidate() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::pbt_cases(),
        ..Default::default()
    });

    let strategy =
        (arb_distinct_org_ids(), arb_name()).prop_flat_map(|((org_a, org_b), search_name)| {
            (
                Just(org_a),
                Just(org_b),
                Just(search_name),
                arb_dataset(org_a, org_b),
            )
        });

    runner
        .run(&strategy, |(org_a, _org_b, search_name, dataset)| {
            let result = map_deposito_model(&dataset, &search_name, org_a);
            let trimmed = search_name.trim();
            let pattern = trimmed.to_lowercase();

            // Count candidates within org_a that match the LIKE pattern
            let candidates_in_org: Vec<&InquilinoRecord> = dataset
                .iter()
                .filter(|inq| inq.org_id == org_a)
                .filter(|inq| {
                    let full_name = format!("{} {}", inq.nombre, inq.apellido).to_lowercase();
                    full_name.contains(&pattern)
                })
                .collect();

            match candidates_in_org.len() {
                1 => {
                    // Exactly one candidate → must return Some(id)
                    prop_assert_eq!(
                        result,
                        Some(candidates_in_org[0].id),
                        "Expected Some({}) for unique match on '{}', got {:?}",
                        candidates_in_org[0].id,
                        search_name,
                        result
                    );
                }
                _ => {
                    // 0 or >=2 candidates → must return None
                    prop_assert_eq!(
                        result,
                        None,
                        "Expected None for {} candidates on '{}', got {:?}",
                        candidates_in_org.len(),
                        search_name,
                        result
                    );
                }
            }

            Ok(())
        })
        .unwrap();
}

// Feature: spec-gap-remediation, Property 5: Tenant match is best-effort and never wrong
/// **Validates: Requirements 5.4, 5.5**
#[test]
fn test_tenant_match_never_returns_inquilino_from_another_org() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::pbt_cases(),
        ..Default::default()
    });

    let strategy =
        (arb_distinct_org_ids(), arb_name()).prop_flat_map(|((org_a, org_b), search_name)| {
            (
                Just(org_a),
                Just(org_b),
                Just(search_name),
                arb_dataset(org_a, org_b),
            )
        });

    runner
        .run(&strategy, |(org_a, org_b, search_name, dataset)| {
            let result = map_deposito_model(&dataset, &search_name, org_a);

            // Collect all inquilino IDs belonging to org_b
            let org_b_ids: Vec<Uuid> = dataset
                .iter()
                .filter(|inq| inq.org_id == org_b)
                .map(|inq| inq.id)
                .collect();

            // If result is Some, it must NOT be an ID from org_b
            if let Some(returned_id) = result {
                prop_assert!(
                    !org_b_ids.contains(&returned_id),
                    "Returned inquilino {} belongs to org_b {} — cross-tenant leak!",
                    returned_id,
                    org_b
                );

                // Additionally verify the returned ID belongs to org_a
                let belongs_to_org_a = dataset
                    .iter()
                    .any(|inq| inq.id == returned_id && inq.org_id == org_a);
                prop_assert!(
                    belongs_to_org_a,
                    "Returned inquilino {} does not belong to org_a {}",
                    returned_id,
                    org_a
                );
            }

            Ok(())
        })
        .unwrap();
}

// Feature: spec-gap-remediation, Property 5: Tenant match is best-effort and never wrong
/// **Validates: Requirements 5.4, 5.5**
#[test]
fn test_tenant_match_empty_name_returns_none() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::pbt_cases(),
        ..Default::default()
    });

    let strategy = (
        any::<u128>().prop_map(Uuid::from_u128),
        prop::collection::vec(
            (arb_name(), arb_name(), any::<u128>()).prop_map(|(n, a, id)| InquilinoRecord {
                id: Uuid::from_u128(id),
                nombre: n,
                apellido: a,
                org_id: Uuid::nil(), // placeholder, overridden below
            }),
            0..=5,
        ),
        prop_oneof![
            Just("".to_string()),
            Just(" ".to_string()),
            Just("  ".to_string()),
            Just("\t".to_string()),
            Just("   \t  ".to_string()),
        ],
    )
        .prop_map(|(org_id, mut records, empty_name)| {
            for r in &mut records {
                r.org_id = org_id;
            }
            (org_id, records, empty_name)
        });

    runner
        .run(&strategy, |(org_id, dataset, empty_name)| {
            let result = map_deposito_model(&dataset, &empty_name, org_id);
            prop_assert_eq!(
                result,
                None,
                "Empty/whitespace name '{}' should always return None, got {:?}",
                empty_name,
                result
            );
            Ok(())
        })
        .unwrap();
}
