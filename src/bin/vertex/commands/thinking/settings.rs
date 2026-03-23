use anyhow::{bail, Result};
use threatflux_vertex_rust_sdk::types::ThinkingLevel;

use super::rules::{gemini_thinking_rules, GeminiThinkingRules};

#[derive(Clone, Copy, Debug, Default)]
pub struct ThinkingSettings {
    pub enabled: bool,
    pub thinking_budget: Option<i32>,
    pub thinking_level: Option<ThinkingLevel>,
}

impl ThinkingSettings {
    pub const fn disabled() -> Self {
        Self { enabled: false, thinking_budget: None, thinking_level: None }
    }
}

pub fn validate_thinking_settings(
    model: &str,
    thinking_requested: bool,
    thinking_budget: Option<i32>,
    thinking_level: Option<ThinkingLevel>,
) -> Result<ThinkingSettings> {
    if thinking_budget.is_some() && thinking_level.is_some() {
        bail!("Cannot specify both thinkingBudget and thinkingLevel in the same request");
    }

    let enabled = thinking_requested || thinking_budget.is_some() || thinking_level.is_some();
    let final_budget = thinking_budget;
    let final_level = thinking_level;

    if !enabled {
        return Ok(ThinkingSettings::disabled());
    }

    if let Some(rules) = gemini_thinking_rules(model) {
        return validate_known_model(rules, enabled, final_budget, final_level);
    }

    Ok(match (final_level, final_budget) {
        (Some(level), _) => {
            ThinkingSettings { enabled: true, thinking_budget: None, thinking_level: Some(level) }
        }
        (_, Some(budget)) => ThinkingSettings {
            enabled: budget != 0,
            thinking_budget: Some(budget),
            thinking_level: None,
        },
        _ => ThinkingSettings::disabled(),
    })
}

fn validate_known_model(
    rules: GeminiThinkingRules,
    enabled: bool,
    budget: Option<i32>,
    level: Option<ThinkingLevel>,
) -> Result<ThinkingSettings> {
    if rules.supports_level {
        return level_for_rule(rules, level);
    }

    if level.is_some() {
        bail!("{} does not support thinkingLevel; supply thinkingBudget instead", rules.label);
    }

    let budget = match budget {
        Some(value) => value,
        None if rules.allow_dynamic => -1,
        None => {
            bail!(
                "{} requires an explicit thinkingBudget value; dynamic thinking is not supported",
                rules.label
            )
        }
    };

    if budget == 0 && !rules.allow_disable {
        bail!("{} cannot disable thinking; remove thinkingBudget = 0", rules.label);
    }

    if budget == -1 && !rules.allow_dynamic {
        bail!("{} does not support dynamic thinking budgets (thinkingBudget = -1)", rules.label);
    }

    if budget < 0 && budget != -1 {
        bail!(
            "Invalid thinkingBudget {} supplied for {}. Use -1 for dynamic thinking or a value within {}-{}.",
            budget,
            rules.label,
            rules.min_budget.unwrap_or(0),
            rules.max_budget.unwrap_or(32_768)
        );
    }

    if let Some(min) = rules.min_budget {
        if budget != -1 && budget < min {
            bail!(
                "{} requires thinkingBudget >= {} (or -1 for dynamic thinking); received {}",
                rules.label,
                min,
                budget
            );
        }
    }

    if let Some(max) = rules.max_budget {
        if budget != -1 && budget > max {
            bail!("{} supports thinkingBudget up to {}; received {}", rules.label, max, budget);
        }
    }

    Ok(ThinkingSettings {
        enabled: enabled && budget != 0,
        thinking_budget: Some(budget),
        thinking_level: None,
    })
}

fn level_for_rule(
    rules: GeminiThinkingRules,
    level: Option<ThinkingLevel>,
) -> Result<ThinkingSettings> {
    if level.is_some() {
        return Ok(ThinkingSettings {
            enabled: true,
            thinking_budget: None,
            thinking_level: level,
        });
    }

    if let Some(default) = rules.default_level {
        return Ok(ThinkingSettings {
            enabled: true,
            thinking_budget: None,
            thinking_level: Some(default),
        });
    }

    bail!("{} only supports thinkingLevel; remove thinkingBudget", rules.label)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errors_on_both_budget_and_level() {
        let err = validate_thinking_settings(
            "gemini-2.5-pro",
            true,
            Some(100),
            Some(ThinkingLevel::High),
        )
        .unwrap_err();
        assert!(err.to_string().contains("Cannot specify both thinkingBudget and thinkingLevel"));
    }

    #[test]
    fn applies_level_rules_when_supported() {
        let settings = validate_thinking_settings(
            "gemini-3-pro-preview",
            true,
            None,
            Some(ThinkingLevel::Low),
        )
        .unwrap();
        assert!(settings.enabled);
        assert_eq!(settings.thinking_level, Some(ThinkingLevel::Low));
        assert!(settings.thinking_budget.is_none());
    }

    #[test]
    fn fills_default_level_when_not_provided() {
        let settings =
            validate_thinking_settings("gemini-3-pro-preview", true, None, None).unwrap();
        assert_eq!(settings.thinking_level, Some(ThinkingLevel::High));
    }

    #[test]
    fn rejects_level_for_budget_only_models() {
        let err =
            validate_thinking_settings("gemini-2.5-flash", true, None, Some(ThinkingLevel::High))
                .unwrap_err();
        assert!(err.to_string().contains("does not support thinkingLevel"));
    }

    #[test]
    fn rejects_disabling_when_not_allowed() {
        let err = validate_thinking_settings("gemini-2.5-pro", true, Some(0), None).unwrap_err();
        assert!(err.to_string().contains("cannot disable thinking"));
    }

    #[test]
    fn rejects_negative_budget() {
        let err = validate_thinking_settings("gemini-2.5-flash", true, Some(-5), None).unwrap_err();
        assert!(err.to_string().contains("Invalid thinkingBudget"));
    }

    #[test]
    fn rejects_out_of_range_budget() {
        let err =
            validate_thinking_settings("gemini-2.5-flash", true, Some(30_000), None).unwrap_err();
        assert!(err.to_string().contains("supports thinkingBudget up to"));
    }

    #[test]
    fn defaults_dynamic_budget_when_allowed() {
        let settings = validate_thinking_settings("gemini-2.5-flash", true, None, None).unwrap();
        assert_eq!(settings.thinking_budget, Some(-1));
        assert!(settings.enabled);
    }

    #[test]
    fn unknown_model_passes_through_budget() {
        let settings = validate_thinking_settings("unknown", true, Some(10), None).unwrap();
        assert_eq!(settings.thinking_budget, Some(10));
        assert!(settings.enabled);
    }

    #[test]
    fn unknown_model_disables_on_zero_budget() {
        let settings = validate_thinking_settings("unknown", true, Some(0), None).unwrap();
        assert!(!settings.enabled);
    }
}
