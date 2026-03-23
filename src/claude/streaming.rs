use crate::claude::StreamEvent;
use crate::error::{Result, VertexError};
use crate::streaming_support::SsePayloadParser;

/// Parser for Anthropic SSE payloads returned by Vertex AI.
#[derive(Clone, Default)]
pub struct ClaudeSseParser;

impl ClaudeSseParser {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    fn parse_payload(text: &str) -> Result<Option<StreamEvent>> {
        let mut data_payload = String::new();

        for line in text.lines() {
            let line = line.trim();
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

        if data_payload == "[DONE]" {
            return Ok(None);
        }

        serde_json::from_str::<StreamEvent>(&data_payload).map(Some).map_err(|e| {
            VertexError::streaming(format!("Failed to parse Claude streaming payload: {e}"))
        })
    }
}

impl SsePayloadParser<StreamEvent> for ClaudeSseParser {
    fn parse(&self, payload: &str) -> Result<Option<StreamEvent>> {
        Self::parse_payload(payload)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::claude::{ContentBlock, MessageResponse, Role, StreamEvent};

    fn sample_message_response() -> MessageResponse {
        MessageResponse {
            id: "msg_test".to_string(),
            object_type: "message".to_string(),
            role: Role::Assistant,
            content: vec![ContentBlock::text("Hello")],
            stop_reason: None,
            stop_sequence: None,
            usage: None,
            model: None,
            created_at: None,
        }
    }

    #[test]
    fn parses_message_start_event() {
        let parser = ClaudeSseParser::new();
        let payload = format!(
            "data: {{\"type\":\"message_start\",\"message\":{}}}\n\n",
            serde_json::to_string(&sample_message_response()).unwrap()
        );

        let event = parser.parse(&payload).unwrap().unwrap();
        match event {
            StreamEvent::MessageStart { message } => {
                assert_eq!(message.id, "msg_test");
                assert_eq!(message.text(), "Hello");
            }
            other => panic!("Unexpected event parsed: {other:?}"),
        }
    }

    #[test]
    fn skips_done_marker() {
        let parser = ClaudeSseParser::new();
        let payload = "data: [DONE]\n\n";
        assert!(parser.parse(payload).unwrap().is_none());
    }

    #[test]
    fn ignores_comments_and_blank_lines() {
        let parser = ClaudeSseParser::new();
        let payload = format!(
            ": comment\n\n\n data:    {}\n\n",
            serde_json::to_string(&StreamEvent::MessageStop).unwrap()
        );

        let event = parser.parse(&payload).unwrap().unwrap();
        assert!(matches!(event, StreamEvent::MessageStop));
    }

    #[test]
    fn returns_error_for_invalid_json() {
        let parser = ClaudeSseParser::new();
        let payload = "data: {invalid json}\n\n";
        let err = parser.parse(payload).unwrap_err();
        assert!(matches!(err, VertexError::Streaming { .. }));
    }
}
