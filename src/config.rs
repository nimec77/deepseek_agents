use anyhow::{Context, Result};
use std::env;

const DEFAULT_BASE_URL: &str = "https://api.deepseek.com/v1";
const DEFAULT_MODEL: &str = "deepseek-chat";
const DEFAULT_MAX_TOKENS: u32 = 4096;
const DEFAULT_TEMPERATURE: f32 = 0.7;
const DEFAULT_TIMEOUT: u64 = 180;

/// Configuration structure for the DeepSeek client
#[derive(Debug, Clone)]
pub struct Config {
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    pub max_tokens: u32,
    pub temperature: f32,
    pub timeout: u64,
}

impl Config {
    /// Load configuration from environment variables
    pub fn load() -> Result<Self> {
        let api_key = env::var("DEEPSEEK_API_KEY")
            .context("DEEPSEEK_API_KEY environment variable not set")?;

        let base_url =
            env::var("DEEPSEEK_BASE_URL").unwrap_or_else(|_| DEFAULT_BASE_URL.to_string());

        let model = env::var("DEEPSEEK_MODEL").unwrap_or_else(|_| DEFAULT_MODEL.to_string());

        let max_tokens = env::var("DEEPSEEK_MAX_TOKENS")
            .unwrap_or_else(|_| DEFAULT_MAX_TOKENS.to_string())
            .parse::<u32>()
            .context("DEEPSEEK_MAX_TOKENS must be a valid number")?;

        let temperature = env::var("DEEPSEEK_TEMPERATURE")
            .unwrap_or_else(|_| DEFAULT_TEMPERATURE.to_string())
            .parse::<f32>()
            .context("DEEPSEEK_TEMPERATURE must be a valid number")?;

        let timeout = env::var("DEEPSEEK_TIMEOUT")
            .unwrap_or_else(|_| DEFAULT_TIMEOUT.to_string())
            .parse::<u64>()
            .context("DEEPSEEK_TIMEOUT must be a valid number")?;

        Ok(Self {
            api_key,
            base_url,
            model,
            max_tokens,
            temperature,
            timeout,
        })
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        if self.api_key.is_empty() {
            anyhow::bail!("API key cannot be empty");
        }

        if self.temperature < 0.0 || self.temperature > 2.0 {
            anyhow::bail!("Temperature must be between 0.0 and 2.0");
        }

        if self.max_tokens == 0 {
            anyhow::bail!("Max tokens must be greater than 0");
        }

        if self.timeout == 0 {
            anyhow::bail!("Timeout must be greater than 0");
        }

        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            base_url: DEFAULT_BASE_URL.to_string(),
            model: DEFAULT_MODEL.to_string(),
            max_tokens: DEFAULT_MAX_TOKENS,
            temperature: DEFAULT_TEMPERATURE,
            timeout: DEFAULT_TIMEOUT,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::sync::{Mutex, OnceLock};

    // Serialize tests that mutate process env vars
    static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    fn lock_env() -> std::sync::MutexGuard<'static, ()> {
        ENV_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .expect("env mutex poisoned")
    }

    const ENV_KEYS: &[&str] = &[
        "DEEPSEEK_API_KEY",
        "DEEPSEEK_BASE_URL",
        "DEEPSEEK_MODEL",
        "DEEPSEEK_MAX_TOKENS",
        "DEEPSEEK_TEMPERATURE",
        "DEEPSEEK_TIMEOUT",
    ];

    fn clear_env() {
        for key in ENV_KEYS {
            unsafe {
                env::remove_var(key);
            }
        }
    }

    #[test]
    fn load_missing_api_key_errors() {
        let _guard = lock_env();
        clear_env();

        let err = Config::load().unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("DEEPSEEK_API_KEY environment variable not set"),
            "unexpected error: {}",
            msg
        );
    }

    #[test]
    fn load_defaults_when_only_api_key_set() -> Result<()> {
        let _guard = lock_env();
        clear_env();
        unsafe {
            env::set_var("DEEPSEEK_API_KEY", "test_key");
        }

        let config = Config::load()?;
        assert_eq!(config.api_key, "test_key");
        assert_eq!(config.base_url, DEFAULT_BASE_URL);
        assert_eq!(config.model, DEFAULT_MODEL);
        assert_eq!(config.max_tokens, DEFAULT_MAX_TOKENS);
        assert!((config.temperature - DEFAULT_TEMPERATURE).abs() < f32::EPSILON);
        assert_eq!(config.timeout, DEFAULT_TIMEOUT);

        // Also ensure validate passes on defaults
        config.validate()?;
        Ok(())
    }

    #[test]
    fn load_overrides_all_envs() -> Result<()> {
        let _guard = lock_env();
        clear_env();
        unsafe {
            env::set_var("DEEPSEEK_API_KEY", "k");
            env::set_var("DEEPSEEK_BASE_URL", "https://example.com");
            env::set_var("DEEPSEEK_MODEL", "custom-model");
            env::set_var("DEEPSEEK_MAX_TOKENS", "1234");
            env::set_var("DEEPSEEK_TEMPERATURE", "1.25");
            env::set_var("DEEPSEEK_TIMEOUT", "33");
        }

        let config = Config::load()?;
        assert_eq!(config.api_key, "k");
        assert_eq!(config.base_url, "https://example.com");
        assert_eq!(config.model, "custom-model");
        assert_eq!(config.max_tokens, 1234);
        assert!((config.temperature - 1.25).abs() < f32::EPSILON);
        assert_eq!(config.timeout, 33);
        Ok(())
    }

    #[test]
    fn load_invalid_max_tokens_errors() {
        let _guard = lock_env();
        clear_env();
        unsafe {
            env::set_var("DEEPSEEK_API_KEY", "k");
            env::set_var("DEEPSEEK_MAX_TOKENS", "not-a-number");
        }

        let err = Config::load().unwrap_err();
        assert!(
            err.to_string()
                .contains("DEEPSEEK_MAX_TOKENS must be a valid number"),
            "unexpected error: {}",
            err
        );
    }

    #[test]
    fn load_invalid_temperature_errors() {
        let _guard = lock_env();
        clear_env();
        unsafe {
            env::set_var("DEEPSEEK_API_KEY", "k");
            env::set_var("DEEPSEEK_TEMPERATURE", "abc");
        }

        let err = Config::load().unwrap_err();
        assert!(
            err.to_string()
                .contains("DEEPSEEK_TEMPERATURE must be a valid number"),
            "unexpected error: {}",
            err
        );
    }

    #[test]
    fn load_invalid_timeout_errors() {
        let _guard = lock_env();
        clear_env();
        unsafe {
            env::set_var("DEEPSEEK_API_KEY", "k");
            env::set_var("DEEPSEEK_TIMEOUT", "oops");
        }

        let err = Config::load().unwrap_err();
        assert!(
            err.to_string()
                .contains("DEEPSEEK_TIMEOUT must be a valid number"),
            "unexpected error: {}",
            err
        );
    }

    #[test]
    fn validate_rejects_empty_api_key() {
        let config = Config {
            api_key: String::new(),
            base_url: DEFAULT_BASE_URL.to_string(),
            model: DEFAULT_MODEL.to_string(),
            max_tokens: DEFAULT_MAX_TOKENS,
            temperature: DEFAULT_TEMPERATURE,
            timeout: DEFAULT_TIMEOUT,
        };
        let err = config.validate().unwrap_err();
        assert!(
            err.to_string().contains("API key cannot be empty"),
            "unexpected error: {}",
            err
        );
    }

    #[test]
    fn validate_rejects_temperature_out_of_range() {
        let mut config = Config {
            api_key: "k".to_string(),
            base_url: DEFAULT_BASE_URL.to_string(),
            model: DEFAULT_MODEL.to_string(),
            max_tokens: DEFAULT_MAX_TOKENS,
            temperature: -0.1,
            timeout: DEFAULT_TIMEOUT,
        };
        let err = config.validate().unwrap_err();
        assert!(
            err.to_string()
                .contains("Temperature must be between 0.0 and 2.0"),
            "unexpected error: {}",
            err
        );

        config.temperature = 2.1;
        let err = config.validate().unwrap_err();
        assert!(
            err.to_string()
                .contains("Temperature must be between 0.0 and 2.0"),
            "unexpected error: {}",
            err
        );
    }

    #[test]
    fn validate_rejects_zero_values() {
        let mut config = Config {
            api_key: "k".to_string(),
            base_url: DEFAULT_BASE_URL.to_string(),
            model: DEFAULT_MODEL.to_string(),
            max_tokens: 0,
            temperature: DEFAULT_TEMPERATURE,
            timeout: DEFAULT_TIMEOUT,
        };
        let err = config.validate().unwrap_err();
        assert!(
            err.to_string()
                .contains("Max tokens must be greater than 0"),
            "unexpected error: {}",
            err
        );

        config.max_tokens = 1;
        config.timeout = 0;
        let err = config.validate().unwrap_err();
        assert!(
            err.to_string().contains("Timeout must be greater than 0"),
            "unexpected error: {}",
            err
        );
    }
}
