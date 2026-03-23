use std::time::Duration;

use threatflux_vertex_rust_sdk::cache::CachedContent;

pub const MODEL: &str = "gemini-2.5-flash";
pub const DEFAULT_TTL_HOURS: u64 = 2;
pub const REQUEST_PAUSE: Duration = Duration::from_millis(500);
pub const RUST_GUIDE: &str = include_str!("data/rust_guide.md");

const SYSTEM_PROMPT: &str = "You are a helpful Rust programming expert. Use the cached document to answer questions accurately and provide relevant examples from the guide.";
const UNCACHED_PROMPT: &str = "You are a helpful Rust programming expert. Answer the question based on general knowledge about Rust programming.";

const QUESTIONS: [&str; 4] = [
    "What are the three ownership rules in Rust?",
    "Explain the difference between recoverable and unrecoverable errors in Rust.",
    "What are the scalar data types available in Rust?",
    "How does Rust handle concurrency and prevent data races?",
];

#[must_use]
pub fn cached_document() -> CachedContent {
    CachedContent::from_text(RUST_GUIDE)
        .with_display_name("Rust Programming Guide")
        .with_system_text(SYSTEM_PROMPT)
        .with_ttl_hours(DEFAULT_TTL_HOURS)
}

#[must_use]
pub const fn uncached_prompt() -> &'static str {
    UNCACHED_PROMPT
}

#[must_use]
pub fn questions() -> Vec<String> {
    QUESTIONS.iter().map(|question| (*question).to_string()).collect()
}
