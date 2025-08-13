use async_trait::async_trait;

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

pub mod producer;
pub mod auditor;

pub use producer::ProducerAgent;
pub use auditor::{AuditorAgent, AuditInput};


