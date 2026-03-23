use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;

use crate::model_descriptor::ModelDescriptor;

pub const DEFAULT_ANTHROPIC_LOCATION: &str = "us-east5";

/// Configuration for Vertex AI client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// GCP project ID
    pub project_id: String,

    /// GCP region (e.g., "us-central1")
    pub region: String,

    /// API version (v1 or v1beta1)
    pub api_version: String,

    /// Request timeout in seconds
    pub timeout_secs: u64,

    /// Max retries for failed requests
    pub max_retries: u32,

    /// Enable debug logging
    pub debug: bool,

    /// Model to use (e.g., "gemini-1.5-pro" or "claude-sonnet-4-5")
    pub model: String,

    /// Optional publisher-specific location overrides (e.g., Anthropic -> us-east5)
    #[serde(default)]
    pub publisher_locations: HashMap<String, String>,

    /// Optional override for the Vertex API base URL (useful for mocks/tests)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_url_override: Option<String>,
}

impl Config {
    /// Create a new configuration with defaults
    ///
    /// # Errors
    ///
    /// Returns an error when the required environment variables cannot be read.
    pub fn new() -> Result<Self> {
        let mut publisher_locations = Self::publisher_locations_from_env();
        Self::ensure_default_publisher_locations(&mut publisher_locations);

        Ok(Self {
            project_id: Self::get_project_id()?,
            region: Self::get_region(),
            api_version: "v1".to_string(),
            timeout_secs: 60,
            max_retries: Self::get_max_retries(),
            debug: env::var("DEBUG").is_ok(),
            model: env::var("VERTEX_MODEL").unwrap_or_else(|_| "gemini-1.5-flash".to_string()),
            publisher_locations,
            base_url_override: env::var("VERTEX_BASE_URL").ok(),
        })
    }

    /// Get project ID from environment
    fn get_project_id() -> Result<String> {
        env::var("VERTEX_PROJECT_ID")
            .or_else(|_| env::var("GCP_PROJECT_ID"))
            .or_else(|_| env::var("GOOGLE_CLOUD_PROJECT"))
            .context("Project ID not found. Set VERTEX_PROJECT_ID environment variable.")
    }

    /// Get region from environment
    fn get_region() -> String {
        env::var("VERTEX_REGION")
            .or_else(|_| env::var("VERTEX_LOCATION"))
            .or_else(|_| env::var("VERTEX_ANTHROPIC_LOCATION"))
            .or_else(|_| env::var("GCP_REGION"))
            .or_else(|_| env::var("GOOGLE_CLOUD_REGION"))
            .unwrap_or_else(|_| "us-central1".to_string())
    }

    fn get_max_retries() -> u32 {
        env::var("VERTEX_MAX_RETRIES").ok().and_then(|value| value.parse::<u32>().ok()).unwrap_or(3)
    }

    /// Load configuration from environment variables
    ///
    /// # Errors
    ///
    /// Returns an error when required environment variables are missing or
    /// invalid.
    pub fn from_env() -> Result<Self> {
        let mut publisher_locations = Self::publisher_locations_from_env();
        Self::ensure_default_publisher_locations(&mut publisher_locations);

        Ok(Self {
            project_id: Self::get_project_id()?,
            region: Self::get_region(),
            api_version: env::var("VERTEX_API_VERSION").unwrap_or_else(|_| "v1".to_string()),
            timeout_secs: env::var("VERTEX_TIMEOUT")
                .unwrap_or_else(|_| "60".to_string())
                .parse()
                .unwrap_or(60),
            max_retries: Self::get_max_retries(),
            debug: env::var("DEBUG").is_ok() || env::var("VERTEX_DEBUG").is_ok(),
            model: env::var("VERTEX_MODEL").unwrap_or_else(|_| "gemini-1.5-flash".to_string()),
            publisher_locations,
            base_url_override: env::var("VERTEX_BASE_URL").ok(),
        })
    }

    /// Load configuration from a TOML file
    ///
    /// # Errors
    ///
    /// Returns an error when reading or parsing the file fails.
    pub fn from_file(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let path_ref = path.as_ref();
        let content = std::fs::read_to_string(path_ref)
            .with_context(|| format!("Failed to read config file: {}", path_ref.display()))?;

        let mut parsed: Self = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {}", path_ref.display()))?;

        Self::ensure_default_publisher_locations(&mut parsed.publisher_locations);

        // Allow environment variables to override Anthropics location even when using config files.
        if let Some(env_override) = Self::anthropic_location_override() {
            parsed.publisher_locations.insert("anthropic".to_string(), env_override);
        }

        if let Ok(overridden) = env::var("VERTEX_BASE_URL") {
            parsed.base_url_override = Some(overridden);
        }

        Ok(parsed)
    }

    /// Get the base URL for the Vertex AI API
    #[must_use]
    pub fn base_url(&self) -> String {
        if let Some(override_url) = &self.base_url_override {
            return override_url.clone();
        }

        if let Ok(overridden) = env::var("VERTEX_BASE_URL") {
            return overridden;
        }
        endpoint_for_region(&self.region)
    }

    /// Get the full model name
    #[must_use]
    pub fn model_name(&self) -> String {
        ModelDescriptor::parse(&self.model)
            .map_or_else(|_| self.model.clone(), |descriptor| descriptor.relative_path())
    }

    /// Get the project location path
    #[must_use]
    pub fn project_location(&self) -> String {
        format!("projects/{}/locations/{}", self.project_id, self.region)
    }

    /// Validate the configuration
    ///
    /// # Errors
    ///
    /// Returns an error when required configuration fields are missing or
    /// invalid.
    pub fn validate(&self) -> Result<()> {
        if self.project_id.is_empty() {
            anyhow::bail!("Project ID is required");
        }

        if self.region.is_empty() {
            anyhow::bail!("Region is required");
        }

        if !["v1", "v1beta1"].contains(&self.api_version.as_str()) {
            anyhow::bail!("Invalid API version. Must be 'v1' or 'v1beta1'");
        }

        Ok(())
    }

    fn publisher_locations_from_env() -> HashMap<String, String> {
        let mut map = HashMap::new();
        if let Some(location) = Self::anthropic_location_override() {
            map.insert("anthropic".to_string(), location);
        }
        map
    }

    fn ensure_default_publisher_locations(map: &mut HashMap<String, String>) {
        if map.get("anthropic").is_none_or(|value| value.trim().is_empty()) {
            map.insert("anthropic".to_string(), DEFAULT_ANTHROPIC_LOCATION.to_string());
        }
    }

    pub(crate) fn anthropic_location_override() -> Option<String> {
        for key in ["VERTEX_ANTHROPIC_LOCATION", "VERTEX_ANTHROPIC_REGION"] {
            if let Ok(value) = env::var(key) {
                let trimmed = value.trim();
                if !trimmed.is_empty() {
                    return Some(trimmed.to_string());
                }
            }
        }
        None
    }
}

impl Default for Config {
    fn default() -> Self {
        let mut publisher_locations = HashMap::new();
        publisher_locations.insert("anthropic".to_string(), DEFAULT_ANTHROPIC_LOCATION.to_string());

        Self {
            project_id: String::new(),
            region: "us-central1".to_string(),
            api_version: "v1".to_string(),
            timeout_secs: 60,
            max_retries: Self::get_max_retries(),
            debug: false,
            model: "gemini-1.5-flash".to_string(),
            publisher_locations,
            base_url_override: None,
        }
    }
}

/// Resolve the Vertex endpoint hostname for a given region/location.
#[must_use]
pub(crate) fn endpoint_for_region(region: &str) -> String {
    if region.eq_ignore_ascii_case("global") {
        "https://aiplatform.googleapis.com".to_string()
    } else {
        format!("https://{}-aiplatform.googleapis.com", region.to_ascii_lowercase())
    }
}

/// Environment variable configuration helper
pub struct EnvConfig;

impl EnvConfig {
    /// Check if all required environment variables are set
    ///
    /// # Errors
    ///
    /// Returns an error listing the missing environment variables.
    pub fn check_required() -> Result<()> {
        let required = [
            ("GCP_PRIVATE_KEY", "Private key from service account"),
            ("GCP_CLIENT_EMAIL", "Service account email"),
            ("GCP_CLIENT_ID", "Service account client ID"),
            ("VERTEX_PROJECT_ID", "GCP project ID"),
        ];

        let mut missing = Vec::new();
        for &(var, desc) in &required {
            if env::var(var).is_err() {
                missing.push(format!("{var} ({desc})"));
            }
        }

        if !missing.is_empty() {
            anyhow::bail!("Missing required environment variables:\n{}", missing.join("\n"));
        }

        Ok(())
    }

    /// Print current configuration
    pub fn print_config() {
        println!("Vertex AI SDK Configuration:");
        println!("============================");

        let vars = [
            ("VERTEX_PROJECT_ID", "Project ID"),
            ("VERTEX_REGION", "Region"),
            ("VERTEX_MODEL", "Default Model"),
            ("VERTEX_API_VERSION", "API Version"),
            ("GCP_CLIENT_EMAIL", "Service Account"),
        ];

        for &(var, desc) in &vars {
            match env::var(var) {
                Ok(val) => {
                    if var.contains("KEY") || var.contains("SECRET") {
                        println!("{desc}: [REDACTED]");
                    } else {
                        println!("{desc}: {val}");
                    }
                }
                Err(_) => println!("{desc}: [NOT SET]"),
            }
        }
    }

    /// Load .env file if it exists
    pub fn load_dotenv() {
        if dotenvy::dotenv().is_ok() {
            eprintln!("Loaded .env file");
        }
    }
}
