use anyhow::Error;
use colored::*;

use crate::deepseek::{DeepSeekError, DeepSeekResponse};
use crate::taskfinisher::TechnicalTaskArtifact;

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

pub fn display_taskfinisher_artifact(artifact: &TechnicalTaskArtifact) {
    println!(
        "\n{}",
        "📦 Technical Task (Artifact):".bright_green().bold()
    );
    println!(
        "{}",
        "┌─────────────────────────────────────────────────────────────".green()
    );
    println!(
        "{} {}",
        "│ 🏷️  Title:".green(),
        artifact.title.bright_white().bold()
    );
    println!(
        "{} {} ({})",
        "│ 🧩 Artifact:".green(),
        artifact.artifact_name.bright_cyan(),
        format!("v{}", artifact.version).cyan().italic()
    );
    println!("{} {}", "│ 📝 Summary:".green(), artifact.summary.white());

    println!("{}", "│ — Stakeholders".bright_cyan().bold());
    if artifact.stakeholders.is_empty() {
        println!("{}", "│   (none)".truecolor(180, 180, 180));
    } else {
        for stakeholder in &artifact.stakeholders {
            println!(
                "{} {} — {}",
                "│   •".cyan(),
                stakeholder.role.bright_white().bold(),
                stakeholder.description.white()
            );
        }
    }

    println!("{}", "│ — Scope".bright_cyan().bold());
    if !artifact.scope.in_scope.is_empty() {
        println!("{}", "│   In-scope:".green());
        for item in &artifact.scope.in_scope {
            println!("{} {}", "│     ✔".green(), item.white());
        }
    }
    if !artifact.scope.out_of_scope.is_empty() {
        println!("{}", "│   Out-of-scope:".bright_yellow());
        for item in &artifact.scope.out_of_scope {
            println!("{} {}", "│     ✖".yellow(), item.white());
        }
    }

    println!("{}", "│ — Requirements".bright_cyan().bold());
    if artifact.requirements.functional.is_empty() {
        println!("{}", "│   Functional: (none)".truecolor(180, 180, 180));
    } else {
        println!("{}", "│   Functional:".green());
        for fr in &artifact.requirements.functional {
            println!(
                "{} {} {}",
                "│     •".green(),
                fr.id.bright_white().bold(),
                fr.statement.white()
            );
            if let Some(rationale) = &fr.rationale
                && !rationale.is_empty() {
                    println!(
                        "{} {}",
                        "│       ↳ rationale:".truecolor(150, 150, 255),
                        rationale.truecolor(170, 170, 255).italic()
                    );
                }
        }
    }
    if artifact.requirements.non_functional.is_empty() {
        println!("{}", "│   Non-functional: (none)".truecolor(180, 180, 180));
    } else {
        println!("{}", "│   Non-functional:".green());
        for nfr in &artifact.requirements.non_functional {
            println!(
                "{} {} [{}] → {}",
                "│     •".green(),
                nfr.id.bright_white().bold(),
                nfr.category.bright_cyan(),
                nfr.target.white()
            );
        }
    }

    println!("{}", "│ — Data Integrations".bright_cyan().bold());
    if !artifact
        .data_integrations
        .rpc_providers
        .selection
        .is_empty()
    {
        println!(
            "{} {}",
            "│   RPC providers:".green(),
            format!("{:?}", artifact.data_integrations.rpc_providers.selection).white()
        );
    }
    if !artifact
        .data_integrations
        .rpc_providers
        .endpoints
        .is_empty()
    {
        println!("{}", "│   Endpoints:".green());
        for (name, value) in &artifact.data_integrations.rpc_providers.endpoints {
            println!(
                "{} {} = {}",
                "│     •".green(),
                name.bright_white(),
                value.to_string().white()
            );
        }
    }
    println!(
        "{} {}{}",
        "│   Price source:".green(),
        artifact
            .data_integrations
            .price_source
            .provider
            .bright_white(),
        match artifact.data_integrations.price_source.ttl_seconds {
            Some(ttl) => format!(" (ttl={}s)", ttl).truecolor(180, 180, 180),
            None => "".normal(),
        }
    );

    println!("{}", "│ — Constraints".bright_cyan().bold());
    if artifact.constraints.is_empty() {
        println!("{}", "│   (none)".truecolor(180, 180, 180));
    } else {
        for c in &artifact.constraints {
            println!("{} {}", "│   •".green(), c.white());
        }
    }

    println!("{}", "│ — Assumptions".bright_cyan().bold());
    if artifact.assumptions.is_empty() {
        println!("{}", "│   (none)".truecolor(180, 180, 180));
    } else {
        for a in &artifact.assumptions {
            println!("{} {}", "│   •".green(), a.white());
        }
    }

    println!("{}", "│ — Risks".bright_cyan().bold());
    if artifact.risks.is_empty() {
        println!("{}", "│   (none)".truecolor(180, 180, 180));
    } else {
        for r in &artifact.risks {
            println!(
                "{} {}: {}",
                "│   ⚠".bright_yellow(),
                r.id.bright_yellow().bold(),
                r.description.white()
            );
            println!(
                "{} {}",
                "│     mitigation:".green(),
                r.mitigation.bright_green()
            );
        }
    }

    println!("{}", "│ — Milestones".bright_cyan().bold());
    if artifact.milestones.is_empty() {
        println!("{}", "│   (none)".truecolor(180, 180, 180));
    } else {
        for m in &artifact.milestones {
            println!(
                "{} {} — {}",
                "│   ⏳".cyan(),
                m.id.bright_white().bold(),
                m.name.white()
            );
            if !m.deliverables.is_empty() {
                println!("{}", "│     deliverables:".green());
                for d in &m.deliverables {
                    println!("{} {}", "│       •".green(), d.white());
                }
            }
        }
    }

    println!("{}", "│ — Acceptance criteria".bright_cyan().bold());
    if artifact.acceptance_criteria.is_empty() {
        println!("{}", "│   (none)".truecolor(180, 180, 180));
    } else {
        for ac in &artifact.acceptance_criteria {
            println!("{} {}", "│   ✅".green(), ac.id.bright_white().bold());
            println!(
                "{} {}",
                "│     Given:".truecolor(180, 180, 255),
                ac.given.white()
            );
            println!(
                "{} {}",
                "│     When:".truecolor(180, 180, 255),
                ac.when.white()
            );
            println!(
                "{} {}",
                "│     Then:".truecolor(180, 180, 255),
                ac.then.white()
            );
        }
    }

    println!("{}", "│ — Open questions".bright_cyan().bold());
    if artifact.open_questions.is_empty() {
        println!("{}", "│   (none)".truecolor(180, 180, 180));
    } else {
        for q in &artifact.open_questions {
            println!("{} {}", "│   •".bright_yellow(), q.bright_yellow());
        }
    }

    println!(
        "{} {} {} {}",
        "│ Status:".green(),
        artifact.status.bright_white().bold(),
        "End:".truecolor(180, 180, 180),
        artifact.end_token.truecolor(180, 180, 180)
    );
    println!(
        "{}",
        "└─────────────────────────────────────────────────────────────".green()
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
