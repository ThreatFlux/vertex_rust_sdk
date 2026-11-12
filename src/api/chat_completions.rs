//! OpenAI-compatible chat completions for managed Vertex models.

use eventsource_stream::Eventsource;
use futures_util::{future, Stream, StreamExt};
use reqwest::Response;
use serde_json::Value;
use std::pin::Pin;

use crate::{Result, VertexClient, VertexError};

impl VertexClient {
    /// Call the global Chat Completions API with the model's documented JSON
    /// request, such as `"model": "google/gemma-4-26b-a4b-it-maas"`.
    ///
    /// # Errors
    ///
    /// Returns an error for streaming requests or when the HTTP call or JSON
    /// decoding fails.
    pub async fn chat_completions(&self, request: &Value) -> Result<Value> {
        if request.get("stream").and_then(Value::as_bool) == Some(true) {
            return Err(VertexError::configuration(
                "Use stream_chat_completions for streaming requests",
            ));
        }
        let response = self.post_chat_completions(request).await?;
        Ok(response.json().await?)
    }

    /// Stream JSON chunks from the global Chat Completions API.
    /// The SDK sets `stream` on the request and consumes the final SSE marker.
    ///
    /// # Errors
    ///
    /// Returns an error when the request is invalid, the HTTP call fails, or a
    /// streamed chunk cannot be decoded.
    pub async fn stream_chat_completions(
        &self,
        request: &Value,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<Value>> + Send>>> {
        let mut request = request.clone();
        request
            .as_object_mut()
            .ok_or_else(|| VertexError::configuration("Expected a Chat Completions JSON object"))?
            .insert("stream".to_string(), Value::Bool(true));
        let response = self.post_chat_completions(&request).await?;
        let stream = response
            .bytes_stream()
            .eventsource()
            .take_while(|event| {
                future::ready(!matches!(event, Ok(event) if event.data.trim() == "[DONE]"))
            })
            .map(|event| {
                let event = event.map_err(|err| VertexError::streaming(err.to_string()))?;
                Ok(serde_json::from_str(&event.data)?)
            });
        Ok(Box::pin(stream))
    }

    async fn post_chat_completions(&self, request: &Value) -> Result<Response> {
        let endpoint = self.endpoint_for_location("global");
        let project = self.project_id();
        let url = format!(
            "{endpoint}/v1/projects/{project}/locations/global/endpoints/openapi/chat/completions"
        );
        let response = self.make_authenticated_request(&url, request).await?;
        let status = response.status();
        if status.is_success() {
            Ok(response)
        } else {
            Err(VertexError::http(status.as_u16(), response.text().await?))
        }
    }
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use futures_util::TryStreamExt;
    use mockito::{Matcher, Server};
    use serde_json::json;

    use super::*;
    use crate::{auth::AuthProvider, config::Config};

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
    async fn calls_managed_gemma_in_both_response_modes() {
        let mut server = Server::new_async().await;
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
        let path = "/v1/projects/test-project/locations/global/endpoints/openapi/chat/completions";
        let request = json!({
            "model": "google/gemma-4-26b-a4b-it-maas",
            "messages": [{"role": "user", "content": "Say hello."}]
        });
        let sync = server
            .mock("POST", path)
            .match_header("authorization", "Bearer test-token")
            .match_body(Matcher::Json(request.clone()))
            .with_status(200)
            .with_body(r#"{"choices":[{"message":{"content":"Hello!"}}]}"#)
            .create_async()
            .await;
        let mut streaming_request = request.clone();
        streaming_request["stream"] = json!(true);
        let streaming = server
            .mock("POST", path)
            .match_header("authorization", "Bearer test-token")
            .match_body(Matcher::Json(streaming_request))
            .with_status(200)
            .with_header("content-type", "text/event-stream")
            .with_body(concat!(
                "data: {\"choices\":[{\"delta\":{\"content\":\"Hello!\"}}]}\n\n",
                "data: [DONE]\n\n"
            ))
            .create_async()
            .await;

        let completed = client.chat_completions(&request).await.unwrap();
        assert_eq!(completed["choices"][0]["message"]["content"], "Hello!");
        let chunks: Vec<_> =
            client.stream_chat_completions(&request).await.unwrap().try_collect().await.unwrap();
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0]["choices"][0]["delta"]["content"], "Hello!");
        sync.assert_async().await;
        streaming.assert_async().await;
    }
}
