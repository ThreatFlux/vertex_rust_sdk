use anyhow::Result;
use threatflux_vertex_rust_sdk::cache::{
    CachedContent, ListCachedContentsResponse, UpdateCachedContentRequest,
};

use super::context::CacheContext;

pub async fn create(
    context: &CacheContext,
    cached_content: CachedContent,
) -> Result<CachedContent> {
    let api = context.api();
    api.create_cache(cached_content).await.map_err(Into::into)
}

pub async fn list(
    context: &CacheContext,
    page_size: Option<i32>,
) -> Result<ListCachedContentsResponse> {
    let api = context.api();
    api.list_caches(page_size, None).await.map_err(Into::into)
}

pub async fn get(context: &CacheContext, cache_id: &str) -> Result<CachedContent> {
    let api = context.api();
    api.get_cache(cache_id).await.map_err(Into::into)
}

pub async fn delete(context: &CacheContext, cache_id: &str) -> Result<()> {
    let api = context.api();
    api.delete_cache(cache_id).await.map_err(Into::into)
}

pub async fn update_ttl(context: &CacheContext, cache_id: &str, ttl: u64) -> Result<CachedContent> {
    let api = context.api();
    let request = UpdateCachedContentRequest::with_ttl_seconds(ttl);
    api.update_cache_ttl(cache_id, request).await.map_err(Into::into)
}
