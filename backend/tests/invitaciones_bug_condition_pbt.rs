// Feature: e2e-exploratory-bugfixes, Property 11: Bug Condition
// Empty Invitaciones returns a well-formed PaginatedResponse
//
// **Validates: Requirements 1.6**
//
// Bug Condition: The handler `handlers::invitaciones::listar` serializes the
// result of `services::invitaciones::listar` directly as JSON. Since the
// service returns `Vec<InvitacionResponse>`, the HTTP body is a JSON array
// (`[]` when empty). The frontend expects `PaginatedResponse<Invitacion>` —
// an object with `{ data, total, page, perPage }`. Deserializing `[]` as a
// 4-field struct yields the serde error:
//   "invalid length 0, expected struct PaginatedResponse with 4 elements"
//
// GOAL: Show the empty list serializes as a bare array, not a PaginatedResponse.
// EXPECTED OUTCOME: Test FAILS — body is `[]`, not `{ data: [], total: 0, ... }`
#![allow(clippy::needless_return)]

use proptest::prelude::*;
use proptest::test_runner::{Config as ProptestConfig, TestRunner};
use serde::Deserialize;

use crate::common;

// ── Deserialization target (mirrors what the frontend expects) ───────────

/// The frontend deserializes the response as this struct.
/// If the backend returns a bare array `[]`, this will fail to parse.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct PaginatedResponseShape {
    data: Vec<serde_json::Value>,
    total: u64,
    page: u64,
    per_page: u64,
}

// ── Strategy ────────────────────────────────────────────────────────────

/// The bug condition scope: `{invitations dataset = empty}` for the org.
/// We generate org IDs representing orgs with zero invitations to demonstrate
/// the shape mismatch.
fn org_id_strategy() -> impl Strategy<Value = uuid::Uuid> {
    any::<[u8; 16]>().prop_map(|bytes| uuid::Uuid::from_bytes(bytes))
}

// ── Property Test ───────────────────────────────────────────────────────

/// Property 11: Bug Condition — Empty Invitaciones returns a well-formed PaginatedResponse.
///
/// Models the handler behavior: `services::invitaciones::listar` returns a
/// `Vec<InvitacionResponse>`. The handler serializes it directly with
/// `HttpResponse::Ok().json(result)`. When the list is empty, the JSON body is `[]`.
///
/// The frontend tries to deserialize this as `PaginatedResponse { data, total, page, perPage }`.
/// This test asserts that the serialized empty result IS a valid PaginatedResponse —
/// which FAILS on unfixed code because `[]` is not an object with 4 fields.
#[test]
fn property_11_empty_invitaciones_is_well_formed_paginated_response() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::pbt_cases(),
        ..ProptestConfig::default()
    });

    runner
        .run(&org_id_strategy(), |_org_id| {
            // Model: the service returns an empty Vec when there are no invitations.
            // The handler serializes it directly: HttpResponse::Ok().json(vec![])
            let empty_result: Vec<serde_json::Value> = vec![];
            let serialized =
                serde_json::to_string(&empty_result).expect("Serialization should not fail");

            // Frontend attempts to deserialize as PaginatedResponse
            let parse_result: Result<PaginatedResponseShape, _> =
                serde_json::from_str(&serialized);

            // BUG CONDITION ASSERTION:
            // The response SHOULD be a well-formed PaginatedResponse with data=[], total=0.
            // On UNFIXED code, this fails because `[]` cannot be deserialized as the struct.
            prop_assert!(
                parse_result.is_ok(),
                "Empty invitaciones response should be a well-formed PaginatedResponse \
                 with data=[] and total=0, but got deserialization error: {:?}. \
                 Actual response body: {}",
                parse_result.err(),
                serialized
            );

            // If parsing succeeds, verify the shape
            if let Ok(paginated) = parse_result {
                prop_assert_eq!(
                    paginated.data.len(),
                    0,
                    "Expected empty data array in PaginatedResponse"
                );
                prop_assert_eq!(paginated.total, 0, "Expected total=0 for empty dataset");
            }

            Ok(())
        })
        .expect(
            "Property 11 failed: empty invitaciones response is not a well-formed PaginatedResponse",
        );
}

/// Integration test: exercises the full service + handler path against a real database.
/// Creates an org with zero invitations, calls `listar`, serializes the result the
/// same way the handler does (`serde_json::to_string`), and attempts to deserialize
/// as `PaginatedResponse`. On UNFIXED code, the deserialization FAILS because the
/// body is `[]`, not `{ data, total, page, perPage }`.
#[test]
fn property_11_integration_empty_invitaciones_response_shape() {
    common::with_db(|db| async move {
        use chrono::Utc;
        use realestate_backend::entities::organizacion;
        use realestate_backend::services::invitaciones;
        use sea_orm::{ActiveModelTrait, EntityTrait, Set};
        use uuid::Uuid;

        // Create an organization with no invitations
        let org_id = Uuid::new_v4();
        let now = Utc::now().into();
        organizacion::ActiveModel {
            id: Set(org_id),
            tipo: Set("propietario".to_string()),
            nombre: Set(format!("Invitaciones PBT Org {org_id}")),
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

        // Call the service the same way the handler does
        let result = invitaciones::listar(&db, org_id)
            .await
            .expect("listar should not error on empty dataset");

        // The handler does: HttpResponse::Ok().json(result)
        // This serializes to a JSON array, not a PaginatedResponse object.
        let serialized = serde_json::to_string(&result).expect("Serialization should not fail");

        // Frontend attempts to deserialize as PaginatedResponse
        let parse_result: Result<PaginatedResponseShape, _> = serde_json::from_str(&serialized);

        // BUG CONDITION: On unfixed code, this FAILS.
        // The response is `[]` (empty array), not `{"data":[],"total":0,"page":1,"perPage":20}`
        assert!(
            parse_result.is_ok(),
            "Empty invitaciones response should be a well-formed PaginatedResponse, \
             but deserialization failed: {:?}. \
             Actual JSON body: '{}'. \
             Frontend error: 'invalid length 0, expected struct PaginatedResponse with 4 elements'",
            parse_result.err(),
            serialized
        );

        if let Ok(paginated) = parse_result {
            assert_eq!(paginated.data.len(), 0, "Expected empty data array");
            assert_eq!(paginated.total, 0, "Expected total=0");
        }

        // Cleanup
        let _ = organizacion::Entity::delete_by_id(org_id).exec(&db).await;
    });
}
