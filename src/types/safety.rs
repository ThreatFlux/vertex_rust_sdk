use serde::{Deserialize, Serialize};

use super::content::Content;

/// Safety rating returned with model output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyRating {
    pub category: String,
    pub probability: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocked: Option<bool>,
}

/// Safety settings supplied with the request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetySetting {
    pub category: String,
    pub threshold: String,
}

/// Finish reason for generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FinishReason {
    #[serde(rename = "FINISH_REASON_UNSPECIFIED")]
    Unspecified,
    #[serde(rename = "STOP")]
    Stop,
    #[serde(rename = "MAX_TOKENS")]
    MaxTokens,
    #[serde(rename = "SAFETY")]
    Safety,
    #[serde(rename = "RECITATION")]
    Recitation,
    #[serde(rename = "FINISH_REASON_BLOCKLIST")]
    Blocklist,
    #[serde(rename = "FINISH_REASON_PROHIBITED_CONTENT")]
    ProhibitedContent,
    #[serde(rename = "FINISH_REASON_IMAGE_PROHIBITED_CONTENT")]
    ImageProhibitedContent,
    #[serde(rename = "FINISH_REASON_NO_IMAGE")]
    NoImage,
    #[serde(rename = "FINISH_REASON_SPII")]
    SensitivePersonallyIdentifiableInformation,
    #[serde(rename = "FINISH_REASON_MALFORMED_FUNCTION_CALL")]
    MalformedFunctionCall,
    #[serde(rename = "OTHER")]
    Other,
    #[serde(other)]
    Unknown,
}

/// Generation candidate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candidate {
    pub content: Content,
    #[serde(rename = "finishReason")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<FinishReason>,
    #[serde(rename = "safetyRatings")]
    #[serde(default)]
    pub safety_ratings: Vec<SafetyRating>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<i32>,
}
