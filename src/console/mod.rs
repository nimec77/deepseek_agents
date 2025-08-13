use anyhow::{Error, Result};
use tokio::select;
use uuid::Uuid;
use std::path::Path;
use colored::*;

use crate::deepseek::{DeepSeekClient, DeepSeekError, DeepSeekResponse};
use crate::agents::{Agent, ProducerAgent};
use crate::types::{TaskSpec, DeliverableType};

mod input;
mod render;

/// Console interface for the DeepSeek application
pub struct Console {
    client: DeepSeekClient,
}

impl Console {
    /// Create a new console interface with the provided DeepSeek client
    pub fn new(client: DeepSeekClient) -> Self {
        Self { client }
    }

    /// Display a welcome banner
    pub fn display_welcome() {
        render::display_welcome();
    }

    /// Get user input from the console (async)
    pub async fn get_user_input() -> Result<String> {
        input::get_user_input().await
    }

    /// Prompt the user with a custom message and return the entered line (trimmed)
    pub async fn prompt_user(prompt_text: &str) -> Result<String> {
        input::prompt_user(prompt_text).await
    }

    /// Check if the input is a quit command
    pub fn is_quit_command(input_text: &str) -> bool {
        input::is_quit_command(input_text)
    }

    /// Display a loading message
    pub fn display_loading() {
        render::display_loading();
    }

    /// Display the structured response from DeepSeek
    pub fn display_response(response: &DeepSeekResponse) {
        render::display_response(response);
    }

    /// Display an error message with context-aware messaging
    pub fn display_error(error: &Error) {
        render::display_error(error);
    }

    /// Display a DeepSeekError with appropriate styling and context
    pub fn display_deepseek_error(error: &DeepSeekError) {
        render::display_deepseek_error(error);
    }

    /// Display a goodbye message
    pub fn display_goodbye() {
        render::display_goodbye();
    }

    /// Run the main console loop (interactive mode)
    pub async fn run(&self) -> Result<()> {
        Self::display_welcome();
        println!(
            "{}",
            "ℹ️  Interactive mode: Enter a task for the agent. The app will send it, process the response, and display the result. Type '/quit' to exit.".blue()
        );

        loop {
            select! {
                // Handle Ctrl+C gracefully
                _ = tokio::signal::ctrl_c() => {
                    Self::display_goodbye();
                    break;
                }
                // Handle user input
                input_result = Self::get_user_input() => {
                    println!("{}", "📨 Received input from user".bright_white());
                    let input = match input_result {
                        Ok(input) => input,
                        Err(e) => {
                            println!("Error reading input: {}", e);
                            continue;
                        }
                    };

                    if input.is_empty() {
                        continue;
                    }

                    if Self::is_quit_command(&input) {
                        Self::display_goodbye();
                        break;
                    }

                    Self::display_loading();

                    // Allow request to be cancelled by Ctrl+C
                    select! {
                        _ = tokio::signal::ctrl_c() => {
                            println!("\n⚠️ Request cancelled by user");
                            Self::display_goodbye();
                            break;
                        }
                        result = self.client.send_request(&input) => {
                            println!("{}", "🛠️ Processing input with agent".bright_white());
                            match result {
                                Ok(response) => {
                                    println!("{}", "💾 Processed. Displaying result".bright_white());
                                    Self::display_response(&response)
                                },
                                Err(e) => Self::display_deepseek_error(&e),
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Collect a TaskSpec from the user via interactive prompts.
    async fn collect_task_spec(&self) -> Result<TaskSpec> {
        let goal = input::prompt_user("🎯 Goal: ").await?;
        let input_text = input::prompt_user("📥 Input/context: ").await?;

        let ac_raw = input::prompt_user(
            "✅ Acceptance criteria (comma or semicolon separated): ",
        )
        .await?;
        let acceptance_criteria: Vec<String> = ac_raw
            .split([',', ';', '\n'])
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();

        println!(
            "{}",
            "📦 Deliverable type: [1] text  [2] json  [3] code (enter 1/2/3 or name)".blue()
        );
        let deliverable_raw = input::prompt_user("Type: ").await?;
        let deliverable_type = match deliverable_raw.trim().to_lowercase().as_str() {
            "1" | "text" => DeliverableType::Text,
            "2" | "json" => DeliverableType::Json,
            "3" | "code" => DeliverableType::Code,
            other => {
                println!(
                    "{} {}",
                    "⚠️ Unknown type, defaulting to 'text':".bright_yellow(),
                    other
                );
                DeliverableType::Text
            }
        };

        let hints = input::prompt_user("💡 Hints (optional, Enter to skip): ").await?;
        let hints = if hints.trim().is_empty() { None } else { Some(hints) };

        let task_spec = TaskSpec {
            task_id: Uuid::new_v4(),
            goal,
            input: input_text,
            acceptance_criteria,
            deliverable_type,
            hints,
        };

        // Show the JSON that will be sent to the agent
        let pretty = serde_json::to_string_pretty(&task_spec)?;
        println!("\n{}\n{}\n", "🧾 TaskSpec JSON:".bright_green().bold(), pretty);

        Ok(task_spec)
    }

    /// Interactive flow: collect a task and run ProducerAgent. Saves to out_dir/solution.json
    pub async fn run_producer_agent(&self, out_dir: &Path) -> Result<()> {
        Self::display_welcome();
        println!(
            "{}",
            "ℹ️  Interactive mode: Enter a task for the ProducerAgent. It will process your input and save the result to a file.".blue()
        );

        let task_spec = self.collect_task_spec().await?;
        println!("{}", "📨 Received task specification from user".bright_white());

        tokio::fs::create_dir_all(out_dir).await?;
        let out_path = out_dir.join("solution.json");

        let agent = ProducerAgent::new(self.client.clone(), out_path.clone());
        println!("{}", "🛠️ ProducerAgent is processing the task".bright_white());
        match agent.execute(&task_spec).await {
            Ok(solution) => {
                println!(
                    "{} {}\n  {}",
                    "✅ ProducerAgent completed.".bright_green().bold(),
                    format!("solution_id={}", solution.solution_id).bright_white(),
                    out_path.display()
                );
                println!(
                    "{} {}",
                    "💾 Saved result to".bright_white(),
                    out_path.display()
                );
            }
            Err(e) => {
                let err: Error = anyhow::anyhow!(e);
                Self::display_error(&err);
            }
        }

        Ok(())
    }
}

