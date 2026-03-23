use threatflux_vertex_rust_sdk::types::{GenerationConfig, ThinkingLevel};

use super::settings::ThinkingSettings;

pub fn describe_thinking_settings(settings: &ThinkingSettings) -> String {
    if let Some(level) = settings.thinking_level {
        return match level {
            ThinkingLevel::Low => "Level: Low".to_string(),
            ThinkingLevel::High => "Level: High".to_string(),
        };
    }

    if let Some(budget) = settings.thinking_budget {
        return match budget {
            -1 => "Auto budget".to_string(),
            0 => "Disabled".to_string(),
            value => format!("{value} tokens"),
        };
    }

    if settings.enabled {
        "Auto".to_string()
    } else {
        "Disabled".to_string()
    }
}

pub fn apply_thinking_to_config(
    generation_config: GenerationConfig,
    settings: &ThinkingSettings,
) -> GenerationConfig {
    if settings.enabled {
        if let Some(level) = settings.thinking_level {
            return generation_config.with_thinking_level(level);
        }

        if let Some(budget) = settings.thinking_budget {
            if budget == -1 {
                return generation_config.with_thinking();
            }
            return generation_config.with_thinking_budget(budget);
        }

        return generation_config.with_thinking();
    }

    if let Some(budget) = settings.thinking_budget {
        if budget == 0 {
            return generation_config.without_thinking();
        }
    }

    generation_config
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn describes_thinking_level() {
        let settings = ThinkingSettings {
            enabled: true,
            thinking_budget: None,
            thinking_level: Some(ThinkingLevel::Low),
        };

        assert_eq!(describe_thinking_settings(&settings), "Level: Low");
    }

    #[test]
    fn applies_dynamic_budget() {
        let settings =
            ThinkingSettings { enabled: true, thinking_budget: Some(-1), thinking_level: None };

        let config = apply_thinking_to_config(GenerationConfig::default(), &settings);
        assert_eq!(
            config
                .thinking_config
                .as_ref()
                .and_then(threatflux_vertex_rust_sdk::ThinkingConfig::budget_value),
            Some(-1)
        );
    }

    #[test]
    fn disables_thinking_when_budget_zero() {
        let settings =
            ThinkingSettings { enabled: false, thinking_budget: Some(0), thinking_level: None };

        let config = apply_thinking_to_config(GenerationConfig::default(), &settings);
        assert_eq!(
            config
                .thinking_config
                .as_ref()
                .and_then(threatflux_vertex_rust_sdk::ThinkingConfig::budget_value),
            Some(0)
        );
    }
}
