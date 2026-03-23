use colored::Colorize;
use threatflux_vertex_rust_sdk::{
    cache::{CachedContent, ListCachedContentsResponse},
    types::Part,
};

use super::formatting::{format_remaining_ttl, format_timestamp, preview_text};
use super::input::{CacheInputBuild, ContentSource};

fn print_header(title: &str) {
    println!("{}", title.bold().cyan());
    println!("{}", "═".repeat(60).cyan());
}

pub fn print_create_intro(build: &CacheInputBuild) {
    print_header("Creating Cache...");

    match &build.source {
        ContentSource::File(path) => {
            println!("Reading content from file: {}", path.yellow());
        }
        ContentSource::Text => println!("Using provided text content"),
    }

    println!("TTL: {} seconds", build.ttl_seconds.to_string().yellow());

    if let Some(display_name) = &build.cached_content.display_name {
        println!("Display Name: {}", display_name.green());
    }

    if let Some(system_instruction) = &build.system_preview {
        println!("System Instruction: {}", system_instruction.italic().blue());
    }

    println!();
}

pub fn print_create_success(created_cache: &CachedContent) {
    println!("{} Cache created successfully!", "✅".green());
    println!(
        "Cache ID: {}",
        created_cache.cache_id().unwrap_or_else(|| "N/A".to_string()).bold().green()
    );

    if let Some(name) = &created_cache.name {
        println!("Full Name: {}", name.blue());
    }
    if let Some(display_name) = &created_cache.display_name {
        println!("Display Name: {}", display_name.green());
    }
    if let Some(create_time) = &created_cache.create_time {
        println!("Created: {}", format_timestamp(create_time).yellow());
    }
    if let Some(expire_time) = &created_cache.expire_time {
        println!("Expires: {}", format_timestamp(expire_time).red());
    }
    if let Some(usage) = &created_cache.usage_metadata {
        println!("Token Count: {}", usage.total_token_count.to_string().blue());
    }
}

pub fn print_list_intro() {
    print_header("Listing Caches...");
}

pub fn print_list(response: &ListCachedContentsResponse) {
    if response.cached_contents.is_empty() {
        println!("No caches found.");
        return;
    }

    println!("Found {} cache(s):\n", response.cached_contents.len());

    for (i, cache) in response.cached_contents.iter().enumerate() {
        println!(
            "{} {}",
            format!("{}.", i + 1).bold().blue(),
            cache.cache_id().unwrap_or_else(|| "Unknown ID".to_string()).bold().green()
        );

        if let Some(display_name) = &cache.display_name {
            println!("  Display Name: {}", display_name.green());
        }

        if let Some(create_time) = &cache.create_time {
            println!("  Created: {}", format_timestamp(create_time).yellow());
        }

        print_expiration(cache, "  ");

        if let Some(usage) = &cache.usage_metadata {
            println!("  Token Count: {}", usage.total_token_count.to_string().blue());
        }

        println!("  Contents: {} item(s)", cache.contents.len());
        if cache.system_instruction.is_some() {
            println!("  {} System instruction included", "📋".blue());
        }
        if cache.tools.is_some() {
            println!("  {} Tools included", "🔧".blue());
        }

        println!();
    }

    if let Some(next_page_token) = &response.next_page_token {
        println!("Next page token: {}", next_page_token.italic());
    }
}

pub fn print_get_intro(cache_id: &str) {
    print_header("Getting Cache Details...");
    println!("Cache ID: {}\n", cache_id.yellow());
}

pub fn print_cache_details(cache: &CachedContent) {
    println!("{} Cache found!", "✅".green());

    if let Some(name) = &cache.name {
        println!("Full Name: {}", name.blue());
    }
    if let Some(display_name) = &cache.display_name {
        println!("Display Name: {}", display_name.bold().green());
    }
    if let Some(create_time) = &cache.create_time {
        println!("Created: {}", format_timestamp(create_time).yellow());
    }
    if let Some(update_time) = &cache.update_time {
        println!("Updated: {}", format_timestamp(update_time).yellow());
    }

    print_expiration(cache, "");

    if let Some(ttl) = &cache.ttl {
        println!("TTL Setting: {}", ttl.blue());
    }

    if let Some(usage) = &cache.usage_metadata {
        println!("Token Count: {}", usage.total_token_count.to_string().blue());
    }

    println!("\n{}", "Content Details:".bold().yellow());
    println!("Contents: {} item(s)", cache.contents.len());

    for (i, content) in cache.contents.iter().enumerate() {
        println!(
            "  Content {}: Role = {}, Parts = {}",
            i + 1,
            content.role.cyan(),
            content.parts.len()
        );

        for (j, part) in content.parts.iter().enumerate() {
            if let Part::Text { text } = part {
                let preview = preview_text(text, 100);
                println!("    Part {}: \"{}\"", j + 1, preview.italic());
            }
        }
    }

    if let Some(system_instruction) = &cache.system_instruction {
        println!("\n{} System instruction included:", "📋".blue());
        for part in &system_instruction.parts {
            if let Part::Text { text } = part {
                let preview = preview_text(text, 200);
                println!("  \"{}\"", preview.italic().blue());
            }
        }
    }

    if let Some(tools) = &cache.tools {
        println!("\n{} {} tool(s) included", "🔧".blue(), tools.len());
    }
}

pub fn print_delete_intro(cache_id: &str) {
    print_header("Deleting Cache...");
    println!("Cache ID: {}\n", cache_id.yellow());
}

pub fn print_delete_success() {
    println!("{} Cache deleted successfully!", "✅".green());
}

pub fn print_update_intro(cache_id: &str, ttl: u64) {
    print_header("Updating Cache TTL...");
    println!("Cache ID: {}", cache_id.yellow());
    println!("New TTL: {} seconds\n", ttl.to_string().green());
}

pub fn print_update_success(cache: &CachedContent) {
    println!("{} Cache TTL updated successfully!", "✅".green());

    if let Some(expire_time) = &cache.expire_time {
        println!("New expiration: {}", format_timestamp(expire_time).blue());
    }

    if let Some(remaining) = cache.remaining_ttl_seconds() {
        let (seconds, hours) = format_remaining_ttl(remaining);
        println!("Remaining TTL: {} seconds ({} hours)", seconds.cyan(), hours.cyan());
    }
}

fn print_expiration(cache: &CachedContent, indent: &str) {
    if let Some(expire_time) = &cache.expire_time {
        let is_expired = cache.is_expired();
        let expire_str = format_timestamp(expire_time);
        if is_expired {
            println!("{indent}Expires: {} {}", expire_str.red(), "(EXPIRED)".red().bold());
        } else {
            println!("{indent}Expires: {}", expire_str.blue());
            if let Some(remaining) = cache.remaining_ttl_seconds() {
                let (seconds, hours) = format_remaining_ttl(remaining);
                println!(
                    "{indent}Remaining TTL: {} seconds ({} hours)",
                    seconds.cyan(),
                    hours.cyan()
                );
            }
        }
    }
}
