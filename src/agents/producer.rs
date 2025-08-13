use std::path::PathBuf;

use async_trait::async_trait;
use chrono::Utc;
use serde_json::json;
use tokio::fs;
use tracing::info;

use crate::deepseek::{ChatMessage, DeepSeekClient};
use crate::types::{SolutionV1, TaskSpec};

use super::{Agent, AgentError};

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


