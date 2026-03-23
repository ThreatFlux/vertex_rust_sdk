//! Models API - List and get details about available models

use crate::client::VertexClient;
use crate::error::{ApiError, Result, VertexError};
use crate::model_descriptor::ModelDescriptor;
use serde::{Deserialize, Serialize};

/// Model information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Model {
    /// Model name (e.g., "publishers/google/models/gemini-2.5-flash")
    pub name: String,

    /// Display name for the model
    #[serde(rename = "displayName")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,

    /// Description of the model
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Version of the model
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    /// Supported generation methods (generateContent, streamGenerateContent, etc.)
    #[serde(rename = "supportedGenerationMethods")]
    #[serde(default)]
    pub supported_generation_methods: Vec<String>,

    /// Input token limit
    #[serde(rename = "inputTokenLimit")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_token_limit: Option<i64>,

    /// Output token limit
    #[serde(rename = "outputTokenLimit")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_token_limit: Option<i64>,

    /// Supported languages
    #[serde(rename = "supportedLanguages")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supported_languages: Option<Vec<String>>,

    /// Temperature range
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<TemperatureRange>,

    /// Top-P range
    #[serde(rename = "topP")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<TopPRange>,

    /// Top-K range
    #[serde(rename = "topK")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<i32>,
}

/// Temperature range for a model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemperatureRange {
    pub min: f32,
    pub max: f32,
}

/// Top-P range for a model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopPRange {
    pub min: f32,
    pub max: f32,
}

/// Response from list models API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListModelsResponse {
    #[serde(rename = "publisherModels", alias = "models", default)]
    pub models: Vec<Model>,

    #[serde(rename = "nextPageToken", alias = "next_page_token")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_page_token: Option<String>,
}

/// Location information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    /// Location name (e.g., "projects/PROJECT_ID/locations/us-central1")
    pub name: String,

    /// Location ID (e.g., "us-central1")
    #[serde(rename = "locationId")]
    pub location_id: String,

    /// Display name for the location
    #[serde(rename = "displayName")]
    pub display_name: String,

    /// Labels for the location
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<std::collections::HashMap<String, String>>,

    /// Metadata for the location
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// Response from list locations API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListLocationsResponse {
    pub locations: Vec<Location>,

    #[serde(rename = "nextPageToken")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_page_token: Option<String>,
}

/// Models API implementation
pub struct ModelsApi<'a> {
    client: &'a VertexClient,
}

impl<'a> ModelsApi<'a> {
    /// Create a new models API instance
    #[must_use]
    pub const fn new(client: &'a VertexClient) -> Self {
        Self { client }
    }

    /// List available models
    ///
    /// # Errors
    ///
    /// Returns an error if the publisher request fails or the response payload
    /// cannot be parsed.
    pub async fn list_models(
        &self,
        page_size: Option<i32>,
        page_token: Option<String>,
    ) -> Result<ListModelsResponse> {
        self.list_models_for_publisher("google", page_size, page_token).await
    }

    /// List models for a specific publisher
    ///
    /// # Errors
    ///
    /// Returns an error when the HTTP call fails or the response body cannot be
    /// parsed into `ListModelsResponse`.
    pub async fn list_models_for_publisher(
        &self,
        publisher: &str,
        page_size: Option<i32>,
        page_token: Option<String>,
    ) -> Result<ListModelsResponse> {
        let mut url = self.client.build_url(&format!("/v1beta1/publishers/{publisher}/models"));

        // Add query parameters
        let mut params = Vec::new();
        if let Some(size) = page_size {
            params.push(format!("pageSize={size}"));
        }
        if let Some(token) = page_token {
            params.push(format!("pageToken={token}"));
        }

        if !params.is_empty() {
            url.push('?');
            url.push_str(&params.join("&"));
        }

        let response = self.client.make_authenticated_get_request(&url).await?;

        if response.status().is_success() {
            let text = response.text().await.map_err(VertexError::Request)?;

            // Parse the response
            let result: ListModelsResponse =
                serde_json::from_str(&text).map_err(|e| VertexError::Api {
                    message: format!("Failed to parse models response: {e}"),
                    code: "PARSE_ERROR".to_string(),
                })?;
            Ok(result)
        } else {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            Err(VertexError::Api { message: error_text, code: "UNKNOWN".to_string() })
        }
    }

    /// Get details for a specific model
    ///
    /// # Errors
    ///
    /// Returns an error when the descriptor cannot be parsed or every HTTP
    /// attempt fails to return a valid model.
    pub async fn get_model(&self, model_name: &str) -> Result<Model> {
        let descriptor = ModelDescriptor::parse(model_name)?;
        let context = self.client.model_request_context(&descriptor);

        match self.fetch_model(&context.endpoint, &context.resource_path).await {
            Ok(model) => Ok(model),
            Err(primary_err) => {
                let provided_resource = model_name.trim_start().starts_with("projects/");
                if should_retry_with_publisher(&primary_err) && !provided_resource {
                    match self.fetch_model(&context.endpoint, &context.relative_path).await {
                        Ok(model) => Ok(model),
                        Err(fallback_err) => Err(fallback_err),
                    }
                } else {
                    Err(primary_err)
                }
            }
        }
    }

    /// List available locations/regions
    ///
    /// # Errors
    ///
    /// Returns an error when the HTTP request fails or the response cannot be
    /// deserialized.
    pub async fn list_locations(
        &self,
        page_size: Option<i32>,
        page_token: Option<String>,
    ) -> Result<ListLocationsResponse> {
        let project_id = self.client.project_id();
        let mut url = self.client.build_url(&format!("/v1/projects/{project_id}/locations"));

        // Add query parameters
        let mut params = Vec::new();
        if let Some(size) = page_size {
            params.push(format!("pageSize={size}"));
        }
        if let Some(token) = page_token {
            params.push(format!("pageToken={token}"));
        }

        if !params.is_empty() {
            url.push('?');
            url.push_str(&params.join("&"));
        }

        let response = self.client.make_authenticated_get_request(&url).await?;

        if response.status().is_success() {
            let result =
                response.json::<ListLocationsResponse>().await.map_err(VertexError::Request)?;
            Ok(result)
        } else {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            Err(VertexError::Api { message: error_text, code: "UNKNOWN".to_string() })
        }
    }

    /// Get models that support specific features
    ///
    /// # Errors
    ///
    /// Returns an error if fetching the base model list fails.
    pub async fn get_models_with_features(&self, features: &[&str]) -> Result<Vec<Model>> {
        let response = self.list_models(None, None).await?;

        let filtered_models = response
            .models
            .into_iter()
            .filter(|model| {
                features.iter().all(|feature| {
                    model.supported_generation_methods.iter().any(|method| method == *feature)
                })
            })
            .collect();

        Ok(filtered_models)
    }

    /// Get the latest Gemini models (hardcoded list for now)
    ///
    /// # Errors
    ///
    /// This function currently never returns an error and always yields the
    /// built-in list of Gemini models.
    #[allow(clippy::unused_async)]
    pub async fn get_gemini_models(&self) -> Result<Vec<Model>> {
        // For now, return a hardcoded list of known Gemini models
        // since the list API doesn't return them
        let gemini_models = vec![
            Model {
                name: "publishers/google/models/gemini-2.5-flash".to_string(),
                display_name: Some("Gemini 2.5 Flash".to_string()),
                description: Some(
                    "Best model for price and performance with thinking capabilities".to_string(),
                ),
                version: Some("gemini-2.5-flash".to_string()),
                supported_generation_methods: vec![
                    "generateContent".to_string(),
                    "streamGenerateContent".to_string(),
                    "countTokens".to_string(),
                ],
                input_token_limit: Some(1_048_576),
                output_token_limit: Some(65_535),
                supported_languages: None,
                temperature: None,
                top_p: None,
                top_k: None,
            },
            Model {
                name: "publishers/google/models/gemini-2.5-pro".to_string(),
                display_name: Some("Gemini 2.5 Pro".to_string()),
                description: Some("Most advanced reasoning model for complex problems".to_string()),
                version: Some("gemini-2.5-pro".to_string()),
                supported_generation_methods: vec![
                    "generateContent".to_string(),
                    "streamGenerateContent".to_string(),
                    "countTokens".to_string(),
                ],
                input_token_limit: Some(1_048_576),
                output_token_limit: Some(65_535),
                supported_languages: None,
                temperature: None,
                top_p: None,
                top_k: None,
            },
            Model {
                name: "publishers/google/models/gemini-2.5-flash-lite".to_string(),
                display_name: Some("Gemini 2.5 Flash-Lite".to_string()),
                description: Some("Balanced model optimized for low latency".to_string()),
                version: Some("gemini-2.5-flash-lite".to_string()),
                supported_generation_methods: vec![
                    "generateContent".to_string(),
                    "streamGenerateContent".to_string(),
                    "countTokens".to_string(),
                ],
                input_token_limit: Some(1_048_576),
                output_token_limit: Some(65_535),
                supported_languages: None,
                temperature: None,
                top_p: None,
                top_k: None,
            },
            Model {
                name: "publishers/google/models/gemini-1.5-flash-001".to_string(),
                display_name: Some("Gemini 1.5 Flash".to_string()),
                description: Some("Fast multimodal model".to_string()),
                version: Some("001".to_string()),
                supported_generation_methods: vec![
                    "generateContent".to_string(),
                    "streamGenerateContent".to_string(),
                    "countTokens".to_string(),
                ],
                input_token_limit: Some(1_048_576),
                output_token_limit: Some(65_535),
                supported_languages: None,
                temperature: None,
                top_p: None,
                top_k: None,
            },
        ];

        Ok(gemini_models)
    }
}

impl ModelsApi<'_> {
    async fn fetch_model(&self, endpoint: &str, resource_path: &str) -> Result<Model> {
        let url =
            self.client.build_url_for_endpoint(endpoint, &format!("/v1beta1/{resource_path}"));
        let response = self.client.make_authenticated_get_request(&url).await?;

        if response.status().is_success() {
            let result = response.json::<Model>().await.map_err(VertexError::Request)?;
            Ok(result)
        } else {
            let status = response.status();
            let body = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());

            serde_json::from_str::<ApiError>(&body).map_or_else(
                |_| {
                    if status.is_client_error() || status.is_server_error() {
                        Err(VertexError::Http { status: status.as_u16(), message: body })
                    } else {
                        Err(VertexError::Api { code: "UNKNOWN".to_string(), message: body })
                    }
                },
                |api_error| Err(VertexError::from(api_error)),
            )
        }
    }
}

fn should_retry_with_publisher(error: &VertexError) -> bool {
    match error {
        VertexError::Api { code, .. } => code.eq_ignore_ascii_case("NOT_FOUND"),
        VertexError::Http { status, .. } => *status == 404 || *status == 501,
        _ => false,
    }
}

impl Model {
    /// Get the short name of the model (without the full path)
    #[must_use]
    pub fn short_name(&self) -> &str {
        if let Some(last_part) = self.name.split('/').next_back() {
            last_part
        } else {
            &self.name
        }
    }

    /// Check if the model supports a specific generation method
    #[must_use]
    pub fn supports_method(&self, method: &str) -> bool {
        self.supported_generation_methods.iter().any(|value| value == method)
    }

    /// Check if this is a Gemini model
    #[must_use]
    pub fn is_gemini(&self) -> bool {
        self.name.contains("gemini")
    }

    /// Get the model family (e.g., "gemini-2.5-flash", "gemini-2.5-pro")
    #[must_use]
    pub fn family(&self) -> Option<String> {
        if let Some(short_name) = self.name.split('/').next_back() {
            // Remove version suffix if present (e.g., "-001")
            if let Some(last_dash) = short_name.rfind('-') {
                let potential_version = &short_name[last_dash + 1..];
                if potential_version.chars().all(|c| c.is_ascii_digit()) {
                    return Some(short_name[..last_dash].to_string());
                }
            }
            Some(short_name.to_string())
        } else {
            None
        }
    }
}

impl Location {
    /// Check if this location supports Vertex AI
    #[must_use]
    pub const fn supports_vertex_ai(&self) -> bool {
        // Most locations support Vertex AI, but we can add specific checks here
        true
    }

    /// Get the region code (e.g., "us-central1")
    #[must_use]
    pub fn region(&self) -> &str {
        &self.location_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{auth::AuthProvider, client::VertexClient, config::Config};
    use async_trait::async_trait;
    use mockito::{Matcher, Server};

    #[test]
    fn list_models_response_accepts_models_alias() {
        let payload = r#"{
            "models": [{
                "name": "publishers/anthropic/models/claude-haiku-4-5@20251001",
                "displayName": "Claude 4.5 Haiku",
                "supportedGenerationMethods": ["generateContent"]
            }],
            "next_page_token": "abc123"
        }"#;

        let response: ListModelsResponse =
            serde_json::from_str(payload).expect("models response parsed");
        assert_eq!(response.models.len(), 1);
        assert_eq!(
            response.models[0].name,
            "publishers/anthropic/models/claude-haiku-4-5@20251001"
        );
        assert_eq!(response.next_page_token.as_deref(), Some("abc123"));
    }

    #[test]
    fn test_model_short_name() {
        let model = Model {
            name: "publishers/google/models/gemini-2.5-flash".to_string(),
            display_name: Some("Gemini 2.5 Flash".to_string()),
            description: Some("Test model".to_string()),
            version: None,
            supported_generation_methods: vec!["generateContent".to_string()],
            input_token_limit: None,
            output_token_limit: None,
            supported_languages: None,
            temperature: None,
            top_p: None,
            top_k: None,
        };

        assert_eq!(model.short_name(), "gemini-2.5-flash");
    }

    #[test]
    fn test_model_family() {
        let model = Model {
            name: "publishers/google/models/gemini-2.5-flash".to_string(),
            display_name: Some("Gemini 2.0 Flash".to_string()),
            description: Some("Test model".to_string()),
            version: None,
            supported_generation_methods: vec!["generateContent".to_string()],
            input_token_limit: None,
            output_token_limit: None,
            supported_languages: None,
            temperature: None,
            top_p: None,
            top_k: None,
        };

        assert_eq!(model.family(), Some("gemini-2.5-flash".to_string()));
    }

    #[test]
    fn test_model_supports_method() {
        let model = Model {
            name: "test-model".to_string(),
            display_name: Some("Test Model".to_string()),
            description: Some("Test model".to_string()),
            version: None,
            supported_generation_methods: vec![
                "generateContent".to_string(),
                "streamGenerateContent".to_string(),
            ],
            input_token_limit: None,
            output_token_limit: None,
            supported_languages: None,
            temperature: None,
            top_p: None,
            top_k: None,
        };

        assert!(model.supports_method("generateContent"));
        assert!(model.supports_method("streamGenerateContent"));
        assert!(!model.supports_method("countTokens"));
    }

    struct TestAuthProvider;

    #[async_trait]
    impl AuthProvider for TestAuthProvider {
        async fn get_token(&self) -> anyhow::Result<String> {
            Ok("test-token".to_string())
        }

        async fn refresh_if_needed(&self) -> anyhow::Result<()> {
            Ok(())
        }
    }

    async fn client_for_server(server: &Server) -> VertexClient {
        let mut config = Config {
            project_id: "test-project".to_string(),
            region: "us-central1".to_string(),
            ..Config::default()
        };
        config.base_url_override = Some(server.url().trim_end_matches('/').to_string());

        VertexClient::with_config_and_auth_provider(config, Box::new(TestAuthProvider))
            .await
            .expect("vertex client")
    }

    #[tokio::test]
    #[allow(clippy::significant_drop_tightening)]
    async fn get_model_prefers_project_scoped_resource() {
        let model = {
            let mut server = Server::new_async().await;
            let project_path = "/v1beta1/projects/test-project/locations/us-central1/publishers/google/models/gemini-2.5-flash";

            let _project_mock = server
                .mock("GET", project_path)
                .with_status(200)
                .with_header("content-type", "application/json")
                .with_body(
                    r#"{"name":"publishers/google/models/gemini-2.5-flash","displayName":"Gemini 2.5 Flash"}"#,
                )
                .create_async()
                .await;

            let client = client_for_server(&server).await;

            client
                .models()
                .get_model("publishers/google/models/gemini-2.5-flash")
                .await
                .expect("model fetched")
        };

        assert_eq!(model.name, "publishers/google/models/gemini-2.5-flash");
        assert_eq!(model.display_name.as_deref(), Some("Gemini 2.5 Flash"));
    }

    #[tokio::test]
    #[allow(clippy::significant_drop_tightening)]
    async fn get_model_falls_back_to_publisher_scope_on_not_found() {
        let model = {
            let mut server = Server::new_async().await;

            let _primary_mock = server
                .mock(
                    "GET",
                    Matcher::Regex(
                        r"/v1beta1/.*projects/test-project/locations/.*/publishers/anthropic/models/claude-sonnet-4-5.*"
                            .to_string(),
                    ),
                )
                .with_status(404)
                .with_header("content-type", "application/json")
                .with_body(
                    r#"{"error":{"code":404,"message":"not found","status":"NOT_FOUND"}}"#,
                )
                .create_async()
                .await;

            let _fallback_mock = server
                .mock(
                    "GET",
                    Matcher::Regex(
                        r"/v1beta1/.*publishers/anthropic/models/claude-sonnet-4-5.*".to_string(),
                    ),
                )
                .with_status(200)
                .with_header("content-type", "application/json")
                .with_body(
                    r#"{"name":"publishers/anthropic/models/claude-sonnet-4-5","displayName":"Claude 4.5 Sonnet"}"#,
                )
                .create_async()
                .await;

            let client = client_for_server(&server).await;

            client
                .models()
                .get_model("publishers/anthropic/models/claude-sonnet-4-5")
                .await
                .expect("model fetched")
        };

        assert_eq!(model.name, "publishers/anthropic/models/claude-sonnet-4-5");
        assert_eq!(model.display_name.as_deref(), Some("Claude 4.5 Sonnet"));
    }
}
