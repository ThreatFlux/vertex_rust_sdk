use anyhow::Result;
use async_trait::async_trait;
use threatflux_vertex_rust_sdk::{
    client::VertexClient,
    models::{GenerateContentRequest, GenerateContentResponse},
};

#[async_trait]
pub trait ContentGenerator: Send + Sync {
    async fn generate(
        &self,
        model: &str,
        request: &GenerateContentRequest,
    ) -> Result<GenerateContentResponse>;
}

pub struct VertexContentGenerator {
    client: VertexClient,
}

impl VertexContentGenerator {
    pub const fn new(client: VertexClient) -> Self {
        Self { client }
    }
}

#[async_trait]
impl ContentGenerator for VertexContentGenerator {
    async fn generate(
        &self,
        model: &str,
        request: &GenerateContentRequest,
    ) -> Result<GenerateContentResponse> {
        self.client.generate_content(model, request).await.map_err(Into::into)
    }
}
