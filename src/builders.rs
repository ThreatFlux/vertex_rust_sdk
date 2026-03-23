//! Builder patterns for constructing API requests

use crate::models::{CountTokensRequest, GenerateContentRequest};
use crate::types::{
    Content, FunctionDeclaration, GenerationConfig, RequestMetadata, SafetySetting, ThinkingConfig,
    Tool,
};
use serde_json::Value;

/// Builder for content generation requests
///
/// This builder provides a fluent API for constructing content generation requests
/// with various configuration options.
///
/// # Example
///
/// Note: This example is ignored in doctests to keep documentation builds lightweight.
///
/// ```rust,ignore
/// use threatflux_vertex_rust_sdk::ContentRequestBuilder;
///
/// let request = ContentRequestBuilder::new("Explain quantum computing")
///     .temperature(0.7)
///     .max_tokens(1024)
///     .top_p(0.9)
///     .build();
/// ```
#[derive(Debug, Clone)]
#[must_use]
pub struct ContentRequestBuilder {
    contents: Vec<Content>,
    generation_config: GenerationConfig,
    safety_settings: Vec<SafetySetting>,
    tools: Vec<Tool>,
    system_instruction: Option<Content>,
    metadata: Option<RequestMetadata>,
}

#[allow(clippy::missing_const_for_fn)]
impl ContentRequestBuilder {
    /// Create a new builder with a single user prompt
    pub fn new<S: Into<String>>(prompt: S) -> Self {
        Self {
            contents: vec![Content::user_text(prompt)],
            generation_config: GenerationConfig::default(),
            safety_settings: Vec::new(),
            tools: Vec::new(),
            system_instruction: None,
            metadata: None,
        }
    }

    /// Create a builder with multiple content pieces
    pub fn with_contents(contents: Vec<Content>) -> Self {
        Self {
            contents,
            generation_config: GenerationConfig::default(),
            safety_settings: Vec::new(),
            tools: Vec::new(),
            system_instruction: None,
            metadata: None,
        }
    }

    /// Set the temperature (0.0 to 1.0)
    pub fn temperature(mut self, temperature: f32) -> Self {
        self.generation_config.temperature = Some(temperature);
        self
    }

    /// Set the maximum output tokens
    pub fn max_tokens(mut self, max_tokens: i32) -> Self {
        self.generation_config.max_output_tokens = Some(max_tokens);
        self
    }

    /// Set top-p for nucleus sampling
    pub fn top_p(mut self, top_p: f32) -> Self {
        self.generation_config.top_p = Some(top_p);
        self
    }

    /// Set top-k for top-k sampling
    pub fn top_k(mut self, top_k: i32) -> Self {
        self.generation_config.top_k = Some(top_k);
        self
    }

    /// Add stop sequences
    pub fn stop_sequences<I, S>(mut self, sequences: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let sequences: Vec<String> = sequences.into_iter().map(Into::into).collect();
        self.generation_config.stop_sequences = Some(sequences);
        self
    }

    /// Set candidate count
    pub fn candidate_count(mut self, count: i32) -> Self {
        self.generation_config.candidate_count = Some(count);
        self
    }

    /// Add a safety setting
    pub fn safety_setting(mut self, category: &str, threshold: &str) -> Self {
        self.safety_settings.push(SafetySetting {
            category: category.to_string(),
            threshold: threshold.to_string(),
        });
        self
    }

    /// Add multiple safety settings
    pub fn safety_settings(mut self, settings: Vec<SafetySetting>) -> Self {
        self.safety_settings.extend(settings);
        self
    }

    /// Add a tool for function calling
    pub fn tool(mut self, tool: Tool) -> Self {
        self.tools.push(tool);
        self
    }

    /// Add multiple tools
    pub fn tools(mut self, tools: Vec<Tool>) -> Self {
        self.tools.extend(tools);
        self
    }

    /// Add a function declaration
    pub fn function(mut self, function: FunctionDeclaration) -> Self {
        let tool = Tool::function_calling(vec![function]);
        self.tools.push(tool);
        self
    }

    /// Set system instruction
    pub fn system_instruction<S: Into<String>>(mut self, instruction: S) -> Self {
        self.system_instruction = Some(Content::system_text(instruction));
        self
    }

    /// Add a user message to the conversation
    pub fn user_message<S: Into<String>>(mut self, message: S) -> Self {
        self.contents.push(Content::user_text(message));
        self
    }

    /// Add a model message to the conversation
    pub fn model_message<S: Into<String>>(mut self, message: S) -> Self {
        self.contents.push(Content::model_text(message));
        self
    }

    /// Attach request metadata that will be forwarded to the model.
    pub fn metadata(mut self, metadata: RequestMetadata) -> Self {
        if metadata.is_empty() {
            self.metadata = None;
        } else {
            self.metadata = Some(metadata);
        }
        self
    }

    /// Enable thinking mode with auto budget (-1 legacy behavior)
    pub fn with_thinking(mut self) -> Self {
        self.generation_config.thinking_config = Some(ThinkingConfig::auto());
        self
    }

    /// Enable thinking mode with an explicit thinking level.
    pub fn with_thinking_level(mut self, level: crate::types::ThinkingLevel) -> Self {
        self.generation_config.thinking_config = Some(ThinkingConfig::with_level(level));
        self
    }

    /// Enable thinking mode with explicit budget
    pub fn with_thinking_budget(mut self, budget: i32) -> Self {
        self.generation_config.thinking_config = Some(ThinkingConfig::with_budget(budget));
        self
    }

    /// Build the final request
    #[must_use]
    pub fn build(self) -> GenerateContentRequest {
        GenerateContentRequest {
            contents: self.contents,
            generation_config: Some(self.generation_config),
            safety_settings: if self.safety_settings.is_empty() {
                None
            } else {
                Some(self.safety_settings)
            },
            tools: if self.tools.is_empty() { None } else { Some(self.tools) },
            system_instruction: self.system_instruction,
            cached_content: None,
            tool_config: None,
            metadata: self.metadata,
        }
    }
}

/// Builder for function declarations
///
/// This builder helps create function declarations for tool use.
///
/// # Example
///
/// Note: This example is ignored in doctests to keep documentation builds lightweight.
///
/// ```rust,ignore
/// use threatflux_vertex_rust_sdk::FunctionBuilder;
///
/// let function = FunctionBuilder::new("get_weather", "Get current weather")
///     .parameter("location", "string", "City name")
///     .required_parameter("location")
///     .build();
/// ```
#[derive(Debug, Clone)]
#[must_use]
pub struct FunctionBuilder {
    name: String,
    description: String,
    parameters: serde_json::Map<String, Value>,
    required: Vec<String>,
}

#[allow(clippy::missing_const_for_fn)]
impl FunctionBuilder {
    /// Create a new function builder
    pub fn new<N, D>(name: N, description: D) -> Self
    where
        N: Into<String>,
        D: Into<String>,
    {
        Self {
            name: name.into(),
            description: description.into(),
            parameters: serde_json::Map::new(),
            required: Vec::new(),
        }
    }

    /// Add a parameter
    pub fn parameter<N, T, D>(mut self, name: N, param_type: T, description: D) -> Self
    where
        N: Into<String>,
        T: Into<String>,
        D: Into<String>,
    {
        let param = serde_json::json!({
            "type": param_type.into(),
            "description": description.into()
        });
        self.parameters.insert(name.into(), param);
        self
    }

    /// Add a parameter with enum values
    pub fn enum_parameter<N, T, D, I, V>(
        mut self,
        name: N,
        param_type: T,
        description: D,
        enum_values: I,
    ) -> Self
    where
        N: Into<String>,
        T: Into<String>,
        D: Into<String>,
        I: IntoIterator<Item = V>,
        V: Into<String>,
    {
        let values: Vec<String> = enum_values.into_iter().map(Into::into).collect();
        let param = serde_json::json!({
            "type": param_type.into(),
            "description": description.into(),
            "enum": values
        });
        self.parameters.insert(name.into(), param);
        self
    }

    /// Add a number parameter with min/max constraints
    pub fn number_parameter<N, D>(
        mut self,
        name: N,
        description: D,
        minimum: Option<f64>,
        maximum: Option<f64>,
    ) -> Self
    where
        N: Into<String>,
        D: Into<String>,
    {
        let mut param = serde_json::json!({
            "type": "number",
            "description": description.into()
        });

        if let Some(min) = minimum {
            param["minimum"] = serde_json::json!(min);
        }
        if let Some(max) = maximum {
            param["maximum"] = serde_json::json!(max);
        }

        self.parameters.insert(name.into(), param);
        self
    }

    /// Mark a parameter as required
    pub fn required_parameter<N: Into<String>>(mut self, name: N) -> Self {
        self.required.push(name.into());
        self
    }

    /// Mark multiple parameters as required
    pub fn required_parameters<I, N>(mut self, names: I) -> Self
    where
        I: IntoIterator<Item = N>,
        N: Into<String>,
    {
        self.required.extend(names.into_iter().map(Into::into));
        self
    }

    /// Build the function declaration
    #[must_use]
    pub fn build(self) -> FunctionDeclaration {
        let mut schema = serde_json::json!({
            "type": "object",
            "properties": self.parameters
        });

        if !self.required.is_empty() {
            schema["required"] = serde_json::json!(self.required);
        }

        FunctionDeclaration { name: self.name, description: self.description, parameters: schema }
    }
}

/// Builder for token counting requests
#[derive(Debug, Clone)]
#[must_use]
pub struct TokenCountBuilder {
    contents: Vec<Content>,
}

#[allow(clippy::missing_const_for_fn)]
impl TokenCountBuilder {
    /// Create a new builder with a single text prompt
    pub fn new<S: Into<String>>(text: S) -> Self {
        Self { contents: vec![Content::user_text(text)] }
    }

    /// Create a builder with multiple content pieces
    pub fn with_contents(contents: Vec<Content>) -> Self {
        Self { contents }
    }

    /// Add a user message
    pub fn user_message<S: Into<String>>(mut self, message: S) -> Self {
        self.contents.push(Content::user_text(message));
        self
    }

    /// Add a model message
    pub fn model_message<S: Into<String>>(mut self, message: S) -> Self {
        self.contents.push(Content::model_text(message));
        self
    }

    /// Build the token counting request
    #[must_use]
    pub fn build(self) -> CountTokensRequest {
        CountTokensRequest { contents: self.contents }
    }
}

/// Common safety settings presets
pub mod safety {
    use super::SafetySetting;

    /// Block most harmful content
    #[must_use]
    pub fn strict_safety() -> Vec<SafetySetting> {
        vec![
            SafetySetting {
                category: "HARM_CATEGORY_HATE_SPEECH".to_string(),
                threshold: "BLOCK_LOW_AND_ABOVE".to_string(),
            },
            SafetySetting {
                category: "HARM_CATEGORY_DANGEROUS_CONTENT".to_string(),
                threshold: "BLOCK_LOW_AND_ABOVE".to_string(),
            },
            SafetySetting {
                category: "HARM_CATEGORY_HARASSMENT".to_string(),
                threshold: "BLOCK_LOW_AND_ABOVE".to_string(),
            },
            SafetySetting {
                category: "HARM_CATEGORY_SEXUALLY_EXPLICIT".to_string(),
                threshold: "BLOCK_LOW_AND_ABOVE".to_string(),
            },
        ]
    }

    /// Block only high-confidence harmful content
    #[must_use]
    pub fn balanced_safety() -> Vec<SafetySetting> {
        vec![
            SafetySetting {
                category: "HARM_CATEGORY_HATE_SPEECH".to_string(),
                threshold: "BLOCK_MEDIUM_AND_ABOVE".to_string(),
            },
            SafetySetting {
                category: "HARM_CATEGORY_DANGEROUS_CONTENT".to_string(),
                threshold: "BLOCK_MEDIUM_AND_ABOVE".to_string(),
            },
            SafetySetting {
                category: "HARM_CATEGORY_HARASSMENT".to_string(),
                threshold: "BLOCK_MEDIUM_AND_ABOVE".to_string(),
            },
            SafetySetting {
                category: "HARM_CATEGORY_SEXUALLY_EXPLICIT".to_string(),
                threshold: "BLOCK_MEDIUM_AND_ABOVE".to_string(),
            },
        ]
    }

    /// Minimal safety filtering
    #[must_use]
    pub fn permissive_safety() -> Vec<SafetySetting> {
        vec![
            SafetySetting {
                category: "HARM_CATEGORY_HATE_SPEECH".to_string(),
                threshold: "BLOCK_HIGH_AND_ABOVE".to_string(),
            },
            SafetySetting {
                category: "HARM_CATEGORY_DANGEROUS_CONTENT".to_string(),
                threshold: "BLOCK_HIGH_AND_ABOVE".to_string(),
            },
            SafetySetting {
                category: "HARM_CATEGORY_HARASSMENT".to_string(),
                threshold: "BLOCK_HIGH_AND_ABOVE".to_string(),
            },
            SafetySetting {
                category: "HARM_CATEGORY_SEXUALLY_EXPLICIT".to_string(),
                threshold: "BLOCK_HIGH_AND_ABOVE".to_string(),
            },
        ]
    }
}

/// Common function declarations
pub mod functions {
    use super::{FunctionBuilder, FunctionDeclaration};

    /// Create a basic calculator function
    #[must_use]
    pub fn calculator() -> FunctionDeclaration {
        FunctionBuilder::new("calculate", "Perform basic mathematical calculations")
            .enum_parameter(
                "operation",
                "string",
                "Mathematical operation",
                ["add", "subtract", "multiply", "divide"],
            )
            .parameter("a", "number", "First number")
            .parameter("b", "number", "Second number")
            .required_parameters(["operation", "a", "b"])
            .build()
    }

    /// Create a weather function
    #[must_use]
    pub fn weather() -> FunctionDeclaration {
        FunctionBuilder::new("get_weather", "Get current weather information")
            .parameter("location", "string", "City and state or city and country")
            .enum_parameter("unit", "string", "Temperature unit", ["celsius", "fahrenheit"])
            .required_parameter("location")
            .build()
    }

    /// Create a web search function
    #[must_use]
    pub fn web_search() -> FunctionDeclaration {
        FunctionBuilder::new("web_search", "Search the web for information")
            .parameter("query", "string", "Search query")
            .number_parameter("max_results", "Maximum number of results", Some(1.0), Some(10.0))
            .required_parameter("query")
            .build()
    }

    /// Create a code execution function
    #[must_use]
    pub fn code_execution() -> FunctionDeclaration {
        FunctionBuilder::new("execute_code", "Execute code and return results")
            .enum_parameter(
                "language",
                "string",
                "Programming language",
                ["python", "javascript", "bash", "sql"],
            )
            .parameter("code", "string", "Code to execute")
            .parameter("timeout", "number", "Timeout in seconds (default: 30)")
            .required_parameters(["language", "code"])
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_request_builder() {
        let request = ContentRequestBuilder::new("Hello, world!")
            .temperature(0.8)
            .max_tokens(512)
            .top_p(0.9)
            .build();

        assert_eq!(request.contents.len(), 1);
        assert_eq!(request.generation_config.as_ref().unwrap().temperature, Some(0.8));
        assert_eq!(request.generation_config.as_ref().unwrap().max_output_tokens, Some(512));
    }

    #[test]
    fn test_function_builder() {
        let function = FunctionBuilder::new("test_func", "A test function")
            .parameter("param1", "string", "First parameter")
            .parameter("param2", "number", "Second parameter")
            .required_parameter("param1")
            .build();

        assert_eq!(function.name, "test_func");
        assert_eq!(function.description, "A test function");
    }

    #[test]
    fn test_safety_presets() {
        let strict = safety::strict_safety();
        assert_eq!(strict.len(), 4);
        assert!(strict[0].threshold.contains("BLOCK_LOW_AND_ABOVE"));

        let balanced = safety::balanced_safety();
        assert!(balanced[0].threshold.contains("BLOCK_MEDIUM_AND_ABOVE"));

        let permissive = safety::permissive_safety();
        assert!(permissive[0].threshold.contains("BLOCK_HIGH_AND_ABOVE"));
    }

    #[test]
    fn test_common_functions() {
        let calc = functions::calculator();
        assert_eq!(calc.name, "calculate");

        let weather = functions::weather();
        assert_eq!(weather.name, "get_weather");

        let search = functions::web_search();
        assert_eq!(search.name, "web_search");

        let code = functions::code_execution();
        assert_eq!(code.name, "execute_code");
    }

    #[test]
    fn test_token_count_builder() {
        let request = TokenCountBuilder::new("Count these tokens")
            .user_message("And these")
            .model_message("Response here")
            .build();

        assert_eq!(request.contents.len(), 3);
        assert_eq!(request.contents[0].role, "user");
        assert_eq!(request.contents[2].role, "model");
    }
}
