use threatflux_vertex_rust_sdk::models::GenerateContentRequest;
use threatflux_vertex_rust_sdk::types::{Candidate, Content, FunctionResponse, Part, Tool};

#[derive(Clone, Debug)]
pub struct Conversation {
    contents: Vec<Content>,
}

impl Conversation {
    pub fn new(prompt: &str) -> Self {
        Self { contents: vec![Content::user_text(prompt)] }
    }

    pub fn add_candidate(&mut self, candidate: &Candidate) {
        self.contents.push(candidate.content.clone());
    }

    pub fn add_function_responses(&mut self, responses: &[FunctionResponse]) {
        if responses.is_empty() {
            return;
        }

        let parts = responses
            .iter()
            .cloned()
            .map(|function_response| Part::FunctionResponse { function_response })
            .collect();

        self.contents.push(Content { role: "user".to_string(), parts });
    }

    pub fn build_request(
        &self,
        tool: Tool,
        system_instruction: Option<&str>,
    ) -> GenerateContentRequest {
        let mut request =
            GenerateContentRequest::with_contents(self.contents.clone()).with_tools(vec![tool]);

        if let Some(instruction) = system_instruction {
            request = request.with_system_text(instruction.to_owned());
        }

        request
    }
}
