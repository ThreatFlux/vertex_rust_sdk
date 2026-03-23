use chrono::{DateTime, Utc};

pub fn format_timestamp(timestamp: &DateTime<Utc>) -> String {
    timestamp.format("%Y-%m-%d %H:%M:%S UTC").to_string()
}

pub fn preview_text(text: &str, limit: usize) -> String {
    let mut result = String::new();
    let mut iter = text.chars();

    for ch in iter.by_ref().take(limit) {
        result.push(ch);
    }

    if iter.next().is_some() {
        result.push_str("...");
    }

    result
}

pub fn format_remaining_ttl(remaining_seconds: i64) -> (String, String) {
    #[allow(clippy::cast_precision_loss)]
    let hours = remaining_seconds as f64 / 3600.0;
    (remaining_seconds.to_string(), format!("{hours:.2}"))
}
