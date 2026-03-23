use std::{env, error::Error, fmt};

use threatflux_vertex_rust_sdk::{GenerationConfig, Tool};

use crate::schemas::build_tool;

const DEFAULT_MODEL: &str = "gemini-2.0-flash-001";
const DEFAULT_PROMPT: &str = "What's the weather like in Boston? Also, what's 25 multiplied by 4?";

#[derive(Clone, Debug)]
pub struct ExampleConfig {
    pub project_id: String,
    pub location: String,
    pub generation_config: GenerationConfig,
    pub tool: Tool,
    pub model: String,
    pub prompt: String,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ConfigError {
    MissingProjectId,
    EmptyProjectId,
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingProjectId => {
                write!(f, "Set GOOGLE_CLOUD_PROJECT environment variable")
            }
            Self::EmptyProjectId => {
                write!(f, "GOOGLE_CLOUD_PROJECT cannot be empty")
            }
        }
    }
}

impl Error for ConfigError {}

impl ExampleConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        let project_id =
            env::var("GOOGLE_CLOUD_PROJECT").map_err(|_| ConfigError::MissingProjectId)?;
        if project_id.trim().is_empty() {
            return Err(ConfigError::EmptyProjectId);
        }

        let location =
            env::var("GOOGLE_CLOUD_LOCATION").unwrap_or_else(|_| "us-central1".to_string());

        Ok(Self {
            project_id,
            location,
            generation_config: default_generation_config(),
            tool: build_tool(),
            model: DEFAULT_MODEL.to_string(),
            prompt: DEFAULT_PROMPT.to_string(),
        })
    }
}

pub fn default_generation_config() -> GenerationConfig {
    GenerationConfig {
        temperature: Some(0.0),
        max_output_tokens: Some(1024),
        ..GenerationConfig::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct EnvGuard {
        project: Option<String>,
        location: Option<String>,
    }

    impl EnvGuard {
        fn new() -> Self {
            Self {
                project: env::var("GOOGLE_CLOUD_PROJECT").ok(),
                location: env::var("GOOGLE_CLOUD_LOCATION").ok(),
            }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            if let Some(value) = &self.project {
                env::set_var("GOOGLE_CLOUD_PROJECT", value);
            } else {
                env::remove_var("GOOGLE_CLOUD_PROJECT");
            }

            if let Some(value) = &self.location {
                env::set_var("GOOGLE_CLOUD_LOCATION", value);
            } else {
                env::remove_var("GOOGLE_CLOUD_LOCATION");
            }
        }
    }

    #[test]
    fn rejects_missing_project_id() {
        let _guard = EnvGuard::new();
        env::remove_var("GOOGLE_CLOUD_PROJECT");
        let error = ExampleConfig::from_env().expect_err("missing project id should fail");
        assert_eq!(error, ConfigError::MissingProjectId);
    }

    #[test]
    fn rejects_empty_project_id() {
        let _guard = EnvGuard::new();
        env::set_var("GOOGLE_CLOUD_PROJECT", "   ");
        let error = ExampleConfig::from_env().expect_err("empty project id should fail");
        assert_eq!(error, ConfigError::EmptyProjectId);
    }

    #[test]
    fn builds_with_defaults_and_location_override() {
        let _guard = EnvGuard::new();
        env::set_var("GOOGLE_CLOUD_PROJECT", "demo-project");
        env::remove_var("GOOGLE_CLOUD_LOCATION");

        let config = ExampleConfig::from_env().expect("config should build");
        assert_eq!(config.project_id, "demo-project");
        assert_eq!(config.location, "us-central1");
        assert_eq!(config.model, DEFAULT_MODEL);
        assert_eq!(config.prompt, DEFAULT_PROMPT);
        assert_eq!(config.generation_config.temperature, Some(0.0));
        assert_eq!(config.generation_config.max_output_tokens, Some(1024));
    }
}
