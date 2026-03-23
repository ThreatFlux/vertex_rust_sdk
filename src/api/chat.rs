//! Chat completions API

use crate::client::VertexClient;
use crate::error::{Result, VertexError};
use crate::models::{ChatMessage, GenerateContentRequest};
use crate::types::{Content, GenerationConfig};

impl VertexClient {
    /// Simple chat completion (convenience method)
    ///
    /// This is a high-level convenience method for simple chat interactions.
    /// It converts `ChatMessage` objects to the internal Content format and
    /// handles the request/response cycle.
    ///
    /// # Arguments
    ///
    /// * `model` - The model ID to use
    /// * `messages` - List of chat messages (user, assistant, system)
    ///
    /// # Example
    ///
    /// ```no_run
    /// use threatflux_vertex_rust_sdk::{config::Config, ChatMessage, VertexClient};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = Config {
    ///     project_id: "project-id".into(),
    ///     region: "us-central1".into(),
    ///     ..Config::default()
    /// };
    /// let client = VertexClient::new(config).await?;
    ///
    /// let messages = vec![
    ///     ChatMessage::system("You are a helpful assistant."),
    ///     ChatMessage::user("What is the capital of France?"),
    /// ];
    ///
    /// let response = client.chat_impl("gemini-2.0-flash-001", messages).await?;
    /// println!("Response: {}", response);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error when the Vertex API request fails or when the response
    /// does not contain any text content.
    pub async fn chat_impl(&self, model: &str, messages: Vec<ChatMessage>) -> Result<String> {
        let contents: Vec<Content> = messages.into_iter().map(Into::into).collect();
        let request = GenerateContentRequest::with_contents(contents);
        let response = self.generate_content(model, &request).await?;

        response.text().ok_or_else(|| VertexError::generic("No text content in response"))
    }

    /// Multi-turn chat with conversation management
    ///
    /// This method maintains conversation context and supports streaming responses.
    /// It's useful for building interactive chat applications.
    ///
    /// # Arguments
    ///
    /// * `model` - The model ID to use
    /// * `conversation` - The conversation context
    /// * `config` - Generation configuration
    ///
    /// # Example
    ///
    /// ```no_run
    /// use threatflux_vertex_rust_sdk::{config::Config, ChatConversation, GenerationConfig, VertexClient};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = Config {
    ///     project_id: "project-id".into(),
    ///     region: "us-central1".into(),
    ///     ..Config::default()
    /// };
    /// let client = VertexClient::new(config).await?;
    /// let mut conversation = ChatConversation::new();
    ///
    /// conversation.set_system_message("You are a helpful programming tutor.");
    /// conversation.add_user_message("How do I create a vector in Rust?");
    ///
    /// let config = GenerationConfig {
    ///     temperature: Some(0.7),
    ///     max_output_tokens: Some(1024),
    ///     ..Default::default()
    /// };
    ///
    /// let response = client.chat_with_context("gemini-2.0-flash-001", &mut conversation, &config).await?;
    /// conversation.add_assistant_message(&response);
    ///
    /// println!("Assistant: {}", response);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error when the API call fails or when the service responds
    /// without any textual content.
    pub async fn chat_with_context(
        &self,
        model: &str,
        conversation: &mut ChatConversation,
        config: &GenerationConfig,
    ) -> Result<String> {
        let request = GenerateContentRequest::with_contents(conversation.to_contents())
            .with_generation_config(config.clone());

        let response = self.generate_content(model, &request).await?;

        let text =
            response.text().ok_or_else(|| VertexError::generic("No text content in response"))?;

        // Add the response to conversation context
        conversation.add_assistant_message(&text);

        Ok(text)
    }

    /// Streaming chat completion
    ///
    /// Similar to `chat_with_context` but returns a stream for real-time responses.
    ///
    /// # Errors
    ///
    /// Fails if the request to Vertex cannot be completed or the response stream
    /// cannot be created.
    pub async fn stream_chat(
        &self,
        model: &str,
        messages: Vec<ChatMessage>,
        config: Option<GenerationConfig>,
    ) -> Result<crate::streaming::ChatStream> {
        let contents: Vec<Content> = messages.into_iter().map(Into::into).collect();
        let mut request = GenerateContentRequest::with_contents(contents);

        if let Some(config) = config {
            request = request.with_generation_config(config);
        }

        let stream = self.stream_generate_content_impl(model, &request).await?;
        Ok(crate::streaming::ChatStream::new(stream))
    }
}

/// Chat conversation manager
///
/// This struct helps manage multi-turn conversations by maintaining
/// message history and providing convenient methods for adding messages.
#[derive(Debug, Clone)]
pub struct ChatConversation {
    messages: Vec<ChatMessage>,
    system_message: Option<String>,
}

impl ChatConversation {
    /// Create a new conversation
    #[must_use]
    pub const fn new() -> Self {
        Self { messages: Vec::new(), system_message: None }
    }

    /// Create a conversation with a system message
    #[must_use]
    pub fn with_system_message(system_message: &str) -> Self {
        Self { messages: Vec::new(), system_message: Some(system_message.to_string()) }
    }

    /// Set or update the system message
    pub fn set_system_message(&mut self, message: &str) {
        self.system_message = Some(message.to_string());
    }

    /// Add a user message
    pub fn add_user_message(&mut self, message: &str) {
        self.messages.push(ChatMessage::user(message));
    }

    /// Add an assistant message
    pub fn add_assistant_message(&mut self, message: &str) {
        self.messages.push(ChatMessage::assistant(message));
    }

    /// Add a custom message with any role
    pub fn add_message(&mut self, message: ChatMessage) {
        self.messages.push(message);
    }

    /// Get all messages as Content objects (for API requests)
    #[must_use]
    pub fn to_contents(&self) -> Vec<Content> {
        let mut contents = Vec::new();

        // Add system message first if present
        if let Some(system_msg) = &self.system_message {
            contents.push(Content::system_text(system_msg));
        }

        // Add conversation messages
        for message in &self.messages {
            contents.push(message.clone().into());
        }

        contents
    }

    /// Get the number of messages (excluding system message)
    #[must_use]
    pub const fn message_count(&self) -> usize {
        self.messages.len()
    }

    /// Check if conversation has any messages
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.messages.is_empty() && self.system_message.is_none()
    }

    /// Clear all messages (keeps system message)
    pub fn clear_messages(&mut self) {
        self.messages.clear();
    }

    /// Clear everything including system message
    pub fn clear_all(&mut self) {
        self.messages.clear();
        self.system_message = None;
    }

    /// Get the last user message
    #[must_use]
    pub fn last_user_message(&self) -> Option<&str> {
        self.messages.iter().rev().find(|msg| msg.role == "user").map(|msg| msg.content.as_str())
    }

    /// Get the last assistant message
    #[must_use]
    pub fn last_assistant_message(&self) -> Option<&str> {
        self.messages
            .iter()
            .rev()
            .find(|msg| msg.role == "model" || msg.role == "assistant")
            .map(|msg| msg.content.as_str())
    }

    /// Estimate token count (rough approximation)
    #[must_use]
    pub fn estimate_tokens(&self) -> i32 {
        let total_text: String = self
            .to_contents()
            .iter()
            .flat_map(|content| &content.parts)
            .filter_map(|part| {
                if let crate::types::Part::Text { text } = part {
                    Some(text.as_str())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join(" ");

        crate::api::tokens::utils::estimate_tokens(&total_text)
    }
}

impl Default for ChatConversation {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conversation_creation() {
        let conv = ChatConversation::new();
        assert!(conv.is_empty());
        assert_eq!(conv.message_count(), 0);

        let conv_with_system = ChatConversation::with_system_message("You are helpful.");
        assert!(!conv_with_system.is_empty());
        assert_eq!(conv_with_system.message_count(), 0);
    }

    #[test]
    fn test_conversation_messages() {
        let mut conv = ChatConversation::new();
        conv.set_system_message("System prompt");
        conv.add_user_message("Hello");
        conv.add_assistant_message("Hi there!");

        assert_eq!(conv.message_count(), 2);
        assert_eq!(conv.last_user_message(), Some("Hello"));
        assert_eq!(conv.last_assistant_message(), Some("Hi there!"));

        let contents = conv.to_contents();
        assert_eq!(contents.len(), 3); // system + user + assistant
        assert_eq!(contents[0].role, "system");
        assert_eq!(contents[1].role, "user");
        assert_eq!(contents[2].role, "model");
    }

    #[test]
    fn test_conversation_clearing() {
        let mut conv = ChatConversation::with_system_message("System");
        conv.add_user_message("Test");

        assert_eq!(conv.message_count(), 1);

        conv.clear_messages();
        assert_eq!(conv.message_count(), 0);
        assert!(!conv.is_empty()); // System message still there

        conv.clear_all();
        assert!(conv.is_empty());
    }

    #[test]
    fn test_token_estimation() {
        let mut conv = ChatConversation::new();
        conv.add_user_message("Hello world");

        let estimated = conv.estimate_tokens();
        assert!(estimated > 0);
        assert!(estimated < 100); // Should be reasonable for short text
    }
}
