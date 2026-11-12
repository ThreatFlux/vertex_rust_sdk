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

impl ModelInfoEntry {
    const fn claude_with_large_context(
        canonical_id: &'static str,
        display_name: &'static str,
        aliases: &'static [&'static str],
    ) -> Self {
        Self {
            aliases,
            info: ModelInfo::new(canonical_id, display_name, None, Some(1_000_000), Some(128_000)),
        }
    }
}

const CLAUDE_SONNET_ALIASES: &[&str] = &[
    "claude-sonnet-4-5",
    "claude-sonnet-45",
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

const CLAUDE_OPUS_ALIASES: &[&str] = &[
    "claude-opus-4-5",
    "claude-opus-45",
    "opus-4-5",
    "opus-45",
    "publishers/anthropic/models/claude-opus-4-5",
];

const CLAUDE_OPUS_46_ALIASES: &[&str] = &[
    "claude-opus-4-6",
    "claude-opus-46",
    "opus-4-6",
    "opus-46",
    "publishers/anthropic/models/claude-opus-4-6",
];

const CLAUDE_SONNET_46_ALIASES: &[&str] = &[
    "claude-sonnet-4-6",
    "claude-sonnet-46",
    "sonnet-4-6",
    "sonnet-46",
    "publishers/anthropic/models/claude-sonnet-4-6",
];

const CLAUDE_OPUS_47_ALIASES: &[&str] = &[
    "claude-opus-4-7",
    "claude-opus-47",
    "opus-4-7",
    "opus-47",
    "publishers/anthropic/models/claude-opus-4-7",
];

const CLAUDE_OPUS_48_ALIASES: &[&str] = &[
    "claude-opus-4-8",
    "claude-opus-48",
    "opus-4-8",
    "opus-48",
    "publishers/anthropic/models/claude-opus-4-8",
];

const CLAUDE_SONNET_5_ALIASES: &[&str] =
    &["claude-sonnet-5", "sonnet-5", "publishers/anthropic/models/claude-sonnet-5"];

const CLAUDE_FABLE_5_ALIASES: &[&str] =
    &["claude-fable-5", "fable-5", "publishers/anthropic/models/claude-fable-5"];

const CLAUDE_OPUS_5_ALIASES: &[&str] =
    &["claude-opus-5", "opus-5", "publishers/anthropic/models/claude-opus-5"];

const CLAUDE_OPUS_41_ALIASES: &[&str] = &[
    "claude-opus-4-1",
    "claude-opus-41",
    "opus-4-1",
    "opus-41",
    "publishers/anthropic/models/claude-opus-4-1",
    "publishers/anthropic/models/claude-opus-4-1@20250805",
];

const CLAUDE_SONNET_4_ALIASES: &[&str] = &[
    "claude-sonnet-4",
    "sonnet-4",
    "publishers/anthropic/models/claude-sonnet-4",
    "publishers/anthropic/models/claude-sonnet-4@20250514",
];

const CLAUDE_OPUS_4_ALIASES: &[&str] = &[
    "claude-opus-4",
    "opus-4",
    "publishers/anthropic/models/claude-opus-4",
    "publishers/anthropic/models/claude-opus-4@20250514",
];

pub(crate) struct GeminiModelEntry {
    pub(crate) info: ModelInfo,
    pub(crate) id: &'static str,
    pub(crate) methods: &'static [&'static str],
    default_location: Option<&'static str>,
    aliases: &'static [&'static str],
}

const GENERATE_AND_COUNT: &[&str] = &["generateContent", "streamGenerateContent", "countTokens"];
const GENERATE_COUNT_AND_INTERACTIONS: &[&str] =
    &["generateContent", "streamGenerateContent", "countTokens", "interactions"];
const GENERATE: &[&str] = &["generateContent", "streamGenerateContent"];
const INTERACTIONS_AND_COUNT: &[&str] = &["interactions", "countTokens"];
const LIVE: &[&str] = &["BidiGenerateContent"];
const GENERATE_AND_LIVE: &[&str] = &["generateContent", "BidiGenerateContent"];
const ONE_MILLION_TOKENS: u64 = 1_048_576;
const GENERATION_OUTPUT_LIMIT: u32 = 65_536;
// Gemini 4's v1 non-streaming responses are capped at a single segment.
const GEMINI_4_OUTPUT_LIMIT: u32 = 32_768;

macro_rules! gemini_model {
    ($id:literal, $name:literal, $context:expr, $output:expr, $methods:expr, $location:expr $(, $alias:literal)*) => {
        GeminiModelEntry {
            info: ModelInfo::new(
                concat!("publishers/google/models/", $id),
                $name,
                None,
                $context,
                $output,
            ),
            id: $id,
            methods: $methods,
            default_location: $location,
            aliases: &[$($alias),*],
        }
    };
}

// Published Vertex model IDs and token limits:
// https://docs.cloud.google.com/gemini-enterprise-agent-platform/models/google-models
pub(crate) const GEMINI_MODELS: &[GeminiModelEntry] = &[
    gemini_model!(
        "gemini-3.8-flash",
        "Gemini 3.8 Flash",
        Some(ONE_MILLION_TOKENS),
        Some(GENERATION_OUTPUT_LIMIT),
        GENERATE_COUNT_AND_INTERACTIONS,
        Some("global"),
        "gemini-3-8-flash"
    ),
    gemini_model!(
        "gemini-4-argon",
        "Gemini 4 Argon",
        Some(ONE_MILLION_TOKENS),
        Some(GEMINI_4_OUTPUT_LIMIT),
        GENERATE_COUNT_AND_INTERACTIONS,
        Some("global")
    ),
    gemini_model!(
        "gemini-4-carbon",
        "Gemini 4 Carbon",
        Some(ONE_MILLION_TOKENS),
        Some(GEMINI_4_OUTPUT_LIMIT),
        GENERATE_COUNT_AND_INTERACTIONS,
        Some("global")
    ),
    gemini_model!(
        "gemini-4-argon-cyber-permissive",
        "Gemini 4 Argon Cyber Permissive",
        Some(ONE_MILLION_TOKENS),
        Some(GEMINI_4_OUTPUT_LIMIT),
        GENERATE_COUNT_AND_INTERACTIONS,
        Some("global")
    ),
    gemini_model!(
        "gemini-4-carbon-cyber-permissive",
        "Gemini 4 Carbon Cyber Permissive",
        Some(ONE_MILLION_TOKENS),
        Some(GEMINI_4_OUTPUT_LIMIT),
        GENERATE_COUNT_AND_INTERACTIONS,
        Some("global")
    ),
    gemini_model!(
        "gemini-3.8-flash-cyber",
        "Gemini 3.8 Flash Cyber",
        Some(ONE_MILLION_TOKENS),
        Some(GENERATION_OUTPUT_LIMIT),
        GENERATE_COUNT_AND_INTERACTIONS,
        Some("global"),
        "gemini-3-8-flash-cyber"
    ),
    gemini_model!(
        "gemini-3.7-flash",
        "Gemini 3.7 Flash",
        Some(ONE_MILLION_TOKENS),
        Some(GENERATION_OUTPUT_LIMIT),
        GENERATE_COUNT_AND_INTERACTIONS,
        Some("global"),
        "gemini-3-7-flash"
    ),
    gemini_model!(
        "gemini-3.6-flash",
        "Gemini 3.6 Flash",
        Some(ONE_MILLION_TOKENS),
        Some(GENERATION_OUTPUT_LIMIT),
        GENERATE_COUNT_AND_INTERACTIONS,
        Some("global"),
        "gemini-3-6-flash"
    ),
    gemini_model!(
        "gemini-3.5-flash",
        "Gemini 3.5 Flash",
        Some(ONE_MILLION_TOKENS),
        Some(GENERATION_OUTPUT_LIMIT),
        GENERATE_COUNT_AND_INTERACTIONS,
        Some("global"),
        "gemini-3-5-flash"
    ),
    gemini_model!(
        "gemini-3.5-flash-lite",
        "Gemini 3.5 Flash-Lite",
        Some(ONE_MILLION_TOKENS),
        Some(GENERATION_OUTPUT_LIMIT),
        GENERATE_COUNT_AND_INTERACTIONS,
        Some("global"),
        "gemini-3-5-flash-lite"
    ),
    gemini_model!(
        "gemini-3.1-flash-lite",
        "Gemini 3.1 Flash Lite",
        Some(ONE_MILLION_TOKENS),
        Some(GENERATION_OUTPUT_LIMIT),
        GENERATE_COUNT_AND_INTERACTIONS,
        Some("global"),
        "gemini-3-1-flash-lite"
    ),
    gemini_model!(
        "gemini-3.1-pro-preview",
        "Gemini 3.1 Pro Preview",
        Some(ONE_MILLION_TOKENS),
        Some(GENERATION_OUTPUT_LIMIT),
        GENERATE_COUNT_AND_INTERACTIONS,
        Some("global"),
        "gemini-3-1-pro-preview"
    ),
    gemini_model!(
        "gemini-3.1-pro-preview-customtools",
        "Gemini 3.1 Pro Preview (Custom Tools)",
        Some(ONE_MILLION_TOKENS),
        Some(GENERATION_OUTPUT_LIMIT),
        GENERATE_AND_COUNT,
        Some("global"),
        "gemini-3-1-pro-preview-customtools"
    ),
    gemini_model!(
        "gemini-3-flash-preview",
        "Gemini 3 Flash Preview",
        Some(ONE_MILLION_TOKENS),
        Some(GENERATION_OUTPUT_LIMIT),
        GENERATE_AND_COUNT,
        Some("global")
    ),
    gemini_model!(
        "gemini-robotics-er-2-preview",
        "Gemini Robotics ER 2 Preview",
        Some(131_072),
        Some(GENERATION_OUTPUT_LIMIT),
        GENERATE_AND_COUNT,
        Some("global")
    ),
    gemini_model!(
        "gemini-nano-banana-2.1",
        "Gemini Nano Banana 2.1",
        Some(131_072),
        Some(32_768),
        GENERATE,
        Some("global"),
        "gemini-nano-banana-2-1"
    ),
    gemini_model!(
        "gemini-3-pro-image",
        "Gemini 3 Pro Image",
        Some(65_536),
        Some(32_768),
        GENERATE_AND_COUNT,
        Some("global")
    ),
    gemini_model!(
        "gemini-3.1-flash-image",
        "Gemini 3.1 Flash Image",
        Some(131_072),
        Some(32_768),
        GENERATE_AND_COUNT,
        Some("global"),
        "gemini-3-1-flash-image"
    ),
    gemini_model!(
        "gemini-3.1-flash-lite-image",
        "Gemini 3.1 Flash-Lite Image",
        Some(65_536),
        Some(4_096),
        GENERATE_AND_COUNT,
        Some("global"),
        "gemini-3-1-flash-lite-image"
    ),
    gemini_model!(
        "gemini-2.5-flash-image",
        "Gemini 2.5 Flash Image",
        Some(32_768),
        Some(32_768),
        GENERATE_AND_COUNT,
        Some("global"),
        "gemini-2-5-flash-image"
    ),
    gemini_model!(
        "gemini-3.8-flash-tts",
        "Gemini 3.8 Flash TTS",
        Some(8_192),
        Some(16_384),
        GENERATE,
        Some("global"),
        "gemini-3-8-flash-tts"
    ),
    gemini_model!(
        "gemini-3.8-flash-lite-tts",
        "Gemini 3.8 Flash-Lite TTS",
        Some(8_192),
        Some(16_384),
        GENERATE,
        Some("global"),
        "gemini-3-8-flash-lite-tts"
    ),
    gemini_model!(
        "gemini-3.8-live",
        "Gemini 3.8 Live",
        Some(131_072),
        Some(65_536),
        GENERATE_AND_LIVE,
        Some("us-central1"),
        "gemini-3-8-live"
    ),
    gemini_model!(
        "gemini-3.5-transcribe",
        "Gemini 3.5 Transcribe",
        None,
        None,
        &["generateContent"],
        Some("global"),
        "gemini-3-5-transcribe"
    ),
    gemini_model!(
        "gemini-3.5-transcribe-live",
        "Gemini 3.5 Transcribe Live",
        None,
        None,
        LIVE,
        Some("global"),
        "gemini-3-5-transcribe-live"
    ),
    gemini_model!(
        "gemini-3.5-live-translate-preview",
        "Gemini 3.5 Live Translate Preview",
        None,
        None,
        LIVE,
        Some("global"),
        "gemini-3-5-live-translate-preview"
    ),
    gemini_model!(
        "gemini-live-2.5-flash-native-audio",
        "Gemini Live 2.5 Flash Native Audio",
        Some(131_072),
        Some(65_536),
        GENERATE_AND_LIVE,
        Some("us-central1"),
        "gemini-live-2-5-flash-native-audio"
    ),
    gemini_model!(
        "gemini-omni-1.2-flash",
        "Gemini Omni 1.2 Flash",
        Some(131_072),
        Some(260_640),
        INTERACTIONS_AND_COUNT,
        Some("global"),
        "gemini-omni-1-2-flash"
    ),
    gemini_model!(
        "gemini-omni-1.1-flash-preview",
        "Gemini Omni 1.1 Flash Preview",
        Some(131_072),
        Some(57_920),
        INTERACTIONS_AND_COUNT,
        Some("global"),
        "gemini-omni-1-1-flash-preview"
    ),
    gemini_model!(
        "gemini-omni-flash-preview",
        "Gemini Omni Flash Preview",
        Some(131_072),
        Some(57_920),
        INTERACTIONS_AND_COUNT,
        Some("global")
    ),
    gemini_model!(
        "gemini-embedding-2",
        "Gemini Embedding 2",
        Some(8_192),
        None,
        &["embedContent"],
        Some("global")
    ),
    gemini_model!(
        "gemini-embedding-001",
        "Gemini Embedding 001",
        Some(2_048),
        None,
        &["predict"],
        None
    ),
    gemini_model!(
        "gemini-3.5-transcribe-preview",
        "Gemini 3.5 Transcribe Preview",
        None,
        None,
        &["generateContent"],
        Some("global"),
        "gemini-3-5-transcribe-preview"
    ),
    gemini_model!(
        "gemini-3.5-transcribe-live-preview",
        "Gemini 3.5 Transcribe Live Preview",
        None,
        None,
        LIVE,
        Some("global"),
        "gemini-3-5-transcribe-live-preview"
    ),
    // Keep metadata for historical IDs that SDK callers may still recognize.
    gemini_model!(
        "gemini-2.5-flash",
        "Gemini 2.5 Flash",
        Some(1_000_000),
        Some(8_192),
        &[],
        None,
        "gemini-2-5-flash"
    ),
    gemini_model!(
        "gemini-2.5-pro",
        "Gemini 2.5 Pro",
        Some(2_000_000),
        Some(8_192),
        &[],
        None,
        "gemini-2-5-pro"
    ),
    gemini_model!(
        "gemini-2.5-flash-lite",
        "Gemini 2.5 Flash-Lite",
        Some(1_000_000),
        Some(8_192),
        &[],
        None,
        "gemini-2-5-flash-lite"
    ),
    gemini_model!(
        "gemini-3-pro-preview",
        "Gemini 3 Pro Preview",
        Some(1_000_000),
        Some(64_000),
        &[],
        Some("global")
    ),
    gemini_model!(
        "gemini-3.1-pro",
        "Gemini 3.1 Pro",
        Some(2_000_000),
        Some(64_000),
        &[],
        Some("global"),
        "gemini-3-1-pro"
    ),
    gemini_model!(
        "gemini-3.1-flash",
        "Gemini 3.1 Flash",
        Some(1_000_000),
        Some(8_192),
        &[],
        Some("global"),
        "gemini-3-1-flash",
        "gemini-3-1-flash-preview",
        "gemini-3.1-flash-preview"
    ),
    gemini_model!(
        "gemini-3.1-flash-lite-preview",
        "Gemini 3.1 Flash-Lite Preview",
        Some(1_000_000),
        Some(8_192),
        &[],
        Some("global"),
        "gemini-3-1-flash-lite-preview"
    ),
];

const MODEL_INFO_ENTRIES: &[ModelInfoEntry] = &[
    ModelInfoEntry::claude_with_large_context(
        "publishers/anthropic/models/claude-sonnet-5-5",
        "Claude Sonnet 5.5",
        &["claude-sonnet-5-5", "sonnet-5-5", "sonnet-55"],
    ),
    ModelInfoEntry::claude_with_large_context(
        "publishers/anthropic/models/claude-opus-5-5",
        "Claude Opus 5.5",
        &["claude-opus-5-5", "opus-5-5", "opus-55"],
    ),
    ModelInfoEntry::claude_with_large_context(
        "publishers/anthropic/models/claude-haiku-5-5",
        "Claude Haiku 5.5",
        &["claude-haiku-5-5", "haiku-5-5", "haiku-55"],
    ),
    ModelInfoEntry::claude_with_large_context(
        "publishers/anthropic/models/claude-fable-5-2",
        "Claude Fable 5.2",
        &["claude-fable-5-2", "fable-5-2", "fable-52"],
    ),
    ModelInfoEntry::claude_with_large_context(
        "publishers/anthropic/models/claude-fable-5-1",
        "Claude Fable 5.1",
        &["claude-fable-5-1", "fable-5-1", "fable-51"],
    ),
    ModelInfoEntry::claude_with_large_context(
        "publishers/anthropic/models/claude-mythos-5-2",
        "Claude Mythos 5.2",
        &["claude-mythos-5-2", "mythos-5-2", "mythos-52"],
    ),
    ModelInfoEntry::claude_with_large_context(
        "publishers/anthropic/models/claude-mythos-5-1",
        "Claude Mythos 5.1",
        &["claude-mythos-5-1", "mythos-5-1", "mythos-51"],
    ),
    ModelInfoEntry::claude_with_large_context(
        "publishers/anthropic/models/claude-mythos-5",
        "Claude Mythos 5",
        &["claude-mythos-5", "mythos-5"],
    ),
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
        aliases: CLAUDE_OPUS_46_ALIASES,
        info: ModelInfo::new(
            "publishers/anthropic/models/claude-opus-4-6",
            "Claude 4.6 Opus",
            Some(6_000_000),
            Some(1_000_000),
            Some(128_000),
        ),
    },
    ModelInfoEntry {
        aliases: CLAUDE_SONNET_46_ALIASES,
        info: ModelInfo::new(
            "publishers/anthropic/models/claude-sonnet-4-6",
            "Claude 4.6 Sonnet",
            Some(6_000_000),
            Some(1_000_000),
            Some(64_000),
        ),
    },
    ModelInfoEntry {
        aliases: CLAUDE_OPUS_47_ALIASES,
        info: ModelInfo::new(
            "publishers/anthropic/models/claude-opus-4-7",
            "Claude Opus 4.7",
            None,
            None,
            None,
        ),
    },
    ModelInfoEntry {
        aliases: CLAUDE_OPUS_48_ALIASES,
        info: ModelInfo::new(
            "publishers/anthropic/models/claude-opus-4-8",
            "Claude Opus 4.8",
            None,
            None,
            None,
        ),
    },
    ModelInfoEntry {
        aliases: CLAUDE_SONNET_5_ALIASES,
        info: ModelInfo::new(
            "publishers/anthropic/models/claude-sonnet-5",
            "Claude Sonnet 5",
            None,
            None,
            None,
        ),
    },
    ModelInfoEntry {
        aliases: CLAUDE_FABLE_5_ALIASES,
        info: ModelInfo::new(
            "publishers/anthropic/models/claude-fable-5",
            "Claude Fable 5",
            None,
            None,
            None,
        ),
    },
    ModelInfoEntry {
        aliases: CLAUDE_OPUS_5_ALIASES,
        info: ModelInfo::new(
            "publishers/anthropic/models/claude-opus-5",
            "Claude Opus 5",
            None,
            None,
            None,
        ),
    },
    ModelInfoEntry {
        aliases: CLAUDE_OPUS_41_ALIASES,
        info: ModelInfo::new(
            "publishers/anthropic/models/claude-opus-4-1",
            "Claude 4.1 Opus",
            None,
            Some(200_000),
            Some(64_000),
        ),
    },
    ModelInfoEntry {
        aliases: CLAUDE_SONNET_4_ALIASES,
        info: ModelInfo::new(
            "publishers/anthropic/models/claude-sonnet-4",
            "Claude 4 Sonnet",
            None,
            Some(200_000),
            Some(64_000),
        ),
    },
    ModelInfoEntry {
        aliases: CLAUDE_OPUS_4_ALIASES,
        info: ModelInfo::new(
            "publishers/anthropic/models/claude-opus-4",
            "Claude 4 Opus",
            None,
            Some(200_000),
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
        if let Some(entry) = gemini_entry(&candidate) {
            return Some(entry.info);
        }

        for entry in MODEL_INFO_ENTRIES {
            if entry.aliases.iter().any(|alias| alias.eq_ignore_ascii_case(&candidate)) {
                return Some(entry.info);
            }
        }
    }

    None
}

fn gemini_entry(model: &str) -> Option<&'static GeminiModelEntry> {
    GEMINI_MODELS.iter().find(|entry| {
        model == entry.id || model == entry.info.canonical_id || entry.aliases.contains(&model)
    })
}

pub(crate) fn gemini_default_location(model: &str) -> Option<&'static str> {
    let key = strip_version(&model.to_ascii_lowercase());
    gemini_entry(&key)?.default_location
}

/// Maximum context window (in tokens) across all known models.
#[must_use]
pub fn max_context_window_tokens() -> Option<u64> {
    MODEL_INFO_ENTRIES
        .iter()
        .map(|entry| entry.info)
        .chain(GEMINI_MODELS.iter().map(|entry| entry.info))
        .filter_map(|info| info.context_window_tokens)
        .max()
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
    fn resolves_current_claude_models_and_aliases() {
        for (id, alias) in [
            ("claude-sonnet-5-5", "sonnet-55"),
            ("claude-opus-5-5", "opus-55"),
            ("claude-haiku-5-5", "haiku-55"),
            ("claude-fable-5-2", "fable-52"),
            ("claude-fable-5-1", "fable-51"),
            ("claude-mythos-5-2", "mythos-52"),
            ("claude-mythos-5-1", "mythos-51"),
            ("claude-mythos-5", "mythos-5"),
        ] {
            let resource = format!("publishers/anthropic/models/{id}");
            let info = get_model_info(&resource).expect("known Claude model resource");
            assert_eq!(get_model_info(alias), Some(info));
            assert_eq!(get_model_info(id), Some(info));
            assert_eq!(info.canonical_id, resource);
            assert_eq!(info.context_window_tokens, Some(1_000_000));
            assert_eq!(info.max_output_tokens, Some(128_000));
        }
    }

    #[test]
    fn resolves_known_aliases() {
        let info = get_model_info("claude-sonnet-4-5").unwrap();
        assert_eq!(info.canonical_id, "publishers/anthropic/models/claude-sonnet-4-5");
        assert_eq!(get_model_info("claude-sonnet-45"), Some(info));
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
    fn resolves_gemini_3_1_pro_aliases() {
        let info = get_model_info("gemini-3.1-pro").unwrap();
        assert_eq!(info.canonical_id, "publishers/google/models/gemini-3.1-pro");
        assert_eq!(info.context_window_tokens, Some(2_000_000));
        assert_eq!(info.max_output_tokens, Some(64_000));

        let dashed = get_model_info("gemini-3-1-pro").unwrap();
        assert_eq!(dashed.canonical_id, info.canonical_id);

        let preview = get_model_info("gemini-3.1-pro-preview").unwrap();
        assert_eq!(preview.canonical_id, "publishers/google/models/gemini-3.1-pro-preview");
    }

    #[test]
    fn resolves_gemini_3_1_flash_aliases() {
        let info = get_model_info("gemini-3.1-flash").unwrap();
        assert_eq!(info.canonical_id, "publishers/google/models/gemini-3.1-flash");
        assert_eq!(info.context_window_tokens, Some(1_000_000));

        let lite = get_model_info("gemini-3.1-flash-lite").unwrap();
        assert_eq!(lite.canonical_id, "publishers/google/models/gemini-3.1-flash-lite");
    }

    #[test]
    fn resolves_current_gemini_generation_models() {
        for (model, expected_id) in [
            ("gemini-2.5-flash-lite", "publishers/google/models/gemini-2.5-flash-lite"),
            ("gemini-3-flash-preview", "publishers/google/models/gemini-3-flash-preview"),
            (
                "gemini-3.1-flash-lite-preview",
                "publishers/google/models/gemini-3.1-flash-lite-preview",
            ),
            ("gemini-3.5-flash", "publishers/google/models/gemini-3.5-flash"),
            ("gemini-3.5-flash-lite", "publishers/google/models/gemini-3.5-flash-lite"),
            ("gemini-3.6-flash", "publishers/google/models/gemini-3.6-flash"),
        ] {
            assert_eq!(get_model_info(model).unwrap().canonical_id, expected_id);
        }
    }

    #[test]
    fn resolves_gemini_embedding_aliases() {
        let info = get_model_info("gemini-embedding-001").unwrap();
        assert_eq!(info.canonical_id, "publishers/google/models/gemini-embedding-001");
        assert_eq!(info.context_window_tokens, Some(2_048));
        assert!(info.max_output_tokens.is_none());
    }

    #[test]
    fn resolves_current_gemini_catalog_ids_and_aliases() {
        for entry in GEMINI_MODELS {
            let full_name = format!("projects/test/locations/global/{}", entry.info.canonical_id);
            assert_eq!(get_model_info(&full_name), Some(entry.info));
            assert_eq!(get_model_info(entry.id), Some(entry.info));
            for alias in entry.aliases {
                assert_eq!(get_model_info(alias), Some(entry.info));
            }
        }
    }

    #[test]
    fn resolves_claude_opus_46_aliases() {
        let info = get_model_info("claude-opus-4-6").unwrap();
        assert_eq!(info.canonical_id, "publishers/anthropic/models/claude-opus-4-6");
        assert_eq!(info.context_window_tokens, Some(1_000_000));
        assert_eq!(info.max_output_tokens, Some(128_000));

        let short = get_model_info("opus-46").unwrap();
        assert_eq!(short.canonical_id, info.canonical_id);
    }

    #[test]
    fn resolves_claude_sonnet_46_aliases() {
        let info = get_model_info("claude-sonnet-4-6").unwrap();
        assert_eq!(info.canonical_id, "publishers/anthropic/models/claude-sonnet-4-6");
        assert_eq!(info.context_window_tokens, Some(1_000_000));
        assert_eq!(info.max_output_tokens, Some(64_000));
    }

    #[test]
    fn resolves_latest_claude_metadata_aliases_without_assumed_limits() {
        let cases = [
            ("opus-47", "publishers/anthropic/models/claude-opus-4-7", "Claude Opus 4.7"),
            ("opus-4-8", "publishers/anthropic/models/claude-opus-4-8", "Claude Opus 4.8"),
            ("sonnet-5", "publishers/anthropic/models/claude-sonnet-5", "Claude Sonnet 5"),
            ("fable-5", "publishers/anthropic/models/claude-fable-5", "Claude Fable 5"),
            ("opus-5", "publishers/anthropic/models/claude-opus-5", "Claude Opus 5"),
        ];

        for (alias, canonical_id, display_name) in cases {
            let alias_info = get_model_info(alias).unwrap();
            let canonical_info = get_model_info(canonical_id).unwrap();

            assert_eq!(alias_info, canonical_info);
            assert_eq!(alias_info.canonical_id, canonical_id);
            assert_eq!(alias_info.display_name, display_name);
            assert!(alias_info.max_request_bytes.is_none());
            assert!(alias_info.context_window_tokens.is_none());
            assert!(alias_info.max_output_tokens.is_none());
        }
    }

    #[test]
    fn resolves_claude_opus_41_and_4_aliases() {
        let opus41 = get_model_info("claude-opus-4-1").unwrap();
        assert_eq!(opus41.canonical_id, "publishers/anthropic/models/claude-opus-4-1");

        let sonnet4 = get_model_info("claude-sonnet-4").unwrap();
        assert_eq!(sonnet4.canonical_id, "publishers/anthropic/models/claude-sonnet-4");

        let opus4 = get_model_info("claude-opus-4").unwrap();
        assert_eq!(opus4.canonical_id, "publishers/anthropic/models/claude-opus-4");
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
