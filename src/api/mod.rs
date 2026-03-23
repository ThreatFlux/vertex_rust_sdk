//! API implementations for Vertex AI

pub mod chat;
pub mod claude;
pub mod functions;
pub mod generate;
pub mod models;
pub mod stream;
pub mod tokens;

// The implementations are provided as methods on VertexClient
// No need to re-export here as they're internal implementation details
