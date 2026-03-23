use serde_json::Value;
use std::env;
use threatflux_vertex_rust_sdk::models::Model;
use url::Url;

pub const SUPPORTED_CLAUDE_WEB_SEARCH_PREFIXES: &[&str] = &[
    "claude-sonnet-4-5",
    "claude-sonnet-4",
    "claude-3-7-sonnet",
    "claude-3-5-sonnet",
    "claude-haiku-4-5",
    "claude-3-5-haiku",
    "claude-opus-4-1",
    "claude-opus-4",
];

pub fn claude_model_supports_web_search(model: &str) -> bool {
    let normalized = model.to_ascii_lowercase();
    let base = normalized.split('@').next().unwrap_or(normalized.as_str());

    SUPPORTED_CLAUDE_WEB_SEARCH_PREFIXES.iter().any(|prefix| base.starts_with(prefix))
}

pub fn env_var(name: &str) -> Option<String> {
    env::var(name).ok().map(|value| value.trim().to_string()).filter(|v| !v.is_empty())
}

pub fn model_env_key(raw: &str) -> String {
    raw.chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' => ch.to_ascii_uppercase(),
            _ => '_',
        })
        .collect::<String>()
}

pub fn resolve_model_alias(input: &str) -> String {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return trimmed.to_string();
    }

    if trimmed.contains('@') || trimmed.starts_with("publishers/") {
        return trimmed.to_string();
    }

    let upper_key = model_env_key(trimmed);
    let candidate_env = [
        format!("VERTEX_MODEL_{upper_key}"),
        format!("VERTEX_ANTHROPIC_MODEL_{upper_key}"),
        format!("VERTEX_MODEL_ANTHROPIC_{upper_key}"),
    ];

    for key in candidate_env {
        if let Some(value) = env_var(&key) {
            if !value.is_empty() {
                return value;
            }
        }
    }

    let mut normalized = trimmed.to_ascii_lowercase();
    if let Some(stripped) = normalized.strip_prefix("claude-") {
        normalized = stripped.to_string();
    }
    normalized = normalized.replace('.', "-");

    match normalized.as_str() {
        "sonnet-4-5" | "sonnet-45" => "claude-sonnet-4-5".to_string(),
        "haiku-4-5" | "haiku-45" => "claude-haiku-4-5".to_string(),
        "opus-4-1" | "opus-41" => "claude-opus-4-1".to_string(),
        _ => trimmed.to_string(),
    }
}

pub fn model_display_name(model: &Model) -> String {
    model.display_name.clone().unwrap_or_else(|| model.short_name().to_string())
}

pub fn model_description(model: &Model) -> &str {
    model.description.as_deref().unwrap_or("No description available")
}

pub fn host_from_url_str(url: &str) -> Option<String> {
    Url::parse(url).ok().and_then(|parsed| {
        parsed.host_str().map(|host| host.trim_start_matches("www.").to_string())
    })
}

pub fn extract_query_from_value(value: &Value) -> Option<String> {
    if let Some(obj) = value.as_object() {
        if let Some(query) = obj.get("query").and_then(|v| v.as_str()) {
            let trimmed = query.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }

    if let Some(query) = value.as_str() {
        let trimmed = query.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};

    fn env_lock() -> std::sync::MutexGuard<'static, ()> {
        static ENV_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();
        ENV_MUTEX.get_or_init(|| Mutex::new(())).lock().expect("env mutex poisoned")
    }

    #[test]
    fn resolves_claude_aliases() {
        assert!(claude_model_supports_web_search("claude-sonnet-4-5"));
        assert!(claude_model_supports_web_search("claude-3-5-haiku@20240229"));
        assert!(!claude_model_supports_web_search("gemini-3-pro-preview"));
    }

    #[test]
    fn resolves_env_overrides_and_aliases() {
        let _guard = env_lock();
        let key = "VERTEX_MODEL_SONNET_4_5";
        env::set_var(key, "publishers/test/models/override");
        assert_eq!(resolve_model_alias("sonnet-4.5"), "publishers/test/models/override");
        env::remove_var(key);
        assert_eq!(resolve_model_alias("haiku-4.5"), "claude-haiku-4-5");
        assert_eq!(resolve_model_alias(""), "");
        let explicit = "publishers/acme/models/custom";
        assert_eq!(resolve_model_alias(explicit), explicit);
    }

    #[test]
    fn extracts_query_from_json_values() {
        let value = serde_json::json!({"query": "  hello "});
        assert_eq!(extract_query_from_value(&value), Some("hello".to_string()));

        let str_value = serde_json::json!(" world ");
        assert_eq!(extract_query_from_value(&str_value), Some("world".to_string()));

        let none_value = serde_json::json!({"data": 1});
        assert_eq!(extract_query_from_value(&none_value), None);
    }

    #[test]
    fn parses_host_without_www() {
        assert_eq!(
            host_from_url_str("https://www.example.com/path"),
            Some("example.com".to_string())
        );
        assert_eq!(
            host_from_url_str("https://sub.domain.test"),
            Some("sub.domain.test".to_string())
        );
        assert!(host_from_url_str("not a url").is_none());
    }
}
