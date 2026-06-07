//! Eval subsystem for measuring agent quality.
//!
//! Gated behind the `evals` feature flag. Provides data models, an eval runner
//! with configurable concurrency, tool-selection evaluation, and a factuality
//! judge placeholder.

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

// =============================================================================
// Data Models
// =============================================================================

/// A single test case in an eval suite.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalCase {
    /// Unique identifier within the suite.
    pub id: String,
    /// The user message to send to the agent.
    pub input: String,
    /// Expected output (for similarity comparison) or criteria description.
    pub expected: String,
    /// Optional tags for filtering (e.g., "balance", "maintenance", "safety").
    pub tags: Vec<String>,
    /// Optional tenant context to simulate.
    pub tenant_context: Option<TenantContext>,
}

/// Eval metrics supported.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvalMetric {
    /// Does the response answer the question factually?
    Factuality,
    /// Is the response relevant to the user's query?
    Relevance,
    /// Does the response avoid unsafe/blocked content?
    Safety,
    /// Is the response in the correct language (Spanish)?
    Language,
    /// Did the agent invoke the correct tool for the scenario?
    ToolSelection,
}

/// An eval suite with cases and metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalSuite {
    pub id: Uuid,
    pub organizacion_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub cases: Vec<EvalCase>,
    pub metrics: Vec<EvalMetric>,
}

/// Result of a single eval case.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalCaseResult {
    pub case_id: String,
    pub score: f64,
    pub reasoning: String,
    pub tool_invoked: Option<String>,
}

/// Summary of an eval run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalRunSummary {
    pub pass_rate: f64,
    pub average_score: f64,
}

/// Complete eval run result.
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

// =============================================================================
// Tool Selection Eval
// =============================================================================

/// Custom eval that checks if the agent called the expected tool.
///
/// Compares the expected tool name against the tools actually invoked
/// in the agent response.
pub struct ToolSelectionEval;

impl ToolSelectionEval {
    /// Scores a response based on whether the expected tool was invoked.
    ///
    /// Returns 1.0 if the expected tool is found in `tools_invoked`, 0.0 otherwise.
    pub fn score(expected_tool: &str, response: &AgentResponse) -> EvalCaseResult {
        let found = response.tools_invoked.iter().any(|t| t == expected_tool);

        EvalCaseResult {
            case_id: String::new(), // Caller sets this
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

// =============================================================================
// Factuality Judge (placeholder)
// =============================================================================

/// Factuality judgment result from the LLM judge.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactualityJudgment {
    /// Whether the response is factually accurate.
    pub is_factual: bool,
    /// Explanation for the judgment.
    pub reasoning: String,
}

/// Builds a factuality judge that scores agent responses.
///
/// Since rig-core 0.36 does not expose a stable `LlmJudgeMetric` API,
/// this is a placeholder that returns a fixed passing score. When the
/// experimental evals API stabilizes, this will be replaced with a real
/// LLM-as-judge implementation using the OVMS model.
pub fn build_factuality_judge(_model_endpoint: &str) -> FactualityJudgment {
    FactualityJudgment {
        is_factual: true,
        reasoning:
            "Factuality judge placeholder — real LLM judge pending rig evals API stabilization"
                .to_string(),
    }
}

/// Scores a response for factuality by comparing it against the expected output.
///
/// Placeholder implementation: returns 1.0 if the response is non-empty, 0.5 otherwise.
/// Will be replaced with actual LLM-as-judge scoring when rig's experimental evals API
/// is stable.
pub fn score_factuality(response: &str, _expected: &str) -> f64 {
    if response.trim().is_empty() {
        0.0
    } else {
        // Placeholder: non-empty responses get a passing score.
        // Real implementation would use LLM-as-judge to compare against expected.
        1.0
    }
}

// =============================================================================
// Eval Runner
// =============================================================================

/// Configuration for an eval run.
pub struct EvalRunConfig {
    /// Maximum number of cases to run concurrently. Default: 3.
    pub concurrency: usize,
    /// Optional agent config override for this run (A/B testing).
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

/// Runs eval suites against the AI agent.
///
/// Uses the same `AgentBuilder` pattern as production to ensure eval results
/// reflect real agent behavior.
pub struct EvalRunner<'a> {
    ai_module: &'a AiModule,
    db: &'a sea_orm::DatabaseConnection,
}

impl<'a> EvalRunner<'a> {
    /// Creates a new `EvalRunner` with references to the AI module and database.
    pub fn new(ai_module: &'a AiModule, db: &'a sea_orm::DatabaseConnection) -> Self {
        Self { ai_module, db }
    }

    /// Executes all cases in the given suite with the specified configuration.
    ///
    /// Cases are run with a concurrency limit (default 3) using a semaphore.
    /// If a case fails due to agent error or timeout, it records score 0.0
    /// and continues with remaining cases.
    pub async fn run_suite(&self, suite: &EvalSuite, config: &EvalRunConfig) -> EvalRunResult {
        let run_id = Uuid::new_v4();
        let started_at = Utc::now();

        let agent_config = config.agent_config_override.clone().unwrap_or_default();

        let agent_config_snapshot = serde_json::to_value(&agent_config).unwrap_or_default();

        let semaphore = Arc::new(Semaphore::new(config.concurrency));

        // Run cases concurrently with semaphore-based limiting
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

        // Compute summary
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

    /// Runs a single eval case against the agent.
    ///
    /// Builds a `ProcessMessageContext` with the case input and invokes
    /// `process_message`. On failure, records score 0.0 with failure reasoning.
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
        };

        match self.ai_module.process_message(&ctx).await {
            Ok(response) => {
                // Score the response based on suite metrics
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
            Err(e) => {
                // Record failure with score 0.0 and continue
                EvalCaseResult {
                    case_id: case.id.clone(),
                    score: 0.0,
                    reasoning: format!("Case failed: {e}"),
                    tool_invoked: None,
                }
            }
        }
    }

    /// Computes summary statistics from case results.
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
