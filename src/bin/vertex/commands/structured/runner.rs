use anyhow::Result;
use async_trait::async_trait;
use threatflux_vertex_rust_sdk::{
    client::VertexClient,
    config::Config,
    models::{GenerateContentRequest, GenerateContentResponse},
    types::GenerationConfig,
};

#[derive(Debug)]
pub struct StructuredOptions<'a> {
    pub prompt: &'a str,
    pub model: &'a str,
    pub schema: serde_json::Value,
}

#[async_trait]
pub trait StructuredClient: Send + Sync {
    async fn generate(
        &self,
        model: &str,
        request: &GenerateContentRequest,
    ) -> Result<GenerateContentResponse>;
}

pub struct VertexStructuredClient {
    inner: VertexClient,
}

impl VertexStructuredClient {
    pub async fn from_env() -> Result<Self> {
        let config = Config::from_env()?;
        let client = VertexClient::new(config).await?;
        Ok(Self { inner: client })
    }
}

#[async_trait]
impl StructuredClient for VertexStructuredClient {
    async fn generate(
        &self,
        model: &str,
        request: &GenerateContentRequest,
    ) -> Result<GenerateContentResponse> {
        self.inner.generate_content(model, request).await.map_err(Into::into)
    }
}

pub async fn run_structured<C: StructuredClient>(
    client: &C,
    options: &StructuredOptions<'_>,
) -> Result<GenerateContentResponse> {
    let generation_config = GenerationConfig::default().with_json_schema(options.schema.clone());
    let request =
        GenerateContentRequest::new(options.prompt).with_generation_config(generation_config);
    client.generate(options.model, &request).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use threatflux_vertex_rust_sdk::{
        models::GenerateContentResponse,
        types::{Candidate, Content, Part},
    };

    struct RecordingClient {
        last_model: std::sync::Mutex<Option<String>>,
        last_request: std::sync::Mutex<Option<GenerateContentRequest>>,
        response: GenerateContentResponse,
    }

    #[async_trait]
    impl StructuredClient for RecordingClient {
        async fn generate(
            &self,
            model: &str,
            request: &GenerateContentRequest,
        ) -> Result<GenerateContentResponse> {
            *self.last_model.lock().unwrap() = Some(model.to_string());
            *self.last_request.lock().unwrap() = Some(request.clone());
            Ok(self.response.clone())
        }
    }

    fn sample_response() -> GenerateContentResponse {
        GenerateContentResponse {
            candidates: vec![Candidate {
                content: Content {
                    role: "model".to_string(),
                    parts: vec![Part::Text { text: "hi".to_string() }],
                },
                finish_reason: None,
                safety_ratings: vec![],
                index: None,
            }],
            usage_metadata: None,
            grounding_metadata: None,
        }
    }

    #[tokio::test]
    async fn builds_request_with_schema() {
        let client = RecordingClient {
            last_model: std::sync::Mutex::new(None),
            last_request: std::sync::Mutex::new(None),
            response: sample_response(),
        };

        let options = StructuredOptions {
            prompt: "hello",
            model: "gemini",
            schema: serde_json::json!({"type":"object"}),
        };

        run_structured(&client, &options).await.unwrap();

        let model = client.last_model.lock().unwrap().clone().unwrap();
        assert_eq!(model, "gemini");

        let request = client.last_request.lock().unwrap().clone().unwrap();
        assert_eq!(request.contents[0].parts.len(), 1);
        let config = request.generation_config.expect("config");
        assert_eq!(config.response_schema, Some(serde_json::json!({"type":"object"})));
        assert_eq!(config.response_mime_type.as_deref(), Some("application/json"));
    }
}
