//! Integration tests for the chatbot evals API endpoints.
//!
//! These tests are gated behind the `evals` feature flag since the endpoints
//! are only compiled when that feature is enabled.

use chrono::Utc;
use uuid::Uuid;

use realestate_backend::services::auth::{Claims, encode_jwt};

use crate::common::JWT_SECRET;

/// Creates a JWT token for a user with write access (gerente role) in the given org.
fn make_token_for_org(org_id: Uuid) -> String {
    let claims = Claims {
        sub: Uuid::new_v4(),
        email: format!("eval-test+{}@example.com", Uuid::new_v4()),
        rol: "gerente".to_string(),
        organizacion_id: org_id,
        jti: Uuid::new_v4(),
        iat: Utc::now().timestamp(),
        exp: (Utc::now() + chrono::Duration::hours(1)).timestamp() as usize,
        iss: "realestate-api".to_string(),
        aud: "realestate-api".to_string(),
    };
    encode_jwt(&claims, JWT_SECRET).unwrap()
}

#[test]
fn evals_create_suite_returns_201() {
    crate::common::with_db(|db| async move {
        use actix_web::test;
        use realestate_backend::app::create_app;
        use realestate_backend::services::ocr_preview::PreviewStore;
        use serde_json::{Value, json};

        let app = test::init_service(create_app(
            db.clone(),
            crate::common::test_app_config(crate::common::db_url()),
            actix_web::web::Data::new(PreviewStore::new()),
        ))
        .await;

        let org_id = Uuid::new_v4();
        let token = make_token_for_org(org_id);

        let req = test::TestRequest::post()
            .uri("/api/v1/chatbot/evals/suites")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(json!({
                "name": "Balance Query Accuracy",
                "description": "Tests for balance query responses",
                "cases": [
                    {"id": "case-1", "input": "¿Cuánto debo?", "expected": "balance info", "tags": ["balance"]}
                ],
                "metrics": ["factuality", "relevance"]
            }))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(
            resp.status(),
            201,
            "suite creation should return 201 Created"
        );

        let body: Value = test::read_body_json(resp).await;
        assert!(body["id"].is_string(), "response must have 'id'");
        assert_eq!(body["name"], "Balance Query Accuracy");
        assert_eq!(body["organizacionId"], org_id.to_string());
    });
}

#[test]
fn evals_list_suites_returns_only_org_suites() {
    crate::common::with_db(|db| async move {
        use actix_web::test;
        use realestate_backend::app::create_app;
        use realestate_backend::services::ocr_preview::PreviewStore;
        use serde_json::{Value, json};

        let app = test::init_service(create_app(
            db.clone(),
            crate::common::test_app_config(crate::common::db_url()),
            actix_web::web::Data::new(PreviewStore::new()),
        ))
        .await;

        let org_a = Uuid::new_v4();
        let org_b = Uuid::new_v4();
        let token_a = make_token_for_org(org_a);
        let token_b = make_token_for_org(org_b);

        // Create a suite for org A
        let req = test::TestRequest::post()
            .uri("/api/v1/chatbot/evals/suites")
            .insert_header(("Authorization", format!("Bearer {token_a}")))
            .set_json(json!({
                "name": "Org A Suite",
                "cases": [],
                "metrics": []
            }))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 201);

        // Create a suite for org B
        let req = test::TestRequest::post()
            .uri("/api/v1/chatbot/evals/suites")
            .insert_header(("Authorization", format!("Bearer {token_b}")))
            .set_json(json!({
                "name": "Org B Suite",
                "cases": [],
                "metrics": []
            }))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 201);

        // List suites for org A — should only see org A's suite
        let req = test::TestRequest::get()
            .uri("/api/v1/chatbot/evals/suites")
            .insert_header(("Authorization", format!("Bearer {token_a}")))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let body: Value = test::read_body_json(resp).await;
        let suites = body.as_array().expect("response should be an array");

        // All returned suites must belong to org A
        for suite in suites {
            assert_eq!(
                suite["organizacionId"],
                org_a.to_string(),
                "listed suite must belong to requesting org"
            );
        }

        // At least one suite should be present
        assert!(
            suites.iter().any(|s| s["name"] == "Org A Suite"),
            "org A's suite should be in the list"
        );

        // Org B's suite must NOT appear
        assert!(
            !suites.iter().any(|s| s["name"] == "Org B Suite"),
            "org B's suite must not appear in org A's listing"
        );
    });
}

#[test]
fn evals_run_with_invalid_suite_id_returns_404() {
    crate::common::with_db(|db| async move {
        use actix_web::test;
        use realestate_backend::app::create_app;
        use realestate_backend::services::ocr_preview::PreviewStore;
        use serde_json::json;

        let app = test::init_service(create_app(
            db.clone(),
            crate::common::test_app_config(crate::common::db_url()),
            actix_web::web::Data::new(PreviewStore::new()),
        ))
        .await;

        let org_id = Uuid::new_v4();
        let token = make_token_for_org(org_id);
        let fake_suite_id = Uuid::new_v4();

        let req = test::TestRequest::post()
            .uri("/api/v1/chatbot/evals/run")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(json!({
                "suiteId": fake_suite_id.to_string()
            }))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(
            resp.status(),
            404,
            "running eval with non-existent suite_id should return 404"
        );
    });
}

#[test]
fn evals_list_runs_returns_only_org_runs() {
    crate::common::with_db(|db| async move {
        use actix_web::test;
        use realestate_backend::app::create_app;
        use realestate_backend::services::ocr_preview::PreviewStore;
        use serde_json::Value;

        let app = test::init_service(create_app(
            db.clone(),
            crate::common::test_app_config(crate::common::db_url()),
            actix_web::web::Data::new(PreviewStore::new()),
        ))
        .await;

        let org_id = Uuid::new_v4();
        let token = make_token_for_org(org_id);

        // List runs for a fresh org — should be empty
        let req = test::TestRequest::get()
            .uri("/api/v1/chatbot/evals/runs")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let body: Value = test::read_body_json(resp).await;
        let runs = body.as_array().expect("response should be an array");

        // All returned runs must belong to the requesting org
        for run in runs {
            assert_eq!(
                run["organizacionId"],
                org_id.to_string(),
                "listed run must belong to requesting org"
            );
        }
    });
}
