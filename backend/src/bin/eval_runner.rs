use anyhow::Context;
use clap::Parser;
use sea_orm::{Database, EntityTrait};
use uuid::Uuid;

use realestate_backend::config::ChatbotEnvConfig;
use realestate_backend::entities::chatbot_eval_run;
use realestate_backend::entities::chatbot_eval_suite;
use realestate_backend::services::ai_module::AiModule;
use realestate_backend::services::ai_module::evals::{
    EvalRunConfig, EvalRunResult, EvalRunner, EvalSuite,
};

#[derive(Parser, Debug)]
#[command(name = "eval-runner", about = "Run chatbot eval suites")]
struct EvalRunnerArgs {
    #[arg(long)]
    suite_id: Uuid,

    #[arg(long, default_value_t = 3)]
    concurrency: usize,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let args = EvalRunnerArgs::parse();

    dotenvy::dotenv().ok();

    let database_url = std::env::var("DATABASE_URL").context("DATABASE_URL no está configurado")?;

    let db = Database::connect(&database_url)
        .await
        .context("Error conectando a la base de datos")?;

    let config =
        ChatbotEnvConfig::from_env().context("Error cargando configuración del chatbot")?;

    let suite = load_suite(&db, args.suite_id).await?;
    let ai_module = AiModule::new(&config)?;

    let eval_config = EvalRunConfig {
        concurrency: args.concurrency,
        agent_config_override: None,
    };

    let runner = EvalRunner::new(&ai_module, &db);
    let results = runner.run_suite(&suite, &eval_config).await;

    persist_results(&db, &results).await?;

    println!("Pass rate: {:.1}%", results.summary.pass_rate * 100.0);
    Ok(())
}

async fn load_suite(db: &sea_orm::DatabaseConnection, suite_id: Uuid) -> anyhow::Result<EvalSuite> {
    let entity = chatbot_eval_suite::Entity::find_by_id(suite_id)
        .one(db)
        .await
        .context("Error consultando eval suite")?
        .context("Eval suite no encontrada")?;

    let cases = serde_json::from_value(entity.cases)
        .context("Error deserializando cases del eval suite")?;

    let metrics = serde_json::from_value(entity.metrics)
        .context("Error deserializando metrics del eval suite")?;

    Ok(EvalSuite {
        id: entity.id,
        organizacion_id: entity.organizacion_id,
        name: entity.name,
        description: entity.description,
        cases,
        metrics,
    })
}

async fn persist_results(
    db: &sea_orm::DatabaseConnection,
    results: &EvalRunResult,
) -> anyhow::Result<()> {
    use sea_orm::ActiveModelTrait;
    use sea_orm::Set;

    let active_model = chatbot_eval_run::ActiveModel {
        id: Set(results.id),
        suite_id: Set(results.suite_id),
        organizacion_id: Set(results.organizacion_id),
        status: Set(results.status.clone()),
        results: Set(Some(
            serde_json::to_value(&results.results).context("Error serializando results")?,
        )),
        summary: Set(Some(
            serde_json::to_value(&results.summary).context("Error serializando summary")?,
        )),
        agent_config_snapshot: Set(results.agent_config_snapshot.clone()),
        started_at: Set(results.started_at.into()),
        completed_at: Set(results.completed_at.map(Into::into)),
    };

    active_model
        .insert(db)
        .await
        .context("Error persistiendo eval run")?;

    Ok(())
}
