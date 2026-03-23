use anyhow::{anyhow, Result};
use std::ops::RangeInclusive;

pub const DEFAULT_LOCATION: &str = "us-central1";
pub const DEFAULT_MODEL: &str = "gemini-2.5-flash";
pub const DEFAULT_TEMPERATURE: f32 = 0.9;
pub const DEFAULT_MAX_TOKENS: i32 = 8_192;
pub const DEFAULT_SYSTEM_PROMPT: &str = "You are a helpful AI assistant. Be concise but informative in your responses. If asked about your capabilities, mention that you're powered by Google's Gemini model via the Vertex AI API.";

const TEMP_RANGE: RangeInclusive<f32> = 0.0..=2.0;

#[derive(Clone, Debug, PartialEq)]
pub struct ChatConfig {
    pub project: String,
    pub location: String,
    pub model: String,
    pub temperature: f32,
    pub max_tokens: i32,
    pub system: Option<String>,
    pub debug: bool,
}

impl ChatConfig {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        project: String,
        location: String,
        model: String,
        temperature: f32,
        max_tokens: i32,
        system: Option<String>,
        debug: bool,
    ) -> Result<Self> {
        validate_required(&project, "Project ID")?;
        validate_required(&location, "Location")?;
        validate_temperature(temperature)?;
        validate_max_tokens(max_tokens)?;

        Ok(Self { project, location, model, temperature, max_tokens, system, debug })
    }

    pub fn with_defaults(project: String, location: String) -> Self {
        Self::new(
            project,
            location,
            DEFAULT_MODEL.to_string(),
            DEFAULT_TEMPERATURE,
            DEFAULT_MAX_TOKENS,
            Some(DEFAULT_SYSTEM_PROMPT.to_string()),
            false,
        )
        .expect("default chat configuration is valid")
    }

    pub fn init_logging(&self) -> Result<()> {
        let filter = if self.debug { log::LevelFilter::Debug } else { log::LevelFilter::Warn };

        let init_result = env_logger::Builder::from_default_env().filter_level(filter).try_init();

        match init_result {
            Ok(()) => Ok(()),
            Err(e)
                if e.to_string().to_lowercase().contains("already initialized")
                    || e.to_string().contains("set_logger once") =>
            {
                Ok(())
            }
            Err(e) => Err(e.into()),
        }
    }
}

pub fn validate_temperature(value: f32) -> Result<()> {
    if TEMP_RANGE.contains(&value) {
        Ok(())
    } else {
        Err(anyhow!(
            "Temperature must be between {:.1} and {:.1}",
            TEMP_RANGE.start(),
            TEMP_RANGE.end()
        ))
    }
}

pub fn validate_max_tokens(value: i32) -> Result<()> {
    if value > 0 {
        Ok(())
    } else {
        Err(anyhow!("Max tokens must be greater than zero"))
    }
}

fn validate_required(value: &str, name: &str) -> Result<()> {
    if value.trim().is_empty() {
        Err(anyhow!("{name} required"))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};

    static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    fn env_lock() -> &'static Mutex<()> {
        ENV_LOCK.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn rejects_invalid_temperature() {
        let result = validate_temperature(4.2);
        assert!(result.is_err());
    }

    #[test]
    fn accepts_valid_temperature() {
        assert!(validate_temperature(0.0).is_ok());
        assert!(validate_temperature(1.0).is_ok());
        assert!(validate_temperature(2.0).is_ok());
    }

    #[test]
    fn validates_max_tokens() {
        assert!(validate_max_tokens(1).is_ok());
        assert!(validate_max_tokens(0).is_err());
    }

    #[test]
    fn builds_new_config() {
        let config = ChatConfig::new(
            "project".into(),
            "loc".into(),
            "model".into(),
            1.0,
            10,
            Some("sys".into()),
            true,
        )
        .expect("config builds");

        assert_eq!(config.project, "project");
        assert_eq!(config.location, "loc");
        assert!(config.system.is_some());
        assert!(config.debug);
    }

    #[test]
    fn requires_project_and_location() {
        assert!(
            ChatConfig::new(String::new(), "loc".into(), "m".into(), 0.1, 10, None, false).is_err()
        );

        assert!(
            ChatConfig::new("proj".into(), "   ".into(), "m".into(), 0.1, 10, None, false).is_err()
        );
    }

    #[test]
    fn with_defaults_sets_expected_values() {
        let config = ChatConfig::with_defaults("p".into(), "l".into());
        assert_eq!(config.model, DEFAULT_MODEL);
        assert!((config.temperature - DEFAULT_TEMPERATURE).abs() < f32::EPSILON);
        assert_eq!(config.max_tokens, DEFAULT_MAX_TOKENS);
        assert_eq!(config.system.as_deref(), Some(DEFAULT_SYSTEM_PROMPT));
    }

    #[test]
    fn init_logging_is_idempotent() {
        let _guard = env_lock().lock().unwrap();
        let config = ChatConfig::with_defaults("p".into(), "l".into());
        config.init_logging().expect("first init ok");
        config.init_logging().expect("second init ok");
    }
}
