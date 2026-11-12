//! Gemini Enterprise Agent Platform Interactions API.

use reqwest::{Response, Url};
use serde_json::Value;

use crate::client::VertexClient;
use crate::error::{ApiError, Result, VertexError};

impl VertexClient {
    /// Create a synchronous or background interaction.
    ///
    /// Pass the documented v1beta1 Interactions API JSON body, including the model ID
    /// and input. For background execution, set `"background": true` and use
    /// the returned `id` with [`Self::get_interaction`] to poll for the result.
    /// Gemini Omni and Lyria 3 use this API for video and music generation.
    /// Select the `global` location for those models. For long synchronous
    /// generations, set the client request timeout to cover the generation.
    ///
    /// # Errors
    ///
    /// Returns an error for streaming requests or when authentication, the HTTP
    /// call, or decoding the JSON response fails.
    pub async fn create_interaction(&self, request: &Value) -> Result<Value> {
        if request.get("stream").and_then(Value::as_bool) == Some(true) {
            return Err(VertexError::configuration(
                "create_interaction accepts synchronous and background requests",
            ));
        }

        // Creating an interaction may start a billable job. Send the POST once
        // because the API does not expose an idempotency key for this operation.
        let token = self.get_auth_token().await?;
        let response = self
            .http_client()
            .post(self.interactions_url())
            .bearer_auth(token)
            .json(request)
            .send()
            .await?;
        decode_interaction(response).await
    }

    /// Retrieve an interaction by the `id` returned when it was created.
    ///
    /// Use the same client location as the create call. A background interaction
    /// reports its current state in the response's `status` field.
    ///
    /// # Errors
    ///
    /// Returns an error for an invalid ID or URL, or when the HTTP call or JSON
    /// decoding fails.
    pub async fn get_interaction(&self, id: &str) -> Result<Value> {
        if matches!(id, "" | "." | "..") {
            return Err(VertexError::configuration("Invalid interaction ID"));
        }

        let mut url = Url::parse(&self.interactions_url()).map_err(|err| {
            VertexError::configuration(format!("Invalid interactions URL: {err}"))
        })?;
        url.path_segments_mut()
            .map_err(|()| VertexError::configuration("Interactions URL must have a base path"))?
            .push(id);

        let response = self.make_authenticated_get_request(url.as_str()).await?;
        decode_interaction(response).await
    }

    fn interactions_url(&self) -> String {
        let configured_location = self.location().to_ascii_lowercase();
        let location = match configured_location.as_str() {
            "us" | "eu" => configured_location.as_str(),
            _ => "global",
        };
        let endpoint = self.endpoint_for_location(location);
        let project = self.project_id();
        format!("{endpoint}/v1beta1/projects/{project}/locations/{location}/interactions")
    }
}

async fn decode_interaction(response: Response) -> Result<Value> {
    let status = response.status();
    if status.is_success() {
        return response.json().await.map_err(VertexError::Request);
    }

    let body = response.text().await?;
    let error = serde_json::from_str::<ApiError>(&body)
        .map_or_else(|_| VertexError::http(status.as_u16(), body), VertexError::from);
    Err(error)
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use mockito::{Matcher, Server};
    use serde_json::json;

    use super::*;
    use crate::auth::AuthProvider;
    use crate::config::Config;

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

    #[tokio::test]
    #[allow(clippy::significant_drop_tightening)]
    async fn creates_and_polls_an_omni_background_interaction() {
        let mut server = Server::new_async().await;
        let path = "/v1beta1/projects/test-project/locations/global/interactions";
        let request = json!({
            "model": "gemini-omni-1.2-flash",
            "input": [{"type": "text", "text": "A bird in flight"}],
            "background": true,
            "response_format": [{
                "type": "video",
                "duration": "5s",
                "delivery": "uri",
                "gcs_uri": "gs://test-bucket/videos/"
            }],
            "generation_config": {"video_config": {"task": "text_to_video"}}
        });
        let create = server
            .mock("POST", path)
            .match_header("authorization", "Bearer test-token")
            .match_body(Matcher::Json(request.clone()))
            .with_status(200)
            .with_body(r#"{"id":"interaction-123","status":"in_progress"}"#)
            .create_async()
            .await;
        let get_path = format!("{path}/interaction-123");
        let get = server
            .mock("GET", get_path.as_str())
            .match_header("authorization", "Bearer test-token")
            .with_status(200)
            .with_body(
                json!({
                    "id": "interaction-123",
                    "status": "completed",
                    "steps": [{
                        "type": "model_output",
                        "content": [{
                            "type": "video",
                            "mime_type": "video/mp4",
                            "uri": "gs://test-bucket/videos/bird.mp4"
                        }]
                    }]
                })
                .to_string(),
            )
            .create_async()
            .await;
        let config = Config {
            project_id: "test-project".to_string(),
            region: "us-central1".to_string(),
            base_url_override: Some(server.url()),
            ..Config::default()
        };
        let client =
            VertexClient::with_config_and_auth_provider(config, Box::new(TestAuthProvider))
                .await
                .unwrap();

        let submitted = client.create_interaction(&request).await.unwrap();
        assert_eq!(submitted["status"], "in_progress");
        let completed = client.get_interaction(submitted["id"].as_str().unwrap()).await.unwrap();
        assert_eq!(completed["status"], "completed");
        assert_eq!(completed["steps"][0]["content"][0]["uri"], "gs://test-bucket/videos/bird.mp4");
        create.assert_async().await;
        get.assert_async().await;
    }
}
