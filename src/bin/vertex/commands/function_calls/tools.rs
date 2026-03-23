use serde_json::{json, Value};
use threatflux_vertex_rust_sdk::types::{
    FunctionCall, FunctionDeclaration, FunctionResponse, Tool,
};

pub fn available_tool() -> Tool {
    Tool::function_calling(vec![weather_function(), multiply_function()])
}

pub fn execute_function_call(call: &FunctionCall) -> FunctionResponse {
    let response = match call.name.as_str() {
        "get_weather" => weather_payload(call),
        "get_current_weather" => current_weather_payload(call),
        "multiply" => multiply_payload(call),
        name => json!({
            "error": format!("Unknown function: {name}"),
        }),
    };

    FunctionResponse { name: call.name.clone(), response }
}

fn weather_function() -> FunctionDeclaration {
    FunctionDeclaration {
        name: "get_weather".to_string(),
        description: "Get the current weather in a city".to_string(),
        parameters: json!({
            "type": "object",
            "properties": {
                "city": {
                    "type": "string",
                    "description": "The city to get weather for"
                }
            },
            "required": ["city"]
        }),
    }
}

fn multiply_function() -> FunctionDeclaration {
    FunctionDeclaration {
        name: "multiply".to_string(),
        description: "Multiply two numbers".to_string(),
        parameters: json!({
            "type": "object",
            "properties": {
                "a": {
                    "type": "number",
                    "description": "First number"
                },
                "b": {
                    "type": "number",
                    "description": "Second number"
                }
            },
            "required": ["a", "b"]
        }),
    }
}

fn weather_payload(call: &FunctionCall) -> Value {
    let city = call.args.get("city").and_then(Value::as_str).unwrap_or("Unknown");

    json!({
        "temperature": 72,
        "condition": "Sunny",
        "humidity": 65,
        "city": city
    })
}

fn current_weather_payload(call: &FunctionCall) -> Value {
    let location = call.args.get("location").and_then(Value::as_str).unwrap_or("Unknown");

    json!({
        "temperature": 18,
        "weather": "Cloudy",
        "location": location
    })
}

fn multiply_payload(call: &FunctionCall) -> Value {
    let a = call.args.get("a").and_then(Value::as_f64).unwrap_or(0.0);
    let b = call.args.get("b").and_then(Value::as_f64).unwrap_or(0.0);

    json!({
        "result": a * b,
        "a": a,
        "b": b
    })
}
