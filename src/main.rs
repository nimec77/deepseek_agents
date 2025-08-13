mod config;
mod deepseek;
mod agents;
mod types;
mod console;

use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use tracing_subscriber::{fmt, EnvFilter};

use crate::agents::{Agent, AuditInput, AuditorAgent, ProducerAgent};
use crate::config::Config;
use crate::console::Console;
use crate::deepseek::DeepSeekClient;
use crate::types::{DeliverableType, TaskSpec};

#[derive(Debug, Parser)]
struct Args {
    /// Path to TaskSpec JSON file. If omitted, a demo TaskSpec is used
    #[arg(long)]
    task: Option<PathBuf>,

    /// Output directory for artifacts
    #[arg(long, default_value = "out")] 
    out_dir: PathBuf,

    /// Run interactive console to collect a task and execute ProducerAgent
    #[arg(long, default_value_t = false)]
    console_producer: bool,
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    let args = Args::parse();

    // logging
    let filter_layer = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    fmt().with_env_filter(filter_layer).init();

    // startup information
    tracing::info!("Starting DeepSeek Agents application");

    // base config from env
    let base_cfg = Config::load()?;

    // agent1 uses deepseek-chat (default)
    let agent1_client = DeepSeekClient::new(base_cfg.clone())?;

    // agent2 uses deepseek-reasoner
    let mut reasoner_cfg = base_cfg.clone();
    reasoner_cfg.model = "deepseek-reasoner".to_string();
    let agent2_client = DeepSeekClient::new(reasoner_cfg)?;

    // If console mode is requested, run interactive ProducerAgent flow and exit
    if args.console_producer {
        tracing::info!(
            "Interactive mode: you'll be prompted to enter a task for the ProducerAgent, which will process it and save the result"
        );
        let console = Console::new(agent1_client.clone());
        console.run_producer_agent(&args.out_dir).await?;
        return Ok(());
    }

    // load or construct TaskSpec
    tracing::info!("Pipeline mode: ProducerAgent → AuditorAgent");
    let task_spec: TaskSpec = match &args.task {
        Some(path) => {
            tracing::info!("Loading TaskSpec from file: {}", path.display());
            let bytes = tokio::fs::read(path).await?;
            serde_json::from_slice(&bytes)?
        }
        None => {
            tracing::info!("No --task provided. Using demo TaskSpec");
            demo_task_spec()
        },
    };

    tokio::fs::create_dir_all(&args.out_dir).await?;
    let solution_path = args.out_dir.join("solution.json");
    let validation_path = args.out_dir.join("validation.json");

    let agent1 = ProducerAgent::new(agent1_client, solution_path.clone());
    tracing::info!(
        "Agent1 (Producer): received task_id={} — processing",
        task_spec.task_id
    );
    let solution = agent1.execute(&task_spec).await?;
    tracing::info!("Agent1 produced solution: {}", solution.solution_id);
    tracing::info!("Agent1 saved solution to {}", solution_path.display());

    let agent2 = AuditorAgent::new(agent2_client, validation_path.clone());
    tracing::info!(
        "Agent2 (Auditor): received solution {} from Agent1 — processing",
        solution.solution_id
    );
    let validation = agent2
        .execute(&AuditInput {
            task: task_spec,
            solution,
        })
        .await?;
    tracing::info!("Agent2 verdict: {} (score {:.2})", validation.verdict, validation.score);
    tracing::info!("Agent2 saved validation to {}", validation_path.display());

    println!("Artifacts:\n  {}\n  {}", solution_path.display(), validation_path.display());
    Ok(())
}

fn demo_task_spec() -> TaskSpec {
    use uuid::Uuid;

    TaskSpec {
        task_id: Uuid::new_v4(),
        goal: "Summarize the input text into exactly 3 crisp bullet points".to_string(),
        input: "DeepSeek Agents demo: we need two agents where the first produces a deliverable and the second audits it against acceptance criteria.".to_string(),
        acceptance_criteria: vec![
            "exactly 3 bullets".to_string(),
            "<= 80 words total".to_string(),
            "no marketing fluff".to_string(),
        ],
        deliverable_type: DeliverableType::Text,
        hints: Some("Be concise".to_string()),
    }
}
