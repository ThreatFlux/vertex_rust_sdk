use anyhow::Result;
use threatflux_vertex_rust_sdk::{cache::CacheApi, client::VertexClient, config::Config};

pub struct CacheContext {
    client: VertexClient,
}

impl CacheContext {
    pub async fn new() -> Result<Self> {
        let config = Config::from_env()?;
        let client = VertexClient::new(config).await?;
        Ok(Self { client })
    }

    pub fn api(&self) -> CacheApi<'_> {
        self.client.cache()
    }
}
