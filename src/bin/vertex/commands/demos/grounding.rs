use std::time::Instant;

use anyhow::Result;
use colored::Colorize;
use threatflux_vertex_rust_sdk::{
    models::GenerateContentRequest,
    types::{GenerationConfig, GroundingMetadata},
};
use tokio_stream::StreamExt;

use crate::commands::grounding::{display_grounding_info, display_grounding_metadata};

use super::{
    helpers::{DemoClient, DemoPrinter, StyledPrinter, VertexDemoClient},
    prompt_catalog::resolve_grounding_prompt,
};

pub async fn grounding_demo(
    model: &str,
    example: &str,
    custom_prompt: Option<&str>,
    stream: bool,
) -> Result<()> {
    let client = VertexDemoClient::from_env().await?;
    let mut printer = StyledPrinter::stdout();
    run_grounding_demo(model, example, custom_prompt, stream, &client, &mut printer).await
}

async fn run_grounding_demo<C, P>(
    model: &str,
    example: &str,
    custom_prompt: Option<&str>,
    stream: bool,
    client: &C,
    printer: &mut P,
) -> Result<()>
where
    C: DemoClient,
    P: DemoPrinter,
{
    printer.banner("Google Search Grounding Demo")?;

    let resolution = resolve_grounding_prompt(example, custom_prompt);
    if let Some(notice) = &resolution.notice {
        printer.notice(notice)?;
    }

    printer
        .label_value("Example", format!("{} ({})", resolution.example, resolution.description))?;
    printer.label_value("Model", model)?;
    printer.label_value("Grounding", format!("{} via Google Search", "Enabled".green().bold()))?;
    if stream {
        printer.label_value("Mode", format!("{}", "Streaming".blue().bold()))?;
    }
    printer.section("Prompt:")?;
    printer.prompt(&resolution.prompt)?;

    let generation_config = GenerationConfig {
        temperature: Some(0.3),
        max_output_tokens: Some(2048),
        ..Default::default()
    };

    let request = GenerateContentRequest::new(&resolution.prompt)
        .with_generation_config(generation_config)
        .with_google_search();

    if stream {
        run_grounding_stream(model, request, client, printer).await?;
    } else {
        run_grounding_single(model, request, client, printer).await?;
    }

    printer.notice(
        "Try examples: news, events, facts, weather, stocks, or use --example=custom with --prompt",
    )?;
    if !stream {
        printer.notice("Use --stream for real-time streaming")?;
    }

    Ok(())
}

async fn run_grounding_single<C, P>(
    model: &str,
    request: GenerateContentRequest,
    client: &C,
    printer: &mut P,
) -> Result<()>
where
    C: DemoClient,
    P: DemoPrinter,
{
    let start_time = Instant::now();
    let response = client.generate(model, &request).await?;
    let elapsed = start_time.elapsed();

    display_grounding_info(&response);
    if response.grounding_metadata().is_some() {
        printer.notice("Grounding metadata displayed above.")?;
    }

    if let Some(text) = response.text() {
        printer.final_text("Response:", &text)?;
        printer.response_time(elapsed)?;
    } else {
        printer.missing_response("text in response")?;
    }

    if let Some(usage) = &response.usage_metadata {
        printer.token_usage(usage)?;
    }

    Ok(())
}

async fn run_grounding_stream<C, P>(
    model: &str,
    request: GenerateContentRequest,
    client: &C,
    printer: &mut P,
) -> Result<()>
where
    C: DemoClient,
    P: DemoPrinter,
{
    let start_time = Instant::now();
    let mut stream = client.stream(model, &request).await?;

    let mut final_grounding_metadata: Option<GroundingMetadata> = None;
    let mut final_usage = None;

    printer.stream_prefix("Response:")?;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;

        if chunk.has_grounding() {
            final_grounding_metadata = chunk.grounding_metadata().cloned();
        }

        if let Some(text) = chunk.text() {
            printer.inline_text(&text)?;
        }

        if chunk.is_final() {
            let elapsed = start_time.elapsed();
            printer.stream_complete(elapsed)?;

            if let Some(metadata) = &final_grounding_metadata {
                display_grounding_metadata(metadata);
                printer.notice("Grounding metadata displayed above.")?;
            }

            if let Some(usage) = &chunk.usage_metadata {
                final_usage = Some(usage.clone());
            }

            break;
        }
    }

    if let Some(usage) = final_usage.as_ref() {
        printer.token_usage(usage)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::pin::Pin;
    use threatflux_vertex_rust_sdk::{
        models::{GenerateContentResponse, StreamingResponse},
        types::{Candidate, Content, GroundingChunk, GroundingSupport, Part, UsageMetadata},
    };
    use tokio_stream::iter;

    use crate::commands::demos::helpers::{printer_output, DemoClient, DemoPrinter, StyledPrinter};

    struct MockClient {
        response: GenerateContentResponse,
        stream_chunks: Vec<StreamingResponse>,
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
            Ok(Box::pin(iter(self.stream_chunks.clone().into_iter().map(Ok::<_, anyhow::Error>))))
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

    fn response_with_grounding() -> GenerateContentResponse {
        GenerateContentResponse {
            candidates: vec![Candidate {
                content: Content {
                    role: "model".to_string(),
                    parts: vec![Part::Text { text: "grounded answer".to_string() }],
                },
                finish_reason: None,
                safety_ratings: vec![],
                index: None,
            }],
            usage_metadata: Some(UsageMetadata {
                prompt_token_count: 5,
                candidates_token_count: Some(15),
                total_token_count: 20,
                traffic_type: None,
                modality_token_count: None,
            }),
            grounding_metadata: Some(GroundingMetadata {
                web_search_queries: Some(vec!["rust".into()]),
                search_entry_point: None,
                grounding_chunks: Some(vec![GroundingChunk {
                    content: Some("sample content".into()),
                    uri: Some("https://example.com".into()),
                    title: Some("Example".into()),
                }]),
                grounding_supports: Some(vec![GroundingSupport {
                    grounding_chunk_indices: Some(vec![0]),
                    confidence_score: Some(0.8),
                    start_index: None,
                    end_index: None,
                    text: Some("grounded answer".into()),
                }]),
            }),
        }
    }

    #[tokio::test]
    async fn runs_grounding_demo_in_single_mode() {
        let client = MockClient { response: response_with_grounding(), stream_chunks: vec![] };
        let mut printer = RecordingPrinter::new();

        run_grounding_demo("model", "news", None, false, &client, &mut printer).await.unwrap();

        let output = printer.into_output();
        assert!(output.contains("Grounding Demo"));
        assert!(output.contains("grounded answer"));
        assert!(output.contains("Token Usage"));
    }

    #[tokio::test]
    async fn streams_grounding_demo_and_collects_usage() {
        let client = MockClient {
            response: response_with_grounding(),
            stream_chunks: vec![
                StreamingResponse {
                    candidates: vec![Candidate {
                        content: Content {
                            role: "model".into(),
                            parts: vec![Part::Text { text: "partial".into() }],
                        },
                        finish_reason: None,
                        safety_ratings: vec![],
                        index: None,
                    }],
                    usage_metadata: None,
                    grounding_metadata: None,
                },
                StreamingResponse {
                    candidates: vec![Candidate {
                        content: Content {
                            role: "model".into(),
                            parts: vec![Part::Text { text: "complete".into() }],
                        },
                        finish_reason: None,
                        safety_ratings: vec![],
                        index: None,
                    }],
                    usage_metadata: Some(UsageMetadata {
                        prompt_token_count: 1,
                        candidates_token_count: Some(2),
                        total_token_count: 3,
                        traffic_type: None,
                        modality_token_count: None,
                    }),
                    grounding_metadata: response_with_grounding().grounding_metadata,
                },
            ],
        };

        let mut printer = RecordingPrinter::new();

        run_grounding_demo("model", "news", None, true, &client, &mut printer).await.unwrap();

        let output = printer.into_output();
        assert!(output.contains("partialcomplete"));
        assert!(output.contains("Stream completed"));
        assert!(output.contains("Grounding metadata displayed above."));
        assert!(output.contains("Token Usage"));
    }
}
