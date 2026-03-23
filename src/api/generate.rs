//! Non-streaming content generation API

use crate::client::VertexClient;
use crate::error::{Result, VertexError};
use crate::model_descriptor::ModelDescriptor;
use crate::models::{GenerateContentRequest, GenerateContentResponse};

/// Generate API implementation
pub struct GenerateApi<'a> {
    client: &'a VertexClient,
}

impl<'a> GenerateApi<'a> {
    /// Create a new generate API instance
    #[must_use]
    pub const fn new(client: &'a VertexClient) -> Self {
        Self { client }
    }

    /// Generate content using a model
    ///
    /// # Errors
    ///
    /// Propagates failures from the underlying client request.
    pub async fn generate_content(
        &self,
        model: &str,
        request: GenerateContentRequest,
    ) -> Result<GenerateContentResponse> {
        self.client.generate_content(model, &request).await
    }
}

impl VertexClient {
    /// Generate content using a model (non-streaming)
    ///
    /// This method sends a request to the Vertex AI API to generate content
    /// using the specified model. The response contains the generated content,
    /// usage metadata, and safety ratings.
    ///
    /// # Arguments
    ///
    /// * `model` - The model ID to use (e.g., "gemini-2.0-flash-001")
    /// * `request` - The generation request containing content and configuration
    ///
    /// # Example
    ///
    /// ```no_run
    /// use threatflux_vertex_rust_sdk::{config::Config, GenerateContentRequest, VertexClient};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = Config {
    ///     project_id: "project-id".into(),
    ///     region: "us-central1".into(),
    ///     ..Config::default()
    /// };
    /// let client = VertexClient::new(config).await?;
    /// let request = GenerateContentRequest::new("Explain quantum computing");
    /// let response = client.generate_content("gemini-2.0-flash-001", &request).await?;
    ///
    /// if let Some(text) = response.text() {
    ///     println!("Generated: {}", text);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the API request fails or if the response cannot be
    /// parsed into either a success payload or `ApiError`.
    pub async fn generate_content_impl(
        &self,
        model: &str,
        request: &GenerateContentRequest,
    ) -> Result<GenerateContentResponse> {
        let url = self.build_generate_url(model)?;
        let response = self.make_authenticated_request(&url, request).await?;
        let response_text = response.text().await?;

        // Try to parse as success response first
        serde_json::from_str::<GenerateContentResponse>(&response_text).map_or_else(
            |_| {
                serde_json::from_str::<crate::error::ApiError>(&response_text).map_or_else(
                    |_| {
                        Err(VertexError::generic(format!(
                            "Failed to parse response: {response_text}"
                        )))
                    },
                    |api_error| Err(VertexError::from(api_error)),
                )
            },
            Ok,
        )
    }

    /// Build the URL for content generation
    ///
    /// # Errors
    ///
    /// Returns an error if the provided model identifier cannot be parsed into
    /// a valid endpoint or project resource path.
    fn build_generate_url(&self, model: &str) -> Result<String> {
        let descriptor = ModelDescriptor::parse(model)?;
        let context = self.model_request_context(&descriptor);
        let path = format!("/v1/{}:generateContent", context.resource_path);
        Ok(self.build_url_for_endpoint(&context.endpoint, &path))
    }
}

#[cfg(test)]
mod tests {
    // URL building tests require authentication and are tested via integration tests
}
