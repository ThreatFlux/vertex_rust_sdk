//! Static model metadata used by higher-level clients.
//!
//! The Vertex AI discovery APIs expose field-level information such as
//! `inputTokenLimit` and `outputTokenLimit`, but these responses are not always
//! available (or consistently populated) in every environment. The provider
//! crate consumes this module to surface reasonable defaults for UI display
//! without performing live metadata lookups on every request.

/// High-level metadata describing a supported model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModelInfo {
    /// Canonical Vertex identifier (`publishers/{publisher}/models/{model}`).
    pub canonical_id: &'static str,
    /// Friendly name displayed in UI surfaces.
    pub display_name: &'static str,
    /// Maximum serialized request body size (bytes) supported by the model.
    pub max_request_bytes: Option<u64>,
    /// Maximum total tokens (prompt + completion) supported by the model.
    pub context_window_tokens: Option<u64>,
    /// Maximum output tokens recommended for a single response.
    pub max_output_tokens: Option<u32>,
}

impl ModelInfo {
    const fn new(
        canonical_id: &'static str,
        display_name: &'static str,
        max_request_bytes: Option<u64>,
        context_window_tokens: Option<u64>,
        max_output_tokens: Option<u32>,
    ) -> Self {
        Self {
            canonical_id,
            display_name,
            max_request_bytes,
            context_window_tokens,
            max_output_tokens,
        }
    }
}

struct ModelInfoEntry {
    aliases: &'static [&'static str],
    info: ModelInfo,
}

const CLAUDE_SONNET_ALIASES: &[&str] = &[
    "claude-sonnet-4-5",
    "sonnet-4-5",
    "sonnet-45",
    "publishers/anthropic/models/claude-sonnet-4-5",
];

const CLAUDE_HAIKU_ALIASES: &[&str] = &[
    "claude-haiku-4-5",
    "haiku-4-5",
    "claude-haiku-45",
    "haiku-45",
    "publishers/anthropic/models/claude-haiku-4-5",
    "publishers/anthropic/models/claude-haiku-4-5@20251001",
];

const CLAUDE_OPUS_ALIASES: &[&str] =
    &["claude-opus-4-5", "opus-4-5", "opus-45", "publishers/anthropic/models/claude-opus-4-5"];

const GEMINI_FLASH_ALIASES: &[&str] =
    &["gemini-2-5-flash", "gemini-2.5-flash", "publishers/google/models/gemini-2.5-flash"];

const GEMINI_PRO_ALIASES: &[&str] =
    &["gemini-2-5-pro", "gemini-2.5-pro", "publishers/google/models/gemini-2.5-pro"];

const GEMINI_3_PRO_PREVIEW_ALIASES: &[&str] =
    &["gemini-3-pro-preview", "publishers/google/models/gemini-3-pro-preview"];

const MODEL_INFO_ENTRIES: &[ModelInfoEntry] = &[
    ModelInfoEntry {
        aliases: CLAUDE_SONNET_ALIASES,
        info: ModelInfo::new(
            "publishers/anthropic/models/claude-sonnet-4-5",
            "Claude 4.5 Sonnet",
            // Vertex rejects requests above roughly 5.7MB even though the model
            // advertises a 1M token window. Keep the limit slightly above the
            // observed ceiling so we can pre-validate and truncate tool results
            // before sending to Vertex.
            Some(6_000_000),
            Some(1_000_000),
            Some(64_000),
        ),
    },
    ModelInfoEntry {
        aliases: CLAUDE_HAIKU_ALIASES,
        info: ModelInfo::new(
            "publishers/anthropic/models/claude-haiku-4-5",
            "Claude 4.5 Haiku",
            None,
            Some(200_000),
            Some(4_096),
        ),
    },
    ModelInfoEntry {
        aliases: CLAUDE_OPUS_ALIASES,
        info: ModelInfo::new(
            "publishers/anthropic/models/claude-opus-4-5",
            "Claude 4.5 Opus",
            None,
            Some(200_000),
            Some(64_000),
        ),
    },
    ModelInfoEntry {
        aliases: GEMINI_FLASH_ALIASES,
        info: ModelInfo::new(
            "publishers/google/models/gemini-2.5-flash",
            "Gemini 2.5 Flash",
            None,
            Some(1_000_000),
            Some(8_192),
        ),
    },
    ModelInfoEntry {
        aliases: GEMINI_PRO_ALIASES,
        info: ModelInfo::new(
            "publishers/google/models/gemini-2.5-pro",
            "Gemini 2.5 Pro",
            None,
            Some(2_000_000),
            Some(8_192),
        ),
    },
    ModelInfoEntry {
        aliases: GEMINI_3_PRO_PREVIEW_ALIASES,
        info: ModelInfo::new(
            "publishers/google/models/gemini-3-pro-preview",
            "Gemini 3 Pro Preview",
            None,
            Some(1_000_000),
            Some(64_000),
        ),
    },
];

/// Look up metadata for the supplied model identifier.
///
/// The matcher is tolerant of publisher prefixes (`publishers/...`), shortened
/// aliases (for example `sonnet-4-5`), and versioned identifiers
/// (`model@20250101`). Returns `None` when the model is unknown.
#[must_use]
pub fn get_model_info(model_name: &str) -> Option<ModelInfo> {
    if model_name.trim().is_empty() {
        return None;
    }

    let candidates = candidate_identifiers(model_name);
    if candidates.is_empty() {
        return None;
    }

    for candidate in candidates {
        for entry in MODEL_INFO_ENTRIES {
            if entry.aliases.iter().any(|alias| alias.eq_ignore_ascii_case(&candidate)) {
                return Some(entry.info);
            }
        }
    }

    None
}

/// Maximum context window (in tokens) across all known models.
#[must_use]
pub fn max_context_window_tokens() -> Option<u64> {
    MODEL_INFO_ENTRIES.iter().filter_map(|entry| entry.info.context_window_tokens).max()
}

/// Maximum serialized request size (in bytes) across all known models.
#[must_use]
pub fn max_request_bytes() -> Option<u64> {
    MODEL_INFO_ENTRIES.iter().filter_map(|entry| entry.info.max_request_bytes).max()
}

fn candidate_identifiers(model_name: &str) -> Vec<String> {
    let trimmed = model_name.trim().to_ascii_lowercase();
    if trimmed.is_empty() {
        return Vec::new();
    }

    let mut identifiers = Vec::new();
    identifiers.push(strip_version(&trimmed));

    if let Some(idx) = trimmed.find("publishers/") {
        let remainder = &trimmed[idx..];
        identifiers.push(strip_version(remainder));
        if let Some(after_models) = remainder.rsplit("models/").next() {
            identifiers.push(strip_version(after_models));
        }
    } else if let Some(idx) = trimmed.rfind('/') {
        identifiers.push(strip_version(&trimmed[idx + 1..]));
    }

    if let Some(idx) = trimmed.rfind(':') {
        identifiers.push(strip_version(&trimmed[idx + 1..]));
    }

    identifiers.sort();
    identifiers.dedup();
    identifiers.into_iter().filter(|value| !value.is_empty()).collect()
}

fn strip_version(input: &str) -> String {
    input.split('@').next().unwrap_or_default().trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_known_aliases() {
        let info = get_model_info("claude-sonnet-4-5").unwrap();
        assert_eq!(info.canonical_id, "publishers/anthropic/models/claude-sonnet-4-5");
        assert_eq!(info.context_window_tokens, Some(1_000_000));
        assert_eq!(info.max_request_bytes, Some(6_000_000));

        let versioned = get_model_info(
            "projects/demo/locations/us/publishers/anthropic/models/claude-sonnet-4-5@20250101",
        )
        .unwrap();
        assert_eq!(versioned.canonical_id, info.canonical_id);

        let short = get_model_info("sonnet-45").unwrap();
        assert_eq!(short.canonical_id, info.canonical_id);
    }

    #[test]
    fn reports_request_byte_limits() {
        assert_eq!(
            max_request_bytes(),
            Some(6_000_000),
            "should surface largest known request cap"
        );
    }

    #[test]
    fn returns_none_for_unknown_models() {
        assert!(get_model_info("unknown-model").is_none());
        assert!(get_model_info("").is_none());
    }

    #[test]
    fn resolves_gemini_3_pro_preview_aliases() {
        let info = get_model_info("publishers/google/models/gemini-3-pro-preview").unwrap();
        assert_eq!(info.canonical_id, "publishers/google/models/gemini-3-pro-preview");
        assert_eq!(info.context_window_tokens, Some(1_000_000));

        let short = get_model_info("gemini-3-pro-preview").unwrap();
        assert_eq!(short.canonical_id, info.canonical_id);
    }

    #[test]
    fn resolves_claude_opus_45_aliases() {
        let info = get_model_info("claude-opus-4-5").unwrap();
        assert_eq!(info.canonical_id, "publishers/anthropic/models/claude-opus-4-5");
        assert_eq!(info.context_window_tokens, Some(200_000));
        assert_eq!(info.max_output_tokens, Some(64_000));

        let short = get_model_info("opus-45").unwrap();
        assert_eq!(short.canonical_id, info.canonical_id);

        let versioned = get_model_info("claude-opus-4-5@20251101").unwrap();
        assert_eq!(versioned.canonical_id, info.canonical_id);
    }
}
