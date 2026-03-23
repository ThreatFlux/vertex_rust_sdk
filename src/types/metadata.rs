use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Request-scoped metadata passed to Gemini models.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RequestMetadata {
    /// Optional user identifier used for safety/auditing.
    #[serde(rename = "userId", skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    /// Additional key/value pairs forwarded to the model.
    #[serde(flatten)]
    pub custom: HashMap<String, serde_json::Value>,
}

impl RequestMetadata {
    /// Create a new, empty metadata payload.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Attach a user identifier.
    #[must_use]
    pub fn with_user_id(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    /// Attach a custom metadata field.
    #[must_use]
    pub fn with_custom(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.custom.insert(key.into(), value);
        self
    }

    /// Returns true when no metadata has been configured.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.user_id.is_none() && self.custom.is_empty()
    }
}
