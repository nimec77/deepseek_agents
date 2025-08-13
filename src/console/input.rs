use anyhow::{Context, Result};
use colored::*;
use std::io::{self, Write};
use tokio::io::{AsyncBufReadExt, BufReader};

/// Get user input from the console (async version)
pub async fn get_user_input() -> Result<String> {
    print!("{}", "💬 Enter your question: ".bright_cyan().bold());
    io::stdout().flush().unwrap();

    let mut reader = BufReader::new(tokio::io::stdin());
    let mut input = String::new();
    reader
        .read_line(&mut input)
        .await
        .context("Failed to read user input")?;

    Ok(input.trim().to_string())
}

/// Prompt the user with a custom message and return the entered line (trimmed)
pub async fn prompt_user(prompt_text: &str) -> Result<String> {
    print!("{}", prompt_text.bright_cyan().bold());
    io::stdout().flush().unwrap();

    let mut reader = BufReader::new(tokio::io::stdin());
    let mut input = String::new();
    reader
        .read_line(&mut input)
        .await
        .context("Failed to read user input")?;

    Ok(input.trim().to_string())
}

/// Check if the input is a quit command
pub fn is_quit_command(input: &str) -> bool {
    input.eq_ignore_ascii_case("/quit") || input.eq_ignore_ascii_case("/exit")
}
