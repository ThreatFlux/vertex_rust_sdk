use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use threatflux_vertex_rust_sdk::models::{GenerateContentRequest, GenerateContentResponse};
use threatflux_vertex_rust_sdk::types::{
    Candidate, Content, FunctionCall, Part, Tool, UsageMetadata,
};

use super::conversation::Conversation;
use super::output::{self, OutputSink};
use super::runner::{run_function_demo, ContentGenerator};
use super::tools;

#[derive(Default)]
struct RecordingSink {
    lines: Vec<String>,
}

impl OutputSink for RecordingSink {
    fn line(&mut self, line: impl Into<String>) {
        self.lines.push(line.into());
    }
}

struct StubGenerator {
    responses: Mutex<VecDeque<GenerateContentResponse>>,
}

impl StubGenerator {
    fn new(responses: Vec<GenerateContentResponse>) -> Self {
        Self { responses: Mutex::new(responses.into()) }
    }
}

#[async_trait]
impl ContentGenerator for StubGenerator {
    async fn generate(
        &self,
        _model: &str,
        _request: &GenerateContentRequest,
    ) -> Result<GenerateContentResponse> {
        self.responses.lock().unwrap().pop_front().ok_or_else(|| anyhow!("no responses left"))
    }
}

#[test]
fn builds_function_calling_tool() {
    let tool = tools::available_tool();
    if let Tool::FunctionCalling { function_declarations } = tool {
        let names: Vec<_> = function_declarations.iter().map(|decl| decl.name.as_str()).collect();
        assert_eq!(names, vec!["get_weather", "multiply"]);
    } else {
        panic!("unexpected tool variant");
    }
}

#[test]
fn executes_function_calls() {
    let mut args = HashMap::new();
    args.insert("a".to_string(), 2.into());
    args.insert("b".to_string(), 5.into());

    let multiply_call = FunctionCall { name: "multiply".to_string(), args };

    let response = tools::execute_function_call(&multiply_call);
    assert_eq!(response.name, "multiply");
    assert_eq!(response.response["result"], 10.0);

    let weather_call = FunctionCall { name: "get_weather".to_string(), args: HashMap::new() };

    let weather_response = tools::execute_function_call(&weather_call);
    assert_eq!(weather_response.name, "get_weather");
    assert_eq!(weather_response.response["condition"], "Sunny");

    let unknown_call = FunctionCall { name: "missing".to_string(), args: HashMap::new() };
    let unknown_response = tools::execute_function_call(&unknown_call);
    assert_eq!(unknown_response.response["error"], "Unknown function: missing");
}

#[test]
fn conversation_builds_requests() {
    let mut conversation = Conversation::new("hello");
    let candidate = Candidate {
        content: Content::model_text("model reply"),
        finish_reason: None,
        safety_ratings: vec![],
        index: Some(0),
    };
    conversation.add_candidate(&candidate);

    let function_response = tools::execute_function_call(&FunctionCall {
        name: "multiply".to_string(),
        args: HashMap::new(),
    });
    conversation.add_function_responses(&[function_response]);

    let request = conversation.build_request(Tool::function_calling(vec![]), Some("system"));
    assert_eq!(request.contents.len(), 3);
    assert!(request.tools.is_some());
    assert!(request.system_instruction.is_some());
}

#[test]
fn output_helpers_format_expected_lines() {
    let mut sink = RecordingSink::default();
    output::print_header(&mut sink, "model", "prompt", Some("sys"));
    output::print_function_call_count(&mut sink, 2);
    output::print_usage(
        &mut sink,
        Some(&UsageMetadata {
            prompt_token_count: 1,
            candidates_token_count: Some(2),
            total_token_count: 3,
            traffic_type: None,
            modality_token_count: None,
        }),
    );

    assert!(sink.lines.iter().any(|line| line.contains("Function Calling Demo")));
    assert!(sink.lines.iter().any(|line| line.contains("model")));
    assert!(sink.lines.iter().any(|line| line.contains("Response tokens")));
}

#[tokio::test]
async fn run_demo_processes_function_calls() {
    let initial_response = GenerateContentResponse {
        candidates: vec![Candidate {
            content: Content {
                role: "model".to_string(),
                parts: vec![Part::FunctionCall {
                    function_call: FunctionCall {
                        name: "multiply".to_string(),
                        args: HashMap::from([
                            ("a".to_string(), 3.into()),
                            ("b".to_string(), 4.into()),
                        ]),
                    },
                }],
            },
            finish_reason: None,
            safety_ratings: vec![],
            index: Some(0),
        }],
        usage_metadata: Some(UsageMetadata {
            prompt_token_count: 10,
            candidates_token_count: Some(5),
            total_token_count: 15,
            traffic_type: None,
            modality_token_count: None,
        }),
        grounding_metadata: None,
    };

    let final_response = GenerateContentResponse {
        candidates: vec![Candidate {
            content: Content::model_text("The product is 12"),
            finish_reason: None,
            safety_ratings: vec![],
            index: Some(0),
        }],
        usage_metadata: Some(UsageMetadata {
            prompt_token_count: 12,
            candidates_token_count: Some(6),
            total_token_count: 18,
            traffic_type: None,
            modality_token_count: None,
        }),
        grounding_metadata: None,
    };

    let generator = StubGenerator::new(vec![initial_response, final_response]);
    let mut sink = RecordingSink::default();

    run_function_demo(
        "Do a quick multiply",
        "gemini-model",
        Some("system text"),
        &generator,
        &mut sink,
    )
    .await
    .unwrap();

    assert!(sink.lines.iter().any(|line| line.contains("Function call: multiply")));
    assert!(sink.lines.iter().any(|line| line.contains("The product is 12")));
    assert!(sink.lines.iter().any(|line| line.contains("Function calling demo completed")));
}

#[tokio::test]
async fn run_demo_handles_responses_without_functions() {
    let response_without_calls = GenerateContentResponse {
        candidates: vec![Candidate {
            content: Content::model_text("No functions needed"),
            finish_reason: None,
            safety_ratings: vec![],
            index: Some(0),
        }],
        usage_metadata: Some(UsageMetadata {
            prompt_token_count: 2,
            candidates_token_count: Some(2),
            total_token_count: 4,
            traffic_type: None,
            modality_token_count: None,
        }),
        grounding_metadata: None,
    };

    let generator = StubGenerator::new(vec![response_without_calls]);
    let mut sink = RecordingSink::default();

    run_function_demo("No tools needed", "gemini-model", None, &generator, &mut sink)
        .await
        .unwrap();

    assert!(sink.lines.iter().any(|line| line.contains("No functions needed")));
    assert!(sink.lines.iter().any(|line| line.contains("Function calling demo completed")));
}
