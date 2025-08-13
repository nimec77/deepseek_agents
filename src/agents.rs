use std::path::PathBuf;

use async_trait::async_trait;
use chrono::Utc;
use serde_json::json;
use tokio::fs;
use tracing::info;

use crate::deepseek::{ChatMessage, DeepSeekClient};
use crate::types::{SolutionV1, TaskSpec, ValidationV1};

#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    #[error(transparent)]
    Llm(#[from] crate::deepseek::DeepSeekError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Serde(#[from] serde_json::Error),
    #[error("Unexpected: {0}")]
    Unexpected(String),
}

#[async_trait]
pub trait Agent {
    type Input: Send + Sync;
    type Output: Send + Sync;
    async fn execute(&self, input: &Self::Input) -> Result<Self::Output, AgentError>;
}

#[derive(Clone)]
pub struct ProducerAgent {
    client: DeepSeekClient,
    out_path: PathBuf,
}

impl ProducerAgent {
    pub fn new(client: DeepSeekClient, out_path: PathBuf) -> Self {
        Self { client, out_path }
    }
}

#[async_trait]
impl Agent for ProducerAgent {
    type Input = TaskSpec;
    type Output = SolutionV1;

    async fn execute(&self, task: &Self::Input) -> Result<Self::Output, AgentError> {
        info!("ProducerAgent: preparing output directory at {}", self.out_path.display());
        fs::create_dir_all(
            self.out_path
                .parent()
                .ok_or_else(|| AgentError::Unexpected("invalid output path".into()))?,
        )
        .await?;

        // System prompt: strict JSON SolutionV1
        let system_prompt = r#"
You are Agent 1. Produce a solution strictly as JSON matching the schema below. Do not add commentary or markdown. Output ONLY a JSON object.

Descriptions in the schema indicate expected data and type; replace them with actual values in your output.

Schema (SolutionV1):
{
  "schema_version": "Schema version identifier; must be 'solution_v1' (string)",
  "task_id": "Identifier of the task being solved (string)",
  "solution_id": "Unique identifier for this solution (string)",
  "model_used": {
    "name": "Model name used to generate the solution, e.g., 'deepseek-chat' (string)",
    "temperature": "Sampling temperature used for generation (number)"
  },
  "deliverable_type": "Type of deliverable: 'text' | 'json' | 'code' (string)",
  "deliverable": {
    "text": "Plain text content if deliverable_type='text' (string or null)",
    "json": "JSON content if deliverable_type='json' (object/array/value or null)",
    "code": {
      "language": "Programming language for the code deliverable, e.g., 'rs', 'py' (string)",
      "content": "Source code content if deliverable_type='code' (string)"
    }
  },
  "evidence": {
    "system_prompt": "Truncated copy of the system prompt used (string)",
    "usage_note": "Optional notes about generation context or constraints (string or null)"
  },
  "usage": {
    "prompt_tokens": "Number of prompt tokens consumed (integer)",
    "completion_tokens": "Number of completion tokens generated (integer)"
  },
  "created_at": "Creation timestamp in RFC3339 format, UTC (string)"
}
"#;

        let user_payload = json!({
            "task_spec": task,
            "instructions": "Use the deliverable_type from TaskSpec. Populate created_at with current time. Ensure only one of deliverable.text/json/code is present as per deliverable_type."
        });

        let messages = vec![
            ChatMessage { role: "system".to_string(), content: system_prompt.to_string() },
            ChatMessage { role: "user".to_string(), content: user_payload.to_string() },
        ];

        info!("ProducerAgent: sending task {} to LLM", task.task_id);
        let raw = self.client.send_messages_raw(messages).await?;
        info!("ProducerAgent: received model response, parsing JSON");
        let mut solution: SolutionV1 = serde_json::from_str(&raw)?;

        // Ensure schema_version and timestamps if model forgot
        if solution.schema_version.is_empty() {
            solution.schema_version = "solution_v1".to_string();
        }
        if solution.created_at.trim().is_empty() {
            solution.created_at = Utc::now().to_rfc3339();
        }

        // Persist
        let pretty = serde_json::to_string_pretty(&solution)?;
        fs::write(&self.out_path, pretty).await?;
        info!(
            "ProducerAgent: saved solution {} to {}",
            solution.solution_id,
            self.out_path.display()
        );
        Ok(solution)
    }
}

#[derive(Clone)]
pub struct AuditorAgent {
    client: DeepSeekClient,
    out_path: PathBuf,
}

impl AuditorAgent {
    pub fn new(client: DeepSeekClient, out_path: PathBuf) -> Self {
        Self { client, out_path }
    }
}

pub struct AuditInput {
    pub task: TaskSpec,
    pub solution: SolutionV1,
}

#[async_trait]
impl Agent for AuditorAgent {
    type Input = AuditInput;
    type Output = ValidationV1;

    async fn execute(&self, input: &Self::Input) -> Result<Self::Output, AgentError> {
        info!(
            "AuditorAgent: preparing output directory at {}",
            self.out_path.display()
        );
        fs::create_dir_all(
            self.out_path
                .parent()
                .ok_or_else(|| AgentError::Unexpected("invalid output path".into()))?,
        )
        .await?;

        let system_prompt = r#"
You are Agent 2. Given TaskSpec and a SolutionV1, grade it strictly against acceptance_criteria. Output ONLY JSON matching ValidationV1.

Descriptions in the schema indicate expected data and type; replace them with actual values in your output.

Schema (ValidationV1):
{
  "schema_version": "Schema version identifier; must be 'validation_v1' (string)",
  "task_id": "Identifier of the task being validated (string)",
  "solution_id": "Identifier of the solution under review (string)",
  "verdict": "Overall result: 'pass' | 'warn' | 'fail' (string)",
  "score": "Normalized score in [0.0, 1.0] reflecting quality/compliance (number)",
  "checks": [
    {
      "criterion": "Acceptance criterion being assessed (string)",
      "pass": "Whether this criterion passed (boolean)",
      "reason": "Explanation for the outcome (string)",
      "severity": "Impact level if failing: 'minor' | 'major' (string)",
      "suggested_fix": "Optional suggestion to remediate a failure (string or null)"
    }
  ],
  "suggested_rewrite": "Optional repaired content or structured fix (any JSON value or null)",
  "model_used": {
    "name": "Model name used for auditing, e.g., 'deepseek-reasoner' (string)",
    "temperature": "Sampling temperature used for validation (number)"
  },
  "created_at": "Creation timestamp in RFC3339 format, UTC (string)"
}
"#;

        let user_payload = json!({
            "task_spec": input.task,
            "solution": input.solution,
            "instructions": "Include one check per acceptance_criteria item. Set verdict and a score in [0.0, 1.0]."
        });

        let messages = vec![
            ChatMessage { role: "system".to_string(), content: system_prompt.to_string() },
            ChatMessage { role: "user".to_string(), content: user_payload.to_string() },
        ];

        info!(
            "AuditorAgent: auditing solution {} for task {}",
            input.solution.solution_id,
            input.task.task_id
        );
        let raw = self.client.send_messages_raw(messages).await?;
        info!("AuditorAgent: received model response, parsing JSON");
        let mut validation: ValidationV1 = serde_json::from_str(&raw)?;
        if validation.schema_version.is_empty() {
            validation.schema_version = "validation_v1".to_string();
        }
        if validation.created_at.trim().is_empty() {
            validation.created_at = Utc::now().to_rfc3339();
        }

        let pretty = serde_json::to_string_pretty(&validation)?;
        fs::write(&self.out_path, pretty).await?;
        info!(
            "AuditorAgent: saved validation for solution {} to {}",
            validation.solution_id,
            self.out_path.display()
        );
        Ok(validation)
    }
}


