//! Shared chat utilities used by the CLI binary and examples.

pub mod commands;
pub mod config;
pub mod io;
pub mod runner;
pub mod service;
pub mod session;

pub use commands::{parse_command, Command};
pub use config::{
    ChatConfig, DEFAULT_LOCATION, DEFAULT_MAX_TOKENS, DEFAULT_MODEL, DEFAULT_SYSTEM_PROMPT,
    DEFAULT_TEMPERATURE,
};
pub use io::{ConsoleInput, ConsoleOutput, Input, Output};
pub use runner::run_chat;
pub use service::{ChatService, VertexChatService};
pub use session::{ChatSession, SessionStats};
