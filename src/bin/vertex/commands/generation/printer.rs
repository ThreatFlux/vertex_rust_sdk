use std::{
    io::{self, Write},
    time::Duration,
};

use colored::Colorize;
use threatflux_vertex_rust_sdk::{
    models::GenerateContentResponse,
    types::{GroundingMetadata, UsageMetadata},
};

use crate::commands::{
    grounding::display_grounding_metadata,
    thinking::{describe_thinking_settings, ThinkingSettings},
};

pub struct Printer<W: Write> {
    writer: W,
}

impl<W: Write> Printer<W> {
    pub const fn new(writer: W) -> Self {
        Self { writer }
    }

    pub fn banner(&mut self, title: &str, model: &str) -> io::Result<()> {
        writeln!(self.writer, "{}", title.bold().cyan())?;
        writeln!(self.writer, "{}", "═".repeat(60).cyan())?;
        writeln!(self.writer, "Model: {}", model.yellow())
    }

    pub fn system_prompt(&mut self, instruction: Option<&str>) -> io::Result<()> {
        if let Some(instruction) = instruction {
            writeln!(self.writer, "System: {}", instruction.italic().blue())?;
        }
        Ok(())
    }

    pub fn cache(&mut self, cache_id: Option<&str>) -> io::Result<()> {
        if let Some(cache) = cache_id {
            writeln!(self.writer, "Cache ID: {}", cache.bold().magenta())?;
        }
        Ok(())
    }

    pub fn thinking(
        &mut self,
        thinking_requested: bool,
        settings: &ThinkingSettings,
    ) -> io::Result<()> {
        if thinking_requested || settings.enabled {
            let status = if settings.enabled {
                "Enabled".green().bold()
            } else {
                "Disabled".yellow().bold()
            };
            writeln!(
                self.writer,
                "Thinking Mode: {} ({})",
                status,
                describe_thinking_settings(settings).yellow()
            )?;
        }
        Ok(())
    }

    pub fn grounding(&mut self, enabled: bool) -> io::Result<()> {
        if enabled {
            writeln!(self.writer, "Google Search Grounding: {}", "Enabled".green().bold())?;
        }
        Ok(())
    }

    pub fn structured_output(&mut self, json: bool, has_schema: bool) -> io::Result<()> {
        if json {
            writeln!(self.writer, "JSON Output: {}", "Enabled".green())?;
            if has_schema {
                writeln!(self.writer, "Schema: {}", "Custom".green())?;
            }
        }
        Ok(())
    }

    pub fn prompt(&mut self, prompt: &str) -> io::Result<()> {
        writeln!(self.writer, "Prompt: {}", prompt.italic())?;
        writeln!(self.writer)
    }

    pub fn heading(&mut self, title: &str) -> io::Result<()> {
        writeln!(self.writer, "{}", title.bold().green())
    }

    pub fn print_response_text(
        &mut self,
        response: &GenerateContentResponse,
        json: bool,
    ) -> io::Result<()> {
        if let Some(text) = response.text_without_thinking().or_else(|| response.text()) {
            if json && response.is_json() {
                if let Some(pretty) = response.json_pretty() {
                    writeln!(self.writer, "{pretty}")?;
                } else {
                    writeln!(self.writer, "{text}")?;
                }
            } else {
                writeln!(self.writer, "{text}")?;
            }
            self.print_json_validation(response, json)?;
        } else {
            writeln!(self.writer, "{} No text in response", "⚠️".yellow())?;
        }

        Ok(())
    }

    fn print_json_validation(
        &mut self,
        response: &GenerateContentResponse,
        json: bool,
    ) -> io::Result<()> {
        if json {
            if response.is_json() {
                writeln!(self.writer, "\n{} JSON validation: {}", "✅".green(), "Valid".green())?;
            } else {
                writeln!(
                    self.writer,
                    "\n{} JSON validation: {}",
                    "❌".red(),
                    "Invalid - response is not valid JSON".red()
                )?;
            }
        }
        Ok(())
    }

    pub fn thinking_sections(&mut self, sections: &[String]) -> io::Result<()> {
        if sections.is_empty() {
            return Ok(());
        }

        writeln!(self.writer, "\n{}", "🧠 Thinking Process:".bold().blue())?;
        writeln!(self.writer, "{}", "─".repeat(60).blue())?;
        for section in sections {
            writeln!(self.writer, "{}", section.italic().dimmed())?;
        }
        writeln!(self.writer, "{}", "─".repeat(60).blue())
    }

    pub fn usage(&mut self, usage: &UsageMetadata) -> io::Result<()> {
        writeln!(self.writer, "\n{}", "Token Usage:".bold().blue())?;
        writeln!(self.writer, "  Prompt tokens: {}", usage.prompt_token_count)?;
        if let Some(candidates) = usage.candidates_token_count {
            writeln!(self.writer, "  Response tokens: {candidates}")?;
        }
        writeln!(self.writer, "  Total tokens: {}", usage.total_token_count)
    }

    pub fn response_time(&mut self, elapsed: Duration) -> io::Result<()> {
        writeln!(self.writer, "\n{} Response time: {:?}", "⏱️".blue(), elapsed)
    }

    pub fn inline_text(&mut self, text: &str) -> io::Result<()> {
        write!(self.writer, "{text}")?;
        self.writer.flush()
    }

    pub fn stream_prefix(&mut self) -> io::Result<()> {
        write!(self.writer, "{} ", "Response:".bold().green())?;
        self.writer.flush()
    }

    pub fn stream_complete(&mut self, elapsed: Duration) -> io::Result<()> {
        writeln!(self.writer, "\n\n{} Stream completed in {:?}", "✅".green(), elapsed)
    }
}

pub fn print_grounding(metadata: &GroundingMetadata) {
    display_grounding_metadata(metadata);
}
