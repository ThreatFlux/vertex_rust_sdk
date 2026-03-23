use async_trait::async_trait;
use threatflux_vertex_rust_sdk::{
    cache::{CachedContent, ListCachedContentsResponse, UpdateCachedContentRequest},
    client::VertexClient,
    config::Config,
    error::{Result, VertexError},
    models::{GenerateContentRequest, GenerateContentResponse},
};

pub struct VertexContextCacheClient {
    inner: VertexClient,
}

impl VertexContextCacheClient {
    pub async fn from_env() -> Result<Self> {
        let config =
            Config::from_env().map_err(|error| VertexError::configuration(error.to_string()))?;
        let inner = VertexClient::new(config).await?;
        Ok(Self::new(inner))
    }

    #[must_use]
    pub const fn new(inner: VertexClient) -> Self {
        Self { inner }
    }
}

#[async_trait]
pub trait ContextCacheClient: Send + Sync {
    async fn create_cache(&self, content: CachedContent) -> Result<CachedContent>;
    async fn get_cache(&self, cache_id: &str) -> Result<CachedContent>;
    async fn list_caches(&self, page_size: Option<i32>) -> Result<ListCachedContentsResponse>;
    async fn update_cache_ttl(
        &self,
        cache_id: &str,
        update_request: UpdateCachedContentRequest,
    ) -> Result<CachedContent>;
    async fn delete_cache(&self, cache_id: &str) -> Result<()>;
    async fn generate_content(
        &self,
        model: &str,
        request: &GenerateContentRequest,
    ) -> Result<GenerateContentResponse>;
}

#[async_trait]
impl ContextCacheClient for VertexContextCacheClient {
    async fn create_cache(&self, content: CachedContent) -> Result<CachedContent> {
        self.inner.cache().create_cache(content).await
    }

    async fn get_cache(&self, cache_id: &str) -> Result<CachedContent> {
        self.inner.cache().get_cache(cache_id).await
    }

    async fn list_caches(&self, page_size: Option<i32>) -> Result<ListCachedContentsResponse> {
        self.inner.cache().list_caches(page_size, None).await
    }

    async fn update_cache_ttl(
        &self,
        cache_id: &str,
        update_request: UpdateCachedContentRequest,
    ) -> Result<CachedContent> {
        self.inner.cache().update_cache_ttl(cache_id, update_request).await
    }

    async fn delete_cache(&self, cache_id: &str) -> Result<()> {
        self.inner.cache().delete_cache(cache_id).await
    }

    async fn generate_content(
        &self,
        model: &str,
        request: &GenerateContentRequest,
    ) -> Result<GenerateContentResponse> {
        self.inner.generate_content(model, request).await
    }
}
