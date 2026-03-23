//! Request and response models for Vertex AI API

use crate::types::{
    Candidate, CodeExecutionResult, Content, ExecutableCode, FinishReason, FunctionCall,
    FunctionCallingConfig, GenerationConfig, GroundingChunk, GroundingConfig, GroundingMetadata,
    GroundingSupport, Part, RequestMetadata, SafetySetting, Tool, ToolConfig, UsageMetadata,
};
use serde::{Deserialize, Serialize};

// Re-export models API types for convenience
pub use crate::api::models::{
    ListLocationsResponse, ListModelsResponse, Location, Model, ModelsApi, TemperatureRange,
    TopPRange,
};

/// Request to generate content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateContentRequest {
    /// Input content
    pub contents: Vec<Content>,
    /// Generation configuration
    #[serde(rename = "generationConfig")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation_config: Option<GenerationConfig>,
    /// Safety settings
    #[serde(rename = "safetySettings")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safety_settings: Option<Vec<SafetySetting>>,
    /// Tools for function calling
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
    /// System instruction
    #[serde(rename = "systemInstruction")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_instruction: Option<Content>,
    /// Cached content reference
    #[serde(rename = "cachedContent")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached_content: Option<String>,
    /// Tool configuration
    #[serde(rename = "toolConfig")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_config: Option<ToolConfig>,
    /// Optional metadata retained locally (not serialized to Vertex API).
    #[serde(skip_serializing, skip_deserializing)]
    pub metadata: Option<RequestMetadata>,
}

impl GenerateContentRequest {
    /// Create a new request with a single text prompt
    #[must_use]
    pub fn new<S: Into<String>>(prompt: S) -> Self {
        Self {
            contents: vec![Content::user_text(prompt)],
            generation_config: Some(GenerationConfig::default()),
            safety_settings: None,
            tools: None,
            system_instruction: None,
            cached_content: None,
            tool_config: None,
            metadata: None,
        }
    }

    /// Create a new request with multiple contents (for conversations)
    #[must_use]
    pub fn with_contents(contents: Vec<Content>) -> Self {
        Self {
            contents,
            generation_config: Some(GenerationConfig::default()),
            safety_settings: None,
            tools: None,
            system_instruction: None,
            cached_content: None,
            tool_config: None,
            metadata: None,
        }
    }

    /// Add generation configuration
    #[must_use]
    pub fn with_generation_config(mut self, config: GenerationConfig) -> Self {
        self.generation_config = Some(config);
        self
    }

    /// Add safety settings
    #[must_use]
    pub fn with_safety_settings(mut self, settings: Vec<SafetySetting>) -> Self {
        self.safety_settings = Some(settings);
        self
    }

    /// Add tools for function calling
    #[must_use]
    pub fn with_tools(mut self, tools: Vec<Tool>) -> Self {
        self.tools = Some(tools);
        self
    }

    /// Add system instruction
    #[must_use]
    pub fn with_system_instruction(mut self, instruction: Content) -> Self {
        self.system_instruction = Some(instruction);
        self
    }

    /// Add system instruction from text
    #[must_use]
    pub fn with_system_text<S: Into<String>>(mut self, text: S) -> Self {
        self.system_instruction = Some(Content::system_text(text));
        self
    }

    /// Set cached content reference
    #[must_use]
    pub fn with_cached_content<S: Into<String>>(mut self, cache_id: S) -> Self {
        self.cached_content = Some(cache_id.into());
        self
    }

    /// Set cached content from `CachedContentRef`
    #[must_use]
    pub fn with_cache_ref(mut self, cache_ref: &crate::cache::CachedContentRef) -> Self {
        self.cached_content = Some(cache_ref.name.clone());
        self
    }

    /// Enable Google Search grounding with default configuration
    #[must_use]
    pub fn with_google_search(mut self) -> Self {
        let mut tools = self.tools.unwrap_or_default();
        tools.push(Tool::google_search());
        self.tools = Some(tools);
        self
    }

    /// Enable Google Search grounding with custom configuration
    #[must_use]
    pub fn with_grounding(mut self, config: GroundingConfig) -> Self {
        let mut tools = self.tools.unwrap_or_default();
        tools.push(Tool::google_search_with_config(config));
        self.tools = Some(tools);
        self
    }

    /// Enable Google Search grounding without attribution
    #[must_use]
    pub fn with_google_search_no_attribution(mut self) -> Self {
        let mut tools = self.tools.unwrap_or_default();
        tools.push(Tool::google_search_with_config(GroundingConfig::without_attribution()));
        self.tools = Some(tools);
        self
    }

    /// Set tool configuration
    #[must_use]
    pub fn with_tool_config(mut self, tool_config: ToolConfig) -> Self {
        self.tool_config = Some(tool_config);
        self
    }

    /// Set function calling mode to AUTO (default)
    #[must_use]
    pub fn with_function_calling_auto(mut self) -> Self {
        self.tool_config = Some(ToolConfig::auto());
        self
    }

    /// Set function calling mode to ANY (force function calling)
    #[must_use]
    pub fn with_function_calling_any(mut self) -> Self {
        self.tool_config = Some(ToolConfig::any());
        self
    }

    /// Set function calling mode to NONE (disable function calling)
    #[must_use]
    pub fn with_function_calling_none(mut self) -> Self {
        self.tool_config = Some(ToolConfig::none());
        self
    }

    /// Set function calling mode to VALIDATED
    #[must_use]
    pub fn with_function_calling_validated(mut self) -> Self {
        self.tool_config = Some(ToolConfig::validated());
        self
    }

    /// Force specific functions to be called
    #[must_use]
    pub fn with_force_functions(mut self, function_names: Vec<String>) -> Self {
        self.tool_config = Some(ToolConfig::with_config(
            FunctionCallingConfig::any_with_functions(function_names),
        ));
        self
    }

    /// Attach request metadata.
    #[must_use]
    pub fn with_metadata(mut self, metadata: RequestMetadata) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

/// Response from generate content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateContentResponse {
    /// Generated candidates
    pub candidates: Vec<Candidate>,
    /// Usage metadata
    #[serde(rename = "usageMetadata")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage_metadata: Option<UsageMetadata>,
    /// Grounding metadata from Google Search
    #[serde(rename = "groundingMetadata")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grounding_metadata: Option<GroundingMetadata>,
}

impl GenerateContentResponse {
    /// Get the text from the first candidate, if available
    #[must_use]
    pub fn text(&self) -> Option<String> {
        self.candidates.first().and_then(|candidate| {
            candidate.content.parts.iter().find_map(|part| {
                if let Part::Text { text } = part {
                    Some(text.clone())
                } else {
                    None
                }
            })
        })
    }

    /// Parse the response text as JSON
    ///
    /// # Errors
    ///
    /// Returns an error when the response does not contain text or when the
    /// text cannot be parsed as JSON.
    pub fn json(&self) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        self.text().map_or_else(
            || Err("No text response available".into()),
            |text| serde_json::from_str(&text).map_err(Into::into),
        )
    }

    /// Parse the response text as a specific type
    ///
    /// # Errors
    ///
    /// Returns an error when the response text is absent or cannot be parsed
    /// into the requested type.
    pub fn json_as<T>(&self) -> Result<T, Box<dyn std::error::Error + Send + Sync>>
    where
        T: serde::de::DeserializeOwned,
    {
        self.text().map_or_else(
            || Err("No text response available".into()),
            |text| serde_json::from_str(&text).map_err(Into::into),
        )
    }

    /// Check if the response appears to be valid JSON
    #[must_use]
    pub fn is_json(&self) -> bool {
        self.json().is_ok()
    }

    /// Get structured data as a pretty-printed JSON string
    #[must_use]
    pub fn json_pretty(&self) -> Option<String> {
        self.json().ok().and_then(|value| serde_json::to_string_pretty(&value).ok())
    }

    /// Get all function calls from the first candidate
    #[must_use]
    pub fn function_calls(&self) -> Vec<FunctionCall> {
        self.candidates
            .first()
            .map(|candidate| {
                candidate
                    .content
                    .parts
                    .iter()
                    .filter_map(|part| {
                        if let Part::FunctionCall { function_call } = part {
                            Some(function_call.clone())
                        } else {
                            None
                        }
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get all executable code from the first candidate
    #[must_use]
    pub fn executable_code(&self) -> Vec<ExecutableCode> {
        self.candidates
            .first()
            .map(|candidate| {
                candidate
                    .content
                    .parts
                    .iter()
                    .filter_map(|part| {
                        if let Part::ExecutableCode { executable_code } = part {
                            Some(executable_code.clone())
                        } else {
                            None
                        }
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get all code execution results from the first candidate
    #[must_use]
    pub fn code_execution_results(&self) -> Vec<CodeExecutionResult> {
        self.candidates
            .first()
            .map(|candidate| {
                candidate
                    .content
                    .parts
                    .iter()
                    .filter_map(|part| {
                        if let Part::CodeExecutionResult { code_execution_result } = part {
                            Some(code_execution_result.clone())
                        } else {
                            None
                        }
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get the finish reason from the first candidate
    #[must_use]
    pub fn finish_reason(&self) -> Option<&FinishReason> {
        self.candidates.first()?.finish_reason.as_ref()
    }

    /// Get all thinking content from the first candidate
    #[must_use]
    pub fn thinking_content(&self) -> Vec<String> {
        self.candidates
            .first()
            .map(|candidate| {
                candidate
                    .content
                    .parts
                    .iter()
                    .filter_map(|part| {
                        if let Part::Thinking { thought } = part {
                            Some(thought.clone())
                        } else {
                            None
                        }
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get the first thinking content, if available
    #[must_use]
    pub fn thinking_text(&self) -> Option<String> {
        self.thinking_content().into_iter().next()
    }

    /// Check if the response contains thinking content
    #[must_use]
    pub fn has_thinking(&self) -> bool {
        self.candidates.first().is_some_and(|candidate| {
            candidate.content.parts.iter().any(|part| matches!(part, Part::Thinking { .. }))
        })
    }

    /// Get text content excluding thinking parts
    #[must_use]
    pub fn text_without_thinking(&self) -> Option<String> {
        self.candidates.first().and_then(|candidate| {
            candidate.content.parts.iter().find_map(|part| {
                if let Part::Text { text } = part {
                    Some(text.clone())
                } else {
                    None
                }
            })
        })
    }

    /// Get combined thinking and text content with clear separation
    #[must_use]
    pub fn full_response(&self) -> String {
        let mut response = String::new();

        if let Some(candidate) = self.candidates.first() {
            let thinking_parts: Vec<_> = candidate
                .content
                .parts
                .iter()
                .filter_map(|part| {
                    if let Part::Thinking { thought } = part {
                        Some(thought.as_str())
                    } else {
                        None
                    }
                })
                .collect();

            let text_parts: Vec<_> =
                candidate
                    .content
                    .parts
                    .iter()
                    .filter_map(|part| {
                        if let Part::Text { text } = part {
                            Some(text.as_str())
                        } else {
                            None
                        }
                    })
                    .collect();

            if !thinking_parts.is_empty() {
                response.push_str("--- THINKING ---\n");
                for thought in thinking_parts {
                    response.push_str(thought);
                    response.push('\n');
                }
                response.push_str("--- END THINKING ---\n\n");
            }

            if !text_parts.is_empty() {
                for text in text_parts {
                    response.push_str(text);
                }
            }
        }

        response
    }

    /// Get grounding metadata, if available
    #[must_use]
    pub const fn grounding_metadata(&self) -> Option<&GroundingMetadata> {
        self.grounding_metadata.as_ref()
    }

    /// Check if the response contains grounding information
    #[must_use]
    pub const fn has_grounding(&self) -> bool {
        self.grounding_metadata.is_some()
    }

    /// Get all web search queries used for grounding
    #[must_use]
    pub fn web_search_queries(&self) -> Vec<String> {
        self.grounding_metadata
            .as_ref()
            .and_then(|metadata| metadata.web_search_queries.as_ref())
            .cloned()
            .unwrap_or_default()
    }

    /// Get all grounding chunks (citations/sources)
    #[must_use]
    pub fn grounding_chunks(&self) -> Vec<&GroundingChunk> {
        self.grounding_metadata
            .as_ref()
            .and_then(|metadata| metadata.grounding_chunks.as_ref())
            .map(|chunks| chunks.iter().collect())
            .unwrap_or_default()
    }

    /// Get all grounding supports (confidence scores and text spans)
    #[must_use]
    pub fn grounding_supports(&self) -> Vec<&GroundingSupport> {
        self.grounding_metadata
            .as_ref()
            .and_then(|metadata| metadata.grounding_supports.as_ref())
            .map(|supports| supports.iter().collect())
            .unwrap_or_default()
    }
}

/// Request to count tokens
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CountTokensRequest {
    /// Input content
    pub contents: Vec<Content>,
}

impl CountTokensRequest {
    /// Create a new request with a single text prompt
    #[must_use]
    pub fn new<S: Into<String>>(prompt: S) -> Self {
        Self { contents: vec![Content::user_text(prompt)] }
    }

    /// Create a new request with multiple contents
    #[must_use]
    pub const fn with_contents(contents: Vec<Content>) -> Self {
        Self { contents }
    }
}

/// Response from count tokens
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CountTokensResponse {
    /// Total number of tokens
    #[serde(rename = "totalTokens")]
    pub total_tokens: i32,
}

/// Streaming response chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingResponse {
    /// Generated candidates
    pub candidates: Vec<Candidate>,
    /// Usage metadata (only in final chunk)
    #[serde(rename = "usageMetadata")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage_metadata: Option<UsageMetadata>,
    /// Grounding metadata from Google Search
    #[serde(rename = "groundingMetadata")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grounding_metadata: Option<GroundingMetadata>,
}

impl StreamingResponse {
    /// Get the text from the first candidate, if available
    #[must_use]
    pub fn text(&self) -> Option<String> {
        self.candidates.first().and_then(|candidate| {
            candidate.content.parts.iter().find_map(|part| {
                if let Part::Text { text } = part {
                    Some(text.clone())
                } else {
                    None
                }
            })
        })
    }

    /// Get all executable code from the first candidate
    #[must_use]
    pub fn executable_code(&self) -> Vec<ExecutableCode> {
        self.candidates
            .first()
            .map(|candidate| {
                candidate
                    .content
                    .parts
                    .iter()
                    .filter_map(|part| {
                        if let Part::ExecutableCode { executable_code } = part {
                            Some(executable_code.clone())
                        } else {
                            None
                        }
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get all code execution results from the first candidate
    #[must_use]
    pub fn code_execution_results(&self) -> Vec<CodeExecutionResult> {
        self.candidates
            .first()
            .map(|candidate| {
                candidate
                    .content
                    .parts
                    .iter()
                    .filter_map(|part| {
                        if let Part::CodeExecutionResult { code_execution_result } = part {
                            Some(code_execution_result.clone())
                        } else {
                            None
                        }
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Check if this is the final chunk (has usage metadata)
    #[must_use]
    pub const fn is_final(&self) -> bool {
        self.usage_metadata.is_some()
    }

    /// Get thinking content from the first candidate, if available
    #[must_use]
    pub fn thinking_content(&self) -> Vec<String> {
        self.candidates
            .first()
            .map(|candidate| {
                candidate
                    .content
                    .parts
                    .iter()
                    .filter_map(|part| {
                        if let Part::Thinking { thought } = part {
                            Some(thought.clone())
                        } else {
                            None
                        }
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get the first thinking content, if available
    #[must_use]
    pub fn thinking_text(&self) -> Option<String> {
        self.thinking_content().into_iter().next()
    }

    /// Check if this chunk contains thinking content
    #[must_use]
    pub fn has_thinking(&self) -> bool {
        self.candidates.first().is_some_and(|candidate| {
            candidate.content.parts.iter().any(|part| matches!(part, Part::Thinking { .. }))
        })
    }

    /// Get grounding metadata from this chunk, if available
    #[must_use]
    pub const fn grounding_metadata(&self) -> Option<&GroundingMetadata> {
        self.grounding_metadata.as_ref()
    }

    /// Check if this chunk contains grounding information
    #[must_use]
    pub const fn has_grounding(&self) -> bool {
        self.grounding_metadata.is_some()
    }

    /// Get web search queries from this chunk
    #[must_use]
    pub fn web_search_queries(&self) -> Vec<String> {
        self.grounding_metadata
            .as_ref()
            .and_then(|metadata| metadata.web_search_queries.as_ref())
            .cloned()
            .unwrap_or_default()
    }

    /// Get grounding chunks from this response chunk
    #[must_use]
    pub fn grounding_chunks(&self) -> Vec<&GroundingChunk> {
        self.grounding_metadata
            .as_ref()
            .and_then(|metadata| metadata.grounding_chunks.as_ref())
            .map(|chunks| chunks.iter().collect())
            .unwrap_or_default()
    }
}

/// Chat message for simplified chat interface
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

impl ChatMessage {
    /// Create a user message
    #[must_use]
    pub fn user<S: Into<String>>(content: S) -> Self {
        Self { role: "user".to_string(), content: content.into() }
    }

    /// Create a model/assistant message
    #[must_use]
    pub fn assistant<S: Into<String>>(content: S) -> Self {
        Self { role: "model".to_string(), content: content.into() }
    }

    /// Create a system message
    #[must_use]
    pub fn system<S: Into<String>>(content: S) -> Self {
        Self { role: "system".to_string(), content: content.into() }
    }
}

impl From<ChatMessage> for Content {
    fn from(message: ChatMessage) -> Self {
        Self { role: message.role, parts: vec![Part::text(message.content)] }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_content_request() {
        let request = GenerateContentRequest::new("Hello, world!");
        assert_eq!(request.contents.len(), 1);
        assert!(request.generation_config.is_some());

        if let Part::Text { text } = &request.contents[0].parts[0] {
            assert_eq!(text, "Hello, world!");
        } else {
            panic!("Expected text part");
        }
    }

    #[test]
    fn test_chat_message() {
        let user_msg = ChatMessage::user("Hello");
        assert_eq!(user_msg.role, "user");
        assert_eq!(user_msg.content, "Hello");

        let content: Content = user_msg.into();
        assert_eq!(content.role, "user");
        assert_eq!(content.parts.len(), 1);
    }

    #[test]
    fn test_count_tokens_request() {
        let request = CountTokensRequest::new("Count these tokens");
        assert_eq!(request.contents.len(), 1);
    }

    #[test]
    fn test_generate_content_request_with_grounding() {
        let request = GenerateContentRequest::new("What's the latest news?").with_google_search();

        assert!(request.tools.is_some());
        let tools = request.tools.unwrap();
        assert_eq!(tools.len(), 1);

        if let Tool::GoogleSearchRetrieval { google_search_retrieval } = &tools[0] {
            assert!(google_search_retrieval.disable_attribution.is_none());
        } else {
            panic!("Expected Google Search retrieval tool");
        }
    }

    #[test]
    fn test_generate_content_request_with_custom_grounding() {
        let config = GroundingConfig::without_attribution();
        let request = GenerateContentRequest::new("Current events").with_grounding(config);

        assert!(request.tools.is_some());
        let tools = request.tools.unwrap();
        assert_eq!(tools.len(), 1);

        if let Tool::GoogleSearchRetrieval { google_search_retrieval } = &tools[0] {
            assert_eq!(google_search_retrieval.disable_attribution, Some(true));
        } else {
            panic!("Expected Google Search retrieval tool");
        }
    }
}
