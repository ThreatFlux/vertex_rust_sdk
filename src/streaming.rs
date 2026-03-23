//! Server-Sent Events (SSE) streaming support

use crate::error::{Result, VertexError};
use crate::models::StreamingResponse;
use crate::streaming_support::SsePayloadParser;
use futures_util::stream::{Stream, StreamExt};
use std::pin::Pin;
use std::task::{Context, Poll};

/// Server-Sent Events parser for Vertex AI streaming responses
#[derive(Clone, Default)]
pub struct SseParser;

impl SseParser {
    /// Create a new SSE parser
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Parse a single SSE chunk
    ///
    /// # Errors
    ///
    /// Returns an error when the payload cannot be parsed as a valid streaming
    /// response.
    ///
    /// SSE format from Vertex AI:
    /// ```text
    /// data: {"candidates":[{"content":{"role":"model","parts":[{"text":"Hello"}]},"index":0}]}
    ///
    /// data: {"candidates":[{"content":{"role":"model","parts":[{"text":" world"}]},"index":0}]}
    ///
    /// data: {"candidates":[{"content":{"role":"model","parts":[{"text":"!"}]},"finishReason":"STOP","index":0}],"usageMetadata":{"promptTokenCount":2,"candidatesTokenCount":3,"totalTokenCount":5}}
    /// ```
    pub fn parse_chunk(&self, text: &str) -> Result<Option<StreamingResponse>> {
        let mut data_payload = String::new();

        // Process each line in the chunk and gather data payload
        for line in text.lines() {
            let line = line.trim();

            // Skip empty lines and comments
            if line.is_empty() || line.starts_with(':') {
                continue;
            }

            if let Some(data) = line.strip_prefix("data:") {
                let data = data.trim_start();

                if data.is_empty() {
                    continue;
                }

                if !data_payload.is_empty() {
                    data_payload.push('\n');
                }

                data_payload.push_str(data);
            }
        }

        if data_payload.is_empty() {
            return Ok(None);
        }

        // Handle end-of-stream markers
        if data_payload == "[DONE]" {
            return Ok(None);
        }

        // Try to parse as JSON
        match serde_json::from_str::<StreamingResponse>(&data_payload) {
            Ok(response) => Ok(Some(response)),
            Err(e) => {
                // Log the problematic data for debugging
                log::warn!("Failed to parse SSE data: {data_payload} - Error: {e}");
                Err(VertexError::streaming(format!("Failed to parse streaming response: {e}")))
            }
        }
    }
}

impl SsePayloadParser<StreamingResponse> for SseParser {
    fn parse(&self, payload: &str) -> Result<Option<StreamingResponse>> {
        self.parse_chunk(payload)
    }
}

/// A stream wrapper specifically for chat responses
///
/// This provides a more convenient interface for handling streaming chat responses,
/// with text accumulation and proper error handling.
pub struct ChatStream {
    inner: Pin<Box<dyn Stream<Item = Result<StreamingResponse>> + Send>>,
    accumulated_text: String,
}

impl ChatStream {
    /// Create a new chat stream
    #[must_use]
    pub fn new(stream: Pin<Box<dyn Stream<Item = Result<StreamingResponse>> + Send>>) -> Self {
        Self { inner: stream, accumulated_text: String::new() }
    }

    /// Get the accumulated text so far
    #[must_use]
    pub fn accumulated_text(&self) -> &str {
        &self.accumulated_text
    }

    /// Reset the accumulated text
    pub fn clear_accumulated(&mut self) {
        self.accumulated_text.clear();
    }
}

impl Stream for ChatStream {
    type Item = Result<ChatStreamChunk>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.inner.poll_next_unpin(cx) {
            Poll::Ready(Some(Ok(mut response))) => {
                let text = response.text().unwrap_or_default();
                self.accumulated_text.push_str(&text);

                let chunk = ChatStreamChunk {
                    text,
                    is_final: response.is_final(),
                    usage_metadata: response.usage_metadata.take(),
                };

                Poll::Ready(Some(Ok(chunk)))
            }
            Poll::Ready(Some(Err(e))) => Poll::Ready(Some(Err(e))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// A chunk in a chat stream
#[derive(Debug, Clone)]
pub struct ChatStreamChunk {
    /// The text content in this chunk
    pub text: String,
    /// Whether this is the final chunk
    pub is_final: bool,
    /// Usage metadata (only present in final chunk)
    pub usage_metadata: Option<crate::types::UsageMetadata>,
}

impl ChatStreamChunk {
    /// Check if this chunk contains text
    #[must_use]
    pub const fn has_text(&self) -> bool {
        !self.text.is_empty()
    }
}

/// Helper function to collect a complete response from a stream
///
/// This is useful when you want to get the full response text but still
/// want to use the streaming API (e.g., for better error handling or
/// to show progress).
///
/// # Errors
///
/// Propagates errors from the underlying stream.
pub async fn collect_stream_response(
    mut stream: Pin<Box<dyn Stream<Item = Result<StreamingResponse>> + Send>>,
) -> Result<String> {
    let mut full_text = String::new();

    while let Some(chunk_result) = stream.next().await {
        match chunk_result {
            Ok(chunk) => {
                if let Some(text) = chunk.text() {
                    full_text.push_str(&text);
                }
            }
            Err(e) => return Err(e),
        }
    }

    Ok(full_text)
}

/// Utility to create a simple text stream from a string (for testing)
#[must_use]
pub fn create_mock_stream(
    text: &str,
) -> Pin<Box<dyn Stream<Item = Result<StreamingResponse>> + Send>> {
    use futures_util::stream;

    let chunks: Vec<Result<StreamingResponse>> = text
        .chars()
        .map(|c| {
            Ok(StreamingResponse {
                candidates: vec![crate::types::Candidate {
                    content: crate::types::Content {
                        role: "model".to_string(),
                        parts: vec![crate::types::Part::Text { text: c.to_string() }],
                    },
                    finish_reason: None,
                    safety_ratings: vec![],
                    index: Some(0),
                }],
                usage_metadata: None,
                grounding_metadata: None,
            })
        })
        .collect();

    Box::pin(stream::iter(chunks))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_stream::StreamExt;

    #[test]
    fn test_sse_parser() {
        let parser = SseParser::new();

        // Test valid data line
        let chunk = r#"data: {"candidates":[{"content":{"role":"model","parts":[{"text":"Hello"}]},"index":0}]}"#;
        let result = parser.parse_chunk(chunk).unwrap();
        assert!(result.is_some());

        let response = result.unwrap();
        assert_eq!(response.text(), Some("Hello".to_string()));
    }

    #[test]
    fn test_sse_parser_empty_line() {
        let parser = SseParser::new();
        let result = parser.parse_chunk("").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_sse_parser_done_marker() {
        let parser = SseParser::new();
        let result = parser.parse_chunk("data: [DONE]").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_sse_parser_no_space_after_prefix() {
        let parser = SseParser::new();
        let chunk = r#"data:{"candidates":[{"content":{"role":"model","parts":[{"text":"Hi"}]},"index":0}]}"#;
        let result = parser.parse_chunk(chunk).unwrap();
        assert!(result.is_some());
    }

    #[test]
    fn test_sse_parser_multiline() {
        let parser = SseParser::new();
        let chunk = r#"

data: {"candidates":[{"content":{"role":"model","parts":[{"text":"Hello"}]},"index":0}]}

"#;
        let result = parser.parse_chunk(chunk).unwrap();
        assert!(result.is_some());
    }

    #[test]
    fn test_sse_parser_with_usage_metadata() {
        let parser = SseParser::new();
        // Test with usage metadata (final chunk)
        let chunk = r#"data: {"candidates":[{"content":{"role":"model","parts":[{"text":"Done"}]},"finishReason":"STOP","index":0}],"usageMetadata":{"promptTokenCount":10,"candidatesTokenCount":5,"totalTokenCount":15}}"#;
        let result = parser.parse_chunk(chunk).unwrap();
        assert!(result.is_some());

        let response = result.unwrap();
        assert_eq!(response.text(), Some("Done".to_string()));
        assert!(response.usage_metadata.is_some());
        let usage = response.usage_metadata.unwrap();
        assert_eq!(usage.prompt_token_count, 10);
        assert_eq!(usage.candidates_token_count, Some(5));
        assert_eq!(usage.total_token_count, 15);
    }

    #[test]
    fn test_sse_parser_with_traffic_type() {
        let parser = SseParser::new();
        // Test with trafficType only (intermediate chunk)
        let chunk = r#"data: {"candidates":[{"content":{"role":"model","parts":[{"text":"Test"}]},"index":0}],"usageMetadata":{"trafficType":"ON_DEMAND"}}"#;
        let result = parser.parse_chunk(chunk).unwrap();
        assert!(result.is_some());

        let response = result.unwrap();
        assert_eq!(response.text(), Some("Test".to_string()));
        assert!(response.usage_metadata.is_some());
        let usage = response.usage_metadata.unwrap();
        assert_eq!(usage.traffic_type, Some("ON_DEMAND".to_string()));
        assert_eq!(usage.prompt_token_count, 0); // default value
    }

    #[test]
    fn test_sse_parser_comment_line() {
        let parser = SseParser::new();
        // SSE comments start with ':'
        let chunk = ": this is a comment\ndata: {\"candidates\":[{\"content\":{\"role\":\"model\",\"parts\":[{\"text\":\"Hi\"}]},\"index\":0}]}";
        let result = parser.parse_chunk(chunk).unwrap();
        assert!(result.is_some());
    }

    #[test]
    fn test_sse_parser_multiple_data_lines() {
        let parser = SseParser::new();
        // Multiple data lines should be concatenated
        let chunk = "data: {\"test\":\n data: \"value\"}";
        let result = parser.parse_chunk(chunk);
        // This will fail to parse as valid JSON, which is expected
        assert!(result.is_err());
    }

    #[test]
    fn test_sse_parser_invalid_json() {
        let parser = SseParser::new();
        let chunk = "data: {invalid json}";
        let result = parser.parse_chunk(chunk);
        assert!(result.is_err());
    }

    #[test]
    fn test_sse_parser_empty_data_field() {
        let parser = SseParser::new();
        let chunk = "data: \n\n";
        let result = parser.parse_chunk(chunk).unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_collect_stream_response() {
        let stream = create_mock_stream("Hello");
        let result = collect_stream_response(stream).await.unwrap();
        assert_eq!(result, "Hello");
    }

    #[tokio::test]
    async fn test_collect_stream_response_multiple_chunks() {
        let stream = create_mock_stream("Hello world!");
        let result = collect_stream_response(stream).await.unwrap();
        assert_eq!(result, "Hello world!");
    }

    #[tokio::test]
    async fn test_chat_stream() {
        let stream = create_mock_stream("Hi");
        let mut chat_stream = ChatStream::new(stream);

        let mut collected_text = String::new();
        while let Some(chunk_result) = chat_stream.next().await {
            let chunk = chunk_result.unwrap();
            collected_text.push_str(&chunk.text);
        }

        assert_eq!(collected_text, "Hi");
        assert_eq!(chat_stream.accumulated_text(), "Hi");
    }

    #[tokio::test]
    async fn test_chat_stream_accumulated_text() {
        let stream = create_mock_stream("Hello world");
        let mut chat_stream = ChatStream::new(stream);

        let mut chunk_count = 0;
        while let Some(chunk_result) = chat_stream.next().await {
            chunk_result.unwrap();
            chunk_count += 1;
        }

        assert_eq!(chunk_count, 11); // "Hello world" = 11 chars
        assert_eq!(chat_stream.accumulated_text(), "Hello world");
    }

    #[tokio::test]
    async fn test_chat_stream_clear_accumulated() {
        let stream = create_mock_stream("Test");
        let mut chat_stream = ChatStream::new(stream);

        // Consume first chunk
        chat_stream.next().await;
        assert!(!chat_stream.accumulated_text().is_empty());

        // Clear accumulated text
        chat_stream.clear_accumulated();
        assert_eq!(chat_stream.accumulated_text(), "");
    }

    #[test]
    fn test_chat_stream_chunk() {
        let chunk =
            ChatStreamChunk { text: "Hello".to_string(), is_final: false, usage_metadata: None };

        assert!(chunk.has_text());
        assert!(!chunk.is_final);

        let empty_chunk =
            ChatStreamChunk { text: String::new(), is_final: true, usage_metadata: None };

        assert!(!empty_chunk.has_text());
        assert!(empty_chunk.is_final);
    }

    #[test]
    fn test_chat_stream_chunk_with_usage() {
        let usage = crate::types::UsageMetadata {
            prompt_token_count: 10,
            candidates_token_count: Some(5),
            total_token_count: 15,
            traffic_type: Some("ON_DEMAND".to_string()),
            modality_token_count: None,
        };

        let chunk = ChatStreamChunk {
            text: "Final".to_string(),
            is_final: true,
            usage_metadata: Some(usage),
        };

        assert!(chunk.has_text());
        assert!(chunk.is_final);
        assert!(chunk.usage_metadata.is_some());

        let chunk_usage = chunk.usage_metadata.unwrap();
        assert_eq!(chunk_usage.prompt_token_count, 10);
        assert_eq!(chunk_usage.total_token_count, 15);
    }

    #[test]
    fn test_sse_parser_clone() {
        let parser1 = SseParser::new();
        let parser2 = parser1.clone();

        let chunk = r#"data: {"candidates":[{"content":{"role":"model","parts":[{"text":"Test"}]},"index":0}]}"#;
        let result1 = parser1.parse_chunk(chunk).unwrap();
        let result2 = parser2.parse_chunk(chunk).unwrap();

        assert!(result1.is_some());
        assert!(result2.is_some());
    }
}
