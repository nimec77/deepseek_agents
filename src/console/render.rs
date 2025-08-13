use anyhow::Error;
use colored::*;

use crate::deepseek::{DeepSeekError, DeepSeekResponse};

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
