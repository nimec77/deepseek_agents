use anyhow::Error;
use colored::*;

use crate::deepseek::{DeepSeekError, DeepSeekResponse};
use crate::types::{DeliverableType, SolutionV1, ValidationV1, Verdict, TaskSpec};

pub fn display_welcome() {
    println!(
        "{}",
        "🤖 DeepSeek JSON Chat Application".bright_blue().bold()
    );
    println!(
        "{}",
        "This application sends your queries to DeepSeek and returns structured JSON responses."
            .blue()
    );
    println!(
        "{}",
        "Enter a task for the agent to process. In interactive mode, your input will be sent to the agent and the result will be displayed.".blue()
    );
    println!(
        "{}",
        "Make sure to set DEEPSEEK_API_KEY environment variable.".blue()
    );
    println!("{}", "Type '/quit' or '/exit' to stop.\n".blue());
}

pub fn display_loading() {
    println!("{}", "🔄 Sending request to DeepSeek...".blue().italic());
}

pub fn display_response(response: &DeepSeekResponse) {
    println!("\n{}", "📋 Structured Response:".bright_green().bold());
    println!(
        "{}",
        "┌─────────────────────────────────────────────────────────────".green()
    );
    println!(
        "{} {}",
        "│ 🏷️  Title:".green(),
        response.title.bright_white().bold()
    );
    println!(
        "{} {}",
        "│ 📝 Description:".green(),
        response.description.white()
    );
    println!("{} {}", "│ 📄 Content:".green(), response.content.white());
    if let Some(category) = &response.category {
        println!("{} {}", "│ 🏪 Category:".green(), category.white());
    }
    if let Some(timestamp) = &response.timestamp {
        println!("{} {}", "│ ⏰ Timestamp:".green(), timestamp.white());
    }
    if let Some(confidence) = response.confidence {
        println!(
            "{} {}",
            "│ 🎯 Confidence:".green(),
            format!("{:.2}", confidence).white()
        );
    }
    println!(
        "{}",
        "└─────────────────────────────────────────────────────────────\n".green()
    );
}

pub fn display_error(error: &Error) {
    if let Some(deepseek_error) = error.downcast_ref::<DeepSeekError>() {
        display_deepseek_error(deepseek_error);
    } else {
        println!(
            "{} {}",
            "❌ Error:".bright_red().bold(),
            error.to_string().red()
        );
        println!(
            "{}",
            "Please check your configuration and try again.\n".red()
        );
    }
}

pub fn display_deepseek_error(error: &DeepSeekError) {
    let user_message = error.user_message();
    match error {
        DeepSeekError::ServerBusy => {
            println!("{}", user_message.bright_yellow().bold());
            println!(
                "{}",
                "💡 Tip: Try again in a few minutes when server load is lower.".yellow()
            );
        }
        DeepSeekError::NetworkError { .. } => {
            println!("{}", user_message.bright_red().bold());
            println!(
                "{}",
                "💡 Tip: Check your internet connection and firewall settings.".red()
            );
        }
        DeepSeekError::Timeout { .. } => {
            println!("{}", user_message.bright_yellow().bold());
            println!(
                "{}",
                "💡 Tip: The server might be overloaded. Try again later.".yellow()
            );
        }
        DeepSeekError::ApiError { status, .. } => {
            println!("{}", user_message.bright_red().bold());
            match *status {
                401 => println!(
                    "{}",
                    "💡 Tip: Check your DEEPSEEK_API_KEY environment variable.".red()
                ),
                403 => println!(
                    "{}",
                    "💡 Tip: Your API key may not have sufficient permissions.".red()
                ),
                429 => println!(
                    "{}",
                    "💡 Tip: You've hit the rate limit. Wait before trying again.".red()
                ),
                _ => println!(
                    "{}",
                    "💡 Tip: Check the DeepSeek API documentation for more details.".red()
                ),
            }
        }
        DeepSeekError::ParseError { .. } => {
            println!("{}", user_message.bright_magenta().bold());
            println!(
                "{}",
                "💡 Tip: The server response was unexpected. Try rephrasing your query.".magenta()
            );
        }
        DeepSeekError::ConfigError { .. } => {
            println!("{}", user_message.bright_red().bold());
            println!(
                "{}",
                "💡 Tip: Check your environment variables and configuration.".red()
            );
        }
    }
    println!();
}

pub fn display_goodbye() {
    println!("{}", "👋 Goodbye!".bright_yellow().bold());
}

pub fn display_task(task: &TaskSpec) {
    println!("\n{}", "🗒️  Task Specification".bright_yellow().bold());
    println!("{}", "┌─────────────────────────────────────────────────────────────".yellow());
    println!(
        "{} {}",
        "│ 🧩 Task ID:".yellow(),
        task.task_id.to_string().bright_white()
    );
    println!(
        "{} {}",
        "│ 🎯 Goal:".yellow(),
        task.goal.white()
    );
    println!("{}", "│ 📝 Input:".yellow());
    for line in task.input.lines() {
        println!("│   {}", line.white());
    }
    if !task.acceptance_criteria.is_empty() {
        println!("{}", "│ ✅ Acceptance Criteria:".yellow());
        for (idx, crit) in task.acceptance_criteria.iter().enumerate() {
            println!("│   {}. {}", idx + 1, crit.white());
        }
    }
    println!(
        "{} {}",
        "│ 📦 Deliverable Type:".yellow(),
        format!("{:?}", task.deliverable_type).white()
    );
    if let Some(hints) = &task.hints
        && !hints.trim().is_empty() {
            println!("{}", "│ 💡 Hints:".yellow());
            for line in hints.lines() {
                println!("│   {}", line.white());
            }
        }
    println!("{}", "└─────────────────────────────────────────────────────────────\n".yellow());
}

pub fn display_solution(solution: &SolutionV1) {
    println!("\n{}", "📦 Agent Output (Solution)".bright_cyan().bold());
    println!("{}", "┌─────────────────────────────────────────────────────────────".cyan());
    println!(
        "{} {}",
        "│ 🆔 Solution ID:".cyan(),
        solution.solution_id.to_string().bright_white()
    );
    println!(
        "{} {}",
        "│ 🧩 Task ID:".cyan(),
        solution.task_id.to_string().white()
    );
    println!(
        "{} {} (temp {:.2})",
        "│ 🤖 Model:".cyan(),
        solution.model_used.name.white(),
        solution.model_used.temperature
    );
    println!("{} {}", "│ 🗓️  Created:".cyan(), solution.created_at.white());
    println!(
        "{} {}",
        "│ 📄 Deliverable Type:".cyan(),
        format!("{:?}", solution.deliverable_type).white()
    );

    match solution.deliverable_type {
        DeliverableType::Text => {
            if let Some(text) = &solution.deliverable.text {
                println!("{}", "│ ── Text:".cyan());
                for line in text.lines() {
                    println!("│   {}", line.white());
                }
            }
        }
        DeliverableType::Json => {
            if let Some(json) = &solution.deliverable.json {
                let pretty = serde_json::to_string_pretty(json).unwrap_or_else(|_| json.to_string());
                println!("{}", "│ ── JSON:".cyan());
                for line in pretty.lines() {
                    println!("│   {}", line.white());
                }
            }
        }
        DeliverableType::Code => {
            if let Some(code) = &solution.deliverable.code {
                println!(
                    "{} {}",
                    "│ ── Code (lang):".cyan(),
                    code.language.white()
                );
                println!("{}", "│ ── Content:".cyan());
                for line in code.content.lines() {
                    println!("│   {}", line.white());
                }
            }
        }
    }

    println!(
        "{} {} / {}",
        "│ 🔢 Tokens:".cyan(),
        solution.usage.prompt_tokens.to_string().white(),
        solution.usage.completion_tokens.to_string().white()
    );
    println!("{}", "└─────────────────────────────────────────────────────────────\n".cyan());
}

pub fn display_validation(validation: &ValidationV1) {
    println!("\n{}", "🧪 Agent Output (Validation)".bright_magenta().bold());
    println!("{}", "┌─────────────────────────────────────────────────────────────".magenta());
    println!(
        "{} {}",
        "│ 🆔 Solution ID:".magenta(),
        validation.solution_id.to_string().bright_white()
    );
    println!(
        "{} {}",
        "│ 🧩 Task ID:".magenta(),
        validation.task_id.to_string().white()
    );
    let verdict_str = format!("{}", validation.verdict);
    let verdict_colored = match validation.verdict {
        Verdict::Pass => verdict_str.bright_green().bold(),
        Verdict::Warn => verdict_str.bright_yellow().bold(),
        Verdict::Fail => verdict_str.bright_red().bold(),
    };
    println!("{} {} (score {:.2})", "│ ⚖️  Verdict:".magenta(), verdict_colored, validation.score);
    println!(
        "{} {} (temp {:.2})",
        "│ 🤖 Model:".magenta(),
        validation.model_used.name.white(),
        validation.model_used.temperature
    );
    println!("{} {}", "│ 🗓️  Created:".magenta(), validation.created_at.white());

    if !validation.checks.is_empty() {
        println!("{}", "│ ── Checks:".magenta());
        for (idx, chk) in validation.checks.iter().enumerate() {
            let icon = if chk.pass_ { "✔".bright_green() } else { "✖".bright_red() };
            println!(
                "│   {} {}. {}",
                icon,
                idx + 1,
                chk.criterion.bright_white()
            );
            println!("│     {} {}", "reason:".white(), chk.reason.white());
            println!("│     {} {}", "severity:".white(), format!("{:?}", chk.severity).white());
            if let Some(suggest) = &chk.suggested_fix {
                println!("│     {} {}", "suggested_fix:".white(), suggest.white());
            }
        }
    }

    if let Some(rewrite) = &validation.suggested_rewrite {
        let pretty = serde_json::to_string_pretty(rewrite).unwrap_or_else(|_| rewrite.to_string());
        println!("{}", "│ ── Suggested Rewrite:".magenta());
        for line in pretty.lines() {
            println!("│   {}", line.white());
        }
    }

    println!("{}", "└─────────────────────────────────────────────────────────────\n".magenta());
}
