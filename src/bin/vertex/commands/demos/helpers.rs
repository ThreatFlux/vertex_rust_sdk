use std::{
    fmt::Display,
    io::{self, Write},
    pin::Pin,
    time::Duration,
};

use anyhow::Result;
use async_trait::async_trait;
use colored::Colorize;
use threatflux_vertex_rust_sdk::{
    client::VertexClient,
    config::Config,
    models::{GenerateContentRequest, GenerateContentResponse, StreamingResponse},
    types::UsageMetadata,
};
use tokio_stream::{Stream, StreamExt};

#[async_trait]
pub trait DemoClient: Send + Sync {
    async fn generate(
        &self,
        model: &str,
        request: &GenerateContentRequest,
    ) -> Result<GenerateContentResponse>;

    async fn stream(
        &self,
        model: &str,
        request: &GenerateContentRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamingResponse>> + Send>>>;
}

pub struct VertexDemoClient {
    inner: VertexClient,
}

impl VertexDemoClient {
    pub async fn from_env() -> Result<Self> {
        let config = Config::from_env()?;
        let client = VertexClient::new(config).await?;
        Ok(Self { inner: client })
    }
}

#[async_trait]
impl DemoClient for VertexDemoClient {
    async fn generate(
        &self,
        model: &str,
        request: &GenerateContentRequest,
    ) -> Result<GenerateContentResponse> {
        self.inner.generate_content(model, request).await.map_err(Into::into)
    }

    async fn stream(
        &self,
        model: &str,
        request: &GenerateContentRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamingResponse>> + Send>>> {
        let stream = self.inner.stream_generate_content(model, request).await?;
        Ok(Box::pin(stream.map(|chunk| chunk.map_err(Into::into))))
    }
}

pub trait DemoPrinter {
    fn banner(&mut self, title: &str) -> io::Result<()>;
    fn label_value(&mut self, label: &str, value: impl Display) -> io::Result<()>;
    fn description(&mut self, description: &str) -> io::Result<()>;
    fn notice(&mut self, message: &str) -> io::Result<()>;
    fn section(&mut self, title: &str) -> io::Result<()>;
    fn prompt(&mut self, prompt: &str) -> io::Result<()>;
    fn thinking_sections(&mut self, thoughts: &[String]) -> io::Result<()>;
    fn final_text(&mut self, label: &str, text: &str) -> io::Result<()>;
    fn missing_response(&mut self, label: &str) -> io::Result<()>;
    fn token_usage(&mut self, usage: &UsageMetadata) -> io::Result<()>;
    fn response_time(&mut self, elapsed: Duration) -> io::Result<()>;
    fn stream_prefix(&mut self, label: &str) -> io::Result<()>;
    fn inline_text(&mut self, text: &str) -> io::Result<()>;
    fn stream_complete(&mut self, elapsed: Duration) -> io::Result<()>;
}

pub struct StyledPrinter<W: Write> {
    writer: W,
}

impl StyledPrinter<std::io::Stdout> {
    pub fn stdout() -> Self {
        Self { writer: io::stdout() }
    }
}

#[cfg(test)]
impl StyledPrinter<Vec<u8>> {
    pub fn buffer() -> Self {
        Self { writer: Vec::new() }
    }

    pub fn into_inner(self) -> Vec<u8> {
        self.writer
    }
}

impl<W: Write> StyledPrinter<W> {
    fn write_line(&mut self, content: &str) -> io::Result<()> {
        writeln!(self.writer, "{content}")
    }
}

impl<W: Write> DemoPrinter for StyledPrinter<W> {
    fn banner(&mut self, title: &str) -> io::Result<()> {
        self.write_line(&format!("{}", title.bold().cyan()))?;
        self.write_line(&format!("{}", "═".repeat(60).cyan()))
    }

    fn label_value(&mut self, label: &str, value: impl Display) -> io::Result<()> {
        writeln!(self.writer, "{} {}", format!("{label}:").bold(), value)
    }

    fn description(&mut self, description: &str) -> io::Result<()> {
        self.write_line(&format!("{}", description.italic()))
    }

    fn notice(&mut self, message: &str) -> io::Result<()> {
        self.write_line(&format!("{} {message}", "ℹ️".yellow()))
    }

    fn section(&mut self, title: &str) -> io::Result<()> {
        self.write_line(&format!("\n{}", title.bold().blue()))
    }

    fn prompt(&mut self, prompt: &str) -> io::Result<()> {
        self.write_line(&format!("{}", prompt.italic()))
    }

    fn thinking_sections(&mut self, thoughts: &[String]) -> io::Result<()> {
        if thoughts.is_empty() {
            return Ok(());
        }

        self.write_line(&format!("\n{}", "🧠 Model's Thinking Process:".bold().blue()))?;
        self.write_line(&format!("{}", "═".repeat(60).blue()))?;
        for (i, thought) in thoughts.iter().enumerate() {
            if thoughts.len() > 1 {
                self.write_line(&format!("\n{} Thought {}:", "💭".blue(), i + 1))?;
            }
            self.write_line(&format!("{}", thought.italic().dimmed()))?;
        }
        self.write_line(&format!("{}", "═".repeat(60).blue()))
    }

    fn final_text(&mut self, label: &str, text: &str) -> io::Result<()> {
        self.write_line(&format!("\n{}", label.bold().green()))?;
        self.write_line(text)
    }

    fn missing_response(&mut self, label: &str) -> io::Result<()> {
        self.write_line(&format!("{} No {label} received", "⚠️".yellow()))
    }

    fn token_usage(&mut self, usage: &UsageMetadata) -> io::Result<()> {
        self.write_line(&format!("\n{}", "Token Usage:".bold().blue()))?;
        self.write_line(&format!("  Prompt tokens: {}", usage.prompt_token_count))?;
        if let Some(candidates) = usage.candidates_token_count {
            self.write_line(&format!("  Response tokens: {candidates}"))?;
        }
        self.write_line(&format!("  Total tokens: {}", usage.total_token_count))
    }

    fn response_time(&mut self, elapsed: Duration) -> io::Result<()> {
        self.write_line(&format!("{} Response time: {:?}", "⏱️".blue(), elapsed))
    }

    fn stream_prefix(&mut self, label: &str) -> io::Result<()> {
        write!(self.writer, "{} ", label.bold().green())?;
        self.writer.flush()
    }

    fn inline_text(&mut self, text: &str) -> io::Result<()> {
        write!(self.writer, "{text}")?;
        self.writer.flush()
    }

    fn stream_complete(&mut self, elapsed: Duration) -> io::Result<()> {
        self.write_line("")?;
        self.write_line(&format!("{} Stream completed in {:?}", "✅".green(), elapsed))
    }
}

#[cfg(test)]
pub fn printer_output(printer: StyledPrinter<Vec<u8>>) -> String {
    String::from_utf8(printer.into_inner()).expect("printer output is valid utf-8")
}
