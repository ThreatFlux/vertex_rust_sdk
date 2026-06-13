use std::time::Duration;

use anyhow::Result;

use self::{
    printer::StructuredPrinter,
    runner::{run_structured, StructuredOptions, VertexStructuredClient},
    schema::resolve_schema,
};

mod printer;
mod runner;
mod schema;

pub async fn structured_output(
    prompt: &str,
    model: &str,
    example: &str,
    schema: Option<&str>,
) -> Result<()> {
    let schema_resolution = resolve_schema(schema, example)?;
    let mut printer = StructuredPrinter::stdout();

    printer.banner("Structured Output Generation", model)?;
    printer.example(example)?;
    printer.prompt(prompt)?;
    printer.schema_preview(&schema_resolution)?;

    let client = VertexStructuredClient::from_env().await?;
    let response = run_structured(
        &client,
        &StructuredOptions { prompt, model, schema: schema_resolution.schema.clone() },
    )
    .await?;

    printer.response(&response)?;
    printer.usage(response.usage_metadata.as_ref())?;
    Ok(())
}

pub async fn structured_test(model: &str) -> Result<()> {
    let mut printer = StructuredPrinter::stdout();
    printer.banner("Structured Output Test Suite", model)?;

    let client = VertexStructuredClient::from_env().await?;

    for (i, case) in test_cases().iter().enumerate() {
        printer.test_case_header(i + 1, case.name, case.prompt)?;

        let response = run_structured(
            &client,
            &StructuredOptions { prompt: case.prompt, model, schema: case.schema.clone() },
        )
        .await;

        match response {
            Ok(response) => {
                printer.response(&response)?;
                printer.usage(response.usage_metadata.as_ref())?;
            }
            Err(err) => printer.error(&err)?,
        }

        printer.case_complete()?;
        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    printer.suite_complete()?;
    Ok(())
}

struct StructuredTestCase {
    name: &'static str,
    prompt: &'static str,
    schema: serde_json::Value,
}

fn test_cases() -> Vec<StructuredTestCase> {
    vec![
        StructuredTestCase {
            name: "Person Information Extraction",
            schema: threatflux_vertex_rust_sdk::types::GenerationConfig::person_schema(),
            prompt: "Extract information from: John Smith is 30 years old and works as a software engineer. His email is john.smith@example.com.",
        },
        StructuredTestCase {
            name: "Recipe Ingredients List",
            schema: threatflux_vertex_rust_sdk::types::GenerationConfig::recipe_ingredients_schema(),
            prompt: "Create a recipe for chocolate chip cookies with ingredients and amounts.",
        },
        StructuredTestCase {
            name: "Organization Chart",
            schema: threatflux_vertex_rust_sdk::types::GenerationConfig::org_chart_schema(),
            prompt: "Create an organization chart for a small tech startup with Engineering, Marketing, and Sales departments.",
        },
        StructuredTestCase {
            name: "Programming Languages List",
            schema: threatflux_vertex_rust_sdk::types::GenerationConfig::create_array_schema(serde_json::json!({
                "type": "object",
                "properties": {
                    "name": {"type": "string"},
                    "year_created": {"type": "integer"},
                    "paradigm": {"type": "string"}
                },
                "required": ["name", "year_created", "paradigm"]
            })),
            prompt: "List 3 popular programming languages with their creation year and main paradigm.",
        },
        StructuredTestCase {
            name: "Book Summary",
            schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "title": {"type": "string"},
                    "author": {"type": "string"},
                    "genre": {"type": "string"},
                    "summary": {"type": "string"},
                    "key_themes": {
                        "type": "array",
                        "items": {"type": "string"}
                    },
                    "rating": {"type": "integer", "minimum": 1, "maximum": 5}
                },
                "required": ["title", "author", "genre", "summary"]
            }),
            prompt: "Summarize the book '1984' by George Orwell.",
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::structured::runner::StructuredClient;
    use threatflux_vertex_rust_sdk::{
        models::GenerateContentRequest,
        models::GenerateContentResponse,
        types::{Candidate, Content, Part, UsageMetadata},
    };
    use tokio::sync::Mutex;

    struct MockClient {
        response: GenerateContentResponse,
        calls: Mutex<usize>,
    }

    #[async_trait::async_trait]
    impl StructuredClient for MockClient {
        async fn generate(
            &self,
            _model: &str,
            _request: &GenerateContentRequest,
        ) -> Result<GenerateContentResponse> {
            {
                let mut guard = self.calls.lock().await;
                *guard += 1;
            }
            Ok(self.response.clone())
        }
    }

    fn sample_response() -> GenerateContentResponse {
        GenerateContentResponse {
            candidates: vec![Candidate {
                content: Content {
                    role: "model".to_string(),
                    parts: vec![Part::Text { text: "{\"ok\":true}".to_string() }],
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
            grounding_metadata: None,
        }
    }

    #[tokio::test]
    async fn runs_structured_tests_with_mock() {
        let client = MockClient { response: sample_response(), calls: Mutex::new(0) };

        let result = run_structured(
            &client,
            &StructuredOptions {
                prompt: "hi",
                model: "gemini",
                schema: serde_json::json!({"type":"object"}),
            },
        )
        .await;

        assert!(result.is_ok());
        assert_eq!(*client.calls.lock().await, 1);
    }
}
