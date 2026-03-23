use anyhow::Result;

mod context;
mod formatting;
mod input;
mod ops;
mod render;

#[cfg(test)]
mod tests;

use context::CacheContext;

pub async fn cache_create(
    text: Option<&str>,
    file: Option<&str>,
    name: Option<&str>,
    ttl: u64,
    system: Option<&str>,
) -> Result<()> {
    let context = CacheContext::new().await?;
    let build = input::build_cached_content(text, file, name, ttl, system)?;

    render::print_create_intro(&build);
    let created = ops::create(&context, build.cached_content).await?;
    render::print_create_success(&created);

    Ok(())
}

pub async fn cache_list(page_size: Option<i32>) -> Result<()> {
    let context = CacheContext::new().await?;

    render::print_list_intro();
    let response = ops::list(&context, page_size).await?;
    render::print_list(&response);

    Ok(())
}

pub async fn cache_get(cache_id: &str) -> Result<()> {
    let context = CacheContext::new().await?;

    render::print_get_intro(cache_id);
    let cache = ops::get(&context, cache_id).await?;
    render::print_cache_details(&cache);

    Ok(())
}

pub async fn cache_delete(cache_id: &str) -> Result<()> {
    let context = CacheContext::new().await?;

    render::print_delete_intro(cache_id);
    ops::delete(&context, cache_id).await?;
    render::print_delete_success();

    Ok(())
}

pub async fn cache_update(cache_id: &str, ttl: u64) -> Result<()> {
    let context = CacheContext::new().await?;

    render::print_update_intro(cache_id, ttl);
    let cache = ops::update_ttl(&context, cache_id, ttl).await?;
    render::print_update_success(&cache);

    Ok(())
}
