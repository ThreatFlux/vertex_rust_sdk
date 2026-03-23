use threatflux_vertex_rust_sdk::{model_descriptor::ModelDescriptor, types::ThinkingLevel};

#[derive(Clone, Copy)]
pub struct GeminiThinkingRules {
    pub label: &'static str,
    pub min_budget: Option<i32>,
    pub max_budget: Option<i32>,
    pub allow_disable: bool,
    pub allow_dynamic: bool,
    pub supports_level: bool,
    pub default_level: Option<ThinkingLevel>,
}

#[derive(Clone, Copy)]
struct ModelRule {
    prefix: &'static str,
    label: &'static str,
    preview_label: Option<&'static str>,
    min_budget: Option<i32>,
    max_budget: Option<i32>,
    allow_disable: bool,
    allow_dynamic: bool,
    supports_level: bool,
    default_level: Option<ThinkingLevel>,
}

impl ModelRule {
    fn label_for(&self, model_name: &str) -> &'static str {
        if let Some(preview) = self.preview_label {
            if model_name.contains("preview") {
                return preview;
            }
        }

        self.label
    }

    fn into_gemini_rules(self, model_name: &str) -> GeminiThinkingRules {
        GeminiThinkingRules {
            label: self.label_for(model_name),
            min_budget: self.min_budget,
            max_budget: self.max_budget,
            allow_disable: self.allow_disable,
            allow_dynamic: self.allow_dynamic,
            supports_level: self.supports_level,
            default_level: self.default_level,
        }
    }
}

const MODEL_RULES: &[ModelRule] = &[
    ModelRule {
        prefix: "gemini-2.5-pro",
        label: "Gemini 2.5 Pro",
        preview_label: None,
        min_budget: Some(128),
        max_budget: Some(32_768),
        allow_disable: false,
        allow_dynamic: true,
        supports_level: false,
        default_level: None,
    },
    ModelRule {
        prefix: "gemini-2.5-flash-lite",
        label: "Gemini 2.5 Flash-Lite",
        preview_label: Some("Gemini 2.5 Flash-Lite (Preview)"),
        min_budget: Some(512),
        max_budget: Some(24_576),
        allow_disable: true,
        allow_dynamic: true,
        supports_level: false,
        default_level: None,
    },
    ModelRule {
        prefix: "gemini-2.5-flash",
        label: "Gemini 2.5 Flash",
        preview_label: Some("Gemini 2.5 Flash (Preview)"),
        min_budget: Some(0),
        max_budget: Some(24_576),
        allow_disable: true,
        allow_dynamic: true,
        supports_level: false,
        default_level: None,
    },
    ModelRule {
        prefix: "robotics-er-1.5",
        label: "Robotics-ER 1.5 (Preview)",
        preview_label: None,
        min_budget: Some(512),
        max_budget: Some(24_576),
        allow_disable: true,
        allow_dynamic: true,
        supports_level: false,
        default_level: None,
    },
    ModelRule {
        prefix: "gemini-3-pro-preview",
        label: "Gemini 3 Pro Preview",
        preview_label: None,
        min_budget: None,
        max_budget: None,
        allow_disable: true,
        allow_dynamic: false,
        supports_level: true,
        default_level: Some(ThinkingLevel::High),
    },
];

pub fn normalized_model_name(model: &str) -> String {
    let normalized = ModelDescriptor::parse(model).map_or_else(
        |_| model.trim().to_ascii_lowercase(),
        |descriptor| descriptor.model().to_ascii_lowercase(),
    );

    if let Some((base, _)) = normalized.split_once('@') {
        base.to_string()
    } else {
        normalized
    }
}

pub fn gemini_thinking_rules(model: &str) -> Option<GeminiThinkingRules> {
    let name = normalized_model_name(model);
    MODEL_RULES
        .iter()
        .find(|rule| name.starts_with(rule.prefix))
        .copied()
        .map(|rule| rule.into_gemini_rules(&name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_model_name_lowercase_and_strip_version() {
        assert_eq!(normalized_model_name("GeMiNi-2.5-Pro@001"), "gemini-2.5-pro");
        assert_eq!(normalized_model_name("  robotics-er-1.5  "), "robotics-er-1.5");
    }

    #[test]
    fn finds_rules_with_preview_label() {
        let rules = gemini_thinking_rules("gemini-2.5-flash-lite-preview").unwrap();
        assert_eq!(rules.label, "Gemini 2.5 Flash-Lite (Preview)");
        assert_eq!(rules.min_budget, Some(512));
        assert!(rules.allow_dynamic);
    }

    #[test]
    fn returns_none_for_unknown_model() {
        assert!(gemini_thinking_rules("unknown-model").is_none());
    }

    #[test]
    fn uses_default_level_for_level_based_models() {
        let rules = gemini_thinking_rules("gemini-3-pro-preview").unwrap();
        assert!(rules.supports_level);
        assert_eq!(rules.default_level, Some(ThinkingLevel::High));
        assert!(!rules.allow_dynamic);
    }
}
