use async_trait::async_trait;
use threatflux_vertex_rust_sdk::{GenerateContentRequest, GenerateContentResponse, VertexClient};

pub type ClientResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[async_trait]
pub trait ContentGenerator {
    async fn generate(
        &self,
        model: &str,
        request: &GenerateContentRequest,
    ) -> ClientResult<GenerateContentResponse>;
}

pub struct RealContentGenerator {
    inner: VertexClient,
}

impl RealContentGenerator {
    pub async fn new(project_id: &str, location: &str) -> ClientResult<Self> {
        let inner = VertexClient::new_legacy(project_id, location).await?;
        Ok(Self { inner })
    }
}

#[async_trait]
impl ContentGenerator for RealContentGenerator {
    async fn generate(
        &self,
        model: &str,
        request: &GenerateContentRequest,
    ) -> ClientResult<GenerateContentResponse> {
        Ok(self.inner.generate_content(model, request).await?)
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::sync::Mutex;

    #[derive(Default)]
    pub struct MockContentGenerator {
        responses: Mutex<VecDeque<ClientResult<GenerateContentResponse>>>,
        pub requests: Mutex<Vec<GenerateContentRequest>>,
        pub models: Mutex<Vec<String>>,
    }

    impl MockContentGenerator {
        pub fn new(responses: Vec<GenerateContentResponse>) -> Self {
            Self {
                responses: Mutex::new(responses.into_iter().map(Ok).collect()),
                ..Self::default()
            }
        }
    }

    #[async_trait]
    impl ContentGenerator for MockContentGenerator {
        async fn generate(
            &self,
            model: &str,
            request: &GenerateContentRequest,
        ) -> ClientResult<GenerateContentResponse> {
            self.models.lock().unwrap().push(model.to_string());
            self.requests.lock().unwrap().push(request.clone());

            let mut responses = self.responses.lock().unwrap();
            responses.pop_front().unwrap_or_else(|| Err("no mock responses left".into()))
        }
    }

    #[tokio::test]
    async fn mock_records_requests_and_models() {
        let response = GenerateContentResponse {
            candidates: vec![],
            usage_metadata: None,
            grounding_metadata: None,
        };
        let mock = MockContentGenerator::new(vec![response.clone()]);

        let request = GenerateContentRequest::new("hi");
        let result = mock.generate("model", &request).await.unwrap();
        assert!(result.candidates.is_empty());

        let requests = mock.requests.lock().unwrap();
        let models = mock.models.lock().unwrap();
        assert_eq!(requests.len(), 1);
        assert_eq!(models.len(), 1);
        assert_eq!(models[0], "model");
        assert_eq!(requests[0].contents[0].role, "user");
    }
}
