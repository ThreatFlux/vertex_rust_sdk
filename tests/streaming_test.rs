//! End-to-end streaming tests
//!
//! These tests verify the complete streaming flow from request to response.

use threatflux_vertex_rust_sdk::{
    ChatStream, Content, FinishReason, GenerateContentRequest, GenerationConfig, Part, SseParser,
    StreamingResponse,
};
use tokio_stream::StreamExt;

#[test]
fn test_sse_parser_real_vertex_response() {
    let parser = SseParser::new();

    // Test with real Vertex AI intermediate chunk format
    let chunk = r#"data: {"candidates": [{"content": {"role": "model","parts": [{"text": "Understood! I'm here and ready.\n\nHow can I help you today? Are you testing my:\n*   **Knowledge?** Ask me a question about anything.\n*   **Ability to follow instructions?** Give"}]}}],"usageMetadata": {"trafficType": "ON_DEMAND"},"modelVersion": "gemini-2.5-flash","createTime": "2025-09-30T12:18:25.582641Z","responseId": "kcrbaPHHI_zEptQPrqzc4AI"}"#;

    let result = parser.parse_chunk(chunk);
    assert!(result.is_ok());

    let response = result.unwrap();
    assert!(response.is_some());

    let streaming_response = response.unwrap();
    assert!(streaming_response.text().is_some());
    assert!(streaming_response.usage_metadata.is_some());

    let usage = streaming_response.usage_metadata.unwrap();
    assert_eq!(usage.traffic_type, Some("ON_DEMAND".to_string()));
    assert_eq!(usage.prompt_token_count, 0); // default for intermediate chunk
}

#[test]
fn test_sse_parser_final_chunk_with_usage() {
    let parser = SseParser::new();

    // Test with final chunk that has full usage metadata
    let chunk = r#"data: {"candidates": [{"content": {"role": "model","parts": [{"text": "Done!"}]},"finishReason": "STOP","index": 0}],"usageMetadata": {"promptTokenCount": 10,"candidatesTokenCount": 5,"totalTokenCount": 15,"modalityTokenCount": {"modality.TEXT": {"promptTokenCount": 10,"candidatesTokenCount": 5,"totalTokenCount": 15}}}}"#;

    let result = parser.parse_chunk(chunk);
    assert!(result.is_ok());

    let response = result.unwrap();
    assert!(response.is_some());

    let streaming_response = response.unwrap();
    assert!(streaming_response.text().is_some());
    assert!(streaming_response.usage_metadata.is_some());

    let usage = streaming_response.usage_metadata.unwrap();
    assert_eq!(usage.prompt_token_count, 10);
    assert_eq!(usage.candidates_token_count, Some(5));
    assert_eq!(usage.total_token_count, 15);
    assert!(usage.modality_token_count.is_some());
    let text_usage =
        usage.modality_token_count.as_ref().and_then(|map| map.get("modality.TEXT")).unwrap();
    assert_eq!(text_usage.prompt_token_count, 10);
}

#[tokio::test]
async fn test_chat_stream_with_usage_metadata() {
    // Create mock stream with multiple chunks including final usage
    use futures_util::stream;
    use threatflux_vertex_rust_sdk::{Candidate, UsageMetadata};

    let chunks: Vec<Result<StreamingResponse, threatflux_vertex_rust_sdk::VertexError>> = vec![
        Ok(StreamingResponse {
            candidates: vec![Candidate {
                content: Content {
                    role: "model".to_string(),
                    parts: vec![Part::Text { text: "Hello ".to_string() }],
                },
                finish_reason: None,
                safety_ratings: vec![],
                index: Some(0),
            }],
            usage_metadata: Some(UsageMetadata {
                prompt_token_count: 0,
                candidates_token_count: None,
                total_token_count: 0,
                traffic_type: Some("ON_DEMAND".to_string()),
                modality_token_count: None,
            }),
            grounding_metadata: None,
        }),
        Ok(StreamingResponse {
            candidates: vec![Candidate {
                content: Content {
                    role: "model".to_string(),
                    parts: vec![Part::Text { text: "world!".to_string() }],
                },
                finish_reason: Some(FinishReason::Stop),
                safety_ratings: vec![],
                index: Some(0),
            }],
            usage_metadata: Some(UsageMetadata {
                prompt_token_count: 5,
                candidates_token_count: Some(3),
                total_token_count: 8,
                traffic_type: Some("ON_DEMAND".to_string()),
                modality_token_count: None,
            }),
            grounding_metadata: None,
        }),
    ];

    let stream = Box::pin(stream::iter(chunks));
    let mut chat_stream = ChatStream::new(stream);

    let mut full_text = String::new();
    let mut final_usage = None;

    while let Some(chunk_result) = chat_stream.next().await {
        let chunk = chunk_result.unwrap();
        full_text.push_str(&chunk.text);

        if chunk.is_final {
            final_usage = chunk.usage_metadata;
        }
    }

    assert_eq!(full_text, "Hello world!");
    assert_eq!(chat_stream.accumulated_text(), "Hello world!");
    assert!(final_usage.is_some());

    let usage = final_usage.unwrap();
    assert_eq!(usage.prompt_token_count, 5);
    assert_eq!(usage.candidates_token_count, Some(3));
    assert_eq!(usage.total_token_count, 8);
}

#[test]
fn test_streaming_response_helper_methods() {
    use threatflux_vertex_rust_sdk::{Candidate, StreamingResponse, UsageMetadata};

    // Test text() method
    let response = StreamingResponse {
        candidates: vec![Candidate {
            content: Content {
                role: "model".to_string(),
                parts: vec![Part::Text { text: "Test response".to_string() }],
            },
            finish_reason: None,
            safety_ratings: vec![],
            index: Some(0),
        }],
        usage_metadata: None,
        grounding_metadata: None,
    };

    assert_eq!(response.text(), Some("Test response".to_string()));

    // Test is_final() method
    assert!(!response.is_final());

    let final_response = StreamingResponse {
        candidates: vec![Candidate {
            content: Content {
                role: "model".to_string(),
                parts: vec![Part::Text { text: "Final".to_string() }],
            },
            finish_reason: Some(FinishReason::Stop),
            safety_ratings: vec![],
            index: Some(0),
        }],
        usage_metadata: Some(UsageMetadata {
            prompt_token_count: 5,
            candidates_token_count: Some(3),
            total_token_count: 8,
            traffic_type: None,
            modality_token_count: None,
        }),
        grounding_metadata: None,
    };

    assert!(final_response.is_final());
}

#[test]
fn test_generate_content_request_for_streaming() {
    let request = GenerateContentRequest {
        contents: vec![Content::user_text("Hello, AI!")],
        system_instruction: None,
        generation_config: Some(GenerationConfig {
            temperature: Some(0.9),
            max_output_tokens: Some(8192),
            top_p: Some(0.95),
            top_k: Some(40),
            stop_sequences: None,
            candidate_count: Some(1),
            response_mime_type: None,
            response_schema: None,
            thinking_config: None,
        }),
        safety_settings: None,
        tools: None,
        tool_config: None,
        cached_content: None,
        metadata: None,
    };

    // Verify request can be serialized (required for HTTP request)
    let json = serde_json::to_string(&request);
    assert!(json.is_ok());

    let json_str = json.unwrap();
    assert!(json_str.contains("Hello, AI!"));
    assert!(json_str.contains("temperature"));
}

#[tokio::test]
async fn test_chat_stream_error_handling() {
    use futures_util::stream;
    use threatflux_vertex_rust_sdk::VertexError;

    // Create stream with an error
    let chunks: Vec<Result<StreamingResponse, VertexError>> = vec![
        Ok(StreamingResponse {
            candidates: vec![],
            usage_metadata: None,
            grounding_metadata: None,
        }),
        Err(VertexError::streaming("Test error".to_string())),
    ];

    let stream = Box::pin(stream::iter(chunks));
    let mut chat_stream = ChatStream::new(stream);

    // First chunk should succeed
    let first = chat_stream.next().await;
    assert!(first.is_some());
    assert!(first.unwrap().is_ok());

    // Second chunk should error
    let second = chat_stream.next().await;
    assert!(second.is_some());
    assert!(second.unwrap().is_err());
}

#[test]
fn test_sse_multiple_events() {
    let parser = SseParser::new();

    // Simulate multiple SSE events (should only parse first one per call)
    let chunk = "data: {\"candidates\":[{\"content\":{\"role\":\"model\",\"parts\":[{\"text\":\"First\"}]},\"index\":0}]}\n\ndata: {\"candidates\":[{\"content\":{\"role\":\"model\",\"parts\":[{\"text\":\"Second\"}]},\"index\":0}]}";

    for event in chunk.split("\n\n") {
        let result = parser.parse_chunk(event);
        assert!(result.is_ok());
    }
}

#[tokio::test]
async fn test_empty_stream() {
    use futures_util::stream;

    let chunks: Vec<Result<StreamingResponse, threatflux_vertex_rust_sdk::VertexError>> = vec![];
    let stream = Box::pin(stream::iter(chunks));
    let mut chat_stream = ChatStream::new(stream);

    let result = chat_stream.next().await;
    assert!(result.is_none());
    assert_eq!(chat_stream.accumulated_text(), "");
}
