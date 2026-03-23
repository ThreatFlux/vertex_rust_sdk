use anyhow::{anyhow, Result};
use clap::Parser;
use std::env;

use threatflux_vertex_rust_sdk::chat_core::config::{
    validate_max_tokens, validate_temperature, ChatConfig, DEFAULT_LOCATION, DEFAULT_MAX_TOKENS,
    DEFAULT_MODEL, DEFAULT_TEMPERATURE,
};

/// CLI arguments for the chat binary.
#[derive(Parser, Debug)]
#[command(name = "vertex-chat")]
#[command(about = "Interactive chat with Vertex AI models", version)]
pub struct Cli {
    /// Project ID
    #[arg(short, long)]
    pub project: Option<String>,

    /// Location/region
    #[arg(short, long)]
    pub location: Option<String>,

    /// Model to use
    #[arg(short, long, default_value = DEFAULT_MODEL)]
    pub model: String,

    /// Temperature (0.0 to 2.0)
    #[arg(short = 't', long, default_value_t = DEFAULT_TEMPERATURE)]
    pub temperature: f32,

    /// Max output tokens
    #[arg(short = 'o', long, default_value_t = DEFAULT_MAX_TOKENS)]
    pub max_tokens: i32,

    /// System instruction
    #[arg(short, long)]
    pub system: Option<String>,

    /// Enable debug logging
    #[arg(short, long)]
    pub debug: bool,
}

impl TryFrom<Cli> for ChatConfig {
    type Error = anyhow::Error;

    fn try_from(cli: Cli) -> Result<Self> {
        let project = cli
            .project
            .or_else(|| env::var("VERTEX_PROJECT").ok())
            .ok_or_else(|| anyhow!("Project ID required (--project or VERTEX_PROJECT)"))?;

        let location = cli
            .location
            .or_else(|| env::var("VERTEX_LOCATION").ok())
            .unwrap_or_else(|| DEFAULT_LOCATION.to_string());

        validate_temperature(cli.temperature)?;
        validate_max_tokens(cli.max_tokens)?;

        Self::new(
            project,
            location,
            cli.model,
            cli.temperature,
            cli.max_tokens,
            cli.system,
            cli.debug,
        )
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
    fn builds_from_cli_with_env_fallbacks() {
        let _guard = env_lock().lock().unwrap();
        let project_key = "VERTEX_PROJECT";
        let location_key = "VERTEX_LOCATION";

        let original_project = env::var(project_key).ok();
        let original_location = env::var(location_key).ok();

        env::set_var(project_key, "env-project");
        env::remove_var(location_key);

        let cli = Cli {
            project: None,
            location: None,
            model: "model-a".to_string(),
            temperature: 1.0,
            max_tokens: 100,
            system: Some("sys".to_string()),
            debug: false,
        };

        let config = ChatConfig::try_from(cli).expect("config builds");
        assert_eq!(config.project, "env-project");
        assert_eq!(config.location, DEFAULT_LOCATION);

        if let Some(value) = original_project {
            env::set_var(project_key, value);
        } else {
            env::remove_var(project_key);
        }

        if let Some(value) = original_location {
            env::set_var(location_key, value);
        } else {
            env::remove_var(location_key);
        }
    }

    #[test]
    fn requires_project() {
        let _guard = env_lock().lock().unwrap();
        let original_project = env::var("VERTEX_PROJECT").ok();
        env::remove_var("VERTEX_PROJECT");

        let cli = Cli {
            project: None,
            location: None,
            model: "model-a".to_string(),
            temperature: 1.0,
            max_tokens: 100,
            system: None,
            debug: false,
        };

        let result = ChatConfig::try_from(cli);
        assert!(result.is_err());

        if let Some(value) = original_project {
            env::set_var("VERTEX_PROJECT", value);
        }
    }
}
