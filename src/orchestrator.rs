use std::path::{Path, PathBuf};

use anyhow::Result;
use tracing::info;

use crate::agents::{Agent, AuditInput, AuditorAgent, ProducerAgent};
use crate::config::Config;
use crate::console::Console;
use crate::deepseek::DeepSeekClient;
use crate::types::{SolutionV1, TaskSpec, ValidationV1};

pub struct Orchestrator {
    chat_client: DeepSeekClient,
    reasoner_client: DeepSeekClient,
}

impl Orchestrator {
    pub fn new(base_cfg: Config) -> Result<Self> {
        let chat_client = DeepSeekClient::new(base_cfg.clone())?;

        let mut reasoner_cfg = base_cfg;
        reasoner_cfg.model = "deepseek-reasoner".to_string();
        let reasoner_client = DeepSeekClient::new(reasoner_cfg)?;

        Ok(Self {
            chat_client,
            reasoner_client,
        })
    }

    pub async fn run_console_producer(&self, out_dir: &Path) -> Result<()> {
        info!(
            "Interactive mode: you'll be prompted to enter a task for the ProducerAgent, which will process it and save the result"
        );
        let console = Console::new(self.chat_client.clone());
        console.run_producer_agent(out_dir).await
    }

    pub async fn run_pipeline(
        &self,
        task_spec: TaskSpec,
        out_dir: &Path,
    ) -> Result<(SolutionV1, ValidationV1)> {
        info!("Pipeline mode: ProducerAgent → AuditorAgent");

        tokio::fs::create_dir_all(out_dir).await?;
        let solution_path: PathBuf = out_dir.join("solution.json");
        let validation_path: PathBuf = out_dir.join("validation.json");

        let agent1 = ProducerAgent::new(self.chat_client.clone(), solution_path.clone());
        info!(
            "Agent1 (Producer): received task_id={} — processing",
            task_spec.task_id
        );
        Console::display_task(&task_spec);
        let solution = agent1.execute(&task_spec).await?;
        let solution_for_return = solution.clone();
        info!("Agent1 produced solution: {}", solution.solution_id);
        info!(
            "Agent1 saved solution to {}",
            solution_path.display()
        );
        Console::display_solution(&solution);

        let agent2 = AuditorAgent::new(self.reasoner_client.clone(), validation_path.clone());
        info!(
            "Agent2 (Auditor): received solution {} from Agent1 — processing",
            solution.solution_id
        );
        let validation = agent2
            .execute(&AuditInput {
                task: task_spec,
                solution,
            })
            .await?;
        info!(
            "Agent2 verdict: {} (score {:.2})",
            validation.verdict,
            validation.score
        );
        info!(
            "Agent2 saved validation to {}",
            validation_path.display()
        );
        Console::display_validation(&validation);

        println!(
            "Artifacts:\n  {}\n  {}",
            solution_path.display(),
            validation_path.display()
        );

        Ok((solution_for_return, validation))
    }
}


