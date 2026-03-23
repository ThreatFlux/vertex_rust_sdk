//! Token counting API

use crate::client::VertexClient;
use crate::error::{Result, VertexError};
use crate::model_descriptor::ModelDescriptor;
use crate::models::{CountTokensRequest, CountTokensResponse};

impl VertexClient {
    /// Count tokens in content
    ///
    /// This method sends a request to the Vertex AI API to count the number of tokens
    /// in the provided content. This is useful for:
    /// - Estimating API costs before making requests
    /// - Ensuring content fits within model context windows
    /// - Understanding tokenization patterns for different content types
    ///
    /// # Arguments
    ///
    /// * `model` - The model ID to use for tokenization (e.g., "gemini-2.0-flash-001")
    /// * `request` - The token counting request containing content to analyze
    ///
    /// # Example
    ///
    /// ```no_run
    /// use threatflux_vertex_rust_sdk::{config::Config, CountTokensRequest, VertexClient};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = Config {
    ///     project_id: "project-id".into(),
    ///     region: "us-central1".into(),
    ///     ..Config::default()
    /// };
    /// let client = VertexClient::new(config).await?;
    /// let request = CountTokensRequest::new("How many tokens are in this text?");
    /// let response = client.count_tokens("gemini-2.0-flash-001", &request).await?;
    ///
    /// println!("Token count: {}", response.total_tokens);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error when the HTTP request fails or when neither a successful
    /// token count nor an `ApiError` can be parsed from the response.
    pub async fn count_tokens_impl(
        &self,
        model: &str,
        request: &CountTokensRequest,
    ) -> Result<CountTokensResponse> {
        let url = self.build_count_tokens_url(model)?;
        let response = self.make_authenticated_request(&url, request).await?;
        let response_text = response.text().await?;

        serde_json::from_str::<CountTokensResponse>(&response_text).map_or_else(
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

    /// Count tokens in a simple text string (convenience method)
    ///
    /// # Arguments
    ///
    /// * `model` - The model ID to use for tokenization
    /// * `text` - The text to count tokens for
    ///
    /// # Example
    ///
    /// ```no_run
    /// use threatflux_vertex_rust_sdk::{config::Config, VertexClient};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = Config {
    ///     project_id: "project-id".into(),
    ///     region: "us-central1".into(),
    ///     ..Config::default()
    /// };
    /// let client = VertexClient::new(config).await?;
    /// let token_count = client.count_text_tokens("gemini-2.0-flash-001", "Hello, world!").await?;
    ///
    /// println!("Token count: {}", token_count);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying token counting call fails.
    pub async fn count_text_tokens(&self, model: &str, text: &str) -> Result<i32> {
        let request = CountTokensRequest::new(text);
        let response = self.count_tokens_impl(model, &request).await?;
        Ok(response.total_tokens)
    }

    /// Build the URL for token counting
    ///
    /// # Errors
    ///
    /// Returns an error if the model identifier cannot be parsed into the
    /// required endpoint metadata.
    fn build_count_tokens_url(&self, model: &str) -> Result<String> {
        let descriptor = ModelDescriptor::parse(model)?;
        let context = self.model_request_context(&descriptor);
        let path = format!("/v1/{}:countTokens", context.resource_path);
        Ok(self.build_url_for_endpoint(&context.endpoint, &path))
    }
}

/// Token counting utilities
pub mod utils {
    /// Estimate token count for text (rough approximation)
    ///
    /// This provides a rough estimate without making an API call.
    /// Actual token counts may vary based on the specific model's tokenizer.
    ///
    /// Rule of thumb:
    /// - English: ~4 characters per token
    /// - Code: ~3 characters per token (more symbols/punctuation)
    /// - Other languages may vary significantly
    #[must_use]
    pub fn estimate_tokens(text: &str) -> i32 {
        // Very rough estimate: 4 characters per token on average
        let chars = text.chars().count();
        let tokens = chars.saturating_add(3) / 4;
        i32::try_from(tokens).unwrap_or(i32::MAX)
    }

    /// Check if text is likely to fit within a token limit
    #[must_use]
    pub fn likely_within_limit(text: &str, limit: i32) -> bool {
        estimate_tokens(text) <= limit
    }

    /// Common model token limits
    pub mod limits {
        /// Gemini 1.0 Pro input limit
        pub const GEMINI_1_0_PRO_INPUT: i32 = 32_768;

        /// Gemini 1.5 Pro input limit
        pub const GEMINI_1_5_PRO_INPUT: i32 = 1_048_576;

        /// Gemini 2.0 Flash input limit
        pub const GEMINI_2_0_FLASH_INPUT: i32 = 1_048_576;

        /// Common output limits
        pub const COMMON_OUTPUT_LIMIT: i32 = 8_192;
    }
}

#[cfg(test)]
mod tests {
    use super::utils::*;

    // URL building tests require authentication and are tested via integration tests

    #[test]
    fn test_token_estimation() {
        // Test basic estimation
        let text = "Hello, world!";
        let estimated = estimate_tokens(text);
        assert!(estimated > 0);
        assert!(estimated <= 10); // Should be reasonable

        // Test limit checking
        let short_text = "Hi";
        assert!(likely_within_limit(short_text, 100));

        let long_text = "a".repeat(10000);
        assert!(!likely_within_limit(&long_text, 100));
    }

    #[test]
    fn test_token_limits() {
        assert_eq!(limits::GEMINI_1_0_PRO_INPUT, 32_768);
        assert_eq!(limits::GEMINI_1_5_PRO_INPUT, 1_048_576);
        assert_eq!(limits::GEMINI_2_0_FLASH_INPUT, 1_048_576);
    }
}
