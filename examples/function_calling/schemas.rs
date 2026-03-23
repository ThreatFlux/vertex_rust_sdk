use serde_json::json;
use threatflux_vertex_rust_sdk::{FunctionDeclaration, Tool};

pub fn build_tool() -> Tool {
    Tool::function_calling(vec![weather_function(), calculator_function()])
}

pub fn weather_function() -> FunctionDeclaration {
    FunctionDeclaration {
        name: "get_current_weather".to_string(),
        description: "Get the current weather in a given location".to_string(),
        parameters: json!({
            "type": "object",
            "properties": {
                "location": {
                    "type": "string",
                    "description": "The city and state or city and country"
                },
                "unit": {
                    "type": "string",
                    "enum": ["celsius", "fahrenheit"],
                    "description": "Temperature unit",
                    "default": "fahrenheit"
                }
            },
            "required": ["location"]
        }),
    }
}

pub fn calculator_function() -> FunctionDeclaration {
    FunctionDeclaration {
        name: "calculate".to_string(),
        description: "Perform basic mathematical calculations".to_string(),
        parameters: json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": ["add", "subtract", "multiply", "divide"],
                    "description": "Mathematical operation to perform"
                },
                "a": {
                    "type": "number",
                    "description": "First number"
                },
                "b": {
                    "type": "number",
                    "description": "Second number"
                }
            },
            "required": ["operation", "a", "b"]
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_contains_both_functions() {
        let tool = build_tool();
        assert_eq!(tool.function_declarations.len(), 2);
        let names: Vec<_> = tool.function_declarations.iter().map(|f| f.name.as_str()).collect();
        assert!(names.contains(&"get_current_weather"));
        assert!(names.contains(&"calculate"));
    }

    #[test]
    fn schemas_include_defaults_and_required_fields() {
        let weather = weather_function();
        assert!(weather.parameters["properties"]["unit"]["default"].as_str().is_some());
        let calculator = calculator_function();
        let required = calculator.parameters["required"].as_array().unwrap();
        assert!(required.iter().any(|r| r == "operation"));
        assert!(required.iter().any(|r| r == "a"));
        assert!(required.iter().any(|r| r == "b"));
    }
}
