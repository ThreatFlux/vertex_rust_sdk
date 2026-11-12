//! JSON prediction endpoints for Google publisher models such as Veo, Lyria 2, and Virtual Try-On.

use crate::{ModelDescriptor, Result, VertexClient, VertexError};
use serde_json::{json, Value};

impl VertexClient {
    /// Call `predict` with the model's documented `instances` and `parameters` payload.
    /// Use this for models such as Lyria 2 and Virtual Try-On.
    ///
    /// # Errors
    ///
    /// Returns an error if the model, request, or response is invalid.
    pub async fn predict(&self, model: &str, request: &Value) -> Result<Value> {
        self.publisher_prediction(model, "predict", request).await
    }

    /// Start a long-running prediction, such as Veo video generation.
    /// Pass the returned operation `name` to [`Self::fetch_predict_operation`].
    ///
    /// # Errors
    ///
    /// Returns an error if the model, request, or response is invalid.
    pub async fn predict_long_running(&self, model: &str, request: &Value) -> Result<Value> {
        self.publisher_prediction(model, "predictLongRunning", request).await
    }

    /// Poll a long-running prediction on the same client and model used to create it.
    ///
    /// # Errors
    ///
    /// Returns an error if the model, request, or response is invalid.
    pub async fn fetch_predict_operation(&self, model: &str, name: &str) -> Result<Value> {
        self.publisher_prediction(model, "fetchPredictOperation", &json!({"operationName": name}))
            .await
    }

    async fn publisher_prediction(
        &self,
        model: &str,
        method: &str,
        request: &Value,
    ) -> Result<Value> {
        let descriptor = ModelDescriptor::parse(model)?;
        let id = descriptor.model();
        if descriptor.publisher() != "google"
            || !id.as_bytes().first().is_some_and(u8::is_ascii_alphanumeric)
            || !id.bytes().all(|byte| byte.is_ascii_alphanumeric() || b"-_.@".contains(&byte))
        {
            return Err(VertexError::configuration("Expected a Google publisher model ID"));
        }
        let context = self.model_request_context(&descriptor);
        let path = format!("/v1/{}:{method}", context.resource_path);
        let url = self.build_url_for_endpoint(&context.endpoint, &path);
        // A prediction POST can create a billable job. Leave retry decisions to the caller.
        let response = self
            .http_client()
            .post(url)
            .bearer_auth(self.get_auth_token().await?)
            .json(request)
            .send()
            .await?;
        let status = response.status();
        if status.is_success() {
            Ok(response.json().await?)
        } else {
            Err(VertexError::Api { code: status.to_string(), message: response.text().await? })
        }
    }
}
