use super::executor::CommandExecutor;
use super::*;
use crate::cli::Cli;
use async_trait::async_trait;
use std::sync::{Arc, Mutex};

#[derive(Default, Clone)]
struct MockExecutor {
    calls: Arc<Mutex<Vec<String>>>,
    generations: Arc<Mutex<Vec<GenerationOptions>>>,
    streams: Arc<Mutex<Vec<StreamOptions>>>,
    thinking_levels: Arc<Mutex<Vec<Option<ThinkingLevel>>>>,
}

impl MockExecutor {
    fn push(&self, call: &str) {
        self.calls.lock().unwrap().push(call.to_string());
    }

    fn record_generation(&self, options: GenerationOptions) {
        self.generations.lock().unwrap().push(options);
    }

    fn record_stream(&self, options: StreamOptions) {
        self.streams.lock().unwrap().push(options);
    }

    fn record_thinking(&self, level: Option<ThinkingLevel>) {
        self.thinking_levels.lock().unwrap().push(level);
    }

    fn take_calls(&self) -> Vec<String> {
        self.calls.lock().unwrap().drain(..).collect()
    }

    fn take_generations(&self) -> Vec<GenerationOptions> {
        self.generations.lock().unwrap().drain(..).collect()
    }

    fn take_streams(&self) -> Vec<StreamOptions> {
        self.streams.lock().unwrap().drain(..).collect()
    }

    fn take_thinking_levels(&self) -> Vec<Option<ThinkingLevel>> {
        self.thinking_levels.lock().unwrap().drain(..).collect()
    }
}

#[async_trait]
impl CommandExecutor for MockExecutor {
    async fn config_show(&self) -> Result<()> {
        self.push("config_show");
        Ok(())
    }

    async fn config_check(&self) -> Result<()> {
        self.push("config_check");
        Ok(())
    }

    async fn config_init(&self) -> Result<()> {
        self.push("config_init");
        Ok(())
    }

    async fn cache_create(
        &self,
        text: Option<String>,
        file: Option<String>,
        name: Option<String>,
        ttl: u64,
        system: Option<String>,
    ) -> Result<()> {
        self.push("cache_create");
        let ttl_tokens = i32::try_from(ttl).unwrap_or(i32::MAX);
        #[allow(clippy::cast_precision_loss)]
        let ttl_temperature = ttl_tokens as f32;
        self.record_stream(StreamOptions {
            prompt: text.unwrap_or_default(),
            model: file.unwrap_or_default(),
            temperature: ttl_temperature,
            max_output_tokens: ttl_tokens,
            system_instruction: name,
            cache_id: None,
            thinking: system.is_some(),
            thinking_budget: None,
            thinking_level: None,
            grounding: false,
        });
        Ok(())
    }

    async fn cache_list(&self, _page_size: Option<i32>) -> Result<()> {
        self.push("cache_list");
        Ok(())
    }

    async fn cache_get(&self, _cache_id: String) -> Result<()> {
        self.push("cache_get");
        Ok(())
    }

    async fn cache_delete(&self, _cache_id: String) -> Result<()> {
        self.push("cache_delete");
        Ok(())
    }

    async fn cache_update(&self, _cache_id: String, _ttl: u64) -> Result<()> {
        self.push("cache_update");
        Ok(())
    }

    async fn generate(&self, options: GenerationOptions) -> Result<()> {
        self.push("generate");
        self.record_generation(options);
        Ok(())
    }

    async fn stream_generate(&self, options: StreamOptions) -> Result<()> {
        self.push("stream_generate");
        self.record_stream(options);
        Ok(())
    }

    async fn chat(&self, _model: String, _system: Option<String>) -> Result<()> {
        self.push("chat");
        Ok(())
    }

    async fn tokens(&self, _text: String, _model: String) -> Result<()> {
        self.push("tokens");
        Ok(())
    }

    async fn test_auth(&self) -> Result<()> {
        self.push("test_auth");
        Ok(())
    }

    async fn test_generate(&self) -> Result<()> {
        self.push("test_generate");
        Ok(())
    }

    async fn test_stream(&self) -> Result<()> {
        self.push("test_stream");
        Ok(())
    }

    async fn test_functions(&self) -> Result<()> {
        self.push("test_functions");
        Ok(())
    }

    async fn test_all(&self) -> Result<()> {
        self.push("test_all");
        Ok(())
    }

    async fn models_list(&self, _gemini: bool, _page_size: Option<i32>) -> Result<()> {
        self.push("models_list");
        Ok(())
    }

    async fn models_get(&self, _model: String) -> Result<()> {
        self.push("models_get");
        Ok(())
    }

    async fn models_locations(&self, _page_size: Option<i32>) -> Result<()> {
        self.push("models_locations");
        Ok(())
    }

    async fn models_test(&self, _model: String, _prompt: String) -> Result<()> {
        self.push("models_test");
        Ok(())
    }

    async fn functions_test(
        &self,
        _prompt: String,
        _model: String,
        _system: Option<String>,
    ) -> Result<()> {
        self.push("functions_test");
        Ok(())
    }

    async fn code_exec(
        &self,
        _prompt: String,
        _model: String,
        _temperature: f32,
        _max_output_tokens: i32,
        _system: Option<String>,
    ) -> Result<()> {
        self.push("code_exec");
        Ok(())
    }

    async fn code_exec_stream(
        &self,
        _prompt: String,
        _model: String,
        _temperature: f32,
        _max_output_tokens: i32,
        _system: Option<String>,
    ) -> Result<()> {
        self.push("code_exec_stream");
        Ok(())
    }

    async fn system_test(&self, _model: String) -> Result<()> {
        self.push("system_test");
        Ok(())
    }

    async fn structured_output(
        &self,
        _prompt: String,
        _model: String,
        _example: String,
        _schema: Option<String>,
    ) -> Result<()> {
        self.push("structured_output");
        Ok(())
    }

    async fn structured_test(&self, _model: String) -> Result<()> {
        self.push("structured_test");
        Ok(())
    }

    async fn thinking_demo(
        &self,
        _model: String,
        _example: String,
        _prompt: Option<String>,
        _thinking_budget: Option<i32>,
        thinking_level: Option<ThinkingLevel>,
    ) -> Result<()> {
        self.push("thinking_demo");
        self.record_thinking(thinking_level);
        Ok(())
    }

    async fn grounding_demo(
        &self,
        _model: String,
        _example: String,
        _prompt: Option<String>,
        _stream: bool,
    ) -> Result<()> {
        self.push("grounding_demo");
        Ok(())
    }
}

fn cli(command: Commands) -> Cli {
    Cli { debug: false, project: None, region: None, command }
}

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn routes_commands_and_builds_options() {
    let executor = MockExecutor::default();
    let router = CommandRouter::new(executor.clone());

    let commands = vec![
        Commands::Config { subcommand: ConfigCommands::Show },
        Commands::Config { subcommand: ConfigCommands::Check },
        Commands::Config { subcommand: ConfigCommands::Init },
        Commands::Cache {
            subcommand: CacheCommands::Create {
                text: Some("t".into()),
                file: Some("f".into()),
                name: Some("n".into()),
                ttl: 42,
                system: Some("sys".into()),
            },
        },
        Commands::Cache { subcommand: CacheCommands::List { page_size: None } },
        Commands::Cache { subcommand: CacheCommands::Get { cache_id: "one".into() } },
        Commands::Cache { subcommand: CacheCommands::Delete { cache_id: "two".into() } },
        Commands::Cache { subcommand: CacheCommands::Update { cache_id: "three".into(), ttl: 7 } },
        Commands::Generate {
            prompt: "p".into(),
            model: "m".into(),
            stream: false,
            temperature: 0.3,
            max_output_tokens: 10,
            system: Some("sys".into()),
            json: true,
            schema: Some("schema".into()),
            cache: Some("cache".into()),
            thinking: true,
            thinking_budget: Some(4),
            thinking_level: Some(ThinkingLevelArg::High),
            grounding: true,
        },
        Commands::Generate {
            prompt: "p-stream".into(),
            model: "m".into(),
            stream: true,
            temperature: 0.1,
            max_output_tokens: 8,
            system: None,
            json: false,
            schema: None,
            cache: None,
            thinking: false,
            thinking_budget: None,
            thinking_level: None,
            grounding: false,
        },
        Commands::Stream {
            prompt: "stream".into(),
            model: "s-model".into(),
            temperature: 0.1,
            max_output_tokens: 5,
            system: None,
            thinking: true,
            thinking_budget: Some(2),
            thinking_level: Some(ThinkingLevelArg::Low),
            grounding: true,
        },
        Commands::Chat { model: "chat".into(), system: None },
        Commands::Tokens { text: "hello".into(), model: "tok".into() },
        Commands::Test { subcommand: TestCommands::Auth },
        Commands::Test { subcommand: TestCommands::Generate },
        Commands::Test { subcommand: TestCommands::Stream },
        Commands::Test { subcommand: TestCommands::Functions },
        Commands::Test { subcommand: TestCommands::All },
        Commands::Models { subcommand: ModelsCommands::List { gemini: true, page_size: Some(25) } },
        Commands::Models { subcommand: ModelsCommands::Get { model: "id".into() } },
        Commands::Models { subcommand: ModelsCommands::Locations { page_size: None } },
        Commands::Models {
            subcommand: ModelsCommands::Test { model: "id".into(), prompt: "prompt".into() },
        },
        Commands::Functions { prompt: "fn".into(), model: "m".into(), system: None },
        Commands::CodeExec {
            prompt: "code".into(),
            model: "m".into(),
            stream: false,
            temperature: 0.2,
            max_output_tokens: 5,
            system: None,
        },
        Commands::CodeExec {
            prompt: "code".into(),
            model: "m".into(),
            stream: true,
            temperature: 0.2,
            max_output_tokens: 5,
            system: None,
        },
        Commands::SystemTest { model: "sys".into() },
        Commands::StructuredOutput {
            prompt: "so".into(),
            model: "model".into(),
            example: "example".into(),
            schema: Some("schema".into()),
        },
        Commands::StructuredTest { model: "st".into() },
        Commands::ThinkingDemo {
            model: "think".into(),
            example: "math".into(),
            prompt: Some("p".into()),
            thinking_budget: Some(1),
            thinking_level: Some(ThinkingLevelArg::Low),
        },
        Commands::GroundingDemo {
            model: "g".into(),
            example: "example".into(),
            prompt: None,
            stream: true,
        },
    ];

    for command in commands {
        router.run(cli(command)).await.unwrap();
    }

    let calls = executor.take_calls();
    assert_eq!(calls.len(), 30);
    assert!(calls.contains(&"config_show".to_string()));
    assert!(calls.contains(&"generate".to_string()));
    assert!(calls.contains(&"thinking_demo".to_string()));
    assert!(calls.contains(&"grounding_demo".to_string()));

    let generations = executor.take_generations();
    assert_eq!(generations.len(), 1);
    let options = &generations[0];
    assert_eq!(options.prompt, "p");
    assert_eq!(options.schema.as_deref(), Some("schema"));
    assert_eq!(options.thinking_level, Some(ThinkingLevel::High));
    assert!(options.thinking);

    let streams = executor.take_streams();
    assert_eq!(streams.len(), 3);
    let dedicated_stream =
        streams.iter().find(|s| s.prompt == "stream").expect("stream command recorded");
    assert_eq!(dedicated_stream.thinking_level, Some(ThinkingLevel::Low));
    assert!(dedicated_stream.grounding);

    let thinking_levels = executor.take_thinking_levels();
    assert_eq!(thinking_levels, vec![Some(ThinkingLevel::Low)]);
}
