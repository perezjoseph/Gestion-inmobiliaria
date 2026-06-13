// Feature: e2e-exploratory-bugfixes, Property 12: Preservation
// Populated invitations and other paginated endpoints unchanged
//
// **Validates: Requirements 3.7, 3.8**
//
// Observation-first methodology:
// On UNFIXED code, other `PaginatedResponse`-backed endpoints (e.g. `inquilinos::list`)
// return `{ data, total, page, perPage }` and deserialize/render correctly. The
// `PaginatedResponse<T>` struct serializes with `#[serde(rename_all = "camelCase")]`,
// producing `{ "data": [...], "total": N, "page": P, "perPage": PP }`.
//
// This test verifies:
//   1. The PaginatedResponse envelope is always well-formed for any data size (0..N)
//   2. `total == count`, correct `page`/`perPage` echo
//   3. The serialization shape matches the frontend's expected camelCase fields
//   4. Other paginated endpoints produce the same envelope shape (struct-level guarantee)
//
// EXPECTED OUTCOME: Tests PASS on unfixed code (baseline captured).
// The PaginatedResponse struct itself is correct — the bug is that invitaciones::listar
// does NOT use it (returns a bare Vec instead). Other endpoints that DO use it work fine.
#![allow(clippy::needless_return)]

use proptest::prelude::*;
use proptest::test_runner::{Config as ProptestConfig, TestRunner};
use serde::Deserialize;

// ── Deserialization target (mirrors what the frontend expects) ───────────

/// The frontend deserializes paginated responses as this struct.
/// All correctly-implemented paginated endpoints produce this shape.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FrontendPaginatedResponse {
    data: Vec<serde_json::Value>,
    total: u64,
    page: u64,
    per_page: u64,
}

// ── Strategies ──────────────────────────────────────────────────────────

/// Generate item counts from 0 to 50 (covers empty, single, and multi-item pages).
fn item_count_strategy() -> impl Strategy<Value = usize> {
    0usize..=50
}

/// Generate page numbers (1-based, reasonable range).
fn page_strategy() -> impl Strategy<Value = u64> {
    1u64..=100
}

/// Generate per_page values within the clamped range the backend uses.
fn per_page_strategy() -> impl Strategy<Value = u64> {
    1u64..=100
}

/// Generate simple data items to populate the `data` array.
fn data_item_strategy() -> impl Strategy<Value = serde_json::Value> {
    (0i64..10000).prop_map(|id| {
        serde_json::json!({
            "id": id,
            "nombre": format!("Item {id}")
        })
    })
}

// ── Property Tests ──────────────────────────────────────────────────────

/// Property 12a: Preservation — PaginatedResponse envelope is always well-formed.
///
/// For any data size (0..N), page, and perPage, constructing a PaginatedResponse
/// and serializing it produces a valid JSON object with `{ data, total, page, perPage }`
/// that the frontend can deserialize.
///
/// This test exercises the SAME struct that `inquilinos::list`, `propiedades::list`,
/// `pagos::list`, `contratos::list`, etc. all use. If this passes, the envelope shape
/// for all those endpoints is correct.
#[test]
fn property_12_paginated_response_envelope_is_well_formed() {
    use realestate_backend::models::PaginatedResponse;

    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::pbt_cases(),
        ..ProptestConfig::default()
    });

    runner
        .run(
            &(item_count_strategy(), page_strategy(), per_page_strategy()),
            |(count, page, per_page)| {
                // Construct PaginatedResponse the same way services do
                let data: Vec<serde_json::Value> = (0..count)
                    .map(|i| serde_json::json!({"id": i, "nombre": format!("Item {i}")}))
                    .collect();

                let response = PaginatedResponse {
                    data,
                    total: count as u64,
                    page,
                    per_page,
                };

                // Serialize (same as HttpResponse::Ok().json(response))
                let serialized = serde_json::to_string(&response)
                    .expect("PaginatedResponse serialization should never fail");

                // Frontend deserializes as camelCase struct
                let parsed: Result<FrontendPaginatedResponse, _> =
                    serde_json::from_str(&serialized);

                prop_assert!(
                    parsed.is_ok(),
                    "PaginatedResponse should deserialize correctly on the frontend. \
                     count={}, page={}, perPage={}, serialized='{}', error: {:?}",
                    count,
                    page,
                    per_page,
                    serialized,
                    parsed.err()
                );

                let parsed = parsed.unwrap();

                // Verify total == count
                prop_assert_eq!(
                    parsed.total,
                    count as u64,
                    "total should equal the data count"
                );

                // Verify page echo
                prop_assert_eq!(parsed.page, page, "page should echo the requested page");

                // Verify perPage echo
                prop_assert_eq!(
                    parsed.per_page,
                    per_page,
                    "perPage should echo the requested perPage"
                );

                // Verify data length matches count
                prop_assert_eq!(
                    parsed.data.len(),
                    count,
                    "data array length should match count"
                );

                Ok(())
            },
        )
        .expect(
            "Property 12a failed: PaginatedResponse envelope is not well-formed for some inputs",
        );
}

/// Property 12b: Preservation — PaginatedResponse serializes with camelCase field names.
///
/// The frontend relies on `perPage` (camelCase) not `per_page` (snake_case).
/// This verifies the `#[serde(rename_all = "camelCase")]` attribute works correctly
/// across all input variations.
#[test]
fn property_12_paginated_response_uses_camel_case_fields() {
    use realestate_backend::models::PaginatedResponse;

    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::pbt_cases(),
        ..ProptestConfig::default()
    });

    runner
        .run(
            &(item_count_strategy(), page_strategy(), per_page_strategy()),
            |(count, page, per_page)| {
                let data: Vec<serde_json::Value> =
                    (0..count).map(|i| serde_json::json!({"id": i})).collect();

                let response = PaginatedResponse {
                    data,
                    total: count as u64,
                    page,
                    per_page,
                };

                let serialized = serde_json::to_string(&response).unwrap();

                // Parse as raw JSON to check field names
                let raw: serde_json::Value = serde_json::from_str(&serialized).unwrap();
                let obj = raw.as_object().unwrap();

                // Must have camelCase fields
                prop_assert!(
                    obj.contains_key("data"),
                    "Response must contain 'data' field"
                );
                prop_assert!(
                    obj.contains_key("total"),
                    "Response must contain 'total' field"
                );
                prop_assert!(
                    obj.contains_key("page"),
                    "Response must contain 'page' field"
                );
                prop_assert!(
                    obj.contains_key("perPage"),
                    "Response must contain 'perPage' (camelCase) field"
                );

                // Must NOT have snake_case per_page
                prop_assert!(
                    !obj.contains_key("per_page"),
                    "Response must NOT contain 'per_page' (snake_case) — should be 'perPage'"
                );

                // Must have exactly 4 fields (no extras)
                prop_assert_eq!(
                    obj.len(),
                    4,
                    "PaginatedResponse should have exactly 4 fields: data, total, page, perPage"
                );

                Ok(())
            },
        )
        .expect(
            "Property 12b failed: PaginatedResponse does not use camelCase fields consistently",
        );
}

/// Property 12c: Preservation — Other paginated endpoints (inquilinos::list) produce
/// well-formed PaginatedResponse.
///
/// Integration test that exercises `inquilinos::list` against a real database to confirm
/// the existing paginated endpoint behavior is correct and unchanged. This captures
/// the baseline that must be preserved when fixing invitaciones.
#[test]
fn property_12_other_paginated_endpoints_are_well_formed() {
    use crate::common;

    common::with_db(|db| async move {
        use chrono::Utc;
        use realestate_backend::entities::organizacion;
        use realestate_backend::services::inquilinos;
        use sea_orm::{ActiveModelTrait, EntityTrait, Set};
        use uuid::Uuid;

        // Create an org
        let org_id = Uuid::new_v4();
        let now = Utc::now().into();
        organizacion::ActiveModel {
            id: Set(org_id),
            tipo: Set("propietario".to_string()),
            nombre: Set(format!("Preservation PBT Org {org_id}")),
            estado: Set("activo".to_string()),
            cedula: Set(None),
            telefono: Set(None),
            email_organizacion: Set(None),
            rnc: Set(None),
            razon_social: Set(None),
            nombre_comercial: Set(None),
            direccion_fiscal: Set(None),
            representante_legal: Set(None),
            dgii_data: Set(None),
            tipo_fiscal: Set("informal".to_string()),
            regimen_pagos: Set(None),
            fecha_inicio_operaciones: Set(None),
            is_ecf_certificado: Set(false),
            created_at: Set(now),
            updated_at: Set(now),
        }
        .insert(&db)
        .await
        .expect("Failed to create test org");

        // Call inquilinos::list with page=1, per_page=20 (empty dataset for this org)
        let result = inquilinos::list(&db, org_id, None, Some(1), Some(20))
            .await
            .expect("inquilinos::list should succeed");

        // Serialize the same way the handler does
        let serialized = serde_json::to_string(&result).expect("Serialization should not fail");

        // Frontend deserializes as PaginatedResponse
        let parsed: Result<FrontendPaginatedResponse, _> = serde_json::from_str(&serialized);

        assert!(
            parsed.is_ok(),
            "inquilinos::list response should be a well-formed PaginatedResponse. \
             Serialized: '{}', Error: {:?}",
            serialized,
            parsed.err()
        );

        let parsed = parsed.unwrap();
        assert_eq!(parsed.total, 0, "Empty org should have total=0");
        assert_eq!(parsed.page, 1, "Page should echo requested page");
        assert_eq!(parsed.per_page, 20, "PerPage should echo requested perPage");
        assert!(parsed.data.is_empty(), "Data should be empty for new org");

        // Cleanup
        let _ = organizacion::Entity::delete_by_id(org_id).exec(&db).await;
    });
}

/// Property 12d: Preservation — PaginatedResponse with varying data payloads
/// round-trips correctly (simulating populated invitations and other endpoints).
///
/// Over generated data payloads, verify that PaginatedResponse always produces
/// a valid envelope regardless of the inner data type or size.
#[test]
fn property_12_paginated_response_round_trip_with_varied_payloads() {
    use realestate_backend::models::PaginatedResponse;

    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::pbt_cases(),
        ..ProptestConfig::default()
    });

    // Strategy: generate vectors of various data items
    let payload_strategy = proptest::collection::vec(data_item_strategy(), 0..=30);

    runner
        .run(
            &(payload_strategy, page_strategy(), per_page_strategy()),
            |(data, page, per_page)| {
                let count = data.len();

                let response = PaginatedResponse {
                    data,
                    total: count as u64,
                    page,
                    per_page,
                };

                // Serialize
                let serialized = serde_json::to_string(&response).unwrap();

                // Deserialize as frontend would
                let parsed: FrontendPaginatedResponse =
                    serde_json::from_str(&serialized).map_err(|e| {
                        proptest::test_runner::TestCaseError::Fail(
                            format!(
                                "Round-trip failed: count={count}, page={page}, \
                                 perPage={per_page}, error={e}"
                            )
                            .into(),
                        )
                    })?;

                // Invariants
                prop_assert_eq!(parsed.data.len(), count);
                prop_assert_eq!(parsed.total, count as u64);
                prop_assert_eq!(parsed.page, page);
                prop_assert_eq!(parsed.per_page, per_page);

                Ok(())
            },
        )
        .expect("Property 12d failed: PaginatedResponse round-trip broken for some payload");
}
