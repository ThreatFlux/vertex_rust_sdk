//! Streaming content generation API

use super::generate::{generation_api_version, prepare_generation_request, SegmentRequest};
use crate::client::VertexClient;
use crate::error::{Result, VertexError};
use crate::model_descriptor::ModelDescriptor;
use crate::models::{GenerateContentRequest, GenerateContentSegment, StreamingResponse};
use crate::streaming::{SegmentSseParser, SseParser};
use crate::streaming_support::{SsePayloadParser, SseStreamState};
use futures_util::stream::{self, Stream, TryStreamExt};
use reqwest::header;
use serde::Serialize;
use std::pin::Pin;

impl VertexClient {
    /// Generate content using a model (streaming)
    ///
    /// This method sends a request to the Vertex AI API to generate content
    /// with streaming responses. Each chunk of the response is returned as
    /// it becomes available, allowing for real-time display of generated content.
    ///
    /// # Arguments
    ///
    /// * `model` - The model ID to use (e.g., "gemini-2.0-flash-001")
    /// * `request` - The generation request containing content and configuration
    ///
    /// # Returns
    ///
    /// A stream of `StreamingResponse` objects. The final response will contain
    /// usage metadata.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use threatflux_vertex_rust_sdk::{config::Config, GenerateContentRequest, VertexClient};
    /// use tokio_stream::StreamExt;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = Config {
    ///     project_id: "project-id".into(),
    ///     region: "us-central1".into(),
    ///     ..Config::default()
    /// };
    /// let client = VertexClient::new(config).await?;
    /// let request = GenerateContentRequest::new("Write a story");
    /// let mut stream = client.stream_generate_content("gemini-2.0-flash-001", &request).await?;
    ///
    /// while let Some(chunk) = stream.next().await {
    ///     match chunk {
    ///         Ok(response) => {
    ///             if let Some(text) = response.text() {
    ///                 print!("{}", text);
    ///             }
    ///         }
    ///         Err(e) => eprintln!("Error: {}", e),
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error when authentication fails, the HTTP request cannot be
    /// completed, a non-success status is returned, or the SSE stream cannot
    /// be parsed.
    pub async fn stream_generate_content_impl(
        &self,
        model: &str,
        request: &GenerateContentRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamingResponse>> + Send>>> {
        let descriptor = ModelDescriptor::parse(model)?;
        let request = prepare_generation_request(&descriptor, request)?;
        let version = generation_api_version(&descriptor);
        self.stream_generation(&descriptor, version, request.as_ref(), SseParser::new()).await
    }

    /// Stream one segment of a Gemini 4 long decode using the v1beta1 API.
    ///
    /// Set `max_output_tokens` to the desired total across all segments. Use
    /// `None` for the first segment. Pass the token from a continuation chunk
    /// with the original request to get the next segment.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails or a streamed chunk cannot be decoded.
    pub async fn stream_generate_content_segment(
        &self,
        model: &str,
        request: &GenerateContentRequest,
        continuation_token: Option<&str>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<GenerateContentSegment<StreamingResponse>>> + Send>>>
    {
        let descriptor = ModelDescriptor::parse(model)?;
        let request = prepare_generation_request(&descriptor, request)?;
        let segment = SegmentRequest { request: request.as_ref(), continuation_token };
        self.stream_generation(&descriptor, "v1beta1", &segment, SegmentSseParser).await
    }

    async fn stream_generation<P, R, T>(
        &self,
        descriptor: &ModelDescriptor,
        version: &str,
        request: &R,
        parser: P,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<T>> + Send>>>
    where
        P: SsePayloadParser<T> + Send + 'static,
        R: Serialize + Sync,
        T: Send + 'static,
    {
        let context = self.model_request_context(descriptor);
        let path = format!("/{version}/{}:streamGenerateContent?alt=sse", context.resource_path);
        let url = self.build_url_for_endpoint(&context.endpoint, &path);

        let response = self
            .send_with_retry(|| {
                let url = url.clone();
                async move {
                    let token = self.get_auth_token().await?;
                    let response = self
                        .http_client()
                        .post(&url)
                        .header(header::AUTHORIZATION, format!("Bearer {token}"))
                        .header(header::CONTENT_TYPE, "application/json")
                        .header(header::ACCEPT, "text/event-stream")
                        .json(request)
                        .send()
                        .await
                        .map_err(VertexError::Request)?;

                    Ok(response)
                }
            })
            .await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let error_text = response.text().await?;
            return Err(VertexError::http(status, error_text));
        }

        let byte_stream = response.bytes_stream();
        let state = SseStreamState::new(Box::pin(byte_stream), parser);

        let stream = stream::try_unfold(state, |mut state| async move {
            loop {
                if let Some(event) = state.try_parsed_event()? {
                    return Ok(Some((event, state)));
                }

                if !state.advance().await? {
                    if let Some(event) = state.try_parsed_event()? {
                        return Ok(Some((event, state)));
                    }
                    return Ok(None);
                }
            }
        })
        .into_stream();

        Ok(Box::pin(stream))
    }
}

#[cfg(test)]
mod tests {
    // Note: URL building tests require authentication setup
    // These tests are commented out as they need AccessTokenProvider which is not publicly exported
    // The URL building logic is tested via integration tests instead
}
