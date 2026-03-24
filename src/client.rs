//! Vertex AI client implementation

use crate::auth::{from_env, AuthProvider};
use crate::config::{endpoint_for_region, Config, DEFAULT_ANTHROPIC_LOCATION};
use crate::error::{Result, VertexError};
use crate::model_descriptor::ModelDescriptor;
use crate::models::{
    CountTokensRequest, CountTokensResponse, GenerateContentRequest, GenerateContentResponse,
    StreamingResponse,
};
use futures_util::stream::Stream;
use reqwest::{header, Client as HttpClient, Response, StatusCode};
use std::collections::HashMap;
use std::pin::Pin;
use tokio::time::{sleep, Duration};

/// Main Vertex AI client
pub struct VertexClient {
    project_id: String,
    location: String,
    http_client: HttpClient,
    auth_provider: Box<dyn AuthProvider>,
    base_url: String,
    max_retries: u32,
    publisher_locations: HashMap<String, String>,
    custom_base_url: bool,
}

#[derive(Debug, Clone)]
pub struct ModelRequestContext {
    pub endpoint: String,
    pub resource_path: String,
    pub relative_path: String,
}

impl VertexClient {
    /// Create a new Vertex AI client from config
    ///
    /// # Errors
    ///
    /// Returns an error when authentication cannot be configured or the
    /// provided config fails validation.
    pub async fn new(config: Config) -> Result<Self> {
        let auth_provider =
            from_env().await.map_err(|e| VertexError::Authentication { message: e.to_string() })?;
        Self::with_config_and_auth_provider(config, auth_provider).await
    }

    /// Create a new Vertex AI client (legacy interface)
    ///
    /// # Errors
    ///
    /// Returns an error when authentication cannot be configured or the
    /// provided project/location pair is invalid.
    pub async fn new_legacy(project_id: &str, location: &str) -> Result<Self> {
        let auth_provider =
            from_env().await.map_err(|e| VertexError::Authentication { message: e.to_string() })?;
        Self::with_auth_provider(project_id, location, auth_provider).await
    }

    /// Create a client with a custom authentication provider
    ///
    /// # Errors
    ///
    /// Returns an error when the custom configuration fails validation or the
    /// client cannot be initialized.
    pub async fn with_auth_provider(
        project_id: &str,
        location: &str,
        auth_provider: Box<dyn AuthProvider>,
    ) -> Result<Self> {
        let mut config = Config {
            project_id: project_id.to_string(),
            region: location.to_string(),
            ..Config::default()
        };

        if let Some(anthropic_location) = Config::anthropic_location_override() {
            config.publisher_locations.insert("anthropic".to_string(), anthropic_location);
        }

        Self::with_config_and_auth_provider(config, auth_provider).await
    }

    /// Create a client from explicit config + auth provider
    ///
    /// # Errors
    ///
    /// Returns an error when the configuration validation fails or HTTP client
    /// construction encounters an error.
    pub async fn with_config_and_auth_provider(
        config: Config,
        auth_provider: Box<dyn AuthProvider>,
    ) -> Result<Self> {
        Self::build_with_auth(config, auth_provider).await
    }

    #[allow(clippy::unused_async)]
    async fn build_with_auth(config: Config, auth_provider: Box<dyn AuthProvider>) -> Result<Self> {
        if let Err(err) = config.validate() {
            return Err(VertexError::configuration(err.to_string()));
        }

        let http_client = HttpClient::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .user_agent("vertex-rust-sdk/0.1.0")
            .no_proxy()
            .build()
            .map_err(VertexError::Request)?;

        let base_url = config.base_url();
        let default_endpoint = endpoint_for_region(&config.region);
        let custom_base_url =
            config.base_url_override.is_some() || !base_url.eq_ignore_ascii_case(&default_endpoint);
        let publisher_locations = Self::prepare_publisher_locations(config.publisher_locations);

        Ok(Self {
            project_id: config.project_id.clone(),
            location: config.region.clone(),
            http_client,
            auth_provider,
            base_url,
            max_retries: config.max_retries,
            publisher_locations,
            custom_base_url,
        })
    }

    fn prepare_publisher_locations(
        mut overrides: HashMap<String, String>,
    ) -> HashMap<String, String> {
        if let Some(env_override) = Config::anthropic_location_override() {
            overrides.insert("anthropic".to_string(), env_override);
        } else if overrides.get("anthropic").is_none_or(|value| value.trim().is_empty()) {
            overrides.insert("anthropic".to_string(), DEFAULT_ANTHROPIC_LOCATION.to_string());
        }

        overrides
    }

    /// Get the project ID
    #[must_use]
    pub fn project_id(&self) -> &str {
        &self.project_id
    }

    /// Get the location
    #[must_use]
    pub fn location(&self) -> &str {
        &self.location
    }

    pub(crate) fn model_request_context(
        &self,
        descriptor: &ModelDescriptor,
    ) -> ModelRequestContext {
        let location = self.location_for_model(descriptor);
        let model_name = effective_model_name(descriptor);
        let relative_path = format!("publishers/{}/models/{}", descriptor.publisher(), model_name);
        let endpoint = self.endpoint_for_location(&location);
        let resource_path =
            format!("projects/{}/locations/{}/{}", self.project_id(), location, relative_path);

        ModelRequestContext { endpoint, resource_path, relative_path }
    }

    /// Expose the computed endpoint/location for debugging and logging.
    #[must_use]
    pub fn context_for_model(&self, descriptor: &ModelDescriptor) -> ModelRequestContext {
        self.model_request_context(descriptor)
    }

    fn location_for_model(&self, descriptor: &ModelDescriptor) -> String {
        if let Some(location) = model_location_override(descriptor) {
            return location.to_string();
        }

        self.publisher_locations
            .get(descriptor.publisher())
            .cloned()
            .unwrap_or_else(|| self.location.clone())
    }

    fn endpoint_for_location(&self, location: &str) -> String {
        if self.custom_base_url || location.eq_ignore_ascii_case(self.location()) {
            self.base_url.clone()
        } else {
            endpoint_for_region(location)
        }
    }

    #[allow(clippy::unused_self)]
    pub(crate) fn build_url_for_endpoint(&self, endpoint: &str, path: &str) -> String {
        format!("{endpoint}{path}")
    }

    /// Get the HTTP client
    #[allow(clippy::missing_const_for_fn)]
    pub(crate) fn http_client(&self) -> &HttpClient {
        &self.http_client
    }

    /// Get the models API
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn models(&self) -> crate::api::models::ModelsApi<'_> {
        crate::api::models::ModelsApi::new(self)
    }

    /// Get the cache API
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn cache(&self) -> crate::cache::CacheApi<'_> {
        crate::cache::CacheApi::new(self)
    }

    /// Get an authentication token
    pub(crate) async fn get_auth_token(&self) -> Result<String> {
        self.auth_provider
            .get_token()
            .await
            .map_err(|e| VertexError::Authentication { message: e.to_string() })
    }

    /// Make an authenticated request
    pub(crate) async fn make_authenticated_request<T: serde::Serialize + Sync>(
        &self,
        url: &str,
        payload: &T,
    ) -> Result<Response> {
        self.make_authenticated_request_with_headers(url, payload, &[]).await
    }

    /// Make an authenticated request with additional headers.
    pub(crate) async fn make_authenticated_request_with_headers<T: serde::Serialize + Sync>(
        &self,
        url: &str,
        payload: &T,
        extra_headers: &[(String, String)],
    ) -> Result<Response> {
        let url = url.to_owned();
        self.send_with_retry(|| {
            let url = url.clone();
            let extra_headers = extra_headers.to_vec();
            async move {
                let token = self
                    .auth_provider
                    .get_token()
                    .await
                    .map_err(|e| VertexError::Authentication { message: e.to_string() })?;

                let mut request_builder = self
                    .http_client
                    .post(&url)
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .header(header::CONTENT_TYPE, "application/json");

                for (key, value) in &extra_headers {
                    request_builder = request_builder.header(key, value);
                }

                let response =
                    request_builder.json(payload).send().await.map_err(VertexError::Request)?;

                Ok(response)
            }
        })
        .await
    }

    /// Make an authenticated GET request
    pub(crate) async fn make_authenticated_get_request(&self, url: &str) -> Result<Response> {
        let url = url.to_owned();
        self.send_with_retry(|| {
            let url = url.clone();
            async move {
                let token = self
                    .auth_provider
                    .get_token()
                    .await
                    .map_err(|e| VertexError::Authentication { message: e.to_string() })?;

                let response = self
                    .http_client
                    .get(&url)
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .header(header::CONTENT_TYPE, "application/json")
                    .send()
                    .await
                    .map_err(VertexError::Request)?;

                Ok(response)
            }
        })
        .await
    }

    /// Build the full URL for an API endpoint
    pub(crate) fn build_url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    /// Generate content
    ///
    /// # Errors
    ///
    /// Returns an error when the request fails or the response body cannot be
    /// parsed as either success or structured error.
    pub async fn generate_content(
        &self,
        model: &str,
        request: &GenerateContentRequest,
    ) -> Result<GenerateContentResponse> {
        let descriptor = ModelDescriptor::parse(model)?;
        let context = self.model_request_context(&descriptor);
        let path = format!("/v1/{}:generateContent", context.resource_path);
        let url = self.build_url_for_endpoint(&context.endpoint, &path);

        let response = self.make_authenticated_request(&url, request).await?;

        if response.status().is_success() {
            let result =
                response.json::<GenerateContentResponse>().await.map_err(VertexError::Request)?;
            Ok(result)
        } else {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            Err(VertexError::Api {
                message: format!("Status {status}: {error_text}"),
                code: status.to_string(),
            })
        }
    }

    /// Stream generate content
    ///
    /// # Errors
    ///
    /// Returns an error when the SSE request cannot be established or yields a
    /// non-success status.
    pub async fn stream_generate_content(
        &self,
        model: &str,
        request: &GenerateContentRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamingResponse>> + Send>>> {
        // Delegate to the proper SSE streaming implementation
        self.stream_generate_content_impl(model, request).await
    }

    /// Count tokens
    ///
    /// # Errors
    ///
    /// Returns an error when the count request fails or the response cannot be
    /// parsed.
    pub async fn count_tokens(
        &self,
        model: &str,
        request: &CountTokensRequest,
    ) -> Result<CountTokensResponse> {
        let descriptor = ModelDescriptor::parse(model)?;
        let context = self.model_request_context(&descriptor);
        let path = format!("/v1/{}:countTokens", context.resource_path);
        let url = self.build_url_for_endpoint(&context.endpoint, &path);

        let response = self.make_authenticated_request(&url, request).await?;

        if response.status().is_success() {
            let result =
                response.json::<CountTokensResponse>().await.map_err(VertexError::Request)?;
            Ok(result)
        } else {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            Err(VertexError::Api {
                message: format!("Status {status}: {error_text}"),
                code: status.to_string(),
            })
        }
    }

    pub(crate) async fn send_with_retry<F, Fut>(&self, mut send: F) -> Result<Response>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<Response>>,
    {
        let mut retries = 0u32;
        loop {
            let response = send().await?;

            if response.status().is_success() {
                return Ok(response);
            }

            if retries >= self.max_retries || !Self::is_retryable_status(response.status()) {
                return Ok(response);
            }

            let delay = Self::retry_delay(&response, retries + 1);
            drop(response);
            retries += 1;
            sleep(delay).await;
        }
    }

    fn retry_delay(response: &Response, attempt: u32) -> Duration {
        if let Some(retry_after) = response.headers().get(header::RETRY_AFTER) {
            if let Ok(value) = retry_after.to_str() {
                if let Ok(seconds) = value.parse::<u64>() {
                    return Duration::from_secs(seconds);
                }
            }
        }

        let capped_attempt = attempt.saturating_sub(1).min(5);
        let multiplier = 1u64 << capped_attempt;
        let millis = 500 * multiplier;
        Duration::from_millis(millis).min(Duration::from_secs(10))
    }

    #[allow(clippy::missing_const_for_fn)]
    fn is_retryable_status(status: StatusCode) -> bool {
        matches!(
            status,
            StatusCode::TOO_MANY_REQUESTS
                | StatusCode::INTERNAL_SERVER_ERROR
                | StatusCode::BAD_GATEWAY
                | StatusCode::SERVICE_UNAVAILABLE
                | StatusCode::GATEWAY_TIMEOUT
        )
    }
}

/// Builder for creating a `VertexClient`
pub struct VertexClientBuilder {
    config: Config,
    auth_provider: Option<Box<dyn AuthProvider>>,
}

impl VertexClientBuilder {
    /// Create a new builder from config
    #[must_use]
    pub fn from_config(config: Config) -> Self {
        Self { config, auth_provider: None }
    }

    /// Create a new builder (legacy interface)
    #[must_use]
    pub fn new(project_id: &str, location: &str) -> Self {
        let config = Config {
            project_id: project_id.to_string(),
            region: location.to_string(),
            ..Default::default()
        };

        Self { config, auth_provider: None }
    }

    /// Set custom authentication provider
    #[must_use]
    pub fn with_auth_provider(mut self, provider: Box<dyn AuthProvider>) -> Self {
        self.auth_provider = Some(provider);
        self
    }

    /// Build the client
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying client initialization fails.
    pub async fn build(self) -> Result<VertexClient> {
        if let Some(auth_provider) = self.auth_provider {
            VertexClient::with_config_and_auth_provider(self.config, auth_provider).await
        } else {
            VertexClient::new(self.config).await
        }
    }
}

fn model_location_override(descriptor: &ModelDescriptor) -> Option<&'static str> {
    match descriptor.publisher().to_ascii_lowercase().as_str() {
        "anthropic" => anthropic_global_model(descriptor.model()).then_some("global"),
        "google" => google_global_model(descriptor.model()).then_some("global"),
        _ => None,
    }
}

fn effective_model_name(descriptor: &ModelDescriptor) -> String {
    anthropic_version_override(descriptor).unwrap_or_else(|| descriptor.model().trim().to_string())
}

fn anthropic_version_override(descriptor: &ModelDescriptor) -> Option<String> {
    if !descriptor.publisher().eq_ignore_ascii_case("anthropic") {
        return None;
    }

    let raw_model = descriptor.model().trim();
    if raw_model.contains('@') {
        return None;
    }

    match normalize_model_key(raw_model).as_str() {
        // 4.5 family
        "claude-sonnet-4-5" | "claude-sonnet-45" | "sonnet-4-5" | "sonnet-45" => {
            Some(format!("{raw_model}@20250929"))
        }
        "claude-opus-4-5" | "claude-opus-45" | "opus-4-5" | "opus-45" => {
            Some(format!("{raw_model}@20251101"))
        }
        // 4.1 family
        "claude-opus-4-1" | "claude-opus-41" | "opus-4-1" | "opus-41" => {
            Some(format!("{raw_model}@20250805"))
        }
        // 4.0 family
        "claude-sonnet-4" | "sonnet-4" | "claude-opus-4" | "opus-4" => {
            Some(format!("{raw_model}@20250514"))
        }
        // 4.6 models use bare IDs — no @date suffix.
        // All other Anthropic models pass through unchanged.
        _ => None,
    }
}

fn anthropic_global_model(model_name: &str) -> bool {
    matches!(
        normalize_model_key(model_name).as_str(),
        // 4.5 family
        "claude-haiku-4-5"
            | "claude-haiku-45"
            | "haiku-4-5"
            | "haiku-45"
            | "claude-sonnet-4-5"
            | "claude-sonnet-45"
            | "sonnet-4-5"
            | "sonnet-45"
            | "claude-opus-4-5"
            | "claude-opus-45"
            | "opus-4-5"
            | "opus-45"
            // 4.6 family
            | "claude-opus-4-6"
            | "claude-opus-46"
            | "opus-4-6"
            | "opus-46"
            | "claude-sonnet-4-6"
            | "claude-sonnet-46"
            | "sonnet-4-6"
            | "sonnet-46"
            // 4.1 / 4.0 family
            | "claude-opus-4-1"
            | "claude-opus-41"
            | "opus-4-1"
            | "opus-41"
            | "claude-sonnet-4"
            | "sonnet-4"
            | "claude-opus-4"
            | "opus-4"
    )
}

fn google_global_model(model_name: &str) -> bool {
    matches!(
        normalize_model_key(model_name).as_str(),
        "gemini-3-pro-preview"
            | "gemini-3-1-pro"
            | "gemini-3-1-pro-preview"
            | "gemini-3-1-flash"
            | "gemini-3-1-flash-preview"
            | "gemini-3-1-flash-lite"
            | "gemini-3-1-flash-lite-preview"
    )
}

fn normalize_model_key(model_name: &str) -> String {
    let cleaned = model_name.trim().to_ascii_lowercase().replace('.', "-");
    if let Some((head, _)) = cleaned.split_once('@') {
        head.to_string()
    } else {
        cleaned
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::AuthProvider;
    use async_trait::async_trait;

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

    async fn build_test_client() -> VertexClient {
        let mut config = Config {
            project_id: "test-project".to_string(),
            region: "us-central1".to_string(),
            ..Config::default()
        };
        config.publisher_locations.insert("anthropic".to_string(), "us-east5".to_string());

        VertexClient::with_config_and_auth_provider(config, Box::new(TestAuthProvider))
            .await
            .expect("vertex client")
    }

    #[tokio::test]
    async fn haiku_45_models_use_global_location() {
        let client = build_test_client().await;
        let descriptor =
            ModelDescriptor::parse("publishers/anthropic/models/claude-haiku-4-5").unwrap();
        let context = client.model_request_context(&descriptor);

        assert_eq!(
            context.resource_path,
            "projects/test-project/locations/global/publishers/anthropic/models/claude-haiku-4-5"
        );
    }

    #[tokio::test]
    async fn sonnet_45_models_use_global_location_and_version() {
        let client = build_test_client().await;
        let descriptor =
            ModelDescriptor::parse("publishers/anthropic/models/claude-sonnet-4-5").unwrap();
        let context = client.model_request_context(&descriptor);

        assert_eq!(
            context.resource_path,
            "projects/test-project/locations/global/publishers/anthropic/models/claude-sonnet-4-5@20250929"
        );
        assert_eq!(context.relative_path, "publishers/anthropic/models/claude-sonnet-4-5@20250929");
    }

    #[tokio::test]
    async fn gemini_3_pro_preview_models_use_global_location() {
        let client = build_test_client().await;
        let descriptor =
            ModelDescriptor::parse("publishers/google/models/gemini-3-pro-preview").unwrap();
        let context = client.model_request_context(&descriptor);

        assert_eq!(
            context.resource_path,
            "projects/test-project/locations/global/publishers/google/models/gemini-3-pro-preview"
        );
    }

    #[test]
    fn anthropic_global_model_detects_4_5_variants() {
        assert!(anthropic_global_model("claude-haiku-4-5@20251001"));
        assert!(anthropic_global_model("haiku-4.5"));
        assert!(anthropic_global_model(" HAIKU-45 "));
        assert!(anthropic_global_model("claude-sonnet-4-5"));
        assert!(anthropic_global_model("sonnet-4.5"));
        assert!(anthropic_global_model(" CLAUDE-SONNET-45 "));
    }

    #[test]
    fn anthropic_version_override_adds_release_tag() {
        let descriptor =
            ModelDescriptor::parse("publishers/anthropic/models/claude-sonnet-4-5").unwrap();
        assert_eq!(effective_model_name(&descriptor), "claude-sonnet-4-5@20250929");
    }

    #[test]
    fn anthropic_version_override_respects_existing_tag() {
        let descriptor = ModelDescriptor::parse(
            "projects/demo/locations/global/publishers/anthropic/models/claude-sonnet-4-5@20250929",
        )
        .unwrap();
        assert_eq!(effective_model_name(&descriptor), "claude-sonnet-4-5@20250929");
    }

    #[test]
    fn anthropic_version_override_adds_opus_45_release_tag() {
        let descriptor =
            ModelDescriptor::parse("publishers/anthropic/models/claude-opus-4-5").unwrap();
        assert_eq!(effective_model_name(&descriptor), "claude-opus-4-5@20251101");
    }

    #[test]
    fn opus_46_uses_bare_id_no_version_suffix() {
        let descriptor =
            ModelDescriptor::parse("publishers/anthropic/models/claude-opus-4-6").unwrap();
        assert_eq!(effective_model_name(&descriptor), "claude-opus-4-6");
    }

    #[test]
    fn sonnet_46_uses_bare_id_no_version_suffix() {
        let descriptor =
            ModelDescriptor::parse("publishers/anthropic/models/claude-sonnet-4-6").unwrap();
        assert_eq!(effective_model_name(&descriptor), "claude-sonnet-4-6");
    }

    #[test]
    fn opus_41_gets_version_suffix() {
        let descriptor =
            ModelDescriptor::parse("publishers/anthropic/models/claude-opus-4-1").unwrap();
        assert_eq!(effective_model_name(&descriptor), "claude-opus-4-1@20250805");
    }

    #[test]
    fn sonnet_4_and_opus_4_get_version_suffix() {
        let s4 = ModelDescriptor::parse("publishers/anthropic/models/claude-sonnet-4").unwrap();
        assert_eq!(effective_model_name(&s4), "claude-sonnet-4@20250514");

        let o4 = ModelDescriptor::parse("publishers/anthropic/models/claude-opus-4").unwrap();
        assert_eq!(effective_model_name(&o4), "claude-opus-4@20250514");
    }

    #[test]
    fn claude_46_models_use_global_location() {
        assert!(anthropic_global_model("claude-opus-4-6"));
        assert!(anthropic_global_model("opus-4.6"));
        assert!(anthropic_global_model("claude-sonnet-4-6"));
        assert!(anthropic_global_model("sonnet-4.6"));
    }

    #[test]
    fn claude_41_and_4_models_use_global_location() {
        assert!(anthropic_global_model("claude-opus-4-1"));
        assert!(anthropic_global_model("claude-sonnet-4"));
        assert!(anthropic_global_model("claude-opus-4"));
    }

    #[test]
    fn normalize_model_key_unifies_variants() {
        assert_eq!(normalize_model_key("claude-haiku-4-5@20251001"), "claude-haiku-4-5");
        assert_eq!(normalize_model_key("haiku-4.5"), "haiku-4-5");
        assert_eq!(normalize_model_key("  HAIKU-45  "), "haiku-45");
    }

    #[test]
    fn google_global_model_detects_gemini_3_pro_preview() {
        assert!(google_global_model("gemini-3-pro-preview"));
        assert!(!google_global_model("gemini-2.5-pro"));
    }
}
