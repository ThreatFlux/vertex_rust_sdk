use threatflux_vertex_rust_sdk::{
    FunctionCall, FunctionResponse, GenerateContentRequest, GenerateContentResponse,
    GenerationConfig, UsageMetadata,
};

use crate::{
    client::{ClientResult, ContentGenerator},
    config::ExampleConfig,
    conversation::{
        append_function_response, append_model_response, build_request, initial_history,
    },
    simulator::simulate,
};

#[derive(Debug)]
pub struct FlowResult {
    pub initial_request: GenerateContentRequest,
    pub initial_response: GenerateContentResponse,
    pub function_calls: Vec<FunctionCall>,
    pub function_responses: Vec<FunctionResponse>,
    pub final_request: Option<GenerateContentRequest>,
    pub final_response: Option<GenerateContentResponse>,
}

impl FlowResult {
    pub fn final_text(&self) -> Option<String> {
        self.final_response
            .as_ref()
            .and_then(GenerateContentResponse::text)
            .or_else(|| self.initial_response.text())
    }

    pub fn final_usage(&self) -> Option<&UsageMetadata> {
        self.final_response
            .as_ref()
            .and_then(|response| response.usage_metadata.as_ref())
            .or(self.initial_response.usage_metadata.as_ref())
    }
}

pub async fn run_flow<C: ContentGenerator + Sync>(
    client: &C,
    config: &ExampleConfig,
) -> ClientResult<FlowResult> {
    let mut history = initial_history(&config.prompt);
    let initial_request =
        build_request_with_config(&history, &config.tool, &config.generation_config);
    let initial_response = client.generate(&config.model, &initial_request).await?;
    let function_calls = initial_response.function_calls();

    if function_calls.is_empty() {
        return Ok(FlowResult {
            initial_request,
            initial_response,
            function_calls,
            function_responses: vec![],
            final_request: None,
            final_response: None,
        });
    }

    append_model_response(&mut history, &initial_response);

    let mut function_responses = Vec::with_capacity(function_calls.len());
    for function_call in &function_calls {
        let response = simulate(function_call);
        append_function_response(&mut history, response.clone());
        function_responses.push(response);
    }

    let final_request =
        build_request_with_config(&history, &config.tool, &config.generation_config);
    let final_response = client.generate(&config.model, &final_request).await?;

    Ok(FlowResult {
        initial_request,
        initial_response,
        function_calls,
        function_responses,
        final_request: Some(final_request),
        final_response: Some(final_response),
    })
}

fn build_request_with_config(
    history: &[threatflux_vertex_rust_sdk::Content],
    tool: &threatflux_vertex_rust_sdk::Tool,
    config: &GenerationConfig,
) -> GenerateContentRequest {
    build_request(history, tool, config)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use serde_json::json;
    use threatflux_vertex_rust_sdk::{
        Candidate, Content, FunctionCall, FunctionResponse, Part, UsageMetadata,
    };

    use crate::{
        client::tests::MockContentGenerator, config::ExampleConfig, schemas::build_tool,
        simulator::simulate,
    };

    use super::*;

    fn build_usage(prompt_tokens: i32) -> UsageMetadata {
        UsageMetadata {
            prompt_token_count: prompt_tokens,
            candidates_token_count: Some(prompt_tokens + 5),
            total_token_count: prompt_tokens + 5,
            traffic_type: None,
            modality_token_count: None,
        }
    }

    fn response_with_parts(parts: Vec<Part>, usage: UsageMetadata) -> GenerateContentResponse {
        GenerateContentResponse {
            candidates: vec![Candidate {
                content: Content { role: "model".to_string(), parts },
                finish_reason: None,
                safety_ratings: vec![],
                index: None,
            }],
            usage_metadata: Some(usage),
            grounding_metadata: None,
        }
    }

    fn build_example_config() -> ExampleConfig {
        ExampleConfig {
            project_id: "test-project".to_string(),
            location: "us-central1".to_string(),
            generation_config: GenerationConfig {
                temperature: Some(0.0),
                max_output_tokens: Some(256),
                ..GenerationConfig::default()
            },
            tool: build_tool(),
            model: "gemini-2.0-flash-001".to_string(),
            prompt: "prompt".to_string(),
        }
    }

    #[tokio::test]
    async fn flow_returns_text_when_no_function_calls() {
        let config = build_example_config();
        let response = response_with_parts(vec![Part::text("hello")], build_usage(10));
        let mock = MockContentGenerator::new(vec![response.clone()]);

        let result = run_flow(&mock, &config).await.unwrap();
        assert!(result.function_calls.is_empty());
        assert_eq!(result.final_text().as_deref(), Some("hello"));
        assert!(result.final_request.is_none());
        assert_eq!(
            result.final_usage().unwrap().total_token_count,
            response.usage_metadata.unwrap().total_token_count
        );
    }

    #[tokio::test]
    async fn flow_executes_functions_and_builds_follow_up_request() {
        let config = build_example_config();
        let mut args = HashMap::new();
        args.insert("location".to_string(), json!("Boston"));
        args.insert("operation".to_string(), json!("add"));
        args.insert("a".to_string(), json!(2.0));
        args.insert("b".to_string(), json!(3.0));

        let initial_parts = vec![
            Part::FunctionCall {
                function_call: FunctionCall {
                    name: "get_current_weather".to_string(),
                    args: args.clone(),
                },
            },
            Part::FunctionCall {
                function_call: FunctionCall { name: "calculate".to_string(), args },
            },
        ];

        let initial_response = response_with_parts(initial_parts, build_usage(5));

        let final_response = response_with_parts(vec![Part::text("done")], build_usage(15));

        let mock =
            MockContentGenerator::new(vec![initial_response.clone(), final_response.clone()]);

        let result = run_flow(&mock, &config).await.unwrap();
        assert_eq!(result.function_calls.len(), 2);
        assert_eq!(result.function_responses.len(), 2);
        assert!(result.final_request.is_some());
        assert_eq!(result.final_text().as_deref(), Some("done"));

        let generated_response = simulate(&result.function_calls[1]);
        assert_eq!(generated_response.name, "calculate");
        assert!(result.function_responses[1].response.get("result").is_some());
        assert_eq!(
            result.final_usage().unwrap().total_token_count,
            final_response.usage_metadata.unwrap().total_token_count
        );
    }
}
