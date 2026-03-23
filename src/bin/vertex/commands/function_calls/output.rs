use anyhow::Result;
use colored::Colorize;
use serde_json::to_string_pretty;
use threatflux_vertex_rust_sdk::models::GenerateContentResponse;
use threatflux_vertex_rust_sdk::types::{
    Candidate, FunctionCall, FunctionResponse, Part, UsageMetadata,
};

pub trait OutputSink {
    fn line(&mut self, line: impl Into<String>);
}

pub struct StdoutSink;

impl OutputSink for StdoutSink {
    fn line(&mut self, line: impl Into<String>) {
        println!("{}", line.into());
    }
}

pub fn print_header(
    sink: &mut impl OutputSink,
    model: &str,
    prompt: &str,
    system_instruction: Option<&str>,
) {
    sink.line("Function Calling Demo".bold().cyan().to_string());
    sink.line("═".repeat(60).cyan().to_string());
    sink.line(format!("Model: {}", model.yellow()));

    if let Some(instruction) = system_instruction {
        sink.line(format!("System: {}", instruction.italic().blue()));
    }

    sink.line(format!("Prompt: {}\n", prompt.italic()));
    sink.line("Initial Request:".bold().blue().to_string());
}

pub fn print_function_call_count(sink: &mut impl OutputSink, count: usize) {
    sink.line(format!("{} Model requested {} function call(s)", "🔧".blue(), count));
}

pub fn print_function_call(sink: &mut impl OutputSink, call: &FunctionCall) -> Result<()> {
    sink.line(format!("\n{} Function call: {}", "🔧".blue(), call.name));
    sink.line(format!("Arguments: {}", to_string_pretty(&call.args)?));
    Ok(())
}

pub fn print_function_result(
    sink: &mut impl OutputSink,
    response: &FunctionResponse,
) -> Result<()> {
    sink.line(format!("Result: {}", to_string_pretty(&response.response)?));
    Ok(())
}

pub fn print_text_response(sink: &mut impl OutputSink, response: &GenerateContentResponse) {
    sink.line("Response:".bold().green().to_string());
    if let Some(text) = response.text() {
        sink.line(text);
    } else {
        sink.line("No text response received");
    }
}

pub fn print_final_response(sink: &mut impl OutputSink, response: &GenerateContentResponse) {
    sink.line("\nFinal Response:".bold().green().to_string());
    if let Some(text) = response.text() {
        sink.line(text);
    } else {
        sink.line("No final text response received");
    }
}

pub fn print_usage(sink: &mut impl OutputSink, usage: Option<&UsageMetadata>) {
    if let Some(usage) = usage {
        sink.line("\nToken Usage:".bold().blue().to_string());
        sink.line(format!("  Prompt tokens: {}", usage.prompt_token_count));
        if let Some(candidates) = usage.candidates_token_count {
            sink.line(format!("  Response tokens: {candidates}"));
        }
        sink.line(format!("  Total tokens: {}", usage.total_token_count));
    }
}

pub fn print_final_request_banner(sink: &mut impl OutputSink) {
    sink.line("\nFinal Response Request:".bold().blue().to_string());
}

pub fn print_candidate_function_calls(
    sink: &mut impl OutputSink,
    candidate: &Candidate,
    label: &str,
) {
    for part in &candidate.content.parts {
        if let Part::FunctionCall { function_call } = part {
            sink.line(format!("\n{} {label}: {}", "🔧".blue(), function_call.name.bold()));
        }
    }
}

pub fn print_completion(sink: &mut impl OutputSink) {
    sink.line(format!("\n{} Function calling demo completed successfully!", "✅".green()));
}
