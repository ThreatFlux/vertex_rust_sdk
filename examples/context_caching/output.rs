use threatflux_vertex_rust_sdk::cache::{CachedContent, ListCachedContentsResponse};

use crate::metrics::{RunSummary, Savings};

pub fn print_banner() {
    println!("🚀 Vertex AI Context Caching Example");
    println!("=====================================\n");
}

pub fn print_cache_creation_start(document_length: usize) {
    println!("📚 Step 1: Creating cache with large document context");
    println!("-----------------------------------------------------");
    println!("📝 Creating cache with {document_length} characters of content...");
}

pub fn print_cache_created(cache_id: &str, cached_content: &CachedContent) {
    println!("✅ Cache created successfully!");
    println!("   Cache ID: {cache_id}");
    if let Some(token_count) =
        cached_content.usage_metadata.as_ref().map(|usage| usage.total_token_count)
    {
        println!("   Cached tokens: {token_count}");
    }
    if let Some(ttl) = &cached_content.ttl {
        println!("   TTL: {ttl}");
    }
    println!();
}

pub fn print_run_summary(label: &str, summary: &RunSummary) {
    println!("\n🔍 Testing {label}:");
    let total_time = summary.total_time();

    for (index, outcome) in summary.outcomes.iter().enumerate() {
        println!("   Question {}: {}", index + 1, outcome.question);
        if let Some(preview) = &outcome.preview {
            println!("   Response: {preview}");
        }
        println!("   Time: {:?}", outcome.elapsed);
    }

    println!("   Total time {label}: {total_time:?}");
}

pub fn print_comparison(
    savings: Option<Savings>,
    without_cache: &RunSummary,
    with_cache: &RunSummary,
) {
    println!("\n📊 Performance Comparison");
    println!("-------------------------");
    println!("Without cache: {:?}", without_cache.total_time());
    println!("With cache:    {:?}", with_cache.total_time());

    if let Some(savings) = savings {
        println!("🎉 Cache saved: {:?} ({:.1}% faster)", savings.duration, savings.percentage);
    } else {
        println!("🤔 Cache did not show a measurable speedup in this run");
        println!("   (Network variability or model latency can affect these numbers)");
    }
}

pub fn print_cache_details(cache: &CachedContent) {
    println!("\n🛠️  Step 3: Cache management operations");
    println!("----------------------------------------");
    println!("📋 Cache details:");
    println!("   Display Name: {:?}", cache.display_name);
    println!("   Contents: {} items", cache.contents.len());

    if let Some(expire_time) = cache.expire_time {
        println!("   Expires: {}", expire_time.format("%Y-%m-%d %H:%M:%S UTC"));
        if let Some(remaining) = cache.remaining_ttl_seconds() {
            #[allow(clippy::cast_precision_loss)]
            let remaining_hours = remaining as f64 / 3600.0;
            println!("   Remaining TTL: {remaining} seconds ({remaining_hours:.1} hours)");
        }
    }
}

pub fn print_cache_list(list: &ListCachedContentsResponse) {
    println!("\n📑 All caches:");
    for (i, cache) in list.cached_contents.iter().enumerate() {
        let cache_name = cache.cache_id().unwrap_or_else(|| "unknown".to_string());
        let display_name = cache.display_name.as_deref().unwrap_or("No name");
        println!("   {}. {} - {}", i + 1, cache_name, display_name);
    }
}

pub fn print_updated_ttl(cache: &CachedContent) {
    println!("\n⏰ Updated cache TTL");
    if let Some(expire_time) = cache.expire_time {
        println!("✅ New expiration time: {}", expire_time.format("%Y-%m-%d %H:%M:%S UTC"));
    } else if let Some(ttl) = &cache.ttl {
        println!("✅ TTL updated to {ttl}");
    } else {
        println!("✅ Cache TTL updated");
    }
}

pub fn print_cleanup() {
    println!("\n🧹 Step 4: Cleanup");
    println!("------------------");
    println!("✅ Cache deleted successfully!");
}

pub fn print_best_practices() {
    println!("\n🎉 Context Caching Demo Complete!");
    println!("==================================");
    println!("Key Benefits Demonstrated:");
    println!("• Reduced latency for subsequent requests with same context");
    println!("• Cost savings by reusing cached tokens");
    println!("• Easy cache management with TTL and CRUD operations");
    println!("• Seamless integration with existing generation requests");
    println!("\nBest Practices:");
    println!("• Cache large, reusable contexts (documents, system instructions)");
    println!("• Set appropriate TTL based on content freshness requirements");
    println!("• Monitor cache usage and clean up expired caches");
    println!("• Use descriptive names for easier cache management");
}
