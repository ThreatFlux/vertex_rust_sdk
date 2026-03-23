use serde::{Deserialize, Serialize};

/// Configuration for Google Search grounding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroundingConfig {
    /// Whether to disable attribution for grounding sources.
    #[serde(rename = "disableAttribution")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_attribution: Option<bool>,
}

impl GroundingConfig {
    /// Create a new grounding config with default settings.
    #[must_use]
    pub const fn new() -> Self {
        Self { disable_attribution: None }
    }

    /// Create grounding config with attribution disabled.
    #[must_use]
    pub const fn without_attribution() -> Self {
        Self { disable_attribution: Some(true) }
    }

    /// Create grounding config with attribution enabled (default).
    #[must_use]
    pub const fn with_attribution() -> Self {
        Self { disable_attribution: Some(false) }
    }
}

impl Default for GroundingConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Grounding metadata in response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroundingMetadata {
    /// Web search queries performed.
    #[serde(rename = "webSearchQueries")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub web_search_queries: Option<Vec<String>>,

    /// Search entry points.
    #[serde(rename = "searchEntryPoint")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_entry_point: Option<SearchEntryPoint>,

    /// Grounding chunks and citations.
    #[serde(rename = "groundingChunks")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grounding_chunks: Option<Vec<GroundingChunk>>,

    /// Grounding supports.
    #[serde(rename = "groundingSupports")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grounding_supports: Option<Vec<GroundingSupport>>,
}

/// Search entry point for web searches.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchEntryPoint {
    /// Rendered content from the search.
    #[serde(rename = "renderedContent")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rendered_content: Option<String>,

    /// SDK blob containing search metadata.
    #[serde(rename = "sdkBlob")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sdk_blob: Option<String>,
}

/// A chunk of grounded content from web search.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroundingChunk {
    /// The grounded content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,

    /// Source URL for this chunk.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,

    /// Title of the source.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

/// Support information for grounding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroundingSupport {
    /// Grounding chunk indices that support this content.
    #[serde(rename = "groundingChunkIndices")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grounding_chunk_indices: Option<Vec<i32>>,

    /// Confidence score for this grounding.
    #[serde(rename = "confidenceScore")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence_score: Option<f32>,

    /// Start index in the generated content.
    #[serde(rename = "startIndex")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_index: Option<i32>,

    /// End index in the generated content.
    #[serde(rename = "endIndex")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_index: Option<i32>,

    /// The text that is supported by grounding.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}
