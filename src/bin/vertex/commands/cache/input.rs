use anyhow::{anyhow, Context, Result};
use threatflux_vertex_rust_sdk::cache::CachedContent;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContentSource {
    File(String),
    Text,
}

#[derive(Debug, Clone)]
pub struct CacheInputBuild {
    pub cached_content: CachedContent,
    pub source: ContentSource,
    pub ttl_seconds: u64,
    pub system_preview: Option<String>,
}

pub fn build_cached_content(
    text: Option<&str>,
    file: Option<&str>,
    name: Option<&str>,
    ttl: u64,
    system: Option<&str>,
) -> Result<CacheInputBuild> {
    let (mut cached_content, source) = if let Some(file_path) = file {
        let content = CachedContent::from_file(file_path)
            .with_context(|| format!("Failed to read cache content from file: {file_path}"))?;
        (content, ContentSource::File(file_path.to_string()))
    } else if let Some(content_text) = text {
        (CachedContent::from_text(content_text), ContentSource::Text)
    } else {
        return Err(anyhow!("Must provide either --text or --file"));
    };

    if let Some(display_name) = name {
        cached_content = cached_content.with_display_name(display_name);
    }

    cached_content = cached_content.with_ttl_seconds(ttl);

    if let Some(system_instruction) = system {
        cached_content = cached_content.with_system_text(system_instruction);
    }

    Ok(CacheInputBuild {
        cached_content,
        source,
        ttl_seconds: ttl,
        system_preview: system.map(str::to_string),
    })
}
