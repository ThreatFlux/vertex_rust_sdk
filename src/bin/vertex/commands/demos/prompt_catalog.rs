use anyhow::{anyhow, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptResolution {
    pub prompt: String,
    pub description: String,
    pub example: String,
    pub notice: Option<String>,
}

const THINKING_PROMPTS: &[(&str, &str, &str)] = &[
    (
        "math",
        "What is 47 × 83? Show your working step by step.",
        "Mathematical calculation with step-by-step reasoning",
    ),
    (
        "logic",
        "If all roses are flowers, and some flowers are red, can we conclude that some roses are red? Explain your reasoning.",
        "Logical deduction and reasoning",
    ),
    (
        "reasoning",
        "A farmer has chickens and rabbits. In total, there are 35 heads and 94 legs. How many chickens and how many rabbits are there?",
        "Complex problem solving with multiple constraints",
    ),
    (
        "decision",
        "Should I invest in stocks or bonds right now? Consider the current economic climate, risk tolerance for a 30-year-old, and long-term vs short-term goals.",
        "Decision making with multiple factors and trade-offs",
    ),
];

const GROUNDING_PROMPTS: &[(&str, &str, &str)] = &[
    (
        "news",
        "What are the latest news headlines in technology and AI this week?",
        "Current Technology News",
    ),
    (
        "events",
        "What major events are happening around the world today?",
        "Current World Events",
    ),
    (
        "facts",
        "What is the current population of the world's largest cities?",
        "Up-to-date Population Facts",
    ),
    (
        "weather",
        "What's the current weather like in major cities around the world?",
        "Current Weather Information",
    ),
    (
        "stocks",
        "What are the current stock prices for major tech companies like Apple, Google, and Microsoft?",
        "Current Stock Market Data",
    ),
];

pub fn resolve_thinking_prompt(
    example: &str,
    custom_prompt: Option<&str>,
) -> Result<PromptResolution> {
    let normalized = example.trim().to_lowercase();

    if normalized == "custom" {
        let prompt =
            custom_prompt.ok_or_else(|| anyhow!("Custom example requires --prompt argument"))?;
        return Ok(PromptResolution {
            prompt: prompt.to_string(),
            description: "Custom reasoning task".to_string(),
            example: "custom".to_string(),
            notice: None,
        });
    }

    for (key, prompt, description) in THINKING_PROMPTS {
        if *key == normalized {
            return Ok(PromptResolution {
                prompt: (*prompt).to_string(),
                description: (*description).to_string(),
                example: normalized,
                notice: None,
            });
        }
    }

    Ok(PromptResolution {
        prompt: THINKING_PROMPTS[0].1.to_string(),
        description: THINKING_PROMPTS[0].2.to_string(),
        example: THINKING_PROMPTS[0].0.to_string(),
        notice: Some(format!("Unknown example type: {example}. Using math example as fallback.")),
    })
}

pub fn resolve_grounding_prompt(example: &str, custom_prompt: Option<&str>) -> PromptResolution {
    let normalized = example.trim().to_lowercase();

    if normalized == "custom" {
        let prompt = custom_prompt
            .unwrap_or("What are the latest developments in renewable energy technology?");
        return PromptResolution {
            prompt: prompt.to_string(),
            description: "Custom Grounding Query".to_string(),
            example: "custom".to_string(),
            notice: None,
        };
    }

    for (key, prompt, description) in GROUNDING_PROMPTS {
        if *key == normalized {
            return PromptResolution {
                prompt: (*prompt).to_string(),
                description: (*description).to_string(),
                example: normalized,
                notice: None,
            };
        }
    }

    PromptResolution {
        prompt: "What's happening in the news today?".to_string(),
        description: "General News Query".to_string(),
        example: "news".to_string(),
        notice: Some(format!("Unknown example type: {example}. Using default news query.")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_known_thinking_prompt() {
        let result = resolve_thinking_prompt("logic", None).unwrap();
        assert_eq!(result.example, "logic");
        assert!(result.notice.is_none());
        assert!(result.prompt.contains("roses"));
    }

    #[test]
    fn errors_on_missing_custom_prompt_for_thinking() {
        let err = resolve_thinking_prompt("custom", None).unwrap_err();
        assert!(err.to_string().contains("requires --prompt"));
    }

    #[test]
    fn falls_back_to_math_on_unknown_thinking_example() {
        let result = resolve_thinking_prompt("unknown", None).unwrap();
        assert_eq!(result.example, "math");
        assert!(result.notice.unwrap().contains("Unknown example type"));
    }

    #[test]
    fn returns_custom_grounding_prompt_when_supplied() {
        let result = resolve_grounding_prompt("custom", Some("hello"));
        assert_eq!(result.prompt, "hello");
        assert_eq!(result.example, "custom");
    }

    #[test]
    fn falls_back_to_news_on_unknown_grounding_example() {
        let result = resolve_grounding_prompt("other", None);
        assert_eq!(result.example, "news");
        assert!(result.notice.unwrap().contains("Unknown example type"));
    }
}
