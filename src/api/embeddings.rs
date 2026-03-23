//! Vertex AI text embeddings API
//!
//! Wraps the `predict` endpoint for embedding models such as
//! `gemini-embedding-001`. The request/response shapes follow the Vertex AI
//! text-embeddings reference.

use crate::client::VertexClient;
use crate::error::{Result, VertexError};
use crate::model_descriptor::ModelDescriptor;
use serde::{Deserialize, Serialize};

/// A single embedding instance sent to the predict endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingInstance {
    /// The text content to embed.
    pub content: String,
    /// Optional task type that describes the intended downstream use.
    #[serde(rename = "taskType", skip_serializing_if = "Option::is_none")]
    pub task_type: Option<EmbeddingTaskType>,
    /// Optional title for the document (only meaningful for retrieval tasks).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

/// Task types recognised by the Vertex embeddings API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EmbeddingTaskType {
    RetrievalQuery,
    RetrievalDocument,
    SemanticSimilarity,
    Classification,
    Clustering,
    QuestionAnswering,
    FactVerification,
}

/// Optional parameters for the embeddings predict call.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EmbeddingParameters {
    /// The desired output dimensionality. When omitted the model returns its
    /// default dimensionality.
    #[serde(rename = "outputDimensionality", skip_serializing_if = "Option::is_none")]
    pub output_dimensionality: Option<u32>,
    /// Whether to auto-truncate inputs that exceed the model's token limit.
    #[serde(rename = "autoTruncate", skip_serializing_if = "Option::is_none")]
    pub auto_truncate: Option<bool>,
}

/// Request body sent to the Vertex `predict` endpoint for embeddings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingRequest {
    /// One or more instances to embed.
    pub instances: Vec<EmbeddingInstance>,
    /// Optional parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<EmbeddingParameters>,
}

impl EmbeddingRequest {
    /// Create a request to embed a single text string.
    #[must_use]
    pub fn new<S: Into<String>>(text: S) -> Self {
        Self {
            instances: vec![EmbeddingInstance {
                content: text.into(),
                task_type: None,
                title: None,
            }],
            parameters: None,
        }
    }

    /// Create a request to embed multiple text strings.
    #[must_use]
    pub fn batch(texts: Vec<String>) -> Self {
        Self {
            instances: texts
                .into_iter()
                .map(|content| EmbeddingInstance { content, task_type: None, title: None })
                .collect(),
            parameters: None,
        }
    }

    /// Set the task type for all instances.
    #[must_use]
    pub fn with_task_type(mut self, task_type: EmbeddingTaskType) -> Self {
        for instance in &mut self.instances {
            instance.task_type = Some(task_type);
        }
        self
    }

    /// Set the desired output dimensionality.
    #[must_use]
    pub fn with_output_dimensionality(mut self, dims: u32) -> Self {
        self.parameters.get_or_insert_with(EmbeddingParameters::default).output_dimensionality =
            Some(dims);
        self
    }

    /// Enable or disable automatic input truncation.
    #[must_use]
    pub fn with_auto_truncate(mut self, auto_truncate: bool) -> Self {
        self.parameters.get_or_insert_with(EmbeddingParameters::default).auto_truncate =
            Some(auto_truncate);
        self
    }
}

/// Statistics about a single embedding returned by the API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingStatistics {
    /// Whether the input was truncated before embedding.
    #[serde(default)]
    pub truncated: bool,
    /// Number of tokens consumed by the input.
    #[serde(rename = "token_count", default)]
    pub token_count: u32,
}

/// A single embedding vector with associated metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingValues {
    /// The embedding vector.
    pub values: Vec<f32>,
    /// Statistics about the embedding.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub statistics: Option<EmbeddingStatistics>,
}

/// One prediction entry returned by the predict endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingPrediction {
    /// The computed embeddings.
    pub embeddings: EmbeddingValues,
}

/// Response from the Vertex embeddings predict endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingResponse {
    /// One prediction per input instance.
    pub predictions: Vec<EmbeddingPrediction>,
}

impl EmbeddingResponse {
    /// Return the embedding vector for the first (or only) instance.
    #[must_use]
    pub fn embedding(&self) -> Option<&[f32]> {
        self.predictions.first().map(|p| p.embeddings.values.as_slice())
    }

    /// Collect all embedding vectors.
    #[must_use]
    pub fn embeddings(&self) -> Vec<&[f32]> {
        self.predictions.iter().map(|p| p.embeddings.values.as_slice()).collect()
    }
}

/// Thin wrapper exposing embeddings operations.
pub struct EmbeddingsApi<'a> {
    client: &'a VertexClient,
}

impl<'a> EmbeddingsApi<'a> {
    /// Create a new embeddings API handle.
    #[must_use]
    pub const fn new(client: &'a VertexClient) -> Self {
        Self { client }
    }

    /// Embed content using the specified model (e.g. `gemini-embedding-001`).
    ///
    /// # Errors
    ///
    /// Returns an error when the request fails or the response cannot be
    /// parsed.
    pub async fn embed(
        &self,
        model: &str,
        request: &EmbeddingRequest,
    ) -> Result<EmbeddingResponse> {
        self.client.embed(model, request).await
    }
}

impl VertexClient {
    /// Get the embeddings API handle.
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn embeddings(&self) -> EmbeddingsApi<'_> {
        EmbeddingsApi::new(self)
    }

    /// Embed content using the specified model.
    ///
    /// Sends a predict request to the Vertex AI embeddings endpoint and
    /// returns the resulting embedding vectors.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use threatflux_vertex_rust_sdk::{
    ///     api::embeddings::EmbeddingRequest, config::Config, VertexClient,
    /// };
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = Config {
    ///     project_id: "project-id".into(),
    ///     region: "us-central1".into(),
    ///     ..Config::default()
    /// };
    /// let client = VertexClient::new(config).await?;
    /// let request = EmbeddingRequest::new("Hello, world!")
    ///     .with_output_dimensionality(256);
    /// let response = client.embed("gemini-embedding-001", &request).await?;
    ///
    /// if let Some(vector) = response.embedding() {
    ///     println!("Embedding dimensions: {}", vector.len());
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error when the HTTP request fails or the response body
    /// cannot be parsed.
    pub async fn embed(
        &self,
        model: &str,
        request: &EmbeddingRequest,
    ) -> Result<EmbeddingResponse> {
        let descriptor = ModelDescriptor::parse(model)?;
        let context = self.model_request_context(&descriptor);
        let path = format!("/v1/{}:predict", context.resource_path);
        let url = self.build_url_for_endpoint(&context.endpoint, &path);

        let response = self.make_authenticated_request(&url, request).await?;
        let response_text = response.text().await?;

        serde_json::from_str::<EmbeddingResponse>(&response_text).map_or_else(
            |_| {
                serde_json::from_str::<crate::error::ApiError>(&response_text).map_or_else(
                    |_| {
                        Err(VertexError::generic(format!(
                            "Failed to parse embedding response: {response_text}"
                        )))
                    },
                    |api_error| Err(VertexError::from(api_error)),
                )
            },
            Ok,
        )
    }

    /// Embed a single text string (convenience wrapper).
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying embed call fails.
    pub async fn embed_text(&self, model: &str, text: &str) -> Result<Vec<f32>> {
        let request = EmbeddingRequest::new(text);
        let response = self.embed(model, &request).await?;
        response
            .embedding()
            .map(<[f32]>::to_vec)
            .ok_or_else(|| VertexError::generic("Empty embedding response".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedding_request_single() {
        let req = EmbeddingRequest::new("hello");
        assert_eq!(req.instances.len(), 1);
        assert_eq!(req.instances[0].content, "hello");
        assert!(req.parameters.is_none());
    }

    #[test]
    fn embedding_request_batch() {
        let req = EmbeddingRequest::batch(vec!["a".into(), "b".into()]);
        assert_eq!(req.instances.len(), 2);
    }

    #[test]
    fn embedding_request_with_options() {
        let req = EmbeddingRequest::new("hello")
            .with_task_type(EmbeddingTaskType::SemanticSimilarity)
            .with_output_dimensionality(256)
            .with_auto_truncate(true);

        assert_eq!(req.instances[0].task_type, Some(EmbeddingTaskType::SemanticSimilarity));
        assert_eq!(req.parameters.as_ref().unwrap().output_dimensionality, Some(256));
        assert_eq!(req.parameters.as_ref().unwrap().auto_truncate, Some(true));
    }

    #[test]
    fn embedding_response_accessors() {
        let response = EmbeddingResponse {
            predictions: vec![
                EmbeddingPrediction {
                    embeddings: EmbeddingValues {
                        values: vec![0.1, 0.2, 0.3],
                        statistics: Some(EmbeddingStatistics { truncated: false, token_count: 3 }),
                    },
                },
                EmbeddingPrediction {
                    embeddings: EmbeddingValues { values: vec![0.4, 0.5, 0.6], statistics: None },
                },
            ],
        };

        assert_eq!(response.embedding().unwrap(), &[0.1, 0.2, 0.3]);
        assert_eq!(response.embeddings().len(), 2);
    }

    #[test]
    fn embedding_request_serialization() {
        let req = EmbeddingRequest::new("test").with_output_dimensionality(128);
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["instances"][0]["content"], "test");
        assert_eq!(json["parameters"]["outputDimensionality"], 128);
    }

    #[test]
    fn embedding_response_deserialization() {
        let json = serde_json::json!({
            "predictions": [{
                "embeddings": {
                    "values": [0.1, 0.2],
                    "statistics": { "truncated": false, "token_count": 5 }
                }
            }]
        });
        let response: EmbeddingResponse = serde_json::from_value(json).unwrap();
        assert_eq!(response.predictions.len(), 1);
        assert_eq!(response.predictions[0].embeddings.values, vec![0.1, 0.2]);
        assert_eq!(response.predictions[0].embeddings.statistics.as_ref().unwrap().token_count, 5);
    }
}
