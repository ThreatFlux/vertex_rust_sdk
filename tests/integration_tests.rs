//! Integration tests for Vertex Rust SDK
//!
//! Note: These tests require valid Google Cloud credentials to run.
//! Set `GOOGLE_CLOUD_PROJECT` environment variable and ensure you have
//! authenticated with gcloud or set `GOOGLE_APPLICATION_CREDENTIALS`.

use std::process::Command;
use threatflux_vertex_rust_sdk::{
    // auth::AccessTokenProvider, // No longer available
    claude::{MessageRequest, StreamEvent},
    Content,
    CountTokensRequest,
    GenerateContentRequest,
    GenerationConfig,
    VertexClient,
};

fn env_var(name: &str) -> Option<String> {
    std::env::var(name).ok().map(|value| value.trim().to_string()).filter(|v| !v.is_empty())
}

fn anthropic_test_config() -> (String, String, Vec<String>) {
    let project = env_var("VERTEX_ANTHROPIC_PROJECT")
        .or_else(|| env_var("VERTEX_PROJECT_ID"))
        .or_else(|| env_var("VERTEX_PROJECT"))
        .or_else(|| env_var("GOOGLE_CLOUD_PROJECT"))
        .expect("Set VERTEX_ANTHROPIC_PROJECT or GOOGLE_CLOUD_PROJECT for Anthropic tests");

    let location = env_var("VERTEX_ANTHROPIC_LOCATION").unwrap_or_else(|| "global".to_string());

    let models_env = env_var("VERTEX_ANTHROPIC_MODELS")
        .unwrap_or_else(|| "haiku-4.5,sonnet-4.5,opus-4.1".to_string());

    let models = models_env.split(',').filter_map(resolve_model_identifier).collect::<Vec<_>>();

    (project, location, models)
}

fn vertex_project_config() -> (String, String) {
    let project = env_var("VERTEX_PROJECT_ID")
        .or_else(|| env_var("VERTEX_PROJECT"))
        .or_else(|| env_var("GOOGLE_CLOUD_PROJECT"))
        .expect("Set VERTEX_PROJECT_ID or GOOGLE_CLOUD_PROJECT for CLI tests");

    let region = env_var("VERTEX_REGION")
        .or_else(|| env_var("GCP_REGION"))
        .or_else(|| env_var("GOOGLE_CLOUD_REGION"))
        .unwrap_or_else(|| "us-central1".to_string());

    (project, region)
}

fn model_override_key(raw: &str) -> String {
    let mut key = String::from("VERTEX_ANTHROPIC_MODEL_");
    for ch in raw.chars() {
        let mapped = match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' => ch.to_ascii_uppercase(),
            _ => '_',
        };
        key.push(mapped);
    }
    key
}

fn resolve_model_identifier(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    // Allow fully qualified identifiers as-is
    if trimmed.contains('@') || trimmed.starts_with("publishers/") {
        return Some(trimmed.to_string());
    }

    // Check for explicit environment override per short name
    let override_key = model_override_key(trimmed);
    if let Some(value) = env_var(&override_key) {
        return Some(value);
    }

    let mut normalized = trimmed.to_ascii_lowercase();
    if let Some(stripped) = normalized.strip_prefix("claude-") {
        normalized = stripped.to_string();
    }
    normalized = normalized.replace('.', "-");

    let default = match normalized.as_str() {
        "haiku-4-5" | "haiku-45" => Some("claude-haiku-4-5"),
        "sonnet-4-5" | "sonnet-45" => Some("claude-sonnet-4-5"),
        "opus-4-1" | "opus-41" => Some("claude-opus-4-1"),
        _ => None,
    };

    if let Some(mapped) = default {
        return Some(mapped.to_string());
    }

    Some(trimmed.to_string())
}

fn cli_alias_for_model(model: &str) -> String {
    if model.contains("claude-haiku-4-5") {
        "haiku-4.5".to_string()
    } else if model.contains("claude-sonnet-4-5") {
        "sonnet-4.5".to_string()
    } else if model.contains("claude-opus-4-1") {
        "opus-4.1".to_string()
    } else {
        model.to_string()
    }
}

/// Test client creation with default authentication
#[tokio::test]
#[ignore = "requires real credentials"]
async fn test_client_creation() {
    let project_id = std::env::var("GOOGLE_CLOUD_PROJECT")
        .expect("Set GOOGLE_CLOUD_PROJECT for integration tests");

    let client = VertexClient::new_legacy(&project_id, "us-central1").await;
    assert!(client.is_ok());
}

/// Test client creation with builder pattern
// Commented out - AccessTokenProvider no longer available
// #[tokio::test]
// async fn test_client_builder() {
//     let auth_provider = Box::new(AccessTokenProvider::new("test-token".to_string()));
//
//     let client = VertexClientBuilder::new("test-project", "us-central1")
//         .with_auth_provider(auth_provider)
//         .build()
//         .await;
//
//     assert!(client.is_ok());
//     let client = client.unwrap();
//     assert_eq!(client.project_id(), "test-project");
//     assert_eq!(client.location(), "us-central1");
// }
/// Test basic content generation
#[tokio::test]
#[ignore = "requires real credentials"]
async fn test_generate_content() {
    let project_id = std::env::var("GOOGLE_CLOUD_PROJECT")
        .expect("Set GOOGLE_CLOUD_PROJECT for integration tests");

    let client = VertexClient::new_legacy(&project_id, "us-central1")
        .await
        .expect("Failed to create client");

    let config = GenerationConfig {
        temperature: Some(0.1),
        max_output_tokens: Some(100),
        ..GenerationConfig::default()
    };

    let request = GenerateContentRequest::new("What is 2 + 2?").with_generation_config(config);

    let response = client.generate_content("gemini-2.0-flash-001", &request).await;
    assert!(response.is_ok());

    let response = response.unwrap();
    assert!(!response.candidates.is_empty());
    assert!(response.text().is_some());
}

/// Test token counting
#[tokio::test]
#[ignore = "requires real credentials"]
async fn test_count_tokens() {
    let project_id = std::env::var("GOOGLE_CLOUD_PROJECT")
        .expect("Set GOOGLE_CLOUD_PROJECT for integration tests");

    let client = VertexClient::new_legacy(&project_id, "us-central1")
        .await
        .expect("Failed to create client");

    let request = CountTokensRequest::new("Hello world");
    let response = client.count_tokens("gemini-2.0-flash-001", &request).await;

    assert!(response.is_ok());
    let response = response.unwrap();
    assert!(response.total_tokens > 0);
}

/// Test chat functionality
// Commented out - .chat() method no longer available
// #[tokio::test]
// #[ignore] // Requires real credentials
// async fn test_chat() {
//     let project_id = std::env::var("GOOGLE_CLOUD_PROJECT")
//         .expect("Set GOOGLE_CLOUD_PROJECT for integration tests");
//
//     let client = VertexClient::new_legacy(&project_id, "us-central1")
//         .await
//         .expect("Failed to create client");
//
//     let messages = vec![
//         ChatMessage::system("You are a helpful assistant. Keep responses brief."),
//         ChatMessage::user("What is the capital of France?"),
//     ];
//
//     let response = client.chat("gemini-2.0-flash-001", messages).await;
//     assert!(response.is_ok());
//
//     let response = response.unwrap();
//     assert!(!response.is_empty());
//     assert!(response.to_lowercase().contains("paris"));
// }
/// Test streaming content generation
#[tokio::test]
#[ignore = "requires real credentials"]
async fn test_streaming_generation() {
    use tokio_stream::StreamExt;

    let project_id = std::env::var("GOOGLE_CLOUD_PROJECT")
        .expect("Set GOOGLE_CLOUD_PROJECT for integration tests");

    let client = VertexClient::new_legacy(&project_id, "us-central1")
        .await
        .expect("Failed to create client");

    let config = GenerationConfig {
        temperature: Some(0.1),
        max_output_tokens: Some(50),
        ..GenerationConfig::default()
    };

    let request = GenerateContentRequest::new("Count from 1 to 5").with_generation_config(config);

    let stream = client.stream_generate_content("gemini-2.0-flash-001", &request).await;
    assert!(stream.is_ok());

    let mut stream = stream.unwrap();
    let mut chunks_received = 0;
    let mut has_content = false;

    while let Some(chunk_result) = stream.next().await {
        match chunk_result {
            Ok(chunk) => {
                chunks_received += 1;
                if chunk.text().is_some() {
                    has_content = true;
                }

                // Break after a few chunks to avoid long test
                if chunks_received >= 5 {
                    break;
                }
            }
            Err(e) => {
                panic!("Streaming error: {e}");
            }
        }
    }

    assert!(chunks_received > 0);
    assert!(has_content);
}

#[tokio::test]
#[ignore = "requires Anthropic models enabled via Vertex"]
async fn test_claude_message_integration() {
    let (project_id, location, models) = anthropic_test_config();

    let client = VertexClient::new_legacy(&project_id, &location)
        .await
        .expect("Failed to create Vertex client for Anthropic models");

    for model in models {
        let request = MessageRequest::new()
            .max_tokens(128)
            .system("You are a terse assistant. Reply with 'ACK' only.")
            .add_user_message("Acknowledge this message.");

        let response = client
            .claude_message(&model, &request)
            .await
            .unwrap_or_else(|e| panic!("Claude message call failed for {model}: {e}"));

        assert!(response.text().to_ascii_uppercase().contains("ACK"));
    }
}

#[tokio::test]
#[ignore = "requires Anthropic models enabled via Vertex"]
async fn test_claude_stream_integration() {
    use futures_util::StreamExt;

    let (project_id, location, models) = anthropic_test_config();

    let client = VertexClient::new_legacy(&project_id, &location)
        .await
        .expect("Failed to create Vertex client for Anthropic models");

    for model in models {
        let request = MessageRequest::new()
            .max_tokens(128)
            .system("Respond with a single color word.")
            .add_user_message("Name one color.");

        let mut stream = client
            .claude_stream(&model, &request)
            .await
            .unwrap_or_else(|e| panic!("Claude streaming call failed for {model}: {e}"));

        let mut received = String::new();
        let mut events = 0usize;

        while let Some(event) = stream.next().await {
            let event = event.expect("stream event error");
            match event {
                StreamEvent::ContentBlockDelta { delta, .. } => {
                    if let Some(text) = delta.text {
                        received.push_str(&text);
                        if !received.trim().is_empty() {
                            break;
                        }
                    }
                }
                StreamEvent::MessageStop => break,
                _ => {}
            }

            events += 1;
            if events > 100 {
                break;
            }
        }

        assert!(received.trim().len() >= 3, "Expected color-like response for {model}");
    }
}

#[test]
#[ignore = "requires configured Vertex project and credentials"]
fn test_vertex_cli_stream_gemini() {
    let (project_id, region) = vertex_project_config();

    let output = Command::new(env!("CARGO_BIN_EXE_vertex-test"))
        .env("VERTEX_PROJECT_ID", &project_id)
        .env("VERTEX_REGION", &region)
        .env("NO_COLOR", "1")
        .args(["stream", "--prompt", "Count from 1 to 10", "--model", "gemini-3-pro-preview"])
        .output()
        .expect("Failed to run vertex-test stream command");

    assert!(output.status.success(), "CLI exited with failure");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Streaming response"), "Expected streaming header in output: {stdout}");
}

#[test]
#[ignore = "requires Anthropic models enabled via Vertex"]
fn test_vertex_cli_stream_claude_models() {
    let (project_id, location, models) = anthropic_test_config();
    assert!(!models.is_empty(), "Provide at least one Anthropic model");

    for model in models {
        let cli_model = cli_alias_for_model(&model);

        let output = Command::new(env!("CARGO_BIN_EXE_vertex-test"))
            .env("VERTEX_PROJECT_ID", &project_id)
            .env("VERTEX_REGION", &location)
            .env("NO_COLOR", "1")
            .args(["stream", "--prompt", "Count from 1 to 10", "--model", &cli_model])
            .output()
            .expect("Failed to run vertex-test stream command for Claude");

        assert!(output.status.success(), "CLI exited with failure for model {model}");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("Streaming response"),
            "Expected streaming header for model {model} in output: {stdout}"
        );
    }
}

/// Test error handling for invalid model
#[tokio::test]
#[ignore = "requires real credentials"]
async fn test_invalid_model_error() {
    let project_id = std::env::var("GOOGLE_CLOUD_PROJECT")
        .expect("Set GOOGLE_CLOUD_PROJECT for integration tests");

    let client = VertexClient::new_legacy(&project_id, "us-central1")
        .await
        .expect("Failed to create client");

    let request = GenerateContentRequest::new("Hello");
    let response = client.generate_content("invalid-model-name", &request).await;

    assert!(response.is_err());

    let error = response.unwrap_err();
    assert!(error.is_http() || error.is_api());
}

/// Test content types
#[test]
fn test_content_creation() {
    let user_content = Content::user_text("Hello");
    assert_eq!(user_content.role, "user");
    assert_eq!(user_content.parts.len(), 1);

    let model_content = Content::model_text("Hi there");
    assert_eq!(model_content.role, "model");

    let system_content = Content::system_text("You are helpful");
    assert_eq!(system_content.role, "system");
}

/// Test generation config
#[test]
fn test_generation_config() {
    let config = GenerationConfig::default();
    assert_eq!(config.temperature, Some(0.7));
    assert_eq!(config.max_output_tokens, Some(2048));
    assert_eq!(config.candidate_count, Some(1));

    let custom_config = GenerationConfig {
        temperature: Some(0.1),
        max_output_tokens: Some(100),
        top_k: Some(10),
        ..GenerationConfig::default()
    };
    assert_eq!(custom_config.temperature, Some(0.1));
    assert_eq!(custom_config.max_output_tokens, Some(100));
    assert_eq!(custom_config.top_k, Some(10));
}
