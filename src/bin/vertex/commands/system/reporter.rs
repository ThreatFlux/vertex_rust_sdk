use std::io::{self, Write};

use colored::Colorize;
use threatflux_vertex_rust_sdk::{models::GenerateContentResponse, types::UsageMetadata};

use super::cases::SystemTestCase;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComparisonMode {
    WithoutSystem,
    WithSystem,
}

impl ComparisonMode {
    pub const fn label(self) -> &'static str {
        match self {
            Self::WithoutSystem => "Without System Instruction",
            Self::WithSystem => "With System Instruction",
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ResponseSummary {
    pub text: Option<String>,
    pub usage: Option<UsageMetadata>,
}

impl ResponseSummary {
    pub fn from_response(response: &GenerateContentResponse) -> Self {
        Self { text: response.text(), usage: response.usage_metadata.clone() }
    }
}

pub trait Reporter: Send {
    fn suite_start(&mut self, model: &str, fast_mode: bool, case_limit: usize);
    fn case_start(&mut self, index: usize, case: &SystemTestCase);
    fn case_success(&mut self, index: usize, summary: &ResponseSummary);
    fn case_missing_text(&mut self, index: usize);
    fn case_error(&mut self, index: usize, error: &str);
    fn after_case(&mut self, index: usize);
    fn comparison_start(&mut self, prompt: &str, system_instruction: &str);
    fn comparison_success(&mut self, mode: ComparisonMode, summary: &ResponseSummary);
    fn comparison_missing_text(&mut self, mode: ComparisonMode);
    fn comparison_error(&mut self, mode: ComparisonMode, error: &str);
    fn comparison_end(&mut self);
    fn suite_end(&mut self, fast_mode: bool);
}

pub struct StdoutReporter<W: Write + Send> {
    writer: W,
}

impl StdoutReporter<std::io::Stdout> {
    #[must_use]
    pub fn new() -> Self {
        Self { writer: io::stdout() }
    }
}

impl<W: Write + Send> StdoutReporter<W> {
    #[cfg(test)]
    #[must_use]
    pub const fn with_writer(writer: W) -> Self {
        Self { writer }
    }

    #[cfg(test)]
    #[must_use]
    pub fn into_inner(self) -> W {
        self.writer
    }

    fn line(&mut self, value: impl AsRef<str>) {
        let _ = writeln!(self.writer, "{}", value.as_ref());
    }

    fn separator(&mut self) {
        self.line(format!("{}", "═".repeat(60).dimmed()));
    }

    fn emit_usage(&mut self, usage: &UsageMetadata) {
        self.line(format!(
            "\n{} Tokens: {} prompt, {} total",
            "📊".blue(),
            usage.prompt_token_count,
            usage.total_token_count
        ));
    }
}

impl<W: Write + Send> Reporter for StdoutReporter<W> {
    fn suite_start(&mut self, model: &str, fast_mode: bool, case_limit: usize) {
        self.line(format!("{}", "System Instructions Test Suite".bold().cyan()));
        self.separator();
        self.line(format!("Model: {}", model.yellow()));

        if fast_mode {
            self.line(format!("{}", "Fast mode enabled (VERTEX_TEST_FAST)".italic()));
        }

        if case_limit != usize::MAX {
            self.line(format!("Running first {case_limit} test case(s) (VERTEX_TEST_CASE_LIMIT)"));
        }

        self.line("");
    }

    fn case_start(&mut self, index: usize, case: &SystemTestCase) {
        self.line(format!("{} Test {}: {}", "🧪".blue(), index, case.name.bold().yellow()));
        self.line(format!("System: {}", case.system_instruction.italic().blue()));
        self.line(format!("Prompt: {}", case.prompt.italic()));
        self.separator();
    }

    fn case_success(&mut self, _index: usize, summary: &ResponseSummary) {
        if let Some(text) = &summary.text {
            self.line(format!("{}", "Response:".bold().green()));
            self.line(text);
        }

        if let Some(usage) = &summary.usage {
            self.emit_usage(usage);
        }
    }

    fn case_missing_text(&mut self, _index: usize) {
        self.line(format!("{}", "⚠️ No text in response".yellow()));
    }

    fn case_error(&mut self, _index: usize, error: &str) {
        self.line(format!("{} Error: {}", "❌".red(), error));
    }

    fn after_case(&mut self, _index: usize) {
        self.line("");
        self.separator();
        self.line("");
    }

    fn comparison_start(&mut self, prompt: &str, system_instruction: &str) {
        self.line(format!(
            "{}",
            "Comparison Test: With vs Without System Instructions".bold().cyan()
        ));
        self.separator();
        self.line(format!("Prompt: {}", prompt.italic()));
        self.line(format!("System Instruction: {}", system_instruction.italic().blue()));
        self.line("");
    }

    fn comparison_success(&mut self, mode: ComparisonMode, summary: &ResponseSummary) {
        self.line(format!(
            "{} {}:",
            match mode {
                ComparisonMode::WithoutSystem => "1️⃣".blue(),
                ComparisonMode::WithSystem => "2️⃣".blue(),
            },
            mode.label()
        ));

        if let Some(text) = &summary.text {
            self.line(text);
        }

        if let Some(usage) = &summary.usage {
            self.emit_usage(usage);
        }

        self.line("");
    }

    fn comparison_missing_text(&mut self, mode: ComparisonMode) {
        self.line(format!("{} {} had no text in response", "⚠️".yellow(), mode.label()));
        self.line("");
    }

    fn comparison_error(&mut self, mode: ComparisonMode, error: &str) {
        self.line(format!("{} {} failed: {}", "❌".red(), mode.label(), error));
        self.line("");
    }

    fn comparison_end(&mut self) {
        self.separator();
    }

    fn suite_end(&mut self, fast_mode: bool) {
        if fast_mode {
            self.line(format!(
                "{}",
                "System instructions test suite completed (fast mode).".green()
            ));
            return;
        }

        self.line(format!("{} System instructions test suite completed!", "✅".green()));
        self.line("Notice how the system instruction changes the response style and approach!");
    }
}
