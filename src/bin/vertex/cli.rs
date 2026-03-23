use clap::{Parser, Subcommand, ValueEnum};

use threatflux_vertex_rust_sdk::types::ThinkingLevel;

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum ThinkingLevelArg {
    Low,
    High,
}

impl From<ThinkingLevelArg> for ThinkingLevel {
    fn from(value: ThinkingLevelArg) -> Self {
        match value {
            ThinkingLevelArg::Low => Self::Low,
            ThinkingLevelArg::High => Self::High,
        }
    }
}

/// Vertex AI SDK CLI
#[derive(Parser)]
#[command(name = "vertex")]
#[command(about = "Vertex AI SDK CLI - Interact with Google Vertex AI", long_about = None)]
#[command(version)]
pub struct Cli {
    /// Enable debug output
    #[arg(short, long, global = true)]
    pub debug: bool,

    /// Project ID (overrides `VERTEX_PROJECT_ID`)
    #[arg(short, long, global = true)]
    pub project: Option<String>,

    /// Region (overrides `VERTEX_REGION`)
    #[arg(short, long, global = true)]
    pub region: Option<String>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Configure authentication and settings
    Config {
        #[command(subcommand)]
        subcommand: ConfigCommands,
    },

    /// Manage context caches
    Cache {
        #[command(subcommand)]
        subcommand: CacheCommands,
    },

    /// Generate content using AI models
    Generate {
        /// The prompt to send to the model
        prompt: String,

        /// Model to use
        #[arg(short, long, default_value = "gemini-3-pro-preview")]
        model: String,

        /// Enable streaming output
        #[arg(short, long)]
        stream: bool,

        /// Temperature (0.0 to 1.0)
        #[arg(short = 't', long, default_value = "0.7")]
        temperature: f32,

        /// Maximum output tokens
        #[arg(short = 'o', long, default_value = "1000")]
        max_output_tokens: i32,

        /// System instruction
        #[arg(long)]
        system: Option<String>,

        /// Enable JSON structured output
        #[arg(long)]
        json: bool,

        /// Custom JSON schema for structured output
        #[arg(long)]
        schema: Option<String>,

        /// Use cached content by ID
        #[arg(short = 'c', long)]
        cache: Option<String>,

        /// Enable thinking mode (shows model's reasoning process)
        #[arg(long)]
        thinking: bool,

        /// Thinking budget tokens for legacy models (-1 auto; Flash 0-24576; Flash-Lite 512-24576; Pro 128-32768)
        #[arg(long)]
        thinking_budget: Option<i32>,

        /// Thinking level for Gemini 3 models (low or high)
        #[arg(long, value_enum)]
        thinking_level: Option<ThinkingLevelArg>,

        /// Enable Google Search grounding for up-to-date information
        #[arg(long)]
        grounding: bool,
    },

    /// Stream content generation (dedicated streaming command)
    Stream {
        /// The prompt to send to the model
        prompt: String,

        /// Model to use
        #[arg(short, long, default_value = "gemini-3-pro-preview")]
        model: String,

        /// Temperature (0.0 to 1.0)
        #[arg(short = 't', long, default_value = "0.7")]
        temperature: f32,

        /// Maximum output tokens
        #[arg(short = 'o', long, default_value = "1000")]
        max_output_tokens: i32,

        /// System instruction
        #[arg(long)]
        system: Option<String>,

        /// Enable thinking mode (shows model's reasoning process)
        #[arg(long)]
        thinking: bool,

        /// Thinking budget tokens for legacy models (-1 auto; Flash 0-24576; Flash-Lite 512-24576; Pro 128-32768)
        #[arg(long)]
        thinking_budget: Option<i32>,

        /// Thinking level for Gemini 3 models (low or high)
        #[arg(long, value_enum)]
        thinking_level: Option<ThinkingLevelArg>,

        /// Enable Google Search grounding for up-to-date information
        #[arg(long)]
        grounding: bool,
    },

    /// Interactive chat session
    Chat {
        /// Model to use
        #[arg(short, long, default_value = "gemini-1.5-flash")]
        model: String,

        /// System instruction
        #[arg(short, long)]
        system: Option<String>,
    },

    /// Count tokens in text
    Tokens {
        /// Text to count tokens for
        text: String,

        /// Model to use for tokenization
        #[arg(short, long, default_value = "gemini-1.5-flash")]
        model: String,
    },

    /// Test the SDK functionality
    Test {
        #[command(subcommand)]
        subcommand: TestCommands,
    },

    /// List available models
    Models {
        #[command(subcommand)]
        subcommand: ModelsCommands,
    },

    /// Test function calling capabilities
    Functions {
        /// The prompt to send to the model
        prompt: String,

        /// Model to use
        #[arg(short, long, default_value = "gemini-2.5-flash")]
        model: String,

        /// System instruction
        #[arg(long)]
        system: Option<String>,
    },

    /// Execute code using AI models
    #[command(name = "code-exec")]
    CodeExec {
        /// The prompt to send to the model
        prompt: String,

        /// Model to use
        #[arg(short, long, default_value = "gemini-2.5-flash")]
        model: String,

        /// Enable streaming output
        #[arg(short, long)]
        stream: bool,

        /// Temperature (0.0 to 1.0)
        #[arg(short = 't', long, default_value = "0.7")]
        temperature: f32,

        /// Maximum output tokens
        #[arg(short = 'o', long, default_value = "2048")]
        max_output_tokens: i32,

        /// System instruction
        #[arg(long)]
        system: Option<String>,
    },

    /// Test system instructions with various examples
    #[command(name = "system-test")]
    SystemTest {
        /// Model to use
        #[arg(short, long, default_value = "gemini-2.5-flash")]
        model: String,
    },

    /// Generate structured output with predefined examples
    #[command(name = "structured-output")]
    StructuredOutput {
        /// The prompt to send to the model
        prompt: String,

        /// Model to use
        #[arg(short, long, default_value = "gemini-2.5-flash")]
        model: String,

        /// Example type: person, recipe, orgchart
        #[arg(short, long, default_value = "person")]
        example: String,

        /// Custom schema (JSON string)
        #[arg(long)]
        schema: Option<String>,
    },

    /// Test structured output with built-in examples
    #[command(name = "structured-test")]
    StructuredTest {
        /// Model to use
        #[arg(short, long, default_value = "gemini-2.5-flash")]
        model: String,
    },

    /// Demonstrate thinking mode capabilities with examples
    #[command(name = "thinking-demo")]
    ThinkingDemo {
        /// Model to use (thinking mode requires Gemini 2.5 Flash or later)
        #[arg(short, long, default_value = "gemini-2.5-flash")]
        model: String,

        /// Example type: math, logic, reasoning, decision, custom
        #[arg(short, long, default_value = "math")]
        example: String,

        /// Custom prompt for the demo (if example=custom)
        #[arg(long)]
        prompt: Option<String>,

        /// Thinking budget tokens (-1 auto; Flash 0-24576; Flash-Lite 512-24576; Pro 128-32768)
        #[arg(long)]
        thinking_budget: Option<i32>,

        /// Thinking level for Gemini 3 models (low or high)
        #[arg(long, value_enum)]
        thinking_level: Option<ThinkingLevelArg>,
    },

    /// Demonstrate Google Search grounding capabilities with examples
    #[command(name = "grounding-demo")]
    GroundingDemo {
        /// Model to use (grounding requires Gemini 2.5 Flash or later)
        #[arg(short, long, default_value = "gemini-2.5-flash")]
        model: String,

        /// Example type: news, events, facts, weather, stocks, custom
        #[arg(short, long, default_value = "news")]
        example: String,

        /// Custom prompt for the demo (if example=custom)
        #[arg(long)]
        prompt: Option<String>,

        /// Enable streaming output
        #[arg(short, long)]
        stream: bool,
    },
}

#[derive(Subcommand)]
pub enum ConfigCommands {
    /// Show current configuration
    Show,

    /// Check environment variables
    Check,

    /// Initialize configuration file
    Init,
}

#[derive(Subcommand)]
pub enum CacheCommands {
    /// Create a new cache from text or file
    Create {
        /// Text content to cache (use --file for file content)
        text: Option<String>,

        /// File path to read content from
        #[arg(short, long)]
        file: Option<String>,

        /// Display name for the cache
        #[arg(short, long)]
        name: Option<String>,

        /// TTL in seconds
        #[arg(short = 't', long, default_value = "3600")]
        ttl: u64,

        /// System instruction to include in cache
        #[arg(long)]
        system: Option<String>,
    },

    /// List all cached contents
    List {
        /// Number of caches to list per page
        #[arg(short, long)]
        page_size: Option<i32>,
    },

    /// Get cache details by ID
    Get {
        /// Cache ID
        cache_id: String,
    },

    /// Delete a cache by ID
    Delete {
        /// Cache ID
        cache_id: String,
    },

    /// Update cache TTL
    Update {
        /// Cache ID
        cache_id: String,

        /// New TTL in seconds
        #[arg(short = 't', long)]
        ttl: u64,
    },
}

#[derive(Subcommand)]
pub enum TestCommands {
    /// Test authentication
    Auth,

    /// Test generation API
    Generate,

    /// Test streaming API
    Stream,

    /// Test function calling API
    Functions,

    /// Run all tests
    All,
}

#[derive(Subcommand)]
pub enum ModelsCommands {
    /// List available models
    List {
        /// Only show Gemini models
        #[arg(short, long)]
        gemini: bool,

        /// Number of models to list per page
        #[arg(short, long)]
        page_size: Option<i32>,
    },

    /// Get model info
    Get {
        /// Model name
        model: String,
    },

    /// List available locations
    Locations {
        /// Number of locations to list per page
        #[arg(short, long)]
        page_size: Option<i32>,
    },

    /// Test a model
    Test {
        /// Model name
        model: String,

        /// Test prompt
        #[arg(default_value = "Hello, world!")]
        prompt: String,
    },
}

impl Cli {
    pub fn parse() -> Self {
        <Self as Parser>::parse()
    }
}
