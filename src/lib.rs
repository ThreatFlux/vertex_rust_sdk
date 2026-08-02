//! # ThreatFlux Vertex Rust SDK
//!
//! An async, community-maintained Rust client for generative AI APIs on Google
//! Cloud Vertex AI. This crate is not an official Google, Google Cloud, Vertex
//! AI, Gemini, Anthropic, or Claude SDK.
//!
//! ## Features
//!
//! - Gemini content generation and server-sent event streaming
//! - Function calling, structured output, grounding, and safety types
//! - Embeddings, token counting, chat helpers, and context caching
//! - Claude messages and streaming through Vertex publisher endpoints
//! - Pluggable bearer-token authentication via [`AuthProvider`]
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use threatflux_vertex_rust_sdk::{config::Config, GenerateContentRequest, VertexClient};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = Config::from_env()?;
//!     let model = config.model.clone();
//!     let client = VertexClient::new(config).await?;
//!
//!     let request = GenerateContentRequest::new("Why is the sky blue?");
//!     let response = client.generate_content(&model, &request).await?;
//!
//!     if let Some(text) = response.text() {
//!         println!("{text}");
//!     }
//!     Ok(())
//! }
//! ```
//!
//! Model IDs and regional availability are provider-controlled. Set
//! `VERTEX_PROJECT_ID`, `VERTEX_REGION`, and `VERTEX_MODEL` explicitly for
//! repeatable deployments. See the repository configuration guide for the
//! implemented credential precedence, timeout, retry, and proxy behavior.

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
pub use api::embeddings::{
    EmbeddingInstance, EmbeddingParameters, EmbeddingPrediction, EmbeddingRequest,
    EmbeddingResponse, EmbeddingTaskType, EmbeddingValues, EmbeddingsApi,
};
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
