use serde::{Deserialize, Serialize};

/// Thinking levels supported by Gemini models with thinking mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ThinkingLevel {
    Low,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThinkingBudgetConfig {
    /// Thinking budget in tokens (0-32_768), -1 for auto, 0 to disable.
    #[serde(rename = "thinkingBudget")]
    pub thinking_budget: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThinkingLevelConfig {
    #[serde(rename = "thinkingLevel")]
    pub thinking_level: ThinkingLevel,
}

/// Thinking configuration for models that support thinking mode.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ThinkingConfig {
    Budget(ThinkingBudgetConfig),
    Level(ThinkingLevelConfig),
}

impl ThinkingConfig {
    /// Create new thinking config with specific budget.
    #[must_use]
    pub fn with_budget(budget: i32) -> Self {
        Self::Budget(ThinkingBudgetConfig { thinking_budget: budget.clamp(0, 32_768) })
    }

    /// Enable thinking with automatic budget control (-1).
    #[must_use]
    pub const fn auto() -> Self {
        Self::Budget(ThinkingBudgetConfig { thinking_budget: -1 })
    }

    /// Disable thinking mode via budget 0.
    #[must_use]
    pub const fn disabled() -> Self {
        Self::Budget(ThinkingBudgetConfig { thinking_budget: 0 })
    }

    /// Default thinking config with a moderate 1024 token budget.
    #[must_use]
    pub fn default_budget() -> Self {
        Self::with_budget(1024)
    }

    /// Enable thinking with a specified level.
    #[must_use]
    pub const fn with_level(level: ThinkingLevel) -> Self {
        Self::Level(ThinkingLevelConfig { thinking_level: level })
    }

    /// Convenience for enabling low-level thinking.
    #[must_use]
    pub const fn low() -> Self {
        Self::with_level(ThinkingLevel::Low)
    }

    /// Convenience for enabling high-level thinking.
    #[must_use]
    pub const fn high() -> Self {
        Self::with_level(ThinkingLevel::High)
    }

    /// Returns the configured thinking budget, if any.
    #[must_use]
    pub const fn budget_value(&self) -> Option<i32> {
        match self {
            Self::Budget(cfg) => Some(cfg.thinking_budget),
            Self::Level(_) => None,
        }
    }

    /// Returns the configured thinking level, if any.
    #[must_use]
    pub const fn level_value(&self) -> Option<ThinkingLevel> {
        match self {
            Self::Level(cfg) => Some(cfg.thinking_level),
            Self::Budget(_) => None,
        }
    }
}

/// Generation configuration for content generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationConfig {
    /// Temperature for randomness (0.0 to 1.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,

    /// Top-p for nucleus sampling.
    #[serde(rename = "topP")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,

    /// Top-k for top-k sampling.
    #[serde(rename = "topK")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<i32>,

    /// Maximum number of output tokens.
    #[serde(rename = "maxOutputTokens")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<i32>,

    /// List of stop sequences.
    #[serde(rename = "stopSequences")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,

    /// Candidate count.
    #[serde(rename = "candidateCount")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub candidate_count: Option<i32>,

    /// Response MIME type for structured output (e.g., "application/json").
    #[serde(rename = "responseMimeType")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_mime_type: Option<String>,

    /// Response schema for structured output.
    #[serde(rename = "responseSchema")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_schema: Option<serde_json::Value>,

    /// Thinking configuration for models that support thinking mode.
    #[serde(rename = "thinkingConfig")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking_config: Option<ThinkingConfig>,
}

impl Default for GenerationConfig {
    fn default() -> Self {
        Self {
            temperature: Some(0.7),
            top_p: Some(1.0),
            top_k: Some(32),
            max_output_tokens: Some(2048),
            stop_sequences: None,
            candidate_count: Some(1),
            response_mime_type: None,
            response_schema: None,
            thinking_config: None,
        }
    }
}

impl GenerationConfig {
    /// Enable JSON structured output.
    #[must_use]
    pub fn with_json_response(mut self) -> Self {
        self.response_mime_type = Some("application/json".to_string());
        self
    }

    /// Set custom response MIME type.
    #[must_use]
    pub fn with_response_mime_type<S: Into<String>>(mut self, mime_type: S) -> Self {
        self.response_mime_type = Some(mime_type.into());
        self
    }

    /// Set response schema for structured output.
    #[must_use]
    pub fn with_response_schema(mut self, schema: serde_json::Value) -> Self {
        self.response_schema = Some(schema);
        self
    }

    /// Set JSON response with schema.
    #[must_use]
    pub fn with_json_schema(mut self, schema: serde_json::Value) -> Self {
        self.response_mime_type = Some("application/json".to_string());
        self.response_schema = Some(schema);
        self
    }

    /// Create a simple object schema with properties.
    #[must_use]
    #[allow(clippy::needless_pass_by_value)]
    pub fn create_object_schema(properties: serde_json::Value) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": properties
        })
    }

    /// Create a person extraction schema (example).
    #[must_use]
    pub fn person_schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "name": {"type": "string", "description": "Person's full name"},
                "age": {"type": "integer", "description": "Person's age in years"},
                "email": {"type": "string", "description": "Person's email address"}
            },
            "required": ["name"]
        })
    }

    /// Create a list schema for arrays.
    #[must_use]
    #[allow(clippy::needless_pass_by_value)]
    pub fn create_array_schema(item_schema: serde_json::Value) -> serde_json::Value {
        serde_json::json!({
            "type": "array",
            "items": item_schema
        })
    }

    /// Create a recipe ingredients schema (example).
    #[must_use]
    pub fn recipe_ingredients_schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "recipe_name": {"type": "string"},
                "ingredients": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "name": {"type": "string"},
                            "amount": {"type": "string"},
                            "unit": {"type": "string"}
                        },
                        "required": ["name", "amount"]
                    }
                }
            },
            "required": ["recipe_name", "ingredients"]
        })
    }

    /// Create an org chart schema (example).
    #[must_use]
    pub fn org_chart_schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "company_name": {"type": "string"},
                "departments": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "name": {"type": "string"},
                            "position": {"type": "string"},
                            "level": {"type": "string"}
                        },
                        "required": ["name", "position"]
                    }
                }
            },
            "required": ["company_name", "departments"]
        })
    }

    /// Enable thinking mode with automatic budget.
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn with_thinking(mut self) -> Self {
        self.thinking_config = Some(ThinkingConfig::auto());
        self
    }

    /// Enable thinking mode with a specific level (Low/High).
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn with_thinking_level(mut self, level: ThinkingLevel) -> Self {
        self.thinking_config = Some(ThinkingConfig::with_level(level));
        self
    }

    /// Enable thinking mode with specific budget.
    #[must_use]
    pub fn with_thinking_budget(mut self, budget: i32) -> Self {
        self.thinking_config = Some(ThinkingConfig::with_budget(budget));
        self
    }

    /// Set custom thinking configuration.
    #[must_use]
    pub const fn with_thinking_config(mut self, config: ThinkingConfig) -> Self {
        self.thinking_config = Some(config);
        self
    }

    /// Disable thinking mode explicitly.
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn without_thinking(mut self) -> Self {
        self.thinking_config = Some(ThinkingConfig::disabled());
        self
    }
}
