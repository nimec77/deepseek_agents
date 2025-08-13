mod config;
mod deepseek;
mod agents;
mod types;
mod console;
mod orchestrator;

use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use tracing_subscriber::{fmt, EnvFilter};

use crate::config::Config;
use crate::orchestrator::Orchestrator;
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

    // base config from env and orchestrator setup
    let base_cfg = Config::load()?;
    let orchestrator = Orchestrator::new(base_cfg)?;

    // If console mode is requested, run interactive ProducerAgent flow and exit
    if args.console_producer {
        orchestrator.run_console_producer(&args.out_dir).await?;
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
    let _ = orchestrator.run_pipeline(task_spec, &args.out_dir).await?;
    Ok(())
}

fn demo_task_spec() -> TaskSpec {
    TaskSpec {
        task_id: uuid::Uuid::new_v4().to_string(),
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
