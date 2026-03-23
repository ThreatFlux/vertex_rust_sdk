use anyhow::Result;
use async_trait::async_trait;
use threatflux_vertex_rust_sdk::types::ThinkingLevel;

use crate::commands::{
    cache, chat, code_exec, config, demos, function_calls, generation, models, structured, system,
    tests, tokens,
};

use crate::commands::generation::{GenerationOptions, StreamOptions};

#[async_trait]
pub(super) trait CommandExecutor {
    async fn config_show(&self) -> Result<()>;
    async fn config_check(&self) -> Result<()>;
    async fn config_init(&self) -> Result<()>;

    async fn cache_create(
        &self,
        text: Option<String>,
        file: Option<String>,
        name: Option<String>,
        ttl: u64,
        system: Option<String>,
    ) -> Result<()>;
    async fn cache_list(&self, page_size: Option<i32>) -> Result<()>;
    async fn cache_get(&self, cache_id: String) -> Result<()>;
    async fn cache_delete(&self, cache_id: String) -> Result<()>;
    async fn cache_update(&self, cache_id: String, ttl: u64) -> Result<()>;

    async fn generate(&self, options: GenerationOptions) -> Result<()>;
    async fn stream_generate(&self, options: StreamOptions) -> Result<()>;

    async fn chat(&self, model: String, system: Option<String>) -> Result<()>;
    async fn tokens(&self, text: String, model: String) -> Result<()>;

    async fn test_auth(&self) -> Result<()>;
    async fn test_generate(&self) -> Result<()>;
    async fn test_stream(&self) -> Result<()>;
    async fn test_functions(&self) -> Result<()>;
    async fn test_all(&self) -> Result<()>;

    async fn models_list(&self, gemini: bool, page_size: Option<i32>) -> Result<()>;
    async fn models_get(&self, model: String) -> Result<()>;
    async fn models_locations(&self, page_size: Option<i32>) -> Result<()>;
    async fn models_test(&self, model: String, prompt: String) -> Result<()>;

    async fn functions_test(
        &self,
        prompt: String,
        model: String,
        system: Option<String>,
    ) -> Result<()>;

    async fn code_exec(
        &self,
        prompt: String,
        model: String,
        temperature: f32,
        max_output_tokens: i32,
        system: Option<String>,
    ) -> Result<()>;

    async fn code_exec_stream(
        &self,
        prompt: String,
        model: String,
        temperature: f32,
        max_output_tokens: i32,
        system: Option<String>,
    ) -> Result<()>;

    async fn system_test(&self, model: String) -> Result<()>;

    async fn structured_output(
        &self,
        prompt: String,
        model: String,
        example: String,
        schema: Option<String>,
    ) -> Result<()>;

    async fn structured_test(&self, model: String) -> Result<()>;

    async fn thinking_demo(
        &self,
        model: String,
        example: String,
        prompt: Option<String>,
        thinking_budget: Option<i32>,
        thinking_level: Option<ThinkingLevel>,
    ) -> Result<()>;

    async fn grounding_demo(
        &self,
        model: String,
        example: String,
        prompt: Option<String>,
        stream: bool,
    ) -> Result<()>;
}

pub(super) struct RealCommandExecutor;

#[async_trait]
impl CommandExecutor for RealCommandExecutor {
    async fn config_show(&self) -> Result<()> {
        config::show_config()
    }

    async fn config_check(&self) -> Result<()> {
        config::check_config().await
    }

    async fn config_init(&self) -> Result<()> {
        config::init_config()
    }

    async fn cache_create(
        &self,
        text: Option<String>,
        file: Option<String>,
        name: Option<String>,
        ttl: u64,
        system: Option<String>,
    ) -> Result<()> {
        cache::cache_create(
            text.as_deref(),
            file.as_deref(),
            name.as_deref(),
            ttl,
            system.as_deref(),
        )
        .await
    }

    async fn cache_list(&self, page_size: Option<i32>) -> Result<()> {
        cache::cache_list(page_size).await
    }

    async fn cache_get(&self, cache_id: String) -> Result<()> {
        cache::cache_get(&cache_id).await
    }

    async fn cache_delete(&self, cache_id: String) -> Result<()> {
        cache::cache_delete(&cache_id).await
    }

    async fn cache_update(&self, cache_id: String, ttl: u64) -> Result<()> {
        cache::cache_update(&cache_id, ttl).await
    }

    async fn generate(&self, options: GenerationOptions) -> Result<()> {
        generation::generate_with_options_cache_thinking_and_grounding(options).await
    }

    async fn stream_generate(&self, options: StreamOptions) -> Result<()> {
        generation::stream_generate_with_cache_thinking_and_grounding(options).await
    }

    async fn chat(&self, model: String, system: Option<String>) -> Result<()> {
        chat::chat(&model, system.as_deref()).await
    }

    async fn tokens(&self, text: String, model: String) -> Result<()> {
        tokens::count_tokens(&text, &model).await
    }

    async fn test_auth(&self) -> Result<()> {
        tests::test_auth().await
    }

    async fn test_generate(&self) -> Result<()> {
        tests::test_generate().await
    }

    async fn test_stream(&self) -> Result<()> {
        tests::test_stream().await
    }

    async fn test_functions(&self) -> Result<()> {
        tests::test_functions().await
    }

    async fn test_all(&self) -> Result<()> {
        tests::test_all().await
    }

    async fn models_list(&self, gemini: bool, page_size: Option<i32>) -> Result<()> {
        models::list_models(gemini, page_size).await
    }

    async fn models_get(&self, model: String) -> Result<()> {
        models::get_model(&model).await
    }

    async fn models_locations(&self, page_size: Option<i32>) -> Result<()> {
        models::list_locations(page_size).await
    }

    async fn models_test(&self, model: String, prompt: String) -> Result<()> {
        models::test_model(&model, &prompt).await
    }

    async fn functions_test(
        &self,
        prompt: String,
        model: String,
        system: Option<String>,
    ) -> Result<()> {
        function_calls::test_functions_with_prompt(&prompt, &model, system.as_deref()).await
    }

    async fn code_exec(
        &self,
        prompt: String,
        model: String,
        temperature: f32,
        max_output_tokens: i32,
        system: Option<String>,
    ) -> Result<()> {
        code_exec::code_exec(&prompt, &model, temperature, max_output_tokens, system.as_deref())
            .await
    }

    async fn code_exec_stream(
        &self,
        prompt: String,
        model: String,
        temperature: f32,
        max_output_tokens: i32,
        system: Option<String>,
    ) -> Result<()> {
        code_exec::code_exec_stream(
            &prompt,
            &model,
            temperature,
            max_output_tokens,
            system.as_deref(),
        )
        .await
    }

    async fn system_test(&self, model: String) -> Result<()> {
        system::system_test(&model).await
    }

    async fn structured_output(
        &self,
        prompt: String,
        model: String,
        example: String,
        schema: Option<String>,
    ) -> Result<()> {
        structured::structured_output(&prompt, &model, &example, schema.as_deref()).await
    }

    async fn structured_test(&self, model: String) -> Result<()> {
        structured::structured_test(&model).await
    }

    async fn thinking_demo(
        &self,
        model: String,
        example: String,
        prompt: Option<String>,
        thinking_budget: Option<i32>,
        thinking_level: Option<ThinkingLevel>,
    ) -> Result<()> {
        demos::thinking_demo(&model, &example, prompt.as_deref(), thinking_budget, thinking_level)
            .await
    }

    async fn grounding_demo(
        &self,
        model: String,
        example: String,
        prompt: Option<String>,
        stream: bool,
    ) -> Result<()> {
        demos::grounding_demo(&model, &example, prompt.as_deref(), stream).await
    }
}
