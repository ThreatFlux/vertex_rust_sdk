use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Usage metadata surfaced in responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageMetadata {
    #[serde(rename = "promptTokenCount")]
    #[serde(default)]
    pub prompt_token_count: i32,
    #[serde(rename = "candidatesTokenCount")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub candidates_token_count: Option<i32>,
    #[serde(rename = "totalTokenCount")]
    #[serde(default)]
    pub total_token_count: i32,
    #[serde(rename = "trafficType")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub traffic_type: Option<String>,
    /// Token usage broken down by modality (for example modality.TEXT, modality.IMAGE).
    #[serde(rename = "modalityTokenCount")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modality_token_count: Option<HashMap<String, ModalityUsage>>,
}

/// Token usage information for a single modality.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModalityUsage {
    #[serde(rename = "promptTokenCount")]
    #[serde(default)]
    pub prompt_token_count: i32,
    #[serde(rename = "candidatesTokenCount")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub candidates_token_count: Option<i32>,
    #[serde(rename = "totalTokenCount")]
    #[serde(default)]
    pub total_token_count: i32,
}
