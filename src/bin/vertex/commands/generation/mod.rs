use anyhow::Result;
use threatflux_vertex_rust_sdk::types::ThinkingLevel;

mod builder;
mod executor;
mod printer;

#[derive(Clone, Debug, PartialEq)]
pub struct GenerationOptions {
    pub prompt: String,
    pub model: String,
    pub temperature: f32,
    pub max_output_tokens: i32,
    pub system_instruction: Option<String>,
    pub json: bool,
    pub schema: Option<String>,
    pub cache_id: Option<String>,
    pub thinking: bool,
    pub thinking_budget: Option<i32>,
    pub thinking_level: Option<ThinkingLevel>,
    pub grounding: bool,
}

impl GenerationOptions {
    #[must_use]
    pub fn basic(
        prompt: &str,
        model: &str,
        temperature: f32,
        max_output_tokens: i32,
        system_instruction: Option<&str>,
    ) -> Self {
        Self {
            prompt: prompt.to_string(),
            model: model.to_string(),
            temperature,
            max_output_tokens,
            system_instruction: system_instruction.map(std::borrow::ToOwned::to_owned),
            json: false,
            schema: None,
            cache_id: None,
            thinking: false,
            thinking_budget: None,
            thinking_level: None,
            grounding: false,
        }
    }

    pub const fn thinking_requested(&self) -> bool {
        self.thinking || self.thinking_budget.is_some() || self.thinking_level.is_some()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct StreamOptions {
    pub prompt: String,
    pub model: String,
    pub temperature: f32,
    pub max_output_tokens: i32,
    pub system_instruction: Option<String>,
    pub cache_id: Option<String>,
    pub thinking: bool,
    pub thinking_budget: Option<i32>,
    pub thinking_level: Option<ThinkingLevel>,
    pub grounding: bool,
}

impl StreamOptions {
    #[must_use]
    pub fn basic(
        prompt: &str,
        model: &str,
        temperature: f32,
        max_output_tokens: i32,
        system_instruction: Option<&str>,
    ) -> Self {
        Self {
            prompt: prompt.to_string(),
            model: model.to_string(),
            temperature,
            max_output_tokens,
            system_instruction: system_instruction.map(std::borrow::ToOwned::to_owned),
            cache_id: None,
            thinking: false,
            thinking_budget: None,
            thinking_level: None,
            grounding: false,
        }
    }

    pub const fn thinking_requested(&self) -> bool {
        self.thinking || self.thinking_budget.is_some() || self.thinking_level.is_some()
    }
}

impl From<GenerationOptions> for StreamOptions {
    fn from(options: GenerationOptions) -> Self {
        Self {
            prompt: options.prompt,
            model: options.model,
            temperature: options.temperature,
            max_output_tokens: options.max_output_tokens,
            system_instruction: options.system_instruction,
            cache_id: options.cache_id,
            thinking: options.thinking,
            thinking_budget: options.thinking_budget,
            thinking_level: options.thinking_level,
            grounding: options.grounding,
        }
    }
}

pub async fn generate(
    prompt: &str,
    model: &str,
    temperature: f32,
    max_output_tokens: i32,
    system_instruction: Option<&str>,
) -> Result<()> {
    executor::generate(GenerationOptions::basic(
        prompt,
        model,
        temperature,
        max_output_tokens,
        system_instruction,
    ))
    .await
}

pub async fn generate_with_options_cache_thinking_and_grounding(
    options: GenerationOptions,
) -> Result<()> {
    executor::generate_with_options_cache_thinking_and_grounding(options).await
}

pub async fn stream_generate(
    prompt: &str,
    model: &str,
    temperature: f32,
    max_output_tokens: i32,
    system_instruction: Option<&str>,
) -> Result<()> {
    executor::stream_generate(StreamOptions::basic(
        prompt,
        model,
        temperature,
        max_output_tokens,
        system_instruction,
    ))
    .await
}

pub async fn stream_generate_with_cache_thinking_and_grounding(
    options: StreamOptions,
) -> Result<()> {
    executor::stream_generate_with_cache_thinking_and_grounding(options).await
}

#[allow(dead_code)] // kept for future CLI wiring to expose thinking + grounding streaming without cache
pub async fn stream_generate_with_thinking_and_grounding(options: StreamOptions) -> Result<()> {
    stream_generate_with_cache_thinking_and_grounding(options).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::command_tests::{reset_env, set_common_env, ENV_LOCK};
    use mockito::Matcher;
    use serde_json::json;

    #[tokio::test]
    #[allow(clippy::significant_drop_tightening)]
    async fn streaming_generation_consumes_sse_chunks() {
        let _guard = ENV_LOCK.lock().await;
        let mut server = mockito::Server::new_async().await;
        reset_env();
        set_common_env(&server.url());

        let path = "/v1/projects/test-project/locations/us-central1/publishers/google/models/gemini-1.5-flash:streamGenerateContent";
        let first_chunk = json!({
            "candidates": [{
                "content": {
                    "role": "model",
                    "parts": [{"text": "Hello"}]
                },
                "finishReason": "STOP",
                "safetyRatings": []
            }]
        })
        .to_string();
        let final_chunk = json!({
            "candidates": [{
                "content": {
                    "role": "model",
                    "parts": [{"text": " world"}]
                },
                "finishReason": "STOP",
                "safetyRatings": []
            }],
            "usageMetadata": {
                "promptTokenCount": 1,
                "totalTokenCount": 2
            }
        })
        .to_string();

        let stream_mock = server
            .mock("POST", path)
            .match_query(Matcher::Any)
            .match_header("authorization", "Bearer test-token")
            .expect(1)
            .with_status(200)
            .with_header("content-type", "text/event-stream")
            .with_body(format!("data: {first_chunk}\n\ndata: {final_chunk}\n\n"))
            .create();

        stream_generate("hi", "gemini-1.5-flash", 0.0, 16, None).await.unwrap();

        stream_mock.assert();
    }

    #[tokio::test]
    #[allow(clippy::significant_drop_tightening)]
    async fn generation_prints_thinking_sections() {
        let _guard = ENV_LOCK.lock().await;
        let mut server = mockito::Server::new_async().await;
        reset_env();
        set_common_env(&server.url());

        let path = "/v1/projects/test-project/locations/us-central1/publishers/google/models/gemini-1.5-flash:generateContent";
        let mock = server
            .mock("POST", path)
            .match_header("authorization", "Bearer test-token")
            .expect(1)
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                json!({
                    "candidates": [{
                        "content": {
                            "role": "model",
                            "parts": [
                                {"thought": "Step one"},
                                {"text": "Done"}
                            ]
                        },
                        "finishReason": "STOP",
                        "safetyRatings": []
                    }],
                    "usageMetadata": {
                        "promptTokenCount": 1,
                        "totalTokenCount": 2
                    }
                })
                .to_string(),
            )
            .create();

        let options = GenerationOptions {
            prompt: "say hi".to_string(),
            model: "gemini-1.5-flash".to_string(),
            temperature: 0.0,
            max_output_tokens: 32,
            system_instruction: None,
            json: false,
            schema: None,
            cache_id: None,
            thinking: true,
            thinking_budget: Some(256),
            thinking_level: None,
            grounding: false,
        };

        generate_with_options_cache_thinking_and_grounding(options).await.unwrap();

        mock.assert();
    }
}
