use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Function call sent to the model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    /// Opaque ID to return with the response when present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub name: String,
    #[serde(default)]
    pub args: HashMap<String, serde_json::Value>,
}

/// Function response returned by the model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionResponse {
    /// ID of the corresponding function call, when supplied by the model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub name: String,
    pub response: serde_json::Value,
}

impl FunctionResponse {
    /// Construct a response with the name and ID required for this call.
    #[must_use]
    pub fn for_call(call: &FunctionCall, response: serde_json::Value) -> Self {
        Self { id: call.id.clone(), name: call.name.clone(), response }
    }
}

/// Function declaration for tool use.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDeclaration {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}
