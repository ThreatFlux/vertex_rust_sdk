use anyhow::{Context, Result};
use serde_json::Value;
use threatflux_vertex_rust_sdk::types::GenerationConfig;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchemaOrigin {
    Custom,
    Example(&'static str),
    Defaulted { requested: String, fallback: &'static str },
}

#[derive(Debug, Clone)]
pub struct SchemaResolution {
    pub schema: Value,
    pub origin: SchemaOrigin,
}

impl SchemaResolution {
    pub fn label(&self) -> String {
        match &self.origin {
            SchemaOrigin::Custom => "Using custom schema".to_string(),
            SchemaOrigin::Example(example) => format!("Using {example} schema"),
            SchemaOrigin::Defaulted { fallback, .. } => {
                format!("Using {fallback} schema (default)")
            }
        }
    }

    pub fn notice(&self) -> Option<String> {
        match &self.origin {
            SchemaOrigin::Defaulted { requested, fallback } => {
                Some(format!("Unknown example type: {requested}. Falling back to {fallback}."))
            }
            _ => None,
        }
    }
}

pub fn resolve_schema(schema: Option<&str>, example: &str) -> Result<SchemaResolution> {
    if let Some(custom_schema) = schema {
        let parsed = serde_json::from_str::<Value>(custom_schema)
            .with_context(|| "Invalid custom schema JSON supplied for structured output")?;
        return Ok(SchemaResolution { schema: parsed, origin: SchemaOrigin::Custom });
    }

    let (schema, origin) = example_schema(example);
    Ok(SchemaResolution { schema, origin })
}

fn example_schema(example: &str) -> (Value, SchemaOrigin) {
    match example {
        "person" => (GenerationConfig::person_schema(), SchemaOrigin::Example("person")),
        "recipe" => (
            GenerationConfig::recipe_ingredients_schema(),
            SchemaOrigin::Example("recipe ingredients"),
        ),
        "orgchart" => {
            (GenerationConfig::org_chart_schema(), SchemaOrigin::Example("organization chart"))
        }
        other => (
            GenerationConfig::person_schema(),
            SchemaOrigin::Defaulted { requested: other.to_string(), fallback: "person" },
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_custom_schema() {
        let resolution = resolve_schema(Some("{\"type\":\"object\"}"), "person").unwrap();
        assert!(matches!(resolution.origin, SchemaOrigin::Custom));
        assert_eq!(resolution.schema, serde_json::json!({"type":"object"}));
    }

    #[test]
    fn errors_on_invalid_custom_schema() {
        let err = resolve_schema(Some("{\"type\":"), "person").unwrap_err();
        assert!(err.to_string().contains("Invalid custom schema JSON"));
    }

    #[test]
    fn defaults_when_example_unknown() {
        let resolution = resolve_schema(None, "unknown").unwrap();
        assert!(matches!(resolution.origin, SchemaOrigin::Defaulted { fallback: "person", .. }));
        assert!(resolution.notice().unwrap().contains("Unknown example type"));
    }
}
