use crate::common::{EnvError, ExampleEnvironment};
use thiserror::Error;
use threatflux_vertex_rust_sdk::{GenerateContentRequest, GenerationConfig};

pub const DEFAULT_MODEL_ID: &str = "gemini-2.0-flash-001";
pub const DEFAULT_PROMPT: &str = "Write a creative short story about a robot that discovers it can dream. Make it about 500 words.";

#[derive(Debug, Default, PartialEq)]
pub struct StreamingArgs {
    pub model: Option<String>,
    pub temperature: Option<f32>,
}

#[derive(Debug, Error)]
pub enum StreamingArgsError {
    #[error("Missing value for {0}")]
    MissingValue(&'static str),
    #[error("Invalid temperature: {0}")]
    InvalidTemperature(String),
}

#[derive(Debug, Error)]
pub enum StreamingConfigError {
    #[error(transparent)]
    Env(#[from] EnvError),
    #[error(transparent)]
    Args(#[from] StreamingArgsError),
}

#[derive(Debug, Clone)]
pub struct StreamingConfig {
    pub environment: ExampleEnvironment,
    pub model_id: String,
    pub generation: GenerationConfig,
    pub prompt: String,
}

impl StreamingArgs {
    pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Self, StreamingArgsError> {
        let mut parsed = Self::default();
        let mut iter = args.into_iter().skip(1);

        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--model" => {
                    let value = iter.next().ok_or(StreamingArgsError::MissingValue("--model"))?;
                    parsed.model = Some(value);
                }
                "--temperature" => {
                    let value =
                        iter.next().ok_or(StreamingArgsError::MissingValue("--temperature"))?;
                    let temperature: f32 = value
                        .parse()
                        .map_err(|_| StreamingArgsError::InvalidTemperature(value.clone()))?;

                    if !(0.0..=1.0).contains(&temperature) {
                        return Err(StreamingArgsError::InvalidTemperature(value));
                    }

                    parsed.temperature = Some(temperature);
                }
                _ => {}
            }
        }

        Ok(parsed)
    }
}

impl StreamingConfig {
    pub fn from_env() -> Result<Self, StreamingConfigError> {
        let environment = ExampleEnvironment::from_env()?;
        let generation = GenerationConfig {
            temperature: Some(0.8),
            max_output_tokens: Some(2048),
            top_p: Some(0.95),
            top_k: Some(40),
            ..GenerationConfig::default()
        };

        Ok(Self {
            environment,
            model_id: DEFAULT_MODEL_ID.to_string(),
            generation,
            prompt: DEFAULT_PROMPT.to_string(),
        })
    }

    pub fn apply_args(&mut self, args: &StreamingArgs) -> Result<(), StreamingConfigError> {
        if let Some(model) = &args.model {
            if model.trim().is_empty() {
                return Err(StreamingArgsError::MissingValue("--model").into());
            }

            self.model_id.clone_from(model);
        }

        if let Some(temperature) = args.temperature {
            self.generation.temperature = Some(temperature);
        }

        Ok(())
    }

    pub fn build_request(&self) -> GenerateContentRequest {
        GenerateContentRequest::new(self.prompt.clone())
            .with_generation_config(self.generation.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::LazyLock;
    use std::sync::Mutex;

    static ENV_MUTEX: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

    struct EnvGuard {
        project: Option<String>,
        location: Option<String>,
    }

    impl EnvGuard {
        fn new(project: Option<&str>, location: Option<&str>) -> Self {
            let guard = Self {
                project: std::env::var("GOOGLE_CLOUD_PROJECT").ok(),
                location: std::env::var("GOOGLE_CLOUD_LOCATION").ok(),
            };

            if let Some(value) = project {
                std::env::set_var("GOOGLE_CLOUD_PROJECT", value);
            } else {
                std::env::remove_var("GOOGLE_CLOUD_PROJECT");
            }

            if let Some(value) = location {
                std::env::set_var("GOOGLE_CLOUD_LOCATION", value);
            } else {
                std::env::remove_var("GOOGLE_CLOUD_LOCATION");
            }

            guard
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            if let Some(value) = &self.project {
                std::env::set_var("GOOGLE_CLOUD_PROJECT", value);
            } else {
                std::env::remove_var("GOOGLE_CLOUD_PROJECT");
            }

            if let Some(value) = &self.location {
                std::env::set_var("GOOGLE_CLOUD_LOCATION", value);
            } else {
                std::env::remove_var("GOOGLE_CLOUD_LOCATION");
            }
        }
    }

    #[test]
    fn parses_empty_args() {
        let args = StreamingArgs::parse(["cmd".to_string()]).expect("parse should succeed");
        assert_eq!(args, StreamingArgs::default());
    }

    #[test]
    fn parses_model_and_temperature() {
        let args = StreamingArgs::parse([
            "cmd".into(),
            "--model".into(),
            "custom".into(),
            "--temperature".into(),
            "0.4".into(),
        ])
        .expect("parse should succeed");

        assert_eq!(args, StreamingArgs { model: Some("custom".into()), temperature: Some(0.4) });
    }

    #[test]
    fn rejects_missing_temperature() {
        let result = StreamingArgs::parse(["cmd".into(), "--temperature".into()]);
        assert!(matches!(result, Err(StreamingArgsError::MissingValue("--temperature"))));
    }

    #[test]
    fn rejects_out_of_range_temperature() {
        let result = StreamingArgs::parse(["cmd".into(), "--temperature".into(), "2.3".into()]);

        assert!(matches!(
            result,
            Err(StreamingArgsError::InvalidTemperature(temp)) if temp == "2.3"
        ));
    }

    #[test]
    fn builds_default_config_from_env() {
        let _lock = ENV_MUTEX.lock().unwrap();
        let _guard = EnvGuard::new(Some("project"), None);

        let config = StreamingConfig::from_env().expect("config should load");

        assert_eq!(config.environment.project_id, "project");
        assert_eq!(config.environment.location, "us-central1");
        assert_eq!(config.model_id, DEFAULT_MODEL_ID);
        assert_eq!(config.prompt, DEFAULT_PROMPT);
    }

    #[test]
    fn applies_overrides() {
        let _lock = ENV_MUTEX.lock().unwrap();
        let _guard = EnvGuard::new(Some("project"), None);

        let mut config = StreamingConfig::from_env().expect("config should load");
        let args = StreamingArgs { model: Some("alternate".into()), temperature: Some(0.2) };

        config.apply_args(&args).expect("apply should succeed");

        assert_eq!(config.model_id, "alternate");
        assert_eq!(config.generation.temperature, Some(0.2));
    }
}
