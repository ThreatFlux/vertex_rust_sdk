//! Streaming content generation API

use crate::client::VertexClient;
use crate::error::{Result, VertexError};
use crate::model_descriptor::ModelDescriptor;
use crate::models::{GenerateContentRequest, StreamingResponse};
use crate::streaming::SseParser;
use crate::streaming_support::SseStreamState;
use futures_util::stream::{self, Stream, TryStreamExt};
use reqwest::header;
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
        let url = self.build_stream_url(model)?;

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
        let state = SseStreamState::new(Box::pin(byte_stream), SseParser::new());

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

    /// Build the URL for streaming content generation
    ///
    /// # Errors
    ///
    /// Returns an error when the model identifier cannot be parsed into a valid
    /// endpoint and resource path.
    fn build_stream_url(&self, model: &str) -> Result<String> {
        let descriptor = ModelDescriptor::parse(model)?;
        let context = self.model_request_context(&descriptor);
        let path = format!("/v1/{}:streamGenerateContent?alt=sse", context.resource_path);
        Ok(self.build_url_for_endpoint(&context.endpoint, &path))
    }
}

#[cfg(test)]
mod tests {
    // Note: URL building tests require authentication setup
    // These tests are commented out as they need AccessTokenProvider which is not publicly exported
    // The URL building logic is tested via integration tests instead
}
