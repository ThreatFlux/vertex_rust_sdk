use anyhow::{Context, Result};
use threatflux_vertex_rust_sdk::{
    models::GenerateContentRequest,
    types::{GenerationConfig, ThinkingLevel},
};

use crate::commands::thinking::{
    apply_thinking_to_config, validate_thinking_settings, ThinkingSettings,
};

use super::{GenerationOptions, StreamOptions};

pub fn resolve_thinking_settings(
    model: &str,
    requested: bool,
    budget: Option<i32>,
    level: Option<ThinkingLevel>,
) -> Result<ThinkingSettings> {
    validate_thinking_settings(model, requested, budget, level)
}

pub fn build_generation_config(
    options: &GenerationOptions,
    thinking: &ThinkingSettings,
) -> Result<GenerationConfig> {
    let mut config = GenerationConfig {
        temperature: Some(options.temperature),
        max_output_tokens: Some(options.max_output_tokens),
        ..Default::default()
    };

    config = apply_thinking_to_config(config, thinking);

    if options.json {
        config = config.with_json_response();
    }

    if let Some(schema) = options.schema.as_deref() {
        let schema_value = serde_json::from_str(schema)
            .with_context(|| "Invalid schema JSON supplied for structured output")?;
        config = config.with_response_schema(schema_value);
    }

    Ok(config)
}

pub fn build_stream_config(
    options: &StreamOptions,
    thinking: &ThinkingSettings,
) -> GenerationConfig {
    let config = GenerationConfig {
        temperature: Some(options.temperature),
        max_output_tokens: Some(options.max_output_tokens),
        ..Default::default()
    };

    apply_thinking_to_config(config, thinking)
}

pub fn build_request_from_generation(
    prompt: &str,
    config: GenerationConfig,
    system_instruction: Option<&str>,
    cache_id: Option<&str>,
    grounding: bool,
) -> GenerateContentRequest {
    let mut request = GenerateContentRequest::new(prompt).with_generation_config(config);

    if let Some(instruction) = system_instruction {
        request = request.with_system_text(instruction);
    }

    if let Some(cache) = cache_id {
        request = request.with_cached_content(cache);
    }

    if grounding {
        request = request.with_google_search();
    }

    request
}

#[cfg(test)]
mod tests {
    use super::*;
    use threatflux_vertex_rust_sdk::types::Part;

    #[test]
    fn config_sets_json_and_schema() {
        let options = GenerationOptions {
            prompt: "hello".to_string(),
            model: "gemini".to_string(),
            temperature: 0.3,
            max_output_tokens: 100,
            system_instruction: None,
            json: true,
            schema: Some("{\"type\":\"object\"}".into()),
            cache_id: None,
            thinking: false,
            thinking_budget: None,
            thinking_level: None,
            grounding: false,
        };

        let thinking = ThinkingSettings::disabled();
        let config = build_generation_config(&options, &thinking).unwrap();

        assert_eq!(config.temperature, Some(0.3));
        assert_eq!(config.max_output_tokens, Some(100));
        assert_eq!(config.response_mime_type.as_deref(), Some("application/json"));
        assert_eq!(config.response_schema, Some(serde_json::json!({"type": "object"})));
    }

    #[test]
    fn config_errors_on_bad_schema() {
        let options = GenerationOptions {
            prompt: "hello".to_string(),
            model: "gemini".to_string(),
            temperature: 0.3,
            max_output_tokens: 100,
            system_instruction: None,
            json: true,
            schema: Some("{\"type\":".into()),
            cache_id: None,
            thinking: false,
            thinking_budget: None,
            thinking_level: None,
            grounding: false,
        };

        let thinking = ThinkingSettings::disabled();
        let err = build_generation_config(&options, &thinking).unwrap_err();
        assert!(err.to_string().contains("Invalid schema JSON"));
    }

    #[test]
    fn request_applies_system_cache_and_grounding() {
        let config = GenerationConfig::default();
        let request =
            build_request_from_generation("prompt", config, Some("sys"), Some("cache"), true);

        let system_text = request.system_instruction.as_ref().and_then(|content| {
            content.parts.first().and_then(|part| match part {
                Part::Text { text } => Some(text.clone()),
                _ => None,
            })
        });
        assert_eq!(system_text, Some("sys".to_string()));
        assert_eq!(request.cached_content.as_deref(), Some("cache"));
        assert!(matches!(
            request.tools,
            Some(tools) if !tools.is_empty()
        ));
    }

    #[test]
    fn stream_config_preserves_thinking_budget() {
        let options = StreamOptions {
            prompt: "hello".to_string(),
            model: "gemini".to_string(),
            temperature: 0.3,
            max_output_tokens: 100,
            system_instruction: None,
            cache_id: None,
            thinking: true,
            thinking_budget: Some(256),
            thinking_level: None,
            grounding: false,
        };

        let thinking =
            resolve_thinking_settings("gemini-2.5-flash", true, Some(256), None).unwrap();
        let config = build_stream_config(&options, &thinking);

        assert_eq!(config.max_output_tokens, Some(100));
        assert_eq!(
            config
                .thinking_config
                .as_ref()
                .and_then(threatflux_vertex_rust_sdk::ThinkingConfig::budget_value),
            Some(256)
        );
    }
}
