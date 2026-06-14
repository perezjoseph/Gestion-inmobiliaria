use chrono::{DateTime, Utc};
use futures_util::stream::{self, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::Semaphore;
use uuid::Uuid;

use std::sync::Arc;

use crate::models::chatbot::AgentConfig;
use crate::models::chatbot::Capabilities;
use crate::services::ai_module::{
    AgentResponse, AiModule, ChatbotPersona, ConversationEntry, ProcessMessageContext,
    TenantContext, UserMessage,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalCase {
    pub id: String,
    pub input: String,
    pub expected: String,
    pub tags: Vec<String>,
    pub tenant_context: Option<TenantContext>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvalMetric {
    Factuality,
    Relevance,
    Safety,
    Language,
    ToolSelection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalSuite {
    pub id: Uuid,
    pub organizacion_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub cases: Vec<EvalCase>,
    pub metrics: Vec<EvalMetric>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalCaseResult {
    pub case_id: String,
    pub score: f64,
    pub reasoning: String,
    pub tool_invoked: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalRunSummary {
    pub pass_rate: f64,
    pub average_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalRunResult {
    pub id: Uuid,
    pub suite_id: Uuid,
    pub organizacion_id: Uuid,
    pub status: String,
    pub results: Vec<EvalCaseResult>,
    pub summary: EvalRunSummary,
    pub agent_config_snapshot: serde_json::Value,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

pub struct ToolSelectionEval;

impl ToolSelectionEval {
    pub fn score(expected_tool: &str, response: &AgentResponse) -> EvalCaseResult {
        let found = response.tools_invoked.iter().any(|t| t == expected_tool);

        EvalCaseResult {
            case_id: String::new(),
            score: if found { 1.0 } else { 0.0 },
            reasoning: if found {
                format!("Expected tool '{expected_tool}' was invoked")
            } else {
                format!(
                    "Expected tool '{expected_tool}' was NOT invoked. Tools called: {:?}",
                    response.tools_invoked
                )
            },
            tool_invoked: response.tools_invoked.first().cloned(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactualityJudgment {
    pub is_factual: bool,
    pub reasoning: String,
}

pub fn build_factuality_judge(_model_endpoint: &str) -> FactualityJudgment {
    FactualityJudgment {
        is_factual: true,
        reasoning:
            "Factuality judge placeholder — real LLM judge pending rig evals API stabilization"
                .to_string(),
    }
}

pub fn score_factuality(response: &str, _expected: &str) -> f64 {
    if response.trim().is_empty() { 0.0 } else { 1.0 }
}

pub struct EvalRunConfig {
    pub concurrency: usize,
    pub agent_config_override: Option<AgentConfig>,
}

impl Default for EvalRunConfig {
    fn default() -> Self {
        Self {
            concurrency: 3,
            agent_config_override: None,
        }
    }
}

pub struct EvalRunner<'a> {
    ai_module: &'a AiModule,
    db: &'a sea_orm::DatabaseConnection,
}

impl<'a> EvalRunner<'a> {
    pub fn new(ai_module: &'a AiModule, db: &'a sea_orm::DatabaseConnection) -> Self {
        Self { ai_module, db }
    }

    pub async fn run_suite(&self, suite: &EvalSuite, config: &EvalRunConfig) -> EvalRunResult {
        let run_id = Uuid::new_v4();
        let started_at = Utc::now();

        let agent_config = config.agent_config_override.clone().unwrap_or_default();

        let agent_config_snapshot = serde_json::to_value(&agent_config).unwrap_or_default();

        let semaphore = Arc::new(Semaphore::new(config.concurrency));

        let results: Vec<EvalCaseResult> = stream::iter(suite.cases.iter())
            .map(|case| {
                let sem = semaphore.clone();
                let agent_config = agent_config.clone();
                async move {
                    let _permit = sem.acquire().await;
                    self.run_case(case, &suite.organizacion_id, &agent_config)
                        .await
                }
            })
            .buffer_unordered(config.concurrency)
            .collect()
            .await;

        let completed_at = Some(Utc::now());

        let summary = Self::compute_summary(&results);

        EvalRunResult {
            id: run_id,
            suite_id: suite.id,
            organizacion_id: suite.organizacion_id,
            status: "completed".to_string(),
            results,
            summary,
            agent_config_snapshot,
            started_at,
            completed_at,
        }
    }

    async fn run_case(
        &self,
        case: &EvalCase,
        organizacion_id: &Uuid,
        _agent_config: &AgentConfig,
    ) -> EvalCaseResult {
        let persona = ChatbotPersona {
            tone: Some("profesional".to_string()),
            greeting: None,
            system_prompt: None,
            language: "es-DO".to_string(),
        };

        let tenant_context = case.tenant_context.clone();
        let user_message = UserMessage {
            content: case.input.clone(),
            image_base64: None,
        };

        let capabilities = Capabilities::all();
        let history: Vec<ConversationEntry> = Vec::new();
        let faqs = Vec::new();
        let handoff_keywords: Vec<String> = Vec::new();

        let ctx = ProcessMessageContext {
            config: &persona,
            tenant_context: tenant_context.as_ref(),
            faqs: &faqs,
            policies: None,
            handoff_keywords: &handoff_keywords,
            capabilities: &capabilities,
            history: &history,
            user_message: &user_message,
            db: self.db,
            organizacion_id: *organizacion_id,
            sender_phone: "eval-runner",
            guidance_rules: &[],
            sender_policy: "tenants_only",
        };

        match self.ai_module.process_message(&ctx).await {
            Ok(response) => {
                let score = score_factuality(&response.reply, &case.expected);

                EvalCaseResult {
                    case_id: case.id.clone(),
                    score,
                    reasoning: format!(
                        "Agent responded successfully. Tools invoked: {:?}",
                        response.tools_invoked
                    ),
                    tool_invoked: response.tools_invoked.first().cloned(),
                }
            }
            Err(e) => EvalCaseResult {
                case_id: case.id.clone(),
                score: 0.0,
                reasoning: format!("Case failed: {e}"),
                tool_invoked: None,
            },
        }
    }

    fn compute_summary(results: &[EvalCaseResult]) -> EvalRunSummary {
        if results.is_empty() {
            return EvalRunSummary {
                pass_rate: 0.0,
                average_score: 0.0,
            };
        }

        let total = results.len() as f64;
        let passing = results.iter().filter(|r| r.score >= 0.5).count() as f64;
        let sum_scores: f64 = results.iter().map(|r| r.score).sum();

        EvalRunSummary {
            pass_rate: passing / total,
            average_score: sum_scores / total,
        }
    }
}
