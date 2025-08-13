use anyhow::Result;
use colored::*;

use crate::deepseek::ChatMessage;
use crate::taskfinisher::{
    build_system_prompt, parse_taskfinisher_response, AnswerItem, AnswersPayload,
    ClarifyingQuestion, TaskFinisherResult,
};

use super::Console;

impl Console {
    /// Collect answers for clarifying questions interactively.
    /// Users enter answers one-by-one; empty input skips a question; typing '/proceed' finalizes early.
    async fn collect_answers_interactively(
        questions: &[ClarifyingQuestion],
    ) -> Result<AnswersPayload> {
        println!(
            "{}",
            "✍️ Answer the questions one-by-one. Press Enter to skip. Type '/proceed' to finalize now.".blue()
        );

        let mut answers: Vec<AnswerItem> = Vec::new();
        for q in questions {
            println!("\n{} {}", q.id.bright_white().bold(), q.text.white());
            if let Some(opts) = &q.options
                && !opts.is_empty() {
                    println!("{} {:?}", "options:".white(), opts);
                }

            let prompt = format!("Your answer for {}: ", q.id);
            let input = super::input::prompt_user(&prompt).await?;

            if input.is_empty() {
                continue;
            }
            if super::input::is_quit_command(&input) || input.eq_ignore_ascii_case("/proceed") {
                break;
            }

            answers.push(AnswerItem {
                id: q.id.clone(),
                answer: input,
            });
        }

        Ok(AnswersPayload { answers })
    }

    /// Run TaskFinisher-JSON interactive flow.
    pub async fn run_taskfinisher(
        &self,
        initial_prompt: Option<&str>,
        max_questions: u32,
    ) -> Result<()> {
        let max_q = if max_questions == 0 {
            crate::taskfinisher::DEFAULT_MAX_QUESTIONS
        } else {
            max_questions
        };
        println!("{}", "🤖 TaskFinisher-JSON Mode".bright_blue().bold());
        println!("{} {}", "Max clarifying questions:".blue(), max_q);

        let user_prompt = if let Some(p) = initial_prompt {
            p.to_string()
        } else {
            super::input::prompt_user("💬 Enter your technical task request: ").await?
        };

        let system_prompt = build_system_prompt(max_q);
        let mut history: Vec<ChatMessage> = vec![
            ChatMessage { role: "system".to_string(), content: system_prompt.clone() },
            ChatMessage { role: "user".to_string(), content: format!(
                "Describe the result to collect and provide the answer accordingly. Example domain: technical specifications. User request: {}",
                user_prompt
            )},
        ];

        println!("{}", "🔄 Sending TaskFinisher request...".blue().italic());
        let mut raw = self
            .client
            .send_messages_raw(history.clone())
            .await
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;

        let max_rounds = 5u32;
        let mut round = 1u32;

        loop {
            match parse_taskfinisher_response(&raw) {
                Ok(TaskFinisherResult::Artifact(artifact, _)) => {
                    super::render::display_taskfinisher_artifact(&artifact);
                    break;
                }
                Ok(TaskFinisherResult::Clarifying(payload, _)) => {
                    println!(
                        "\n{} (round {})",
                        "❓ Clarifying Questions:".bright_yellow().bold(),
                        round
                    );
                    for q in &payload.questions {
                        println!("- {} {}", q.id.bright_white().bold(), q.text.white());
                        if let Some(opts) = &q.options {
                            println!("  options: {:?}", opts);
                        }
                    }
                    println!("\n{}", "🧾 Checklist:".bright_cyan().bold());
                    for item in &payload.checklist {
                        println!("- {} [{}]", item.field.white(), item.status.green());
                    }
                    println!("\n{}", "💬 Enter answers one-by-one below (Enter = skip, '/proceed' = finalize now).".blue());

                    let answers_payload =
                        Self::collect_answers_interactively(&payload.questions).await?;
                    history.push(ChatMessage {
                        role: "assistant".to_string(),
                        content: raw,
                    });
                    history.push(ChatMessage {
                        role: "user".to_string(),
                        content: serde_json::to_string(&answers_payload).unwrap(),
                    });

                    println!("{}", "🔄 Processing answers...".blue().italic());
                    raw = self
                        .client
                        .send_messages_raw(history.clone())
                        .await
                        .map_err(|e| anyhow::anyhow!(e.to_string()))?;

                    round += 1;
                    if round > max_rounds {
                        println!("{}", "⚠️ Reached maximum clarification rounds. Showing latest assistant output.".bright_yellow());
                        println!("{}", raw);
                        break;
                    }
                }
                Err(e) => {
                    println!("{} {}", "❌ Parse error:".bright_red().bold(), e);
                    println!("{}", raw);
                    break;
                }
            }
        }

        Ok(())
    }
}
