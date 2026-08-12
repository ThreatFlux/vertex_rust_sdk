use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Default Anthropics API version required by Vertex Claude endpoints.
pub const DEFAULT_VERTEX_ANTHROPIC_VERSION: &str = "vertex-2023-10-16";

/// Beta tag required to enable Anthropic Web Search tooling (original).
pub const CLAUDE_WEB_SEARCH_BETA_TAG: &str = "web-search-2025-03-05";
/// Beta tag for the 2026-02-09 web search variant (4.6 models with dynamic filtering).
pub const CLAUDE_WEB_SEARCH_V2_BETA_TAG: &str = "web-search-2026-02-09";
/// Beta tag required to unlock the 1M-token context window for Sonnet 4/4.5.
pub const CLAUDE_LONG_CONTEXT_BETA_TAG: &str = "context-1m-2025-08-07";

/// Anthropics message role for Claude requests.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// User supplied content.
    User,
    /// Assistant generated content.
    Assistant,
    /// System level instruction.
    System,
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::User => write!(f, "user"),
            Self::Assistant => write!(f, "assistant"),
            Self::System => write!(f, "system"),
        }
    }
}

/// Multimodal content blocks supported by Claude.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    /// Plain text content.
    Text {
        text: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        citations: Vec<Citation>,
    },
    /// Inline image encoded as base64.
    Image { source: ImageSource },
    /// Inline document (e.g., PDF) encoded as base64.
    Document { source: DocumentSource },
    /// Request for a tool/function invocation.
    ToolUse { id: String, name: String, input: serde_json::Value },
    /// Result returned from an executed tool.
    ToolResult {
        tool_use_id: String,
        content: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        is_error: Option<bool>,
    },
    /// Invocation of a server-managed tool (e.g., `web_search`).
    ServerToolUse { id: String, name: String, input: serde_json::Value },
    /// Result payload from a server tool invocation.
    WebSearchToolResult { tool_use_id: String, content: WebSearchToolContent },
    /// Extended thinking content block.
    Thinking {
        thinking: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        signature: Option<String>,
    },
}

impl ContentBlock {
    /// Convenience helper for creating text blocks.
    #[must_use]
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text { text: text.into(), citations: Vec::new() }
    }

    /// Convenience helper for creating tool result blocks.
    #[must_use]
    pub fn tool_result(tool_use_id: impl Into<String>, content: Option<String>) -> Self {
        Self::ToolResult { tool_use_id: tool_use_id.into(), content, is_error: Some(false) }
    }

    /// Convenience helper for creating error tool results.
    #[must_use]
    pub fn tool_error(tool_use_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self::ToolResult {
            tool_use_id: tool_use_id.into(),
            content: Some(message.into()),
            is_error: Some(true),
        }
    }

    /// Extract text content if present.
    #[must_use]
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text { text, .. } => Some(text),
            _ => None,
        }
    }
}

/// Citation metadata returned alongside text content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Citation {
    #[serde(rename = "type")]
    pub citation_type: String,
    pub url: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encrypted_index: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cited_text: Option<String>,
}

/// Collection returned from a web search tool invocation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum WebSearchToolContent {
    /// Successful search results.
    Results(Vec<WebSearchResult>),
    /// Error reported by the web search subsystem.
    Error(WebSearchToolError),
}

/// Individual web search result entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebSearchResult {
    #[serde(rename = "type")]
    pub result_type: String,
    pub url: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encrypted_content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_age: Option<String>,
}

/// Error payload returned when web search fails.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebSearchToolError {
    #[serde(rename = "type")]
    pub error_type: String,
    pub error_code: WebSearchErrorCode,
}

/// Enumerates error codes returned by the web search tool.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WebSearchErrorCode {
    TooManyRequests,
    InvalidInput,
    MaxUsesExceeded,
    QueryTooLong,
    Unavailable,
}

/// Image payload supported by Claude messages.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ImageSource {
    /// Base64 encoded bytes of an image or PDF page.
    Base64 { media_type: String, data: String },
}

impl ImageSource {
    /// Create from a media type and base64 string.
    #[must_use]
    pub fn base64(media_type: impl Into<String>, data: impl Into<String>) -> Self {
        Self::Base64 { media_type: media_type.into(), data: data.into() }
    }

    /// Encode raw bytes to base64 content.
    #[must_use]
    pub fn from_bytes(media_type: impl Into<String>, bytes: &[u8]) -> Self {
        use base64::prelude::*;
        let encoded = BASE64_STANDARD.encode(bytes);
        Self::Base64 { media_type: media_type.into(), data: encoded }
    }
}

/// Document payload supported by Claude messages.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DocumentSource {
    /// Base64 encoded bytes of a document (e.g., PDF).
    Base64 { media_type: String, data: String },
}

impl DocumentSource {
    /// Create from a media type and base64 string.
    #[must_use]
    pub fn base64(media_type: impl Into<String>, data: impl Into<String>) -> Self {
        Self::Base64 { media_type: media_type.into(), data: data.into() }
    }

    /// Encode raw bytes to base64 content.
    #[must_use]
    pub fn from_bytes(media_type: impl Into<String>, bytes: &[u8]) -> Self {
        use base64::prelude::*;
        let encoded = BASE64_STANDARD.encode(bytes);
        Self::Base64 { media_type: media_type.into(), data: encoded }
    }
}

/// Tool definition for Claude tool use / function calling.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tool {
    /// Unique tool name.
    pub name: String,
    /// Description shown to the model.
    pub description: String,
    /// JSON schema describing the tool input.
    pub input_schema: serde_json::Value,
}

impl Tool {
    /// Create a new tool definition.
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        input_schema: serde_json::Value,
    ) -> Self {
        Self { name: name.into(), description: description.into(), input_schema }
    }
}

/// Supported server-managed tool identifiers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum WebSearchToolType {
    #[serde(rename = "web_search_20250305")]
    #[default]
    WebSearch,
    /// 2026-02-09 variant with dynamic filtering support (4.6 models).
    #[serde(rename = "web_search_20260209")]
    WebSearchV2,
}

/// Location metadata to localize web search results.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebSearchUserLocation {
    #[serde(rename = "type")]
    pub location_type: WebSearchLocationType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timezone: Option<String>,
}

impl WebSearchUserLocation {
    /// Convenience constructor for approximate locations.
    #[must_use]
    pub fn approximate(
        city: impl Into<String>,
        region: impl Into<String>,
        country: impl Into<String>,
        timezone: impl Into<String>,
    ) -> Self {
        Self {
            location_type: WebSearchLocationType::Approximate,
            city: Some(city.into()),
            region: Some(region.into()),
            country: Some(country.into()),
            timezone: Some(timezone.into()),
        }
    }
}

/// Enumerates supported web search location hint types.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WebSearchLocationType {
    Approximate,
}

/// Configuration for Anthropic's managed web search tool.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebSearchTool {
    #[serde(rename = "type", default)]
    pub tool_type: WebSearchToolType,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_uses: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_domains: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocked_domains: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_location: Option<WebSearchUserLocation>,
}

impl Default for WebSearchTool {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(clippy::missing_const_for_fn)]
impl WebSearchTool {
    /// Create a web search tool configuration with defaults.
    #[must_use]
    pub fn new() -> Self {
        Self {
            tool_type: WebSearchToolType::WebSearch,
            name: "web_search".to_string(),
            max_uses: None,
            allowed_domains: None,
            blocked_domains: None,
            user_location: None,
        }
    }

    /// Create a web search tool using the 2026-02-09 variant (for 4.6 models).
    #[must_use]
    pub fn new_v2() -> Self {
        Self {
            tool_type: WebSearchToolType::WebSearchV2,
            name: "web_search".to_string(),
            max_uses: None,
            allowed_domains: None,
            blocked_domains: None,
            user_location: None,
        }
    }

    /// Set the maximum number of search invocations allowed.
    #[must_use]
    pub fn with_max_uses(mut self, max: Option<u8>) -> Self {
        self.max_uses = max;
        self
    }

    /// Restrict searches to the provided domains.
    #[must_use]
    pub fn with_allowed_domains(mut self, domains: Option<Vec<String>>) -> Self {
        self.allowed_domains = domains;
        self
    }

    /// Exclude the provided domains from search.
    #[must_use]
    pub fn with_blocked_domains(mut self, domains: Option<Vec<String>>) -> Self {
        self.blocked_domains = domains;
        self
    }

    /// Localize search results.
    #[must_use]
    pub fn with_user_location(mut self, location: Option<WebSearchUserLocation>) -> Self {
        self.user_location = location;
        self
    }
}

/// Envelope for request tools passed to the Claude API.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RequestTool {
    /// Custom function defined by the caller.
    Function(Tool),
    /// Managed web search tool.
    WebSearch(WebSearchTool),
}

impl From<Tool> for RequestTool {
    fn from(tool: Tool) -> Self {
        Self::Function(tool)
    }
}

impl From<WebSearchTool> for RequestTool {
    fn from(tool: WebSearchTool) -> Self {
        Self::WebSearch(tool)
    }
}

/// Preferred tool selection behaviour.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(untagged)]
pub enum ToolChoice {
    /// Let Claude decide when to invoke tools.
    #[default]
    Auto,
    /// Force a specific tool invocation.
    Tool { name: String },
    /// Permit any tool use.
    Any,
    /// Disable tool use entirely.
    None,
}

/// Arbitrary metadata associated with a request.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Metadata {
    /// Optional user identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    /// Additional key/value pairs forwarded to Anthropic.
    #[serde(flatten)]
    pub custom: HashMap<String, serde_json::Value>,
}

impl Metadata {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_user_id(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    #[must_use]
    pub fn with_custom(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.custom.insert(key.into(), value);
        self
    }
}

/// Reason why generation stopped.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    MaxTokens,
    EndTurn,
    StopSequence,
    ToolUse,
    ModelContextWindowExceeded,
}

/// Effort level for adaptive thinking mode (Claude 4.6+).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThinkingEffort {
    Low,
    Medium,
    High,
    Max,
}

/// Thinking configuration (Claude 4.x+).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThinkingConfig {
    #[serde(rename = "type")]
    pub thinking_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub budget_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_tool_use: Option<bool>,
    /// Effort level for adaptive thinking (4.6+). Ignored when type is not `adaptive`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effort: Option<ThinkingEffort>,
    /// Display control for thinking content in streaming responses.
    /// Set to `"omitted"` to suppress thinking text while preserving signature.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display: Option<String>,
}

impl ThinkingConfig {
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn enabled(budget_tokens: u32) -> Self {
        Self {
            thinking_type: "enabled".to_string(),
            budget_tokens: Some(budget_tokens),
            allow_tool_use: None,
            effort: None,
            display: None,
        }
    }

    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn enabled_with_tools(budget_tokens: u32) -> Self {
        Self {
            thinking_type: "enabled".to_string(),
            budget_tokens: Some(budget_tokens),
            allow_tool_use: Some(true),
            effort: None,
            display: None,
        }
    }

    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn disabled() -> Self {
        Self {
            thinking_type: "disabled".to_string(),
            budget_tokens: None,
            allow_tool_use: None,
            effort: None,
            display: None,
        }
    }

    /// Adaptive thinking — the model decides how much to think based on effort level.
    /// Recommended for Claude 4.6+ models. Replaces `budget_tokens`.
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn adaptive(effort: ThinkingEffort) -> Self {
        Self {
            thinking_type: "adaptive".to_string(),
            budget_tokens: None,
            allow_tool_use: None,
            effort: Some(effort),
            display: None,
        }
    }

    /// Adaptive thinking with tool use enabled.
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn adaptive_with_tools(effort: ThinkingEffort) -> Self {
        Self {
            thinking_type: "adaptive".to_string(),
            budget_tokens: None,
            allow_tool_use: Some(true),
            effort: Some(effort),
            display: None,
        }
    }

    /// Omit thinking content from streaming responses (faster streaming).
    /// The signature is still preserved for multi-turn continuity.
    #[must_use]
    pub fn with_display_omitted(mut self) -> Self {
        self.display = Some("omitted".to_string());
        self
    }
}

/// Prompt caching control wrapper.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CacheControl {
    #[serde(rename = "type")]
    pub cache_type: String,
}

impl CacheControl {
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn ephemeral() -> Self {
        Self { cache_type: "ephemeral".to_string() }
    }
}

/// Primary Claude message definition used in request payloads.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: Vec<ContentBlock>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Metadata>,
}

impl Message {
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn new(role: Role, content: Vec<ContentBlock>) -> Self {
        Self { role, content, metadata: None }
    }

    #[must_use]
    pub fn user(text: impl Into<String>) -> Self {
        Self::new(Role::User, vec![ContentBlock::text(text)])
    }

    #[must_use]
    pub fn assistant(text: impl Into<String>) -> Self {
        Self::new(Role::Assistant, vec![ContentBlock::text(text)])
    }

    #[must_use]
    pub fn system(text: impl Into<String>) -> Self {
        Self::new(Role::System, vec![ContentBlock::text(text)])
    }

    #[must_use]
    pub fn with_metadata(mut self, metadata: Metadata) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

/// Convenience trait for pushing into optional vectors.
pub trait VecPush<T> {
    fn push_item(&mut self, item: T);
}

impl<T> VecPush<T> for Option<Vec<T>> {
    fn push_item(&mut self, item: T) {
        self.get_or_insert_with(Vec::new).push(item);
    }
}

/// JSON schema definition for structured output.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsonSchemaOutput {
    /// Schema name used for identification.
    pub name: String,
    /// Optional description of the expected output.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// The JSON Schema object defining the output structure.
    pub schema: serde_json::Value,
}

/// Output format specification for structured responses.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OutputFormat {
    /// Constrain output to conform to a JSON schema.
    JsonSchema(JsonSchemaOutput),
}

/// Output configuration for structured responses (Claude 4.5+).
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OutputConfig {
    /// The desired output format.
    pub format: OutputFormat,
}

impl OutputConfig {
    /// Create an output config requiring JSON conforming to the given schema.
    #[must_use]
    pub fn json_schema(name: impl Into<String>, schema: serde_json::Value) -> Self {
        Self {
            format: OutputFormat::JsonSchema(JsonSchemaOutput {
                name: name.into(),
                description: None,
                schema,
            }),
        }
    }

    /// Create an output config with a described JSON schema.
    #[must_use]
    pub fn json_schema_with_description(
        name: impl Into<String>,
        description: impl Into<String>,
        schema: serde_json::Value,
    ) -> Self {
        Self {
            format: OutputFormat::JsonSchema(JsonSchemaOutput {
                name: name.into(),
                description: Some(description.into()),
                schema,
            }),
        }
    }
}

/// Configuration for enabling citations in responses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CitationsConfig {
    pub enabled: bool,
}

impl CitationsConfig {
    #[must_use]
    pub const fn enabled() -> Self {
        Self { enabled: true }
    }
}

/// Request payload for Claude Sonnet via Vertex Anthropic endpoint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MessageRequest {
    #[serde(rename = "anthropic_version")]
    pub anthropic_version: String,
    pub max_tokens: u32,
    pub messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(rename = "top_p", skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(rename = "top_k", skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,
    #[serde(rename = "stop_sequences", skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<RequestTool>>,
    #[serde(rename = "tool_choice", skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ToolChoice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<ThinkingConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Metadata>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<CacheControl>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub beta: Option<Vec<String>>,
    /// Structured output configuration (Claude 4.5+).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_config: Option<OutputConfig>,
    /// Enable citation generation for document content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub citations: Option<CitationsConfig>,
    #[serde(flatten)]
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub extra_params: HashMap<String, serde_json::Value>,
}

#[allow(clippy::missing_const_for_fn)]
impl MessageRequest {
    /// Create a new Vertex Claude request with defaults.
    #[must_use]
    pub fn new() -> Self {
        Self {
            anthropic_version: DEFAULT_VERTEX_ANTHROPIC_VERSION.to_string(),
            max_tokens: 256,
            messages: Vec::new(),
            system: None,
            temperature: None,
            top_p: None,
            top_k: None,
            stop_sequences: None,
            stream: None,
            tools: None,
            tool_choice: None,
            thinking: None,
            metadata: None,
            cache_control: None,
            beta: None,
            output_config: None,
            citations: None,
            extra_params: HashMap::new(),
        }
    }

    #[must_use]
    pub fn max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    #[must_use]
    pub fn system(mut self, system: impl Into<String>) -> Self {
        self.system = Some(system.into());
        self
    }

    #[must_use]
    pub fn temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature.clamp(0.0, 1.0));
        self
    }

    #[must_use]
    pub fn top_p(mut self, top_p: f32) -> Self {
        self.top_p = Some(top_p.clamp(0.0, 1.0));
        self
    }

    #[must_use]
    pub fn top_k(mut self, top_k: u32) -> Self {
        self.top_k = Some(top_k);
        self
    }

    #[must_use]
    pub fn add_stop_sequence(mut self, stop: impl Into<String>) -> Self {
        self.stop_sequences.push_item(stop.into());
        self
    }

    #[must_use]
    pub fn stream(mut self, stream: bool) -> Self {
        self.stream = Some(stream);
        self
    }

    #[must_use]
    pub fn add_message(mut self, message: Message) -> Self {
        self.messages.push(message);
        self
    }

    #[must_use]
    pub fn add_user_message(mut self, text: impl Into<String>) -> Self {
        self.messages.push(Message::user(text));
        self
    }

    #[must_use]
    pub fn add_assistant_message(mut self, text: impl Into<String>) -> Self {
        self.messages.push(Message::assistant(text));
        self
    }

    #[must_use]
    pub fn add_tool(mut self, tool: Tool) -> Self {
        self.tools.push_item(RequestTool::from(tool));
        self
    }

    /// Add a managed tool entry to the request.
    #[must_use]
    pub fn add_request_tool(mut self, tool: RequestTool) -> Self {
        self.tools.push_item(tool);
        self
    }

    /// Enable web search support for this request.
    #[must_use]
    pub fn add_web_search_tool(self, tool: WebSearchTool) -> Self {
        self.add_request_tool(RequestTool::from(tool))
    }

    #[must_use]
    pub fn tool_choice(mut self, choice: ToolChoice) -> Self {
        self.tool_choice = Some(choice);
        self
    }

    #[must_use]
    pub fn metadata(mut self, metadata: Metadata) -> Self {
        self.metadata = Some(metadata);
        self
    }

    #[must_use]
    pub fn thinking(mut self, config: ThinkingConfig) -> Self {
        self.thinking = Some(config);
        self
    }

    #[must_use]
    pub fn cache_control(mut self, cache_control: CacheControl) -> Self {
        self.cache_control = Some(cache_control);
        self
    }

    #[must_use]
    pub fn enable_beta_features(mut self, features: Vec<String>) -> Self {
        if features.is_empty() {
            self.beta = None;
        } else {
            self.beta = Some(features);
        }
        self
    }

    /// Set structured output configuration with JSON schema.
    #[must_use]
    pub fn output_config(mut self, config: OutputConfig) -> Self {
        self.output_config = Some(config);
        self
    }

    /// Enable citations in responses for document content.
    #[must_use]
    pub fn enable_citations(mut self) -> Self {
        self.citations = Some(CitationsConfig::enabled());
        self
    }

    /// Inject provider-specific parameters (e.g., memory tool configuration).
    #[must_use]
    pub fn with_param(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.extra_params.insert(key.into(), value);
        self
    }
}

impl Default for MessageRequest {
    fn default() -> Self {
        Self::new()
    }
}

/// Minimal response representation used when parsing streaming deltas or final responses.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MessageResponse {
    pub id: String,
    #[serde(rename = "type")]
    pub object_type: String,
    pub role: Role,
    pub content: Vec<ContentBlock>,
    pub stop_reason: Option<StopReason>,
    pub stop_sequence: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<Usage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,
}

impl MessageResponse {
    #[must_use]
    pub fn text(&self) -> String {
        self.content.iter().filter_map(|part| part.as_text()).collect::<Vec<_>>().join(" ")
    }
}

/// Token usage metadata returned by Anthropic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Usage {
    #[serde(default)]
    pub input_tokens: u32,
    #[serde(default)]
    pub output_tokens: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_creation_input_tokens: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_read_input_tokens: Option<u32>,
}

impl Usage {
    #[must_use]
    pub const fn total(&self) -> u32 {
        let cache_creation_input_tokens = match self.cache_creation_input_tokens {
            Some(tokens) => tokens,
            None => 0,
        };
        let cache_read_input_tokens = match self.cache_read_input_tokens {
            Some(tokens) => tokens,
            None => 0,
        };

        self.input_tokens
            + self.output_tokens
            + cache_creation_input_tokens
            + cache_read_input_tokens
    }
}

/// Streaming message delta emitted during SSE sessions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageDelta {
    pub stop_reason: Option<StopReason>,
    pub stop_sequence: Option<String>,
}

/// Streaming content block delta emitted during SSE sessions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContentBlockDelta {
    #[serde(rename = "type")]
    pub block_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partial_json: Option<String>,
    /// Thinking text emitted during extended thinking (may be omitted via display config).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<String>,
    /// Cryptographic signature chunks emitted after extended thinking text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
}

/// High level SSE event wrapper for Claude streaming responses.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamEvent {
    MessageStart { message: MessageResponse },
    MessageDelta { delta: MessageDelta, usage: Usage },
    MessageStop,
    ContentBlockStart { index: usize, content_block: ContentBlock },
    ContentBlockDelta { index: usize, delta: ContentBlockDelta },
    ContentBlockStop { index: usize },
    Ping,
    Error { error: HashMap<String, serde_json::Value> },
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;

    #[test]
    fn builds_basic_request() {
        let request = MessageRequest::new()
            .max_tokens(1024)
            .system("You are helpful")
            .add_user_message("Hello")
            .stream(true);

        assert_eq!(request.messages.len(), 1);
        assert_eq!(request.max_tokens, 1024);
        assert_eq!(request.stream, Some(true));
        assert_eq!(request.anthropic_version, DEFAULT_VERTEX_ANTHROPIC_VERSION);
    }

    #[test]
    fn tool_choice_defaults_to_auto() {
        let choice = ToolChoice::default();
        matches!(choice, ToolChoice::Auto);
    }

    #[test]
    fn metadata_helpers_work() {
        let metadata =
            Metadata::new().with_user_id("user-123").with_custom("foo", serde_json::json!("bar"));
        assert_eq!(metadata.user_id, Some("user-123".to_string()));
        assert_eq!(metadata.custom.get("foo").unwrap(), &serde_json::json!("bar"));
    }

    #[test]
    fn usage_total_tokens() {
        let usage = Usage {
            input_tokens: 10,
            output_tokens: 20,
            cache_creation_input_tokens: Some(30),
            cache_read_input_tokens: Some(40),
        };
        assert_eq!(usage.total(), 100);
    }

    #[test]
    fn usage_roundtrips_prompt_cache_fields() {
        let payload = r#"{
            "input_tokens": 100,
            "output_tokens": 40,
            "cache_creation_input_tokens": 512,
            "cache_read_input_tokens": 384
        }"#;
        let usage: Usage = serde_json::from_str(payload).expect("usage should parse");
        assert_eq!(usage.cache_creation_input_tokens, Some(512));
        assert_eq!(usage.cache_read_input_tokens, Some(384));
        let re = serde_json::to_string(&usage).expect("usage should serialize");
        assert!(re.contains("\"cache_creation_input_tokens\":512"));
        assert!(re.contains("\"cache_read_input_tokens\":384"));
    }

    #[test]
    fn usage_omits_cache_fields_when_absent() {
        let payload = r#"{"input_tokens": 5, "output_tokens": 7}"#;
        let usage: Usage = serde_json::from_str(payload).expect("usage should parse");
        assert_eq!(usage.cache_creation_input_tokens, None);
        assert_eq!(usage.cache_read_input_tokens, None);
        let re = serde_json::to_string(&usage).expect("usage should serialize");
        assert!(!re.contains("cache_creation_input_tokens"));
        assert!(!re.contains("cache_read_input_tokens"));
    }

    #[test]
    fn content_block_delta_roundtrips_thinking() {
        let payload = r#"{
            "type": "thinking_delta",
            "thinking": "reasoning chunk"
        }"#;
        let delta: ContentBlockDelta = serde_json::from_str(payload).expect("delta should parse");
        assert_eq!(delta.block_type, "thinking_delta");
        assert_eq!(delta.thinking.as_deref(), Some("reasoning chunk"));
        assert!(delta.signature.is_none());
        let re = serde_json::to_string(&delta).expect("delta should serialize");
        assert!(!re.contains("signature"));
    }

    #[test]
    fn content_block_delta_roundtrips_signature() {
        let payload = r#"{
            "type": "signature_delta",
            "signature": "sig-chunk-abc"
        }"#;
        let delta: ContentBlockDelta = serde_json::from_str(payload).expect("delta should parse");
        assert_eq!(delta.block_type, "signature_delta");
        assert!(delta.thinking.is_none());
        assert_eq!(delta.signature.as_deref(), Some("sig-chunk-abc"));
        let re = serde_json::to_string(&delta).expect("delta should serialize");
        assert!(re.contains("\"signature\":\"sig-chunk-abc\""));
    }

    #[test]
    fn parses_tool_argument_delta_json_fragment() {
        let payload = r#"{
            "type": "content_block_delta",
            "index": 0,
            "delta": {
                "type": "input_json_delta",
                "partial_json": "{\"city\":\"Seat"
            }
        }"#;

        let event: StreamEvent = serde_json::from_str(payload).expect("event should parse");
        match event {
            StreamEvent::ContentBlockDelta { delta, .. } => {
                assert_eq!(delta.block_type, "input_json_delta");
                assert!(delta.text.is_none());
                assert_eq!(delta.partial_json.as_deref(), Some("{\"city\":\"Seat"));
            }
            other => panic!("unexpected event variant: {other:?}"),
        }
    }

    #[test]
    fn document_source_from_bytes_encodes_base64() {
        let source = DocumentSource::from_bytes("application/pdf", b"hello world");

        match source {
            DocumentSource::Base64 { media_type, data } => {
                assert_eq!(media_type, "application/pdf");
                assert_eq!(data, base64::engine::general_purpose::STANDARD.encode(b"hello world"));
            }
        }
    }

    #[test]
    fn document_block_serializes_to_expected_shape() {
        let block = ContentBlock::Document {
            source: DocumentSource::base64("application/pdf", "ZGF0YQ==".to_string()),
        };

        let value = serde_json::to_value(&block).expect("serialize document block");
        let expected = serde_json::json!({
            "type": "document",
            "source": {
                "type": "base64",
                "media_type": "application/pdf",
                "data": "ZGF0YQ=="
            }
        });

        assert_eq!(value, expected);
    }

    #[test]
    fn web_search_tool_serializes() {
        let tool = WebSearchTool::new()
            .with_max_uses(Some(3))
            .with_allowed_domains(Some(vec!["example.com".to_string()]))
            .with_user_location(Some(WebSearchUserLocation {
                location_type: WebSearchLocationType::Approximate,
                city: Some("San Francisco".to_string()),
                region: Some("California".to_string()),
                country: Some("US".to_string()),
                timezone: Some("America/Los_Angeles".to_string()),
            }));

        let value = serde_json::to_value(&tool).expect("serialize web search tool");
        assert_eq!(value.get("type").and_then(|v| v.as_str()), Some("web_search_20250305"));
        assert_eq!(value.get("name").and_then(|v| v.as_str()), Some("web_search"));
        assert_eq!(value.get("allowed_domains").and_then(|v| v.as_array()).map(Vec::len), Some(1));
        assert!(value.get("user_location").is_some());
    }

    #[test]
    fn message_request_adds_web_search_tool() {
        let request = MessageRequest::new().add_web_search_tool(WebSearchTool::new());
        let tools = request.tools.expect("web search tool attached");
        assert_eq!(tools.len(), 1);
        assert!(matches!(tools[0], RequestTool::WebSearch(_)));
    }

    #[test]
    fn parses_web_search_tool_result() {
        let block: ContentBlock = serde_json::from_value(serde_json::json!({
            "type": "web_search_tool_result",
            "tool_use_id": "srvtoolu_123",
            "content": [
                {
                    "type": "web_search_result",
                    "url": "https://example.com",
                    "title": "Example",
                    "page_age": "April 30, 2025"
                }
            ]
        }))
        .expect("parse web search tool result");

        match block {
            ContentBlock::WebSearchToolResult { tool_use_id, content } => {
                assert_eq!(tool_use_id, "srvtoolu_123");
                match content {
                    WebSearchToolContent::Results(results) => {
                        assert_eq!(results.len(), 1);
                        assert_eq!(results[0].url, "https://example.com");
                    }
                    WebSearchToolContent::Error(_) => panic!("expected results variant"),
                }
            }
            other => panic!("unexpected block variant: {other:?}"),
        }
    }

    #[test]
    fn parses_web_search_tool_error() {
        let block: ContentBlock = serde_json::from_value(serde_json::json!({
            "type": "web_search_tool_result",
            "tool_use_id": "srvtoolu_err",
            "content": {
                "type": "web_search_tool_result_error",
                "error_code": "max_uses_exceeded"
            }
        }))
        .expect("parse web search tool error");

        match block {
            ContentBlock::WebSearchToolResult { tool_use_id, content } => {
                assert_eq!(tool_use_id, "srvtoolu_err");
                match content {
                    WebSearchToolContent::Error(error) => {
                        assert!(matches!(error.error_code, WebSearchErrorCode::MaxUsesExceeded));
                    }
                    WebSearchToolContent::Results(_) => panic!("expected error variant"),
                }
            }
            other => panic!("unexpected block variant: {other:?}"),
        }
    }

    #[test]
    fn adaptive_thinking_serializes() {
        let config = ThinkingConfig::adaptive(ThinkingEffort::High);
        let value = serde_json::to_value(&config).unwrap();
        assert_eq!(value["type"], "adaptive");
        assert_eq!(value["effort"], "high");
        assert!(value.get("budget_tokens").is_none());
    }

    #[test]
    fn adaptive_thinking_with_display_omitted() {
        let config = ThinkingConfig::adaptive(ThinkingEffort::Max).with_display_omitted();
        let value = serde_json::to_value(&config).unwrap();
        assert_eq!(value["type"], "adaptive");
        assert_eq!(value["effort"], "max");
        assert_eq!(value["display"], "omitted");
    }

    #[test]
    fn thinking_effort_all_levels() {
        for (effort, expected) in [
            (ThinkingEffort::Low, "low"),
            (ThinkingEffort::Medium, "medium"),
            (ThinkingEffort::High, "high"),
            (ThinkingEffort::Max, "max"),
        ] {
            let json = serde_json::to_value(effort).unwrap();
            assert_eq!(json.as_str().unwrap(), expected);
        }
    }

    #[test]
    fn output_config_json_schema_serializes() {
        let config = OutputConfig::json_schema(
            "person",
            serde_json::json!({
                "type": "object",
                "properties": { "name": { "type": "string" } },
                "required": ["name"]
            }),
        );
        let value = serde_json::to_value(&config).unwrap();
        assert_eq!(value["format"]["type"], "json_schema");
        assert_eq!(value["format"]["name"], "person");
    }

    #[test]
    fn message_request_with_output_config_and_citations() {
        let request = MessageRequest::new()
            .max_tokens(1024)
            .add_user_message("Extract data")
            .output_config(OutputConfig::json_schema("data", serde_json::json!({})))
            .enable_citations();

        assert!(request.output_config.is_some());
        assert!(request.citations.as_ref().unwrap().enabled);
    }

    #[test]
    fn stop_reason_context_window_exceeded_round_trip() {
        let reason = StopReason::ModelContextWindowExceeded;
        let json = serde_json::to_value(&reason).unwrap();
        assert_eq!(json, "model_context_window_exceeded");
        let parsed: StopReason = serde_json::from_value(json).unwrap();
        assert_eq!(parsed, StopReason::ModelContextWindowExceeded);
    }

    #[test]
    fn web_search_v2_tool_serializes() {
        let tool = WebSearchTool::new_v2().with_max_uses(Some(5));
        let value = serde_json::to_value(&tool).unwrap();
        assert_eq!(value["type"], "web_search_20260209");
        assert_eq!(value["max_uses"], 5);
    }

    #[test]
    fn tool_choice_none_variant() {
        // ToolChoice uses #[serde(untagged)], so None serializes as null
        let choice = ToolChoice::None;
        let json = serde_json::to_value(&choice).unwrap();
        assert!(json.is_null());
    }

    #[test]
    fn content_block_delta_with_thinking() {
        let delta_json = serde_json::json!({
            "type": "thinking_delta",
            "thinking": "Let me think about this..."
        });
        let delta: ContentBlockDelta = serde_json::from_value(delta_json).unwrap();
        assert_eq!(delta.block_type, "thinking_delta");
        assert_eq!(delta.thinking.as_deref(), Some("Let me think about this..."));
    }

    #[test]
    fn content_block_thinking_round_trip() {
        let block = ContentBlock::Thinking {
            thinking: "reasoning".to_string(),
            signature: Some("sig123".to_string()),
        };
        let value = serde_json::to_value(&block).unwrap();
        assert_eq!(value["type"], "thinking");
        assert_eq!(value["thinking"], "reasoning");
        assert_eq!(value["signature"], "sig123");

        let parsed: ContentBlock = serde_json::from_value(value).unwrap();
        assert_eq!(parsed, block);
    }

    #[test]
    fn citations_config_serializes() {
        let config = CitationsConfig::enabled();
        let json = serde_json::to_value(&config).unwrap();
        assert_eq!(json["enabled"], true);
    }
}
