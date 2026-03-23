use anyhow::Result;
use async_trait::async_trait;
use threatflux_vertex_rust_sdk::{
    client::VertexClient,
    config::Config,
    models::{GenerateContentRequest, GenerateContentResponse},
};

use super::{
    conversation::Conversation,
    output::{self, OutputSink, StdoutSink},
    tools,
};

#[async_trait]
pub trait ContentGenerator {
    async fn generate(
        &self,
        model: &str,
        request: &GenerateContentRequest,
    ) -> Result<GenerateContentResponse>;
}

pub struct VertexContentGenerator {
    client: VertexClient,
}

impl VertexContentGenerator {
    pub async fn from_env() -> Result<Self> {
        let config = Config::from_env()?;
        let client = VertexClient::new(config).await?;
        Ok(Self { client })
    }
}

#[async_trait]
impl ContentGenerator for VertexContentGenerator {
    async fn generate(
        &self,
        model: &str,
        request: &GenerateContentRequest,
    ) -> Result<GenerateContentResponse> {
        self.client.generate_content(model, request).await.map_err(Into::into)
    }
}

pub async fn test_functions_with_prompt(
    prompt: &str,
    model: &str,
    system_instruction: Option<&str>,
) -> Result<()> {
    let generator = VertexContentGenerator::from_env().await?;
    let mut sink = StdoutSink;

    run_function_demo(prompt, model, system_instruction, &generator, &mut sink).await
}

pub(super) async fn run_function_demo<G, S>(
    prompt: &str,
    model: &str,
    system_instruction: Option<&str>,
    generator: &G,
    sink: &mut S,
) -> Result<()>
where
    G: ContentGenerator + Send + Sync,
    S: OutputSink,
{
    output::print_header(sink, model, prompt, system_instruction);

    let tool = tools::available_tool();
    let mut conversation = Conversation::new(prompt);

    let request = conversation.build_request(tool.clone(), system_instruction);
    let initial_response = generator.generate(model, &request).await?;

    let function_calls = initial_response.function_calls();

    if function_calls.is_empty() {
        output::print_text_response(sink, &initial_response);
        output::print_usage(sink, initial_response.usage_metadata.as_ref());
        output::print_completion(sink);
        return Ok(());
    }

    output::print_function_call_count(sink, function_calls.len());

    if let Some(candidate) = initial_response.candidates.first() {
        conversation.add_candidate(candidate);
        output::print_candidate_function_calls(sink, candidate, "Function call found");
    }

    let mut function_responses = Vec::new();
    for function_call in &function_calls {
        output::print_function_call(sink, function_call)?;
        let response = tools::execute_function_call(function_call);
        output::print_function_result(sink, &response)?;
        function_responses.push(response);
    }

    conversation.add_function_responses(&function_responses);

    output::print_final_request_banner(sink);

    let final_request = conversation.build_request(tool, system_instruction);
    let final_response = generator.generate(model, &final_request).await?;

    output::print_final_response(sink, &final_response);
    output::print_usage(sink, initial_response.usage_metadata.as_ref());
    output::print_usage(sink, final_response.usage_metadata.as_ref());

    if let Some(candidate) = final_response.candidates.first() {
        output::print_candidate_function_calls(
            sink,
            candidate,
            "Additional function call detected",
        );
    }

    output::print_completion(sink);
    Ok(())
}
