use super::*;

#[test]
fn test_content_creation() {
    let content = Content::user_text("Hello, world!");
    assert_eq!(content.role, "user");
    assert_eq!(content.parts.len(), 1);

    if let Part::Text { text } = &content.parts[0] {
        assert_eq!(text, "Hello, world!");
    } else {
        panic!("Expected text part");
    }
}

#[test]
fn test_generation_config_default() {
    let config = GenerationConfig::default();
    assert_eq!(config.temperature, Some(0.7));
    assert_eq!(config.max_output_tokens, Some(2048));
    assert_eq!(config.response_mime_type, None);
    assert_eq!(config.response_schema, None);
}

#[test]
fn test_generation_config_structured_output() {
    let config = GenerationConfig::default()
        .with_json_response()
        .with_response_schema(GenerationConfig::person_schema());

    assert_eq!(config.response_mime_type, Some("application/json".to_string()));
    assert!(config.response_schema.is_some());
}

#[test]
fn test_person_schema() {
    let schema = GenerationConfig::person_schema();
    assert_eq!(schema["type"], "object");
    assert!(schema["properties"]["name"].is_object());
    assert!(schema["required"].is_array());
}

#[test]
fn test_part_text() {
    let part = Part::text("test");
    if let Part::Text { text } = part {
        assert_eq!(text, "test");
    } else {
        panic!("Expected text part");
    }
}

#[test]
fn test_thinking_config() {
    let auto_config = ThinkingConfig::auto();
    assert_eq!(auto_config.budget_value(), Some(-1));

    let disabled_config = ThinkingConfig::disabled();
    assert_eq!(disabled_config.budget_value(), Some(0));

    let budget_config = ThinkingConfig::with_budget(512);
    assert_eq!(budget_config.budget_value(), Some(512));

    let default_config = ThinkingConfig::default_budget();
    assert_eq!(default_config.budget_value(), Some(1024));

    let level_config = ThinkingConfig::with_level(ThinkingLevel::High);
    assert_eq!(level_config.level_value(), Some(ThinkingLevel::High));
    assert_eq!(level_config.budget_value(), None);

    // Test budget clamping.
    let large_budget = ThinkingConfig::with_budget(50_000);
    assert_eq!(large_budget.budget_value(), Some(32_768));

    let negative_budget = ThinkingConfig::with_budget(-5);
    assert_eq!(negative_budget.budget_value(), Some(0));
}

#[test]
fn test_generation_config_with_thinking() {
    let config = GenerationConfig::default().with_thinking();
    assert!(config.thinking_config.is_some());
    assert_eq!(config.thinking_config.as_ref().unwrap().budget_value(), Some(-1));

    let config_budget = GenerationConfig::default().with_thinking_budget(256);
    assert!(config_budget.thinking_config.is_some());
    assert_eq!(config_budget.thinking_config.as_ref().unwrap().budget_value(), Some(256));

    let config_disabled = GenerationConfig::default().without_thinking();
    assert!(config_disabled.thinking_config.is_some());
    assert_eq!(config_disabled.thinking_config.as_ref().unwrap().budget_value(), Some(0));

    let level_config = GenerationConfig::default().with_thinking_level(ThinkingLevel::Low);
    assert_eq!(
        level_config.thinking_config.as_ref().unwrap().level_value(),
        Some(ThinkingLevel::Low)
    );
}

#[test]
fn test_usage_metadata_full() {
    let usage = UsageMetadata {
        prompt_token_count: 100,
        candidates_token_count: Some(50),
        total_token_count: 150,
        traffic_type: Some("ON_DEMAND".to_string()),
        modality_token_count: None,
    };

    assert_eq!(usage.prompt_token_count, 100);
    assert_eq!(usage.candidates_token_count, Some(50));
    assert_eq!(usage.total_token_count, 150);
    assert_eq!(usage.traffic_type, Some("ON_DEMAND".to_string()));
}

#[test]
fn test_usage_metadata_minimal() {
    let usage = UsageMetadata {
        prompt_token_count: 0,
        candidates_token_count: None,
        total_token_count: 0,
        traffic_type: None,
        modality_token_count: None,
    };

    assert_eq!(usage.prompt_token_count, 0);
    assert_eq!(usage.candidates_token_count, None);
    assert_eq!(usage.total_token_count, 0);
    assert_eq!(usage.traffic_type, None);
}

#[test]
fn test_usage_metadata_deserialization_full() {
    let json = r#"{
        "promptTokenCount": 100,
        "candidatesTokenCount": 50,
        "totalTokenCount": 150,
        "trafficType": "ON_DEMAND"
    }"#;

    let usage: UsageMetadata = serde_json::from_str(json).unwrap();
    assert_eq!(usage.prompt_token_count, 100);
    assert_eq!(usage.candidates_token_count, Some(50));
    assert_eq!(usage.total_token_count, 150);
    assert_eq!(usage.traffic_type, Some("ON_DEMAND".to_string()));
}

#[test]
fn test_usage_metadata_deserialization_traffic_type_only() {
    // Test intermediate streaming chunks that only have trafficType.
    let json = r#"{
        "trafficType": "ON_DEMAND"
    }"#;

    let usage: UsageMetadata = serde_json::from_str(json).unwrap();
    assert_eq!(usage.prompt_token_count, 0); // default
    assert_eq!(usage.candidates_token_count, None);
    assert_eq!(usage.total_token_count, 0); // default
    assert_eq!(usage.traffic_type, Some("ON_DEMAND".to_string()));
}

#[test]
fn test_usage_metadata_deserialization_partial() {
    // Test final chunk with counts but no candidatesTokenCount.
    let json = r#"{
        "promptTokenCount": 10,
        "totalTokenCount": 10
    }"#;

    let usage: UsageMetadata = serde_json::from_str(json).unwrap();
    assert_eq!(usage.prompt_token_count, 10);
    assert_eq!(usage.candidates_token_count, None);
    assert_eq!(usage.total_token_count, 10);
    assert_eq!(usage.traffic_type, None);
}

#[test]
fn test_usage_metadata_serialization() {
    let usage = UsageMetadata {
        prompt_token_count: 100,
        candidates_token_count: Some(50),
        total_token_count: 150,
        traffic_type: Some("ON_DEMAND".to_string()),
        modality_token_count: None,
    };

    let json = serde_json::to_string(&usage).unwrap();
    assert!(json.contains("\"promptTokenCount\":100"));
    assert!(json.contains("\"candidatesTokenCount\":50"));
    assert!(json.contains("\"totalTokenCount\":150"));
    assert!(json.contains("\"trafficType\":\"ON_DEMAND\""));
}

#[test]
fn part_function_call_with_thought_signature_deserializes() {
    let json = r#"{
        "functionCall": {
            "name": "check_flight",
            "args": {
                "flight": "AA100"
            }
        },
        "thoughtSignature": "<Signature_A>"
    }"#;

    let part: Part = serde_json::from_str(json).unwrap();
    match part {
        Part::FunctionCall { function_call } => {
            assert_eq!(function_call.name, "check_flight");
            assert_eq!(function_call.args.get("flight").unwrap(), "AA100");
        }
        other => panic!("unexpected part: {other:?}"),
    }
}

#[test]
fn finish_reason_handles_new_and_unknown_variants() {
    let blocklist: FinishReason = serde_json::from_str("\"FINISH_REASON_BLOCKLIST\"").unwrap();
    assert!(matches!(blocklist, FinishReason::Blocklist));

    let malformed: FinishReason =
        serde_json::from_str("\"FINISH_REASON_MALFORMED_FUNCTION_CALL\"").unwrap();
    assert!(matches!(malformed, FinishReason::MalformedFunctionCall));

    let unknown: FinishReason = serde_json::from_str("\"FINISH_REASON_FUTURE\"").unwrap();
    assert!(matches!(unknown, FinishReason::Unknown));
}

#[test]
fn test_usage_metadata_serialization_omits_none() {
    let usage = UsageMetadata {
        prompt_token_count: 100,
        candidates_token_count: None,
        total_token_count: 100,
        traffic_type: None,
        modality_token_count: None,
    };

    let json = serde_json::to_string(&usage).unwrap();
    assert!(json.contains("\"promptTokenCount\":100"));
    assert!(!json.contains("candidatesTokenCount"));
    assert!(json.contains("\"totalTokenCount\":100"));
    assert!(!json.contains("trafficType"));
}

#[test]
fn test_usage_metadata_with_modalities() {
    let json = r#"{
        "promptTokenCount": 120,
        "totalTokenCount": 200,
        "modalityTokenCount": {
            "modality.TEXT": {
                "promptTokenCount": 80,
                "candidatesTokenCount": 40,
                "totalTokenCount": 120
            },
            "modality.IMAGE": {
                "promptTokenCount": 40,
                "totalTokenCount": 80
            }
        }
    }"#;

    let usage: UsageMetadata = serde_json::from_str(json).unwrap();
    let modalities = usage.modality_token_count.unwrap();
    let text = modalities.get("modality.TEXT").unwrap();
    assert_eq!(text.prompt_token_count, 80);
    assert_eq!(text.candidates_token_count, Some(40));
    assert_eq!(text.total_token_count, 120);

    let image = modalities.get("modality.IMAGE").unwrap();
    assert_eq!(image.prompt_token_count, 40);
    assert_eq!(image.candidates_token_count, None);
    assert_eq!(image.total_token_count, 80);
}

#[test]
fn test_request_metadata_helpers() {
    let meta = RequestMetadata::new()
        .with_user_id("user-123")
        .with_custom("session", serde_json::json!("abc"));

    assert_eq!(meta.user_id.as_deref(), Some("user-123"));
    assert_eq!(meta.custom.get("session"), Some(&serde_json::json!("abc")));
    assert!(!meta.is_empty());
}

#[test]
fn test_request_metadata_empty() {
    let meta = RequestMetadata::new();
    assert!(meta.is_empty());
    assert!(meta.custom.is_empty());
    assert!(meta.user_id.is_none());
}

#[test]
fn test_part_thinking() {
    let part = Part::thinking("This is my reasoning...");
    if let Part::Thinking { thought } = part {
        assert_eq!(thought, "This is my reasoning...");
    } else {
        panic!("Expected thinking part");
    }
}

#[test]
fn test_grounding_config() {
    let default_config = GroundingConfig::default();
    assert!(default_config.disable_attribution.is_none());

    let with_attribution = GroundingConfig::with_attribution();
    assert_eq!(with_attribution.disable_attribution, Some(false));

    let without_attribution = GroundingConfig::without_attribution();
    assert_eq!(without_attribution.disable_attribution, Some(true));
}

#[test]
fn test_google_search_tool() {
    let tool = Tool::google_search();
    if let Tool::GoogleSearchRetrieval { google_search_retrieval } = tool {
        assert!(google_search_retrieval.disable_attribution.is_none());
    } else {
        panic!("Expected Google Search retrieval tool");
    }

    let custom_config = GroundingConfig::without_attribution();
    let custom_tool = Tool::google_search_with_config(custom_config);
    if let Tool::GoogleSearchRetrieval { google_search_retrieval } = custom_tool {
        assert_eq!(google_search_retrieval.disable_attribution, Some(true));
    } else {
        panic!("Expected Google Search retrieval tool");
    }
}
