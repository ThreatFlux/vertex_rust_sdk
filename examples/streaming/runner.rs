use std::{io, pin::Pin, time::Duration};

use async_trait::async_trait;
use threatflux_vertex_rust_sdk::{
    models::{GenerateContentRequest, StreamingResponse},
    types::UsageMetadata,
    VertexClient, VertexError,
};
use tokio_stream::{Stream, StreamExt};

#[derive(Debug, Clone)]
pub struct StreamSummary {
    pub elapsed: Duration,
    pub chunk_count: usize,
    pub full_response: String,
    pub usage: Option<UsageMetadata>,
}

#[derive(Debug, thiserror::Error)]
pub enum StreamError {
    #[error("authentication failed: {0}")]
    Authentication(String),
    #[error("quota or rate limit exceeded: {0}")]
    Quota(String),
    #[error("model not found: {0}")]
    NotFound(String),
    #[error("streaming failed: {0}")]
    Transport(String),
    #[error("output failed: {0}")]
    Output(#[from] io::Error),
}

pub trait ChunkSink {
    fn handle_text(&mut self, text: &str) -> io::Result<()>;
}

#[async_trait]
pub trait ContentStreamer: Send + Sync {
    async fn stream(
        &self,
        model: &str,
        request: &GenerateContentRequest,
    ) -> Result<
        Pin<Box<dyn Stream<Item = Result<StreamingResponse, VertexError>> + Send>>,
        VertexError,
    >;
}

pub struct VertexStreamer {
    client: VertexClient,
}

impl VertexStreamer {
    pub const fn new(client: VertexClient) -> Self {
        Self { client }
    }
}

#[async_trait]
impl ContentStreamer for VertexStreamer {
    async fn stream(
        &self,
        model: &str,
        request: &GenerateContentRequest,
    ) -> Result<
        Pin<Box<dyn Stream<Item = Result<StreamingResponse, VertexError>> + Send>>,
        VertexError,
    > {
        self.client.stream_generate_content(model, request).await
    }
}

pub struct StreamingRunner<S> {
    streamer: S,
}

impl<S: ContentStreamer> StreamingRunner<S> {
    pub const fn new(streamer: S) -> Self {
        Self { streamer }
    }

    pub async fn run(
        &self,
        model: &str,
        request: &GenerateContentRequest,
        sink: &mut impl ChunkSink,
    ) -> Result<StreamSummary, StreamError> {
        let mut stream = self.streamer.stream(model, request).await.map_err(StreamError::from)?;

        let start = std::time::Instant::now();
        let mut full_response = String::new();
        let mut chunk_count = 0usize;
        let mut usage: Option<UsageMetadata> = None;

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.map_err(StreamError::from)?;
            chunk_count += 1;

            if let Some(text) = chunk.text() {
                sink.handle_text(&text)?;
                full_response.push_str(&text);
            }

            if chunk.is_final() && chunk.usage_metadata.is_some() {
                usage.clone_from(&chunk.usage_metadata);
            }
        }

        Ok(StreamSummary { elapsed: start.elapsed(), chunk_count, full_response, usage })
    }
}

impl From<VertexError> for StreamError {
    fn from(error: VertexError) -> Self {
        match error {
            VertexError::Authentication { message } => Self::Authentication(message),
            VertexError::Http { status, message } => match status {
                401 | 403 => Self::Authentication(message),
                404 => Self::NotFound(message),
                429 => Self::Quota(message),
                _ => Self::Transport(format!("HTTP {status}: {message}")),
            },
            VertexError::Api { code, message } => {
                let code_upper = code.to_ascii_uppercase();
                if code_upper.contains("NOT_FOUND") {
                    Self::NotFound(message)
                } else if code_upper.contains("RESOURCE_EXHAUSTED")
                    || code_upper.contains("RATE_LIMIT")
                    || code_upper.contains("QUOTA")
                {
                    Self::Quota(message)
                } else {
                    Self::Transport(format!("{code}: {message}"))
                }
            }
            other => Self::Transport(other.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use threatflux_vertex_rust_sdk::types::{Candidate, Content, Part, UsageMetadata};

    struct RecordingSink {
        text: String,
    }

    impl RecordingSink {
        fn new() -> Self {
            Self { text: String::new() }
        }
    }

    impl ChunkSink for RecordingSink {
        fn handle_text(&mut self, text: &str) -> io::Result<()> {
            self.text.push_str(text);
            Ok(())
        }
    }

    #[derive(Clone)]
    struct MockStreamer {
        responses: Vec<Result<StreamingResponse, VertexError>>,
    }

    #[async_trait]
    impl ContentStreamer for MockStreamer {
        async fn stream(
            &self,
            _model: &str,
            _request: &GenerateContentRequest,
        ) -> Result<
            Pin<Box<dyn Stream<Item = Result<StreamingResponse, VertexError>> + Send>>,
            VertexError,
        > {
            let stream = tokio_stream::iter(self.responses.clone());
            Ok(Box::pin(stream))
        }
    }

    fn chunk(text: &str, is_final: bool, usage: Option<UsageMetadata>) -> StreamingResponse {
        StreamingResponse {
            candidates: vec![Candidate {
                content: Content {
                    role: "model".to_string(),
                    parts: vec![Part::Text { text: text.to_string() }],
                },
                finish_reason: None,
                safety_ratings: vec![],
                index: Some(0),
            }],
            usage_metadata: if is_final { usage } else { None },
            grounding_metadata: None,
        }
    }

    fn usage(prompt: i32, response: i32, total: i32) -> UsageMetadata {
        UsageMetadata {
            prompt_token_count: prompt,
            candidates_token_count: Some(response),
            total_token_count: total,
            traffic_type: None,
            modality_token_count: None,
        }
    }

    #[tokio::test]
    async fn aggregates_successful_stream() {
        let streamer = MockStreamer {
            responses: vec![
                Ok(chunk("Hello ", false, None)),
                Ok(chunk("World", true, Some(usage(5, 7, 12)))),
            ],
        };
        let runner = StreamingRunner::new(streamer);
        let request = GenerateContentRequest::new("prompt");
        let mut sink = RecordingSink::new();

        let summary =
            runner.run("model", &request, &mut sink).await.expect("stream should succeed");

        assert_eq!(sink.text, "Hello World");
        assert_eq!(summary.chunk_count, 2);
        assert_eq!(summary.full_response, "Hello World");
        assert_eq!(summary.usage.unwrap().total_token_count, 12);
    }

    #[tokio::test]
    async fn surfaces_authentication_errors() {
        let streamer = MockStreamer {
            responses: vec![Err(VertexError::Authentication { message: "bad creds".into() })],
        };
        let runner = StreamingRunner::new(streamer);
        let request = GenerateContentRequest::new("prompt");
        let mut sink = RecordingSink::new();

        let err = runner.run("model", &request, &mut sink).await.expect_err("stream should fail");

        match err {
            StreamError::Authentication(message) => {
                assert!(message.contains("bad creds"));
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[tokio::test]
    async fn handles_empty_stream() {
        let streamer = MockStreamer { responses: vec![] };
        let runner = StreamingRunner::new(streamer);
        let request = GenerateContentRequest::new("prompt");
        let mut sink = RecordingSink::new();

        let summary =
            runner.run("model", &request, &mut sink).await.expect("empty stream should be ok");

        assert_eq!(summary.chunk_count, 0);
        assert!(summary.usage.is_none());
    }
}
