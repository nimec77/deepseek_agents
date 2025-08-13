use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")] 
pub enum DeliverableType {
    Text,
    Json,
    Code,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSpec {
    pub task_id: String,
    pub goal: String,
    pub input: String,
    pub acceptance_criteria: Vec<String>,
    pub deliverable_type: DeliverableType,
    #[serde(skip_serializing_if = "Option::is_none")] 
    pub hints: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelUsed {
    pub name: String,
    pub temperature: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeArtifact {
    pub language: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Deliverable {
    #[serde(skip_serializing_if = "Option::is_none")] 
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] 
    pub json: Option<JsonValue>,
    #[serde(skip_serializing_if = "Option::is_none")] 
    pub code: Option<CodeArtifact>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    pub system_prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")] 
    pub usage_note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolutionV1 {
    pub schema_version: String, // "solution_v1"
    pub task_id: String,
    pub solution_id: String,
    pub model_used: ModelUsed,
    pub deliverable_type: DeliverableType,
    pub deliverable: Deliverable,
    pub evidence: Evidence,
    pub usage: Usage,
    pub created_at: String, // RFC3339
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")] 
pub enum Verdict {
    Pass,
    Warn,
    Fail,
}

impl std::fmt::Display for Verdict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self { Verdict::Pass => "pass", Verdict::Warn => "warn", Verdict::Fail => "fail" };
        f.write_str(s)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")] 
pub enum Severity {
    Minor,
    Major,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResult {
    pub criterion: String,
    #[serde(rename = "pass")] 
    pub pass_: bool,
    pub reason: String,
    pub severity: Severity,
    #[serde(skip_serializing_if = "Option::is_none")] 
    pub suggested_fix: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationV1 {
    pub schema_version: String, // "validation_v1"
    pub task_id: String,
    pub solution_id: String,
    pub verdict: Verdict,
    pub score: f32, // [0.0, 1.0]
    pub checks: Vec<CheckResult>,
    #[serde(skip_serializing_if = "Option::is_none")] 
    pub suggested_rewrite: Option<JsonValue>,
    pub model_used: ModelUsed,
    pub created_at: String, // RFC3339
}

// (Removed duplicate AuditInput; the canonical type lives in `crate::agents::AuditInput`)
