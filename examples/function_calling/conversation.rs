use threatflux_vertex_rust_sdk::{
    Content, FunctionResponse, GenerateContentRequest, GenerateContentResponse, GenerationConfig,
    Part, Tool,
};

pub fn initial_history(prompt: &str) -> Vec<Content> {
    vec![Content::user_text(prompt)]
}

pub fn append_model_response(history: &mut Vec<Content>, response: &GenerateContentResponse) {
    if let Some(candidate) = response.candidates.first() {
        history.push(candidate.content.clone());
    }
}

pub fn append_function_response(history: &mut Vec<Content>, function_response: FunctionResponse) {
    history.push(Content {
        role: "user".to_string(),
        parts: vec![Part::FunctionResponse { function_response }],
    });
}

pub fn build_request(
    history: &[Content],
    tool: &Tool,
    config: &GenerationConfig,
) -> GenerateContentRequest {
    GenerateContentRequest::with_contents(history.to_vec())
        .with_tools(vec![tool.clone()])
        .with_generation_config(config.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_history_from_prompt() {
        let history = initial_history("Hello");
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].role, "user");
    }

    #[test]
    fn appends_model_and_function_parts() {
        let mut history = initial_history("prompt");
        let response = GenerateContentResponse {
            candidates: vec![threatflux_vertex_rust_sdk::Candidate {
                content: Content { role: "model".to_string(), parts: vec![Part::text("text")] },
                finish_reason: None,
                safety_ratings: vec![],
                index: None,
            }],
            usage_metadata: None,
            grounding_metadata: None,
        };

        append_model_response(&mut history, &response);
        assert_eq!(history.len(), 2);
        assert_eq!(history[1].role, "model");

        append_function_response(
            &mut history,
            FunctionResponse { name: "fn".to_string(), response: serde_json::json!({"ok": true}) },
        );
        assert_eq!(history.len(), 3);
        assert_eq!(history[2].role, "user");
    }

    #[test]
    fn request_uses_tool_and_config() {
        let history = initial_history("prompt");
        let tool = Tool::function_calling(vec![]);
        let config = GenerationConfig { temperature: Some(0.5), ..GenerationConfig::default() };
        let request = build_request(&history, &tool, &config);
        assert_eq!(request.contents.len(), 1);
        assert_eq!(request.tools.unwrap().len(), 1);
        assert_eq!(request.generation_config.unwrap().temperature, Some(0.5));
    }
}
