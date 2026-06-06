//! Eval API endpoints for the chatbot subsystem.
//!
//! All handlers are gated behind the `evals` feature flag.

use actix_web::{HttpResponse, web};
use sea_orm::DatabaseConnection;
use serde::Deserialize;
use uuid::Uuid;

use crate::config::AppConfig;
use crate::errors::AppError;
use crate::middleware::rbac::WriteAccess;
use crate::models::chatbot::AgentConfig;
use crate::services::ai_module::AiModule;
use crate::services::ai_module::evals::{EvalRunConfig, EvalRunner, EvalSuite};
use crate::services::chatbot_evals;

// =============================================================================
// Request / Response DTOs
// =============================================================================

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateEvalSuiteRequest {
    pub name: String,
    pub description: Option<String>,
    pub cases: serde_json::Value,
    pub metrics: serde_json::Value,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunEvalRequest {
    pub suite_id: Uuid,
    pub agent_config_override: Option<AgentConfig>,
    pub concurrency: Option<usize>,
}

// =============================================================================
// Handlers
// =============================================================================

/// GET `/api/v1/chatbot/evals/suites` — list eval suites for the requesting user's org.
pub async fn list_suites(
    db: web::Data<DatabaseConnection>,
    claims: WriteAccess,
) -> Result<HttpResponse, AppError> {
    let suites = chatbot_evals::list_suites(db.get_ref(), claims.0.organizacion_id).await?;
    Ok(HttpResponse::Ok().json(suites))
}

/// POST `/api/v1/chatbot/evals/suites` — create a new eval suite.
pub async fn create_suite(
    db: web::Data<DatabaseConnection>,
    claims: WriteAccess,
    body: web::Json<CreateEvalSuiteRequest>,
) -> Result<HttpResponse, AppError> {
    let request = body.into_inner();

    if request.name.is_empty() || request.name.len() > 100 {
        return Err(AppError::Validation(
            "El nombre debe tener entre 1 y 100 caracteres".to_string(),
        ));
    }

    let inserted = chatbot_evals::create_suite(
        db.get_ref(),
        claims.0.organizacion_id,
        request.name,
        request.description,
        request.cases,
        request.metrics,
    )
    .await?;

    Ok(HttpResponse::Created().json(inserted))
}

/// POST `/api/v1/chatbot/evals/run` — run an eval suite.
///
/// Validates that the suite_id belongs to the requesting org before executing.
pub async fn run_eval(
    db: web::Data<DatabaseConnection>,
    app_config: web::Data<AppConfig>,
    claims: WriteAccess,
    body: web::Json<RunEvalRequest>,
) -> Result<HttpResponse, AppError> {
    let org_id = claims.0.organizacion_id;
    let request = body.into_inner();

    // Validate suite exists and belongs to org
    let suite_model = chatbot_evals::find_suite(db.get_ref(), request.suite_id, org_id).await?;

    // Parse cases from JSONB
    let cases: Vec<crate::services::ai_module::evals::EvalCase> =
        serde_json::from_value(suite_model.cases.clone()).map_err(|e| {
            AppError::Internal(anyhow::anyhow!("Error parseando casos del suite: {e}"))
        })?;

    let metrics: Vec<crate::services::ai_module::evals::EvalMetric> =
        serde_json::from_value(suite_model.metrics.clone()).map_err(|e| {
            AppError::Internal(anyhow::anyhow!("Error parseando métricas del suite: {e}"))
        })?;

    let suite = EvalSuite {
        id: suite_model.id,
        organizacion_id: suite_model.organizacion_id,
        name: suite_model.name,
        description: suite_model.description,
        cases,
        metrics,
    };

    let eval_config = EvalRunConfig {
        concurrency: request.concurrency.unwrap_or(3).clamp(1, 10),
        agent_config_override: request.agent_config_override,
    };

    // Build AI module
    let chatbot_env = &app_config.chatbot;
    let ai_module = AiModule::new(chatbot_env)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Error inicializando AI module: {e}")))?;

    // Create the run record with status "running"
    let run_id = Uuid::new_v4();
    let agent_config_snapshot = serde_json::to_value(
        eval_config
            .agent_config_override
            .as_ref()
            .unwrap_or(&AgentConfig::default()),
    )
    .unwrap_or_default();

    chatbot_evals::create_run(db.get_ref(), run_id, suite.id, org_id, agent_config_snapshot)
        .await?;

    // Execute the eval suite
    let runner = EvalRunner::new(&ai_module, db.get_ref());
    let result = runner.run_suite(&suite, &eval_config).await;

    // Update the run record with results
    let results_json = serde_json::to_value(&result.results).ok();
    let summary_json = serde_json::to_value(&result.summary).ok();

    chatbot_evals::complete_run(db.get_ref(), run_id, results_json, summary_json).await?;

    Ok(HttpResponse::Ok().json(result))
}

/// GET `/api/v1/chatbot/evals/runs` — list eval runs for the org.
pub async fn list_runs(
    db: web::Data<DatabaseConnection>,
    claims: WriteAccess,
) -> Result<HttpResponse, AppError> {
    let runs = chatbot_evals::list_runs(db.get_ref(), claims.0.organizacion_id).await?;
    Ok(HttpResponse::Ok().json(runs))
}

/// GET `/api/v1/chatbot/evals/runs/{id}` — get specific run results.
pub async fn get_run(
    db: web::Data<DatabaseConnection>,
    claims: WriteAccess,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let run =
        chatbot_evals::find_run(db.get_ref(), path.into_inner(), claims.0.organizacion_id).await?;
    Ok(HttpResponse::Ok().json(run))
}
