use std::io::{self, Write};

use crate::common::ExampleEnvironment;
use crate::runner::{ChunkSink, StreamError, StreamSummary};

pub struct ConsolePrinter<W: Write> {
    writer: W,
}

impl ConsolePrinter<std::io::Stdout> {
    pub fn stdout() -> Self {
        Self::new(io::stdout())
    }
}

impl<W: Write> ConsolePrinter<W> {
    pub const fn new(writer: W) -> Self {
        Self { writer }
    }

    pub fn intro(&mut self, env: &ExampleEnvironment, model: &str) -> io::Result<()> {
        writeln!(self.writer, "=== Vertex AI Streaming Example ===")?;
        writeln!(
            self.writer,
            "Project: {}\nLocation: {}\nModel: {}",
            env.project_id, env.location, model
        )?;
        writeln!(self.writer)
    }

    pub fn summary(&mut self, summary: &StreamSummary) -> io::Result<()> {
        writeln!(self.writer, "\n\n--- Stream Complete ---")?;
        writeln!(self.writer, "Time elapsed: {:.2}s", summary.elapsed.as_secs_f64())?;
        writeln!(self.writer, "Chunks received: {}", summary.chunk_count)?;
        writeln!(self.writer, "Response length: {} characters", summary.full_response.len())?;

        if let Some(usage) = &summary.usage {
            writeln!(self.writer, "\nUsage Statistics:")?;
            writeln!(self.writer, "  Prompt tokens: {}", usage.prompt_token_count)?;
            if let Some(candidates) = usage.candidates_token_count {
                writeln!(self.writer, "  Response tokens: {candidates}")?;
            }
            writeln!(self.writer, "  Total tokens: {}", usage.total_token_count)?;

            let throughput =
                f64::from(usage.total_token_count) / summary.elapsed.as_secs_f64().max(0.001);
            writeln!(self.writer, "  Throughput: {throughput:.1} tokens/second")?;
        }

        Ok(())
    }

    pub fn success(&mut self) -> io::Result<()> {
        writeln!(self.writer, "✅ Streaming completed successfully!")
    }

    pub fn no_response(&mut self) -> io::Result<()> {
        writeln!(self.writer, "⚠️  No response received from the model")
    }

    pub fn error(&mut self, error: &StreamError) -> io::Result<()> {
        writeln!(self.writer, "\n\n❌ Streaming error: {error}")?;

        match error {
            StreamError::Authentication(_) => {
                writeln!(
                    self.writer,
                    "Authentication failed. Please run: gcloud auth application-default login"
                )?;
            }
            StreamError::Quota(_) => {
                writeln!(
                    self.writer,
                    "Rate limit or quota exceeded. Check your project quotas in Google Cloud Console."
                )?;
            }
            StreamError::NotFound(_) => {
                writeln!(
                    self.writer,
                    "Model or endpoint not found. Verify the model name and project access."
                )?;
            }
            StreamError::Transport(_) | StreamError::Output(_) => {}
        }

        Ok(())
    }

    pub fn into_inner(self) -> W {
        self.writer
    }
}

impl<W: Write> ChunkSink for ConsolePrinter<W> {
    fn handle_text(&mut self, text: &str) -> io::Result<()> {
        write!(self.writer, "{text}")?;
        self.writer.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use threatflux_vertex_rust_sdk::types::UsageMetadata;

    fn summary() -> StreamSummary {
        StreamSummary {
            elapsed: Duration::from_millis(1200),
            chunk_count: 3,
            full_response: "hi".repeat(5),
            usage: Some(UsageMetadata {
                prompt_token_count: 10,
                candidates_token_count: Some(20),
                total_token_count: 30,
                traffic_type: None,
                modality_token_count: None,
            }),
        }
    }

    #[test]
    fn prints_intro() {
        let env = ExampleEnvironment { project_id: "project".into(), location: "loc".into() };
        let mut printer = ConsolePrinter::new(Vec::new());

        printer.intro(&env, "model").expect("intro should print");

        let output = String::from_utf8(printer.into_inner()).unwrap();
        assert!(output.contains("project"));
        assert!(output.contains("model"));
    }

    #[test]
    fn prints_summary_and_usage() {
        let mut printer = ConsolePrinter::new(Vec::new());
        let summary = summary();

        printer.summary(&summary).expect("summary should print");

        let output = String::from_utf8(printer.into_inner()).unwrap();
        assert!(output.contains("Chunks received: 3"));
        assert!(output.contains("Prompt tokens: 10"));
        assert!(output.contains("Throughput"));
    }

    #[test]
    fn prints_errors_with_hints() {
        let mut printer = ConsolePrinter::new(Vec::new());

        printer.error(&StreamError::Authentication("no creds".into())).expect("error should print");

        let output = String::from_utf8(printer.into_inner()).unwrap();
        assert!(output.contains("authentication"));
        assert!(output.contains("gcloud auth"));
    }

    #[test]
    fn streams_text_through_sink() {
        let mut printer = ConsolePrinter::new(Vec::new());

        printer.handle_text("chunk").expect("chunk should be written");

        let output = String::from_utf8(printer.into_inner()).unwrap();
        assert_eq!(output, "chunk");
    }
}
