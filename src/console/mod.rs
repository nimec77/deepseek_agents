use anyhow::{Error, Result};
use tokio::select;

use crate::deepseek::{DeepSeekClient, DeepSeekError, DeepSeekResponse};
use crate::taskfinisher::TechnicalTaskArtifact;

mod input;
mod render;
mod taskfinisher;

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

    /// Display a TaskFinisher Technical Task artifact with colored sections
    pub fn display_taskfinisher_artifact(artifact: &TechnicalTaskArtifact) {
        render::display_taskfinisher_artifact(artifact);
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

        loop {
            select! {
                // Handle Ctrl+C gracefully
                _ = tokio::signal::ctrl_c() => {
                    Self::display_goodbye();
                    break;
                }
                // Handle user input
                input_result = Self::get_user_input() => {
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
                            match result {
                                Ok(response) => Self::display_response(&response),
                                Err(e) => Self::display_deepseek_error(&e),
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

// Re-export utilities for optional external use
pub use input::{get_user_input, is_quit_command, prompt_user};
pub use render::{
    display_deepseek_error, display_error, display_goodbye, display_loading, display_response,
    display_taskfinisher_artifact, display_welcome,
};
