use std::time::Duration;

use anyhow::Result;
use chrono::Utc;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::config::Config;

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
    pub fn is_server_busy(&self) -> bool {
        matches!(self, DeepSeekError::ServerBusy)
    }

    /// Check if the error is a network-related issue
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
#[derive(Clone, Debug)]
pub struct DeepSeekClient {
    client: Client,
    config: Config,
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

        Ok(Self { client, config })
    }
    /// Send a request to the DeepSeek API with retry logic
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
    async fn send_request_once(&self, user_input: &str) -> Result<DeepSeekResponse, DeepSeekError> {
        let current_timestamp = Utc::now().to_rfc3339();

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

        let request = ChatRequest {
            model: self.config.model.clone(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: "You are a helpful assistant that always responds with valid JSON in the specified format.".to_string(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: combined_prompt,
                },
            ],
            response_format: ResponseFormat {
                format_type: "json_object".to_string(),
            },
            max_tokens: self.config.max_tokens,
            temperature: self.config.temperature,
            stop: None,
        };

        // Send the request
        let response = self
            .client
            .post(format!("{}/chat/completions", self.config.base_url))
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| self.map_reqwest_error(e))?;

        // Handle HTTP status codes
        let status = response.status();
        if !status.is_success() {
            return Err(self.handle_error_response(status, response).await);
        }

        // Parse the response
        let api_response: ApiResponse =
            response
                .json()
                .await
                .map_err(|e| DeepSeekError::ParseError {
                    message: format!("Failed to parse API response: {}", e),
                })?;

        if api_response.choices.is_empty() {
            return Err(DeepSeekError::ParseError {
                message: "No choices in API response".to_string(),
            });
        }

        let content = &api_response.choices[0].message.content;
        let parsed_response: DeepSeekResponse =
            serde_json::from_str(content).map_err(|e| DeepSeekError::ParseError {
                message: format!("Failed to parse JSON response from DeepSeek: {}", e),
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
        let request = ChatRequest {
            model: self.config.model.clone(),
            messages,
            response_format: ResponseFormat {
                format_type: "json_object".to_string(),
            },
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

        let api_response: ApiResponse =
            response
                .json()
                .await
                .map_err(|e| DeepSeekError::ParseError {
                    message: format!("Failed to parse API response: {}", e),
                })?;

        if api_response.choices.is_empty() {
            return Err(DeepSeekError::ParseError {
                message: "No choices in API response".to_string(),
            });
        }

        Ok(api_response.choices[0].message.content.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::advance;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn build_config(base_url: &str) -> Config {
        Config {
            api_key: "test_key".to_string(),
            base_url: base_url.to_string(),
            model: "test-model".to_string(),
            max_tokens: 256,
            temperature: 0.1,
            timeout: 2,
        }
    }

    fn build_client(base_url: &str) -> DeepSeekClient {
        DeepSeekClient::new(build_config(base_url)).expect("client should be created")
    }

    fn api_success_body(content_json: &str) -> serde_json::Value {
        serde_json::json!({
            "choices": [
                { "message": { "role": "assistant", "content": content_json } }
            ]
        })
    }

    #[test]
    fn new_with_invalid_config_returns_config_error() {
        let bad_config = Config {
            api_key: String::new(),
            base_url: "http://localhost".to_string(),
            model: "m".to_string(),
            max_tokens: 1,
            temperature: 0.0,
            timeout: 1,
        };

        let err = DeepSeekClient::new(bad_config).unwrap_err();
        match err {
            DeepSeekError::ConfigError { message } => {
                assert!(message.contains("API key cannot be empty"))
            }
            other => panic!("expected ConfigError, got {other}"),
        }
    }

    #[tokio::test]
    async fn send_request_success_parses_response() {
        let server = MockServer::start().await;
        let client = build_client(&server.uri());

        let content = serde_json::json!({
            "title": "Hello",
            "description": "World",
            "content": "Body",
            "category": "demo",
            "timestamp": "2024-01-01T00:00:00Z",
            "confidence": 0.9
        })
        .to_string();

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(api_success_body(&content)))
            .mount(&server)
            .await;

        let response = client
            .send_request("please respond in json object format")
            .await
            .expect("request should succeed");

        assert_eq!(response.title, "Hello");
        assert_eq!(response.description, "World");
        assert_eq!(response.content, "Body");
        assert_eq!(response.category.as_deref(), Some("demo"));
        assert_eq!(response.timestamp.as_deref(), Some("2024-01-01T00:00:00Z"));
        assert!((response.confidence.unwrap_or_default() - 0.9).abs() < f32::EPSILON);
    }

    #[tokio::test(start_paused = true)]
    async fn send_request_retries_and_returns_server_busy() {
        let server = MockServer::start().await;
        let client = build_client(&server.uri());

        // Always return 503 to trigger retries and final failure
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(503).set_body_string("busy"))
            .mount(&server)
            .await;

        let task = tokio::spawn({
            let client = client.clone();
            async move { client.send_request("x").await }
        });

        // First backoff: 500ms, second: 1000ms
        advance(Duration::from_millis(500)).await;
        tokio::task::yield_now().await;
        advance(Duration::from_millis(1000)).await;
        tokio::task::yield_now().await;

        let err = task.await.expect("join ok").expect_err("should fail");
        match err {
            DeepSeekError::ServerBusy => {}
            DeepSeekError::ApiError { status: 503, .. } => {}
            DeepSeekError::Timeout { .. } => {}
            other => panic!("expected ServerBusy, 503 ApiError, or Timeout, got {other}"),
        }
    }

    #[tokio::test]
    async fn send_messages_raw_maps_http_errors() {
        let server = MockServer::start().await;
        let client = build_client(&server.uri());

        // 400 -> ApiError
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(400).set_body_string("bad req"))
            .mount(&server)
            .await;

        let err = client
            .send_messages_raw(vec![ChatMessage {
                role: "user".to_string(),
                content: "hi".to_string(),
            }])
            .await
            .expect_err("should map to ApiError");

        match err {
            DeepSeekError::ApiError { status, message } => {
                assert_eq!(status, 400);
                assert!(message.contains("bad req"));
            }
            other => panic!("expected ApiError, got {other}"),
        }
    }

    #[tokio::test]
    async fn send_request_empty_choices_is_parse_error() {
        let server = MockServer::start().await;
        let client = build_client(&server.uri());

        let body = serde_json::json!({ "choices": [] });
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(&server)
            .await;

        let err = client
            .send_request("x")
            .await
            .expect_err("should be parse error");
        assert!(matches!(err, DeepSeekError::ParseError { .. }));
    }

    #[tokio::test]
    async fn send_request_invalid_json_in_content_is_parse_error() {
        let server = MockServer::start().await;
        let client = build_client(&server.uri());

        let content = "not-json";
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(api_success_body(content)))
            .mount(&server)
            .await;

        let err = client
            .send_request("x")
            .await
            .expect_err("should be parse error");
        assert!(matches!(err, DeepSeekError::ParseError { .. }));
    }

    #[tokio::test(start_paused = true)]
    async fn send_messages_raw_times_out() {
        let server = MockServer::start().await;
        let mut cfg = build_config(&server.uri());
        cfg.timeout = 1; // seconds
        let client = DeepSeekClient::new(cfg).unwrap();

        // Delay response beyond client timeout
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_delay(Duration::from_secs(2))
                    .set_body_json(api_success_body(
                        &serde_json::json!({
                            "title": "t",
                            "description": "d",
                            "content": "c",
                            "category": null,
                            "timestamp": "2024-01-01T00:00:00Z",
                            "confidence": 0.5
                        })
                        .to_string(),
                    )),
            )
            .mount(&server)
            .await;

        let task = tokio::spawn({
            let client = client.clone();
            async move {
                client
                    .send_messages_raw(vec![ChatMessage {
                        role: "user".to_string(),
                        content: "hello".to_string(),
                    }])
                    .await
            }
        });

        advance(Duration::from_secs(2)).await;
        tokio::task::yield_now().await;

        let err = task.await.unwrap().expect_err("should timeout");
        match err {
            DeepSeekError::Timeout { seconds } => assert_eq!(seconds, 1),
            other => panic!("expected Timeout, got {other}"),
        }
    }
}
