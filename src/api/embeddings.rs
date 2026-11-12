//! Vertex AI text and multimodal embeddings APIs.
//!
//! Uses `predict` for text embedding models such as `gemini-embedding-001`
//! and `embedContent` for Gemini Embedding 2.

use crate::client::{effective_model_name, VertexClient};
use crate::error::{Result, VertexError};
use crate::model_descriptor::ModelDescriptor;
use crate::types::Content;
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

/// Request for one multimodal embedding from Gemini Embedding 2.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmbedContentRequest {
    /// Text, images, documents, audio, or video to embed together.
    pub content: Content,
    /// Optional configuration for this embedding.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embed_content_config: Option<EmbedContentConfig>,
}

/// Options for Gemini Embedding 2. Put task instructions and titles in the input text.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmbedContentConfig {
    /// Desired output dimensionality, up to 3,072.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_dimensionality: Option<u32>,
    /// Whether to truncate input that exceeds the model's token limit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_truncate: Option<bool>,
    /// Enable OCR for document input.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_ocr: Option<bool>,
    /// Extract audio tracks from video input.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_track_extraction: Option<bool>,
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

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct EmbedContentResponse {
    embedding: EmbeddingValues,
    #[serde(default)]
    truncated: bool,
    usage_metadata: Option<crate::types::UsageMetadata>,
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

    /// Embed one multimodal input using Gemini Embedding 2.
    ///
    /// # Errors
    ///
    /// Returns an error when the request fails or the response cannot be parsed.
    pub async fn embed_content(
        &self,
        model: &str,
        request: &EmbedContentRequest,
    ) -> Result<EmbeddingValues> {
        self.client.embed_content(model, request).await
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
    /// Returns one embedding per text instance. For Gemini Embedding 2, the
    /// SDK sends a separate `embedContent` request per instance.
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
        if descriptor.publisher() == "google"
            && matches!(
                effective_model_name(&descriptor).as_str(),
                "gemini-embedding-2" | "gemini-embedding-2-preview"
            )
        {
            let mut predictions = Vec::with_capacity(request.instances.len());
            for instance in &request.instances {
                let embed_request = EmbedContentRequest {
                    content: Content::user_text(embedding_2_text(instance)),
                    embed_content_config: request.parameters.as_ref().map(|params| {
                        EmbedContentConfig {
                            output_dimensionality: params.output_dimensionality,
                            auto_truncate: params.auto_truncate,
                            ..Default::default()
                        }
                    }),
                };
                let embeddings = self.embed_content(model, &embed_request).await?;
                predictions.push(EmbeddingPrediction { embeddings });
            }
            return Ok(EmbeddingResponse { predictions });
        }

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

    /// Embed one multimodal input using Gemini Embedding 2.
    ///
    /// # Errors
    ///
    /// Returns an error when the request fails or the response cannot be parsed.
    pub async fn embed_content(
        &self,
        model: &str,
        request: &EmbedContentRequest,
    ) -> Result<EmbeddingValues> {
        let descriptor = ModelDescriptor::parse(model)?;
        let context = self.model_request_context(&descriptor);
        let path = format!("/v1/{}:embedContent", context.resource_path);
        let url = self.build_url_for_endpoint(&context.endpoint, &path);
        let response = self.make_authenticated_request(&url, request).await?;
        let status = response.status();
        if !status.is_success() {
            return Err(VertexError::Api {
                code: status.to_string(),
                message: response.text().await?,
            });
        }

        let mut response: EmbedContentResponse = response.json().await?;
        if let Some(usage) = response.usage_metadata {
            response.embedding.statistics = Some(EmbeddingStatistics {
                truncated: response.truncated,
                token_count: u32::try_from(usage.prompt_token_count).unwrap_or_default(),
            });
        } else if response.truncated {
            response
                .embedding
                .statistics
                .get_or_insert(EmbeddingStatistics { truncated: true, token_count: 0 })
                .truncated = true;
        }
        Ok(response.embedding)
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

fn embedding_2_text(instance: &EmbeddingInstance) -> String {
    let content = &instance.content;
    let task = match instance.task_type {
        Some(EmbeddingTaskType::RetrievalQuery) => "search result",
        Some(EmbeddingTaskType::SemanticSimilarity) => "sentence similarity",
        Some(EmbeddingTaskType::Classification) => "classification",
        Some(EmbeddingTaskType::Clustering) => "clustering",
        Some(EmbeddingTaskType::QuestionAnswering) => "question answering",
        Some(EmbeddingTaskType::FactVerification) => "fact checking",
        Some(EmbeddingTaskType::RetrievalDocument) => {
            let title = instance.title.as_deref().unwrap_or("none");
            return format!("title: {title} | text: {content}");
        }
        None => return content.clone(),
    };
    format!("task: {task} | query: {content}")
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
