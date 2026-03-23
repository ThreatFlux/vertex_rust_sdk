use std::io::{self, Write};

use anyhow::Error;
use colored::Colorize;
use threatflux_vertex_rust_sdk::{models::GenerateContentResponse, types::UsageMetadata};

use super::schema::SchemaResolution;

pub struct StructuredPrinter<W: Write> {
    writer: W,
}

impl StructuredPrinter<std::io::Stdout> {
    pub fn stdout() -> Self {
        Self { writer: io::stdout() }
    }
}

#[cfg(test)]
impl StructuredPrinter<Vec<u8>> {
    pub fn buffer() -> Self {
        Self { writer: Vec::new() }
    }

    pub fn into_output(self) -> String {
        String::from_utf8(self.writer).expect("printer output is valid utf-8")
    }
}

impl<W: Write> StructuredPrinter<W> {
    pub fn banner(&mut self, title: &str, model: &str) -> io::Result<()> {
        writeln!(self.writer, "{}", title.bold().cyan())?;
        writeln!(self.writer, "{}", "═".repeat(60).cyan())?;
        writeln!(self.writer, "Model: {}", model.yellow())?;
        writeln!(self.writer)
    }

    pub fn example(&mut self, example: &str) -> io::Result<()> {
        writeln!(self.writer, "Example: {}", example.green())
    }

    pub fn prompt(&mut self, prompt: &str) -> io::Result<()> {
        writeln!(self.writer, "Prompt: {}", prompt.italic())
    }

    pub fn schema_preview(&mut self, resolution: &SchemaResolution) -> io::Result<()> {
        writeln!(self.writer, "{}", "Schema Preview:".bold().blue())?;
        writeln!(self.writer, "{}", resolution.label())?;
        if let Some(notice) = resolution.notice() {
            writeln!(self.writer, "{} {notice}", "ℹ️".yellow())?;
        }

        let pretty = serde_json::to_string_pretty(&resolution.schema)
            .unwrap_or_else(|_| "Invalid schema".to_string());
        writeln!(self.writer, "{pretty}")
    }

    pub fn response(&mut self, response: &GenerateContentResponse) -> io::Result<()> {
        writeln!(self.writer, "\n{}", "Structured Response:".bold().green())?;

        if let Some(text) = response.text_without_thinking().or_else(|| response.text()) {
            if response.is_json() {
                if let Some(pretty) = response.json_pretty() {
                    writeln!(self.writer, "{pretty}")?;
                } else {
                    writeln!(self.writer, "{text}")?;
                }
                writeln!(self.writer, "\n{} JSON validation: {}", "✅".green(), "Valid".green())?;
            } else {
                writeln!(self.writer, "{text}")?;
                writeln!(
                    self.writer,
                    "\n{} JSON validation: {}",
                    "❌".red(),
                    "Invalid - response is not valid JSON".red()
                )?;
            }
        } else {
            writeln!(self.writer, "{} No text in response", "⚠️".yellow())?;
        }

        Ok(())
    }

    pub fn usage(&mut self, usage: Option<&UsageMetadata>) -> io::Result<()> {
        if let Some(usage) = usage {
            writeln!(self.writer, "\n{}", "Token Usage:".bold().blue())?;
            writeln!(self.writer, "  Prompt tokens: {}", usage.prompt_token_count)?;
            if let Some(candidates) = usage.candidates_token_count {
                writeln!(self.writer, "  Response tokens: {candidates}")?;
            }
            writeln!(self.writer, "  Total tokens: {}", usage.total_token_count)?;
        }
        Ok(())
    }

    pub fn test_case_header(&mut self, index: usize, name: &str, prompt: &str) -> io::Result<()> {
        writeln!(self.writer, "{} Test {}: {}", "🧪".blue(), index, name.bold().yellow())?;
        writeln!(self.writer, "Prompt: {}", prompt.italic())?;
        writeln!(self.writer, "{}", "─".repeat(60).dimmed())
    }

    pub fn case_complete(&mut self) -> io::Result<()> {
        writeln!(self.writer, "\n{}", "═".repeat(60).dimmed())
    }

    pub fn suite_complete(&mut self) -> io::Result<()> {
        writeln!(self.writer, "\n{} Structured output test suite completed!", "✅".green())
    }

    pub fn error(&mut self, err: &Error) -> io::Result<()> {
        writeln!(self.writer, "{} Error: {}", "❌".red(), err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::structured::schema::SchemaOrigin;
    use threatflux_vertex_rust_sdk::{
        models::GenerateContentResponse,
        types::{Candidate, Content, Part, UsageMetadata},
    };

    fn response_with_json() -> GenerateContentResponse {
        GenerateContentResponse {
            candidates: vec![Candidate {
                content: Content {
                    role: "model".to_string(),
                    parts: vec![Part::Text { text: "{\"hello\":\"world\"}".to_string() }],
                },
                finish_reason: None,
                safety_ratings: vec![],
                index: None,
            }],
            usage_metadata: Some(UsageMetadata {
                prompt_token_count: 10,
                candidates_token_count: Some(20),
                total_token_count: 30,
                traffic_type: None,
                modality_token_count: None,
            }),
            grounding_metadata: None,
        }
    }

    fn response_plain_text() -> GenerateContentResponse {
        GenerateContentResponse {
            candidates: vec![Candidate {
                content: Content {
                    role: "model".to_string(),
                    parts: vec![Part::Text { text: "hello".to_string() }],
                },
                finish_reason: None,
                safety_ratings: vec![],
                index: None,
            }],
            usage_metadata: None,
            grounding_metadata: None,
        }
    }

    #[test]
    fn prints_json_response_and_usage() {
        let mut printer = StructuredPrinter::buffer();
        let resolution = SchemaResolution {
            schema: serde_json::json!({"type":"object"}),
            origin: SchemaOrigin::Custom,
        };

        printer.banner("Structured Output Generation", "gemini").unwrap();
        printer.schema_preview(&resolution).unwrap();
        let response = response_with_json();
        printer.response(&response).unwrap();
        printer.usage(response.usage_metadata.as_ref()).unwrap();

        let output = printer.into_output();
        assert!(output.contains("Structured Output Generation"));
        assert!(output.contains("Using custom schema"));
        assert!(output.contains("JSON validation"));
        assert!(output.contains("Token Usage"));
    }

    #[test]
    fn prints_invalid_json_notice() {
        let mut printer = StructuredPrinter::buffer();
        printer.response(&response_plain_text()).unwrap();
        let output = printer.into_output();
        assert!(output.contains("Invalid - response is not valid JSON"));
    }
}
