use std::time::Instant;

use anyhow::Result;
use colored::Colorize;
use threatflux_vertex_rust_sdk::{
    models::GenerateContentRequest,
    types::{GenerationConfig, ThinkingLevel},
};

use crate::commands::thinking::{
    apply_thinking_to_config, describe_thinking_settings, validate_thinking_settings,
};

use super::{
    helpers::{DemoClient, DemoPrinter, StyledPrinter, VertexDemoClient},
    prompt_catalog::resolve_thinking_prompt,
};

pub async fn thinking_demo(
    model: &str,
    example: &str,
    custom_prompt: Option<&str>,
    thinking_budget: Option<i32>,
    thinking_level: Option<ThinkingLevel>,
) -> Result<()> {
    let client = VertexDemoClient::from_env().await?;
    let mut printer = StyledPrinter::stdout();
    run_thinking_demo(
        model,
        example,
        custom_prompt,
        thinking_budget,
        thinking_level,
        &client,
        &mut printer,
    )
    .await
}

async fn run_thinking_demo<C, P>(
    model: &str,
    example: &str,
    custom_prompt: Option<&str>,
    thinking_budget: Option<i32>,
    thinking_level: Option<ThinkingLevel>,
    client: &C,
    printer: &mut P,
) -> Result<()>
where
    C: DemoClient,
    P: DemoPrinter,
{
    printer.banner("Thinking Mode Demonstration")?;

    let resolution = resolve_thinking_prompt(example, custom_prompt)?;
    if let Some(notice) = &resolution.notice {
        printer.notice(notice)?;
    }

    printer.label_value("Model", model.yellow())?;
    printer.label_value(
        "Example",
        format!("{} ({})", resolution.example.green().bold(), resolution.description.italic()),
    )?;

    let budget_input =
        if thinking_level.is_some() { thinking_budget } else { thinking_budget.or(Some(-1)) };
    let thinking_settings = validate_thinking_settings(model, true, budget_input, thinking_level)?;
    let status = if thinking_settings.enabled {
        "Enabled".green().bold().to_string()
    } else {
        "Disabled".yellow().bold().to_string()
    };
    printer.label_value(
        "Thinking Mode",
        format!("{} ({})", status, describe_thinking_settings(&thinking_settings).yellow()),
    )?;

    printer.section("Prompt:")?;
    printer.description(&resolution.description)?;
    printer.prompt(&resolution.prompt)?;

    let generation_config =
        apply_thinking_to_config(GenerationConfig::default(), &thinking_settings);
    let request =
        GenerateContentRequest::new(&resolution.prompt).with_generation_config(generation_config);

    let start_time = Instant::now();
    let response = client.generate(model, &request).await?;
    let elapsed = start_time.elapsed();

    if response.has_thinking() {
        printer.thinking_sections(&response.thinking_content())?;
    } else {
        printer.notice("No thinking process captured. This may indicate the model does not support thinking mode or it was disabled.")?;
    }

    if let Some(text) = response.text_without_thinking().or_else(|| response.text()) {
        printer.final_text("Final Answer:", &text)?;
    } else {
        printer.missing_response("text response")?;
    }

    if let Some(usage) = &response.usage_metadata {
        printer.token_usage(usage)?;
    }

    printer.response_time(elapsed)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::pin::Pin;
    use threatflux_vertex_rust_sdk::{
        models::{GenerateContentResponse, StreamingResponse},
        types::{Candidate, Content, Part, UsageMetadata},
    };
    use tokio_stream::iter;

    use crate::commands::demos::helpers::{printer_output, DemoClient, DemoPrinter, StyledPrinter};

    struct MockClient {
        response: GenerateContentResponse,
    }

    #[async_trait::async_trait]
    impl DemoClient for MockClient {
        async fn generate(
            &self,
            _model: &str,
            _request: &GenerateContentRequest,
        ) -> Result<GenerateContentResponse> {
            Ok(self.response.clone())
        }

        async fn stream(
            &self,
            _model: &str,
            _request: &GenerateContentRequest,
        ) -> Result<Pin<Box<dyn tokio_stream::Stream<Item = Result<StreamingResponse>> + Send>>>
        {
            Ok(Box::pin(iter([])))
        }
    }

    struct RecordingPrinter {
        inner: StyledPrinter<Vec<u8>>,
    }

    impl RecordingPrinter {
        fn new() -> Self {
            Self { inner: StyledPrinter::buffer() }
        }

        fn into_output(self) -> String {
            printer_output(self.inner)
        }
    }

    impl DemoPrinter for RecordingPrinter {
        fn banner(&mut self, title: &str) -> std::io::Result<()> {
            self.inner.banner(title)
        }

        fn label_value(
            &mut self,
            label: &str,
            value: impl std::fmt::Display,
        ) -> std::io::Result<()> {
            self.inner.label_value(label, value)
        }

        fn description(&mut self, description: &str) -> std::io::Result<()> {
            self.inner.description(description)
        }

        fn notice(&mut self, message: &str) -> std::io::Result<()> {
            self.inner.notice(message)
        }

        fn section(&mut self, title: &str) -> std::io::Result<()> {
            self.inner.section(title)
        }

        fn prompt(&mut self, prompt: &str) -> std::io::Result<()> {
            self.inner.prompt(prompt)
        }

        fn thinking_sections(&mut self, thoughts: &[String]) -> std::io::Result<()> {
            self.inner.thinking_sections(thoughts)
        }

        fn final_text(&mut self, label: &str, text: &str) -> std::io::Result<()> {
            self.inner.final_text(label, text)
        }

        fn missing_response(&mut self, label: &str) -> std::io::Result<()> {
            self.inner.missing_response(label)
        }

        fn token_usage(&mut self, usage: &UsageMetadata) -> std::io::Result<()> {
            self.inner.token_usage(usage)
        }

        fn response_time(&mut self, elapsed: std::time::Duration) -> std::io::Result<()> {
            self.inner.response_time(elapsed)
        }

        fn stream_prefix(&mut self, label: &str) -> std::io::Result<()> {
            self.inner.stream_prefix(label)
        }

        fn inline_text(&mut self, text: &str) -> std::io::Result<()> {
            self.inner.inline_text(text)
        }

        fn stream_complete(&mut self, elapsed: std::time::Duration) -> std::io::Result<()> {
            self.inner.stream_complete(elapsed)
        }
    }

    fn response_with_thinking() -> GenerateContentResponse {
        let parts = vec![
            Part::thinking("first thought"),
            Part::thinking("second thought"),
            Part::Text { text: "final answer".to_string() },
        ];
        GenerateContentResponse {
            candidates: vec![Candidate {
                content: Content { role: "model".to_string(), parts },
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

    #[tokio::test]
    async fn runs_thinking_demo_and_prints_sections() {
        let client = MockClient { response: response_with_thinking() };
        let mut printer = RecordingPrinter::new();

        run_thinking_demo("gemini-1.5-pro", "math", None, Some(-1), None, &client, &mut printer)
            .await
            .unwrap();

        let output = printer.into_output();
        assert!(output.contains("Thinking Mode Demonstration"));
        assert!(output.contains("Prompt:"));
        assert!(output.contains("Model"));
        assert!(output.contains("final answer"));
        assert!(output.contains("Token Usage"));
    }

    #[tokio::test]
    async fn surfaces_notice_for_unknown_example() {
        let client = MockClient { response: response_with_thinking() };
        let mut printer = RecordingPrinter::new();

        run_thinking_demo("gemini-1.5-pro", "unknown", None, None, None, &client, &mut printer)
            .await
            .unwrap();

        let output = printer.into_output();
        assert!(output.contains("Unknown example type"));
        assert!(output.contains("math example"));
    }
}
