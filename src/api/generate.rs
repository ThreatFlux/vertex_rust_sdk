//! Non-streaming content generation API

use crate::client::{effective_model_name, VertexClient};
use crate::error::{Result, VertexError};
use crate::model_descriptor::ModelDescriptor;
use crate::models::{GenerateContentRequest, GenerateContentResponse, GenerateContentSegment};
use serde::{de::DeserializeOwned, Serialize};
use std::borrow::Cow;

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
        let descriptor = ModelDescriptor::parse(model)?;
        let request = prepare_generation_request(&descriptor, request)?;
        let version = generation_api_version(&descriptor);
        self.post_generation(&descriptor, version, request.as_ref()).await
    }

    /// Generate one segment of a Gemini 4 long decode using the v1beta1 API.
    ///
    /// Set `max_output_tokens` to the desired total across all segments. Use
    /// `None` to start generation and pass each returned continuation token
    /// with the original request to get the next segment.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails or the segment cannot be decoded.
    pub async fn generate_content_segment(
        &self,
        model: &str,
        request: &GenerateContentRequest,
        continuation_token: Option<&str>,
    ) -> Result<GenerateContentSegment<GenerateContentResponse>> {
        let descriptor = ModelDescriptor::parse(model)?;
        let request = prepare_generation_request(&descriptor, request)?;
        let segment = SegmentRequest { request: request.as_ref(), continuation_token };
        self.post_generation(&descriptor, "v1beta1", &segment).await
    }

    async fn post_generation<T, R>(
        &self,
        descriptor: &ModelDescriptor,
        version: &str,
        request: &T,
    ) -> Result<R>
    where
        T: Serialize + Sync,
        R: DeserializeOwned,
    {
        let context = self.model_request_context(descriptor);
        let path = format!("/{version}/{}:generateContent", context.resource_path);
        let url = self.build_url_for_endpoint(&context.endpoint, &path);
        let response = self.make_authenticated_request(&url, request).await?;

        if response.status().is_success() {
            response.json().await.map_err(VertexError::Request)
        } else {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            Err(VertexError::Api {
                message: format!("Status {status}: {error_text}"),
                code: status.to_string(),
            })
        }
    }
}

#[derive(Serialize)]
pub(super) struct SegmentRequest<'a> {
    #[serde(flatten)]
    pub request: &'a GenerateContentRequest,
    #[serde(rename = "continuationToken", skip_serializing_if = "Option::is_none")]
    pub continuation_token: Option<&'a str>,
}

pub(super) fn generation_api_version(descriptor: &ModelDescriptor) -> &'static str {
    if descriptor.publisher() == "google"
        && matches!(
            effective_model_name(descriptor).as_str(),
            "gemini-3.5-transcribe" | "gemini-3.5-transcribe-preview"
        )
    {
        "v1beta1"
    } else {
        "v1"
    }
}

pub(super) fn prepare_generation_request<'a>(
    descriptor: &ModelDescriptor,
    request: &'a GenerateContentRequest,
) -> Result<Cow<'a, GenerateContentRequest>> {
    let Some(config) = request.generation_config.as_ref() else {
        return Ok(Cow::Borrowed(request));
    };
    if descriptor.publisher() != "google" {
        return Ok(Cow::Borrowed(request));
    }
    let model = effective_model_name(descriptor);
    let single_candidate = model.starts_with("gemini-4-") || model.starts_with("gemini-3.8-flash");
    let ignores_sampling = single_candidate
        || model.starts_with("gemini-3.6-flash")
        || model.starts_with("gemini-3.7-flash");
    if !ignores_sampling {
        return Ok(Cow::Borrowed(request));
    }
    if single_candidate && config.candidate_count.is_some_and(|count| count != 1) {
        return Err(VertexError::configuration(format!("{model} only returns one candidate")));
    }
    if config.temperature.is_none()
        && config.top_p.is_none()
        && config.top_k.is_none()
        && (!single_candidate || config.candidate_count.is_none())
    {
        return Ok(Cow::Borrowed(request));
    }

    let mut config = config.clone();
    config.temperature = None;
    config.top_p = None;
    config.top_k = None;
    if single_candidate {
        config.candidate_count = None;
    }
    let mut request = request.clone();
    request.generation_config = Some(config);
    Ok(Cow::Owned(request))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AuthProvider, FinishReason, FunctionDeclaration, GenerationConfig, ThinkingLevel, Tool,
    };
    use mockito::{Matcher, Server};
    use serde_json::json;

    struct TestAuth;

    #[async_trait::async_trait]
    impl AuthProvider for TestAuth {
        async fn get_token(&self) -> anyhow::Result<String> {
            Ok("test-token".to_string())
        }

        async fn refresh_if_needed(&self) -> anyhow::Result<()> {
            Ok(())
        }
    }

    async fn test_client(server: &Server) -> VertexClient {
        let config = crate::config::Config {
            project_id: "test-project".into(),
            region: "global".into(),
            base_url_override: Some(server.url()),
            ..Default::default()
        };
        VertexClient::with_config_and_auth_provider(config, Box::new(TestAuth)).await.unwrap()
    }

    #[test]
    fn latest_gemini_requests_omit_unsupported_default_sampling() {
        let request = GenerateContentRequest::new("test").with_generation_config(
            GenerationConfig::default().with_thinking_level(ThinkingLevel::Medium),
        );
        for model in ["gemini-4-argon", "gemini-3.8-flash", "gemini-3-8-flash"] {
            let descriptor = ModelDescriptor::parse(model).unwrap();
            let prepared = prepare_generation_request(&descriptor, &request).unwrap();
            assert_eq!(
                serde_json::to_value(prepared.as_ref()).unwrap()["generationConfig"],
                json!({"maxOutputTokens": 2048, "thinkingConfig": {"thinkingLevel": "MEDIUM"}})
            );
        }
        assert_eq!(request.generation_config.unwrap().candidate_count, Some(1));
    }

    #[test]
    fn routes_transcription_and_its_alias_to_the_beta_endpoint() {
        for model in ["gemini-3.5-transcribe", "gemini-3-5-transcribe-preview"] {
            let descriptor = ModelDescriptor::parse(model).unwrap();
            assert_eq!(generation_api_version(&descriptor), "v1beta1");
        }
    }

    #[tokio::test]
    #[allow(clippy::significant_drop_tightening)]
    async fn gemini_four_continues_using_the_beta_endpoint() {
        use futures_util::StreamExt;

        let mut server = Server::new_async().await;
        let path = "/v1beta1/projects/test-project/locations/global/publishers/google/models/gemini-4-argon";
        let first_segment = json!({"candidates": [{
            "content": {"role": "model", "parts": [{"text": "First"}]},
            "finishReason": "CONTINUATION",
            "continuationToken": "next-segment"
        }]});
        let first = server
            .mock("POST", format!("{path}:generateContent").as_str())
            .match_body(Matcher::Json(json!({
                "contents": [{"role": "user", "parts": [{"text": "Write a long story"}]}],
                "generationConfig": {"maxOutputTokens": 131_072}
            })))
            .with_header("content-type", "application/json")
            .with_body(first_segment.to_string())
            .create_async()
            .await;
        let next = server
            .mock("POST", format!("{path}:streamGenerateContent?alt=sse").as_str())
            .match_body(Matcher::PartialJson(json!({"continuationToken": "next-segment"})))
            .with_header("content-type", "text/event-stream")
            .with_body("data: {\"candidates\":[{\"content\":{\"role\":\"model\",\"parts\":[{\"text\":\"Last\"}]},\"finishReason\":\"STOP\"}]}\n\n")
            .create_async()
            .await;
        let client = test_client(&server).await;
        let request = GenerateContentRequest::new("Write a long story").with_generation_config(
            GenerationConfig { max_output_tokens: Some(131_072), ..GenerationConfig::default() },
        );
        let segment =
            client.generate_content_segment("gemini-4-argon", &request, None).await.unwrap();
        assert_eq!(segment.response.text().as_deref(), Some("First"));
        assert!(matches!(segment.response.finish_reason(), Some(FinishReason::Continuation)));

        let mut stream = client
            .stream_generate_content_segment(
                "gemini-4-argon",
                &request,
                segment.continuation_token.as_deref(),
            )
            .await
            .unwrap();
        let chunk = stream.next().await.unwrap().unwrap();
        assert_eq!(chunk.response.text().as_deref(), Some("Last"));
        assert!(chunk.continuation_token.is_none());
        assert!(stream.next().await.is_none());
        first.assert_async().await;
        next.assert_async().await;
    }

    #[tokio::test]
    #[allow(clippy::significant_drop_tightening)]
    async fn gemini_four_replays_signed_calls_and_matches_parallel_response_ids() {
        let mut server = Server::new_async().await;
        let path = "/v1/projects/test-project/locations/global/publishers/google/models/gemini-4-argon:generateContent";
        let user = json!({"role": "user", "parts": [{"text": "Check both flights"}]});
        let model = json!({"role": "model", "parts": [
            {"functionCall": {"id": "first", "name": "check", "args": {"flight": "AA100"}}, "thoughtSignature": "sig-a"},
            {"functionCall": {"id": "second", "name": "check", "args": {"flight": "AA200"}}}
        ]});
        let tool = Tool::function_calling(vec![FunctionDeclaration {
            name: "check".into(),
            description: "Check flight status".into(),
            parameters: json!({"type": "object", "properties": {"flight": {"type": "string"}}}),
        }]);
        let first = server
            .mock("POST", path)
            .match_body(Matcher::Json(json!({"contents": [user], "generationConfig": {"maxOutputTokens": 2048}, "tools": [tool]})))
            .with_header("content-type", "application/json")
            .with_body(json!({"candidates": [{"content": model}]}).to_string())
            .create_async()
            .await;
        let next = server
            .mock("POST", path)
            .match_body(Matcher::Json(json!({"contents": [user, model, {
                "role": "user", "parts": [
                    {"functionResponse": {"id": "first", "name": "check", "response": {"flight": "AA100"}}},
                    {"functionResponse": {"id": "second", "name": "check", "response": {"flight": "AA200"}}}
                ]
            }], "generationConfig": {"maxOutputTokens": 2048}, "tools": [tool]})))
            .with_header("content-type", "application/json")
            .with_body(json!({"candidates": [{"content": {"role": "model", "parts": [{"text": "Both checked", "thoughtSignature": "sig-b"}]}}]}).to_string())
            .create_async()
            .await;
        let client = test_client(&server).await;
        let response = client
            .execute_function_calling_flow(
                "gemini-4-argon",
                &GenerateContentRequest::new("Check both flights").with_tools(vec![tool]),
                |call| Ok::<_, std::io::Error>(json!({"flight": call.args["flight"]})),
            )
            .await
            .unwrap();
        assert_eq!(response.text().as_deref(), Some("Both checked"));
        first.assert_async().await;
        next.assert_async().await;
    }
}
