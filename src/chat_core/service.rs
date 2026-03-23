use crate::chat_core::config::ChatConfig;
use crate::{ChatStream, ChatStreamChunk, GenerateContentRequest, VertexClient};
use anyhow::{Context, Result};
use async_trait::async_trait;
use futures::Stream;
use futures_util::StreamExt;
use std::pin::Pin;

pub type ChatResultStream = Pin<Box<dyn Stream<Item = Result<ChatStreamChunk>> + Send + 'static>>;

#[async_trait]
pub trait ChatService {
    async fn stream_chat(
        &self,
        model: &str,
        request: GenerateContentRequest,
    ) -> Result<ChatResultStream>;
}

pub struct VertexChatService {
    client: VertexClient,
}

impl VertexChatService {
    pub async fn connect(config: &ChatConfig) -> Result<Self> {
        let client = VertexClient::new_legacy(&config.project, &config.location)
            .await
            .with_context(|| {
                "Make sure GCP_PRIVATE_KEY, GCP_CLIENT_EMAIL, and GCP_CLIENT_ID are set \
                or run `gcloud auth application-default login`"
            })?;

        Ok(Self { client })
    }

    pub const fn from_client(client: VertexClient) -> Self {
        Self { client }
    }
}

#[async_trait]
impl ChatService for VertexChatService {
    async fn stream_chat(
        &self,
        model: &str,
        request: GenerateContentRequest,
    ) -> Result<ChatResultStream> {
        let stream = self.client.stream_generate_content_impl(model, &request).await?;

        let chat_stream = ChatStream::new(stream).map(|result| result.map_err(Into::into));
        Ok(Box::pin(chat_stream))
    }
}

#[cfg(test)]
#[derive(Clone)]
pub enum MockChunk {
    Ok(ChatStreamChunk),
    Err(String),
}

#[cfg(test)]
pub struct MockChatService {
    pub chunks: Vec<MockChunk>,
}

#[cfg(test)]
impl MockChatService {
    pub fn new(chunks: Vec<MockChunk>) -> Self {
        Self { chunks }
    }
}

#[cfg(test)]
#[async_trait]
impl ChatService for MockChatService {
    async fn stream_chat(
        &self,
        _model: &str,
        _request: GenerateContentRequest,
    ) -> Result<ChatResultStream> {
        let stream =
            futures_util::stream::iter(self.chunks.clone().into_iter().map(|item| match item {
                MockChunk::Ok(chunk) => Ok(chunk),
                MockChunk::Err(msg) => Err(anyhow::anyhow!(msg)),
            }));
        Ok(Box::pin(stream))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mock_service_streams_chunks() {
        let service = MockChatService::new(vec![MockChunk::Ok(ChatStreamChunk {
            text: "hi".to_string(),
            is_final: true,
            usage_metadata: None,
        })]);

        let mut stream = service
            .stream_chat(
                "m",
                GenerateContentRequest {
                    contents: vec![],
                    system_instruction: None,
                    generation_config: None,
                    safety_settings: None,
                    tools: None,
                    tool_config: None,
                    cached_content: None,
                    metadata: None,
                },
            )
            .await
            .expect("stream builds");

        let next = stream.next().await.expect("item exists").unwrap();
        assert_eq!(next.text, "hi");
    }

    #[tokio::test]
    async fn mock_service_surfaces_errors() {
        let service = MockChatService::new(vec![MockChunk::Err("boom".to_string())]);
        let mut stream = service
            .stream_chat(
                "m",
                GenerateContentRequest {
                    contents: vec![],
                    system_instruction: None,
                    generation_config: None,
                    safety_settings: None,
                    tools: None,
                    tool_config: None,
                    cached_content: None,
                    metadata: None,
                },
            )
            .await
            .expect("stream builds");

        let err = stream.next().await.expect("item exists").unwrap_err();
        assert!(err.to_string().contains("boom"));
    }
}
