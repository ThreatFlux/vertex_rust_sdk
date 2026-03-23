//! Error types for the Vertex Rust SDK

use std::fmt;
use thiserror::Error;

/// Result type alias for Vertex SDK operations
pub type Result<T> = std::result::Result<T, VertexError>;

/// Main error type for Vertex AI SDK operations
#[derive(Error, Debug)]
pub enum VertexError {
    /// Authentication errors
    #[error("Authentication failed: {message}")]
    Authentication { message: String },

    /// HTTP request/response errors
    #[error("HTTP error: {status} - {message}")]
    Http { status: u16, message: String },

    /// API errors returned by Vertex AI
    #[error("API error: {code} - {message}")]
    Api { code: String, message: String },

    /// JSON serialization/deserialization errors
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// HTTP client errors
    #[error("Request error: {0}")]
    Request(#[from] reqwest::Error),

    /// Invalid configuration or parameters
    #[error("Invalid configuration: {message}")]
    Configuration { message: String },

    /// Token-related errors
    #[error("Token error: {message}")]
    Token { message: String },

    /// Streaming errors
    #[error("Streaming error: {message}")]
    Streaming { message: String },

    /// I/O errors
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Generic error with custom message
    #[error("{message}")]
    Generic { message: String },
}

impl VertexError {
    /// Create a new authentication error
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn authentication<S: Into<String>>(message: S) -> Self {
        Self::Authentication { message: message.into() }
    }

    /// Create a new HTTP error
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn http(status: u16, message: String) -> Self {
        Self::Http { status, message }
    }

    /// Create a new API error
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn api<S: Into<String>>(code: S, message: S) -> Self {
        Self::Api { code: code.into(), message: message.into() }
    }

    /// Create a new configuration error
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn configuration<S: Into<String>>(message: S) -> Self {
        Self::Configuration { message: message.into() }
    }

    /// Create a new token error
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn token<S: Into<String>>(message: S) -> Self {
        Self::Token { message: message.into() }
    }

    /// Create a new streaming error
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn streaming<S: Into<String>>(message: S) -> Self {
        Self::Streaming { message: message.into() }
    }

    /// Create a new generic error
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn generic<S: Into<String>>(message: S) -> Self {
        Self::Generic { message: message.into() }
    }

    /// Check if this is an authentication error
    #[must_use]
    pub const fn is_authentication(&self) -> bool {
        matches!(self, Self::Authentication { .. })
    }

    /// Check if this is an HTTP error
    #[must_use]
    pub const fn is_http(&self) -> bool {
        matches!(self, Self::Http { .. })
    }

    /// Check if this is an API error
    #[must_use]
    pub const fn is_api(&self) -> bool {
        matches!(self, Self::Api { .. })
    }

    /// Get the HTTP status code if this is an HTTP error
    #[must_use]
    pub const fn status_code(&self) -> Option<u16> {
        match self {
            Self::Http { status, .. } => Some(*status),
            _ => None,
        }
    }
}

/// Error response from Vertex AI API
#[derive(Debug, serde::Deserialize)]
pub struct ApiError {
    pub error: ApiErrorDetails,
}

/// Details of an API error
#[derive(Debug, serde::Deserialize)]
pub struct ApiErrorDetails {
    pub code: i32,
    pub message: String,
    pub status: String,
    #[serde(default)]
    pub details: Vec<serde_json::Value>,
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "API Error {}: {} ({})", self.error.code, self.error.message, self.error.status)
    }
}

impl From<ApiError> for VertexError {
    fn from(api_error: ApiError) -> Self {
        Self::Api { code: api_error.error.status, message: api_error.error.message }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let auth_err = VertexError::authentication("Invalid credentials");
        assert!(auth_err.is_authentication());

        let http_err = VertexError::http(404, "Not found".to_string());
        assert!(http_err.is_http());
        assert_eq!(http_err.status_code(), Some(404));

        let api_err = VertexError::api("INVALID_REQUEST", "Bad request");
        assert!(api_err.is_api());
    }
}
