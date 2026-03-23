mod client;
mod content;
mod demo;
mod metrics;
mod output;

use anyhow::Result;
use client::VertexContextCacheClient;
use content::{cached_document, RUST_GUIDE};
use demo::ContextCachingDemo;
use metrics::compare_runs;
use output::{
    print_banner, print_best_practices, print_cache_created, print_cache_creation_start,
    print_cache_details, print_cache_list, print_cleanup, print_comparison, print_run_summary,
    print_updated_ttl,
};

#[tokio::main]
async fn main() -> Result<()> {
    print_banner();

    let cached_content = cached_document();
    print_cache_creation_start(RUST_GUIDE.len());

    let client = VertexContextCacheClient::from_env().await?;
    let demo = ContextCachingDemo::new(client, content::MODEL)
        .with_questions(content::questions())
        .with_pause_between(content::REQUEST_PAUSE);
    let created_cache = demo.create_cache(cached_content).await?;
    let cache_id = created_cache.cache_id().unwrap_or_else(|| "unknown".to_string());
    print_cache_created(&cache_id, &created_cache);

    println!("\n🔍 Step 2: Comparing performance with and without cache");
    println!("--------------------------------------------------------");
    let without_cache = demo.run_questions(None).await?;
    print_run_summary("WITHOUT cache", &without_cache);

    let with_cache = demo.run_questions(Some(&cache_id)).await?;
    print_run_summary("WITH cache", &with_cache);

    let savings = compare_runs(&without_cache, &with_cache);
    print_comparison(savings, &without_cache, &with_cache);

    let cache_details = demo.cache_details(&cache_id).await?;
    print_cache_details(&cache_details);

    let cache_list = demo.list_caches(Some(10)).await?;
    print_cache_list(&cache_list);

    let updated_cache = demo.update_ttl_hours(&cache_id, 4).await?;
    print_updated_ttl(&updated_cache);

    demo.cleanup(&cache_id).await?;
    print_cleanup();
    print_best_practices();

    Ok(())
}
