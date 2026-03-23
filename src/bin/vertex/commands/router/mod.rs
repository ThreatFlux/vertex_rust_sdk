use anyhow::Result;

use crate::cli::{
    CacheCommands, Cli, Commands, ConfigCommands, ModelsCommands, TestCommands, ThinkingLevelArg,
};

use crate::commands::generation::{GenerationOptions, StreamOptions};
use threatflux_vertex_rust_sdk::types::ThinkingLevel;

mod executor;
#[cfg(test)]
mod tests;

use executor::{CommandExecutor, RealCommandExecutor};

/// Routes parsed CLI commands to their handlers while keeping the public `run`
/// entry point small and focused.
pub async fn run(cli: Cli) -> Result<()> {
    CommandRouter::new(RealCommandExecutor).run(cli).await
}

struct CommandRouter<E> {
    executor: E,
}

impl<E> CommandRouter<E>
where
    E: CommandExecutor + Send + Sync,
{
    const fn new(executor: E) -> Self {
        Self { executor }
    }

    async fn run(&self, cli: Cli) -> Result<()> {
        self.dispatch(cli.command).await
    }

    #[allow(clippy::too_many_lines)]
    async fn dispatch(&self, command: Commands) -> Result<()> {
        match command {
            Commands::Config { subcommand } => self.handle_config(subcommand).await,
            Commands::Cache { subcommand } => self.handle_cache(subcommand).await,
            Commands::Generate {
                prompt,
                model,
                stream,
                temperature,
                max_output_tokens,
                system,
                json,
                schema,
                cache,
                thinking,
                thinking_budget,
                thinking_level,
                grounding,
            } => {
                let options = build_generation_options(
                    prompt,
                    model,
                    temperature,
                    max_output_tokens,
                    system,
                    json,
                    schema,
                    cache,
                    thinking,
                    thinking_budget,
                    map_thinking_level(thinking_level),
                    grounding,
                );

                if stream {
                    self.executor.stream_generate(options.into()).await
                } else {
                    self.executor.generate(options).await
                }
            }
            Commands::Stream {
                prompt,
                model,
                temperature,
                max_output_tokens,
                system,
                thinking,
                thinking_budget,
                thinking_level,
                grounding,
            } => {
                let options = build_stream_options(
                    prompt,
                    model,
                    temperature,
                    max_output_tokens,
                    system,
                    thinking,
                    thinking_budget,
                    map_thinking_level(thinking_level),
                    grounding,
                );

                self.executor.stream_generate(options).await
            }
            Commands::Chat { model, system } => self.executor.chat(model, system).await,
            Commands::Tokens { text, model } => self.executor.tokens(text, model).await,
            Commands::Test { subcommand } => self.handle_tests(subcommand).await,
            Commands::Models { subcommand } => self.handle_models(subcommand).await,
            Commands::Functions { prompt, model, system } => {
                self.executor.functions_test(prompt, model, system).await
            }
            Commands::CodeExec {
                prompt,
                model,
                stream,
                temperature,
                max_output_tokens,
                system,
            } => {
                if stream {
                    self.executor
                        .code_exec_stream(prompt, model, temperature, max_output_tokens, system)
                        .await
                } else {
                    self.executor
                        .code_exec(prompt, model, temperature, max_output_tokens, system)
                        .await
                }
            }
            Commands::SystemTest { model } => self.executor.system_test(model).await,
            Commands::StructuredOutput { prompt, model, example, schema } => {
                self.executor.structured_output(prompt, model, example, schema).await
            }
            Commands::StructuredTest { model } => self.executor.structured_test(model).await,
            Commands::ThinkingDemo { model, example, prompt, thinking_budget, thinking_level } => {
                self.executor
                    .thinking_demo(
                        model,
                        example,
                        prompt,
                        thinking_budget,
                        map_thinking_level(thinking_level),
                    )
                    .await
            }
            Commands::GroundingDemo { model, example, prompt, stream } => {
                self.executor.grounding_demo(model, example, prompt, stream).await
            }
        }
    }

    async fn handle_config(&self, subcommand: ConfigCommands) -> Result<()> {
        match subcommand {
            ConfigCommands::Show => self.executor.config_show().await,
            ConfigCommands::Check => self.executor.config_check().await,
            ConfigCommands::Init => self.executor.config_init().await,
        }
    }

    async fn handle_cache(&self, subcommand: CacheCommands) -> Result<()> {
        match subcommand {
            CacheCommands::Create { text, file, name, ttl, system } => {
                self.executor.cache_create(text, file, name, ttl, system).await
            }
            CacheCommands::List { page_size } => self.executor.cache_list(page_size).await,
            CacheCommands::Get { cache_id } => self.executor.cache_get(cache_id).await,
            CacheCommands::Delete { cache_id } => self.executor.cache_delete(cache_id).await,
            CacheCommands::Update { cache_id, ttl } => {
                self.executor.cache_update(cache_id, ttl).await
            }
        }
    }

    async fn handle_tests(&self, subcommand: TestCommands) -> Result<()> {
        match subcommand {
            TestCommands::Auth => self.executor.test_auth().await,
            TestCommands::Generate => self.executor.test_generate().await,
            TestCommands::Stream => self.executor.test_stream().await,
            TestCommands::Functions => self.executor.test_functions().await,
            TestCommands::All => self.executor.test_all().await,
        }
    }

    async fn handle_models(&self, subcommand: ModelsCommands) -> Result<()> {
        match subcommand {
            ModelsCommands::List { gemini, page_size } => {
                self.executor.models_list(gemini, page_size).await
            }
            ModelsCommands::Get { model } => self.executor.models_get(model).await,
            ModelsCommands::Locations { page_size } => {
                self.executor.models_locations(page_size).await
            }
            ModelsCommands::Test { model, prompt } => {
                self.executor.models_test(model, prompt).await
            }
        }
    }
}

fn map_thinking_level(thinking_level: Option<ThinkingLevelArg>) -> Option<ThinkingLevel> {
    thinking_level.map(Into::into)
}

#[allow(clippy::too_many_arguments)] // convenience helper for wiring CLI args into options
const fn build_generation_options(
    prompt: String,
    model: String,
    temperature: f32,
    max_output_tokens: i32,
    system_instruction: Option<String>,
    json: bool,
    schema: Option<String>,
    cache_id: Option<String>,
    thinking: bool,
    thinking_budget: Option<i32>,
    thinking_level: Option<ThinkingLevel>,
    grounding: bool,
) -> GenerationOptions {
    GenerationOptions {
        prompt,
        model,
        temperature,
        max_output_tokens,
        system_instruction,
        json,
        schema,
        cache_id,
        thinking,
        thinking_budget,
        thinking_level,
        grounding,
    }
}

#[allow(clippy::too_many_arguments)] // convenience helper for wiring CLI args into options
const fn build_stream_options(
    prompt: String,
    model: String,
    temperature: f32,
    max_output_tokens: i32,
    system_instruction: Option<String>,
    thinking: bool,
    thinking_budget: Option<i32>,
    thinking_level: Option<ThinkingLevel>,
    grounding: bool,
) -> StreamOptions {
    StreamOptions {
        prompt,
        model,
        temperature,
        max_output_tokens,
        system_instruction,
        cache_id: None,
        thinking,
        thinking_budget,
        thinking_level,
        grounding,
    }
}
