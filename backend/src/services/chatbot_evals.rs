use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
};
use uuid::Uuid;

use crate::entities::{chatbot_eval_run, chatbot_eval_suite};
use crate::errors::AppError;

pub async fn list_suites(
    db: &DatabaseConnection,
    org_id: Uuid,
) -> Result<Vec<chatbot_eval_suite::Model>, AppError> {
    chatbot_eval_suite::Entity::find()
        .filter(chatbot_eval_suite::Column::OrganizacionId.eq(org_id))
        .order_by_desc(chatbot_eval_suite::Column::CreatedAt)
        .all(db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Error listando eval suites: {e}")))
}

pub async fn create_suite(
    db: &DatabaseConnection,
    org_id: Uuid,
    name: String,
    description: Option<String>,
    cases: serde_json::Value,
    metrics: serde_json::Value,
) -> Result<chatbot_eval_suite::Model, AppError> {
    let now = Utc::now().into();
    let id = Uuid::new_v4();

    let model = chatbot_eval_suite::ActiveModel {
        id: Set(id),
        organizacion_id: Set(org_id),
        name: Set(name),
        description: Set(description),
        cases: Set(cases),
        metrics: Set(metrics),
        created_at: Set(now),
        updated_at: Set(now),
    };

    model
        .insert(db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Error creando eval suite: {e}")))
}

pub async fn find_suite(
    db: &DatabaseConnection,
    suite_id: Uuid,
    org_id: Uuid,
) -> Result<chatbot_eval_suite::Model, AppError> {
    chatbot_eval_suite::Entity::find_by_id(suite_id)
        .filter(chatbot_eval_suite::Column::OrganizacionId.eq(org_id))
        .one(db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Error buscando eval suite: {e}")))?
        .ok_or_else(|| {
            AppError::NotFound("Eval suite no encontrada o no pertenece a esta organización".into())
        })
}

pub async fn create_run(
    db: &DatabaseConnection,
    run_id: Uuid,
    suite_id: Uuid,
    org_id: Uuid,
    agent_config_snapshot: serde_json::Value,
) -> Result<chatbot_eval_run::Model, AppError> {
    let now = Utc::now().into();

    let model = chatbot_eval_run::ActiveModel {
        id: Set(run_id),
        suite_id: Set(suite_id),
        organizacion_id: Set(org_id),
        status: Set("running".to_string()),
        results: Set(None),
        summary: Set(None),
        agent_config_snapshot: Set(agent_config_snapshot),
        started_at: Set(now),
        completed_at: Set(None),
    };

    model
        .insert(db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Error creando eval run: {e}")))
}

pub async fn complete_run(
    db: &DatabaseConnection,
    run_id: Uuid,
    results: Option<serde_json::Value>,
    summary: Option<serde_json::Value>,
) -> Result<chatbot_eval_run::Model, AppError> {
    let completed_at = Utc::now().into();

    let model = chatbot_eval_run::ActiveModel {
        id: Set(run_id),
        status: Set("completed".to_string()),
        results: Set(results),
        summary: Set(summary),
        completed_at: Set(Some(completed_at)),
        ..Default::default()
    };

    model
        .update(db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Error actualizando eval run: {e}")))
}

pub async fn list_runs(
    db: &DatabaseConnection,
    org_id: Uuid,
) -> Result<Vec<chatbot_eval_run::Model>, AppError> {
    chatbot_eval_run::Entity::find()
        .filter(chatbot_eval_run::Column::OrganizacionId.eq(org_id))
        .order_by_desc(chatbot_eval_run::Column::StartedAt)
        .all(db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Error listando eval runs: {e}")))
}

pub async fn find_run(
    db: &DatabaseConnection,
    run_id: Uuid,
    org_id: Uuid,
) -> Result<chatbot_eval_run::Model, AppError> {
    chatbot_eval_run::Entity::find_by_id(run_id)
        .filter(chatbot_eval_run::Column::OrganizacionId.eq(org_id))
        .one(db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Error buscando eval run: {e}")))?
        .ok_or_else(|| {
            AppError::NotFound("Eval run no encontrado o no pertenece a esta organización".into())
        })
}
