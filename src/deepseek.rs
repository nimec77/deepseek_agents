use std::time::Duration;
use std::fmt;

use anyhow::Result;
use chrono::Utc;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::config::Config;

#[cfg(feature = "deepseek_api")]
use deepseek_api::{
    request::MessageRequest as ExtMessageRequest,
    response::{ChatResponse as ExtChatResponse, ModelType as ExtModelType},
    CompletionsRequestBuilder as ExtCompletionsRequestBuilder,
    DeepSeekClient as ExtDeepSeekClient, DeepSeekClientBuilder as ExtDeepSeekClientBuilder,
};

/// Custom error types for DeepSeek API interactions
#[derive(Error, Debug)]
pub enum DeepSeekError {
    #[error("DeepSeek servers are currently busy. Please try again in a few moments.")]
    ServerBusy,

    #[error("Network connection failed: {message}")]
    NetworkError { message: String },

    #[error("Request timed out after {seconds} seconds")]
    Timeout { seconds: u64 },

    #[error("API error ({status}): {message}")]
    ApiError { status: u16, message: String },

    #[error("Failed to parse response: {message}")]
    ParseError { message: String },

    #[error("Configuration error: {message}")]
    ConfigError { message: String },
}

impl DeepSeekError {
    /// Check if the error indicates server is busy
    #[allow(dead_code)]
    pub fn is_server_busy(&self) -> bool {
        matches!(self, DeepSeekError::ServerBusy)
    }

    /// Check if the error is a network-related issue
    #[allow(dead_code)]
    pub fn is_network_error(&self) -> bool {
        matches!(self, DeepSeekError::NetworkError { .. })
    }

    /// Get user-friendly error message
    pub fn user_message(&self) -> String {
        match self {
            DeepSeekError::ServerBusy => {
                "🚫 DeepSeek servers are currently busy. Please try again in a few moments."
                    .to_string()
            }
            DeepSeekError::NetworkError { .. } => {
                "🌐 Network connection failed. Please check your internet connection and try again."
                    .to_string()
            }
            DeepSeekError::Timeout { seconds } => {
                format!(
                    "⏰ Request timed out after {} seconds. The server might be overloaded.",
                    seconds
                )
            }
            DeepSeekError::ApiError { status, .. } => match *status {
                429 => {
                    "🚫 Rate limit exceeded. Please wait a moment before trying again.".to_string()
                }
                503 => "🚫 Service temporarily unavailable. Please try again later.".to_string(),
                502 | 504 => {
                    "🚫 Server gateway error. Please try again in a few moments.".to_string()
                }
                _ => format!("❌ API error ({}). Please try again later.", status),
            },
            DeepSeekError::ParseError { .. } => {
                "⚠️ Failed to parse server response. Please try again.".to_string()
            }
            DeepSeekError::ConfigError { message } => {
                format!("⚙️ Configuration error: {}", message)
            }
        }
    }
}

/// Define the expected JSON response structure from DeepSeek
#[derive(Debug, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct DeepSeekResponse {
    pub title: String,
    pub description: String,
    pub content: String,
    pub category: Option<String>,
    pub timestamp: Option<String>,
    pub confidence: Option<f32>,
}

/// API request/response structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    response_format: ResponseFormat,
    max_tokens: u32,
    temperature: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
struct ResponseFormat {
    #[serde(rename = "type")]
    format_type: String,
}

#[derive(Debug, Deserialize)]
struct ApiResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: ChatMessage,
}

/// DeepSeek API client
#[derive(Clone)]
pub struct DeepSeekClient {
    client: Client,
    config: Config,
    #[cfg(feature = "deepseek_api")]
    ext_client: Option<ExtDeepSeekClient>,
}

impl fmt::Debug for DeepSeekClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Do not attempt to format the external client to avoid trait bounds
        f.debug_struct("DeepSeekClient")
            .field("base_url", &self.config.base_url)
            .field("model", &self.config.model)
            .finish()
    }
}

impl DeepSeekClient {
    /// Create a new DeepSeek client with the given configuration
    pub fn new(config: Config) -> Result<Self, DeepSeekError> {
        config.validate().map_err(|e| DeepSeekError::ConfigError {
            message: e.to_string(),
        })?;

        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout))
            .user_agent("deepseek_json/0.1.0")
            .build()
            .map_err(|e| DeepSeekError::ConfigError {
                message: format!("Failed to create HTTP client: {}", e),
            })?;

        #[cfg(feature = "deepseek_api")]
        let ext_client = {
            // Use the external client when the base_url targets the official DeepSeek API host.
            if is_official_deepseek_host(config.base_url.as_str()) {
                match ExtDeepSeekClientBuilder::new(config.api_key.clone())
                    .timeout(config.timeout)
                    .build()
                {
                    Ok(c) => Some(c),
                    Err(e) => {
                        tracing::warn!("Failed to initialize deepseek-api client; falling back to internal HTTP: {}", e);
                        None
                    }
                }
            } else {
                None
            }
        };

        #[cfg(not(feature = "deepseek_api"))]
        let _ext_client: Option<()> = None;

        Ok(Self {
            client,
            config,
            #[cfg(feature = "deepseek_api")]
            ext_client,
        })
    }
    /// Send a request to the DeepSeek API with retry logic
    #[allow(dead_code)]
    pub async fn send_request(&self, user_input: &str) -> Result<DeepSeekResponse, DeepSeekError> {
        let mut attempts = 0;
        let max_attempts = 3;
        let mut backoff = Duration::from_millis(500);

        loop {
            match self.send_request_once(user_input).await {
                Ok(response) => return Ok(response),
                Err(e)
                    if (e.is_server_busy() || e.is_network_error())
                        && attempts < max_attempts - 1 =>
                {
                    attempts += 1;
                    tracing::warn!(
                        "Request attempt {} failed: {}, retrying in {:?}",
                        attempts,
                        e,
                        backoff
                    );
                    tokio::time::sleep(backoff).await;
                    backoff = backoff.saturating_mul(2);
                }
                Err(e) => return Err(e),
            }
        }
    }

    /// Send a single request to the DeepSeek API and return a structured response
    #[allow(dead_code)]
    async fn send_request_once(&self, user_input: &str) -> Result<DeepSeekResponse, DeepSeekError> {
        let current_timestamp = Utc::now().to_rfc3339();

        let system_prompt = "You are a helpful assistant that always responds with valid JSON in the specified format.";
        let json_format_prompt = format!(
            r#"
                Please respond with a JSON object containing the following fields:
                {{
                "title": "A concise title for the topic (string)",
                "description": "A brief description or summary (string)",
                "content": "The main content or detailed response (string)",
                "category": "Optional category classification (string or null)",
                "timestamp": "Current response timestamp: {} (string)",
                "confidence": "Optional confidence score between 0.0 and 1.0 (number or null)"
                }}

                Make sure to provide valid JSON format in your response. Use the provided timestamp as the current response time.
                Do not include any other text or comments in your response.
            "#,
            current_timestamp
        );
        let combined_prompt = format!("{}\n\n{}", user_input, json_format_prompt);

        let raw = self
            .send_messages_raw(vec![
                ChatMessage { role: "system".to_string(), content: system_prompt.to_string() },
                ChatMessage { role: "user".to_string(), content: combined_prompt },
            ])
            .await?;

        let parsed_response: DeepSeekResponse = serde_json::from_str(&raw).map_err(|e| {
            DeepSeekError::ParseError {
                message: format!("Failed to parse JSON response from DeepSeek: {}", e),
            }
        })?;

        Ok(parsed_response)
    }

    /// Map reqwest errors to our custom error types
    fn map_reqwest_error(&self, error: reqwest::Error) -> DeepSeekError {
        if error.is_timeout() {
            return DeepSeekError::Timeout {
                seconds: self.config.timeout,
            };
        }

        if error.is_connect() {
            return DeepSeekError::NetworkError {
                message: "Failed to connect to server".to_string(),
            };
        }

        if error.is_request() {
            return DeepSeekError::NetworkError {
                message: "Request failed".to_string(),
            };
        }

        // Check for specific network-related errors
        let error_msg = error.to_string().to_lowercase();
        if error_msg.contains("dns") {
            return DeepSeekError::NetworkError {
                message: "DNS resolution failed".to_string(),
            };
        }

        if error_msg.contains("connection refused") {
            return DeepSeekError::NetworkError {
                message: "Connection refused by server".to_string(),
            };
        }

        if error_msg.contains("network") || error_msg.contains("connection") {
            return DeepSeekError::NetworkError {
                message: error.to_string(),
            };
        }

        DeepSeekError::NetworkError {
            message: format!("Request error: {}", error),
        }
    }

    /// Handle error responses from the server
    async fn handle_error_response(
        &self,
        status: StatusCode,
        response: reqwest::Response,
    ) -> DeepSeekError {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());

        match status {
            StatusCode::TOO_MANY_REQUESTS => DeepSeekError::ServerBusy,
            StatusCode::SERVICE_UNAVAILABLE => DeepSeekError::ServerBusy,
            StatusCode::BAD_GATEWAY | StatusCode::GATEWAY_TIMEOUT => DeepSeekError::ServerBusy,
            _ => DeepSeekError::ApiError {
                status: status.as_u16(),
                message: error_text,
            },
        }
    }

    /// Send arbitrary chat messages and return the raw assistant content string.
    /// The response is requested as a JSON object to encourage strict JSON outputs.
    pub async fn send_messages_raw(
        &self,
        messages: Vec<ChatMessage>,
    ) -> Result<String, DeepSeekError> {
        // If the external client is available (official host and feature enabled), use it.
        #[cfg(feature = "deepseek_api")]
        {
            if let Some(ext) = &self.ext_client {
                // Map our ChatMessage types to deepseek-api MessageRequest
                let mapped: Vec<ExtMessageRequest> = messages
                    .iter()
                    .map(|m| match m.role.as_str() {
                        "system" => ExtMessageRequest::sys(&m.content),
                        "assistant" => {
                            ExtMessageRequest::Assistant(deepseek_api::response::AssistantMessage::new(&m.content))
                        }
                        _ => ExtMessageRequest::user(&m.content),
                    })
                    .collect();

                // Build request enforcing JSON response format to encourage structured outputs
                // Builder in this crate is by-value; use consuming setters and rebind
                let mut builder = ExtCompletionsRequestBuilder::new(&mapped)
                    .response_format(deepseek_api::request::ResponseType::Json)
                    .use_model(map_model_string_to_ext(&self.config.model));

                let clamped_max = self.config.max_tokens.min(8192).max(1);
                builder = builder.max_tokens(clamped_max).unwrap();
                let clamped_temp = self.config.temperature.max(0.0).min(2.0);
                builder = builder.temperature(clamped_temp).unwrap();

                // Execute
                let resp = ext
                    .send_completion_request(builder)
                    .await
                    .map_err(|e| DeepSeekError::ApiError { status: 0, message: e.to_string() })?;

                return match resp {
                    ExtChatResponse::Full(full) => {
                        let first = full.choices.get(0).ok_or_else(|| DeepSeekError::ParseError { message: "No choices in API response".to_string() })?;
                        if let Some(msg) = &first.message { Ok(msg.content.clone()) }
                        else if let Some(text) = &first.text { Ok(text.clone()) }
                        else { Err(DeepSeekError::ParseError { message: "Empty content in API response".to_string() }) }
                    }
                    ExtChatResponse::Stream(_) => {
                        // We didn't request streaming; treat as error if encountered.
                        Err(DeepSeekError::ParseError { message: "Unexpected streaming response".to_string() })
                    }
                };
            }
        }

        // Fallback: internal HTTP implementation honoring custom base_url (e.g., tests)
        self.send_messages_raw_internal(messages).await
    }
}

impl DeepSeekClient {
    async fn send_messages_raw_internal(
        &self,
        messages: Vec<ChatMessage>,
    ) -> Result<String, DeepSeekError> {
        let request = ChatRequest {
            model: self.config.model.clone(),
            messages,
            response_format: ResponseFormat { format_type: "json_object".to_string() },
            max_tokens: self.config.max_tokens,
            temperature: self.config.temperature,
            stop: None,
        };

        let response = self
            .client
            .post(format!("{}/chat/completions", self.config.base_url))
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| self.map_reqwest_error(e))?;

        let status = response.status();
        if !status.is_success() {
            return Err(self.handle_error_response(status, response).await);
        }

        let api_response: ApiResponse = response
            .json()
            .await
            .map_err(|e| DeepSeekError::ParseError { message: format!("Failed to parse API response: {}", e) })?;

        if api_response.choices.is_empty() {
            return Err(DeepSeekError::ParseError { message: "No choices in API response".to_string() });
        }

        Ok(api_response.choices[0].message.content.clone())
    }
}

#[cfg(feature = "deepseek_api")]
fn is_official_deepseek_host(base_url: &str) -> bool {
    // Accept both https://api.deepseek.com and https://api.deepseek.com/v1
    base_url.starts_with("https://api.deepseek.com")
}

#[cfg(not(feature = "deepseek_api"))]
fn is_official_deepseek_host(_base_url: &str) -> bool {
    false
}

#[cfg(feature = "deepseek_api")]
fn map_model_string_to_ext(model: &str) -> ExtModelType {
    match model {
        "deepseek-reasoner" => ExtModelType::DeepSeekReasoner,
        _ => ExtModelType::DeepSeekChat,
    }
}

#[cfg(not(feature = "deepseek_api"))]
fn map_model_string_to_ext(_model: &str) {}
