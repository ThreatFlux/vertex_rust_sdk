//! # `ThreatFlux` Vertex Rust SDK
//!
//! A Rust SDK for Google Cloud Vertex AI API, providing access to Gemini models and other AI services.
//!
//! ## Features
//!
//! - **Authentication**: `OAuth2`, Service Account, and Application Default Credentials
//! - **Gemini Models**: Content generation with streaming and non-streaming support
//! - **Function Calling**: Tool/function calling capabilities
//! - **Token Counting**: Count tokens in content
//! - **Chat Completions**: Multi-turn conversations
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use threatflux_vertex_rust_sdk::{config::Config, GenerateContentRequest, VertexClient};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = Config {
//!         project_id: "your-project-id".into(),
//!         region: "us-central1".into(),
//!         ..Config::default()
//!     };
//!     let client = VertexClient::new(config).await?;
//!
//!     let request = GenerateContentRequest::new("Why is the sky blue?");
//!     let response = client.generate_content("gemini-2.5-flash", &request).await?;
//!
//!     if let Some(text) = response.text() {
//!         println!("Response: {}", text);
//!     }
//!     Ok(())
//! }
//! ```

pub mod api;
pub mod auth;
pub mod builders;
pub mod cache;
pub mod chat_core;
pub mod claude;
pub mod client;
pub mod config;
pub mod error;
pub mod media;
pub mod model_descriptor;
pub mod model_info;
pub mod models;
pub mod streaming;
pub mod streaming_support;
pub mod types;

// Re-export main types for convenience
pub use api::chat::ChatConversation;
pub use api::generate::GenerateApi;
pub use api::models::{ListLocationsResponse, ListModelsResponse, Location, Model, ModelsApi};
pub use auth::{
    from_env, ApplicationDefaultCredentials, AuthProvider, EnvAuth, ServiceAccountAuth,
};
pub use builders::{ContentRequestBuilder, FunctionBuilder, TokenCountBuilder};
pub use cache::{
    CacheApi, CacheUsageMetadata, CachedContent, CachedContentRef, CreateCachedContentRequest,
    ListCachedContentsResponse, UpdateCachedContentRequest,
};
pub use client::{VertexClient, VertexClientBuilder};
pub use error::{Result, VertexError};
pub use media::{classify_inline_data, InlineDataClassification, InlineDataKind};
pub use model_descriptor::ModelDescriptor;
pub use model_info::{get_model_info, ModelInfo};
pub use models::*;
pub use streaming::{ChatStream, ChatStreamChunk, SseParser};
pub use types::*;

pub use crate as threatflux_vertex_rust_sdk;

/// The default API endpoint for Vertex AI
pub const DEFAULT_ENDPOINT: &str = "https://aiplatform.googleapis.com";

/// The current version of the SDK
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// User agent string for HTTP requests
#[must_use]
pub fn user_agent() -> String {
    format!("vertex-rust-sdk/{VERSION}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_agent() {
        let ua = user_agent();
        assert!(ua.starts_with("vertex-rust-sdk/"));
    }
}
