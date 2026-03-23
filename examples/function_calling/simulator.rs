use std::collections::HashMap;

use serde_json::{json, Value};
use threatflux_vertex_rust_sdk::{FunctionCall, FunctionResponse};

pub fn simulate(function_call: &FunctionCall) -> FunctionResponse {
    let payload = match function_call.name.as_str() {
        "get_current_weather" => simulate_weather(&function_call.args),
        "calculate" => simulate_calculator(&function_call.args),
        _ => json!({"error": format!("Unknown function: {}", function_call.name)}),
    };

    FunctionResponse { name: function_call.name.clone(), response: payload }
}

fn simulate_weather(args: &HashMap<String, Value>) -> Value {
    let location = get_string(args, "location").unwrap_or_else(|| "Unknown".to_string());
    let unit = get_string(args, "unit").unwrap_or_else(|| "fahrenheit".to_string());

    json!({
        "location": location,
        "temperature": 72,
        "unit": unit,
        "description": "Sunny with light clouds",
        "humidity": 65,
        "wind_speed": 8
    })
}

fn simulate_calculator(args: &HashMap<String, Value>) -> Value {
    let operation = get_string(args, "operation").unwrap_or_default();
    let a = args.get("a").and_then(Value::as_f64).unwrap_or_default();
    let b = args.get("b").and_then(Value::as_f64).unwrap_or_default();

    let result = match operation.as_str() {
        "add" => a + b,
        "subtract" => a - b,
        "multiply" => a * b,
        "divide" => {
            if b == 0.0 {
                f64::NAN
            } else {
                a / b
            }
        }
        _ => f64::NAN,
    };

    json!({
        "operation": operation,
        "operand_a": a,
        "operand_b": b,
        "result": result
    })
}

fn get_string(args: &HashMap<String, Value>, key: &str) -> Option<String> {
    args.get(key).and_then(Value::as_str).map(ToString::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_call(name: &str, args: &[(&str, Value)]) -> FunctionCall {
        FunctionCall {
            name: name.to_string(),
            args: args.iter().map(|(k, v)| (k.to_string(), v.clone())).collect(),
        }
    }

    #[test]
    fn weather_simulation_includes_defaults() {
        let call = build_call("get_current_weather", &[("location", Value::from("Boston"))]);
        let response = simulate(&call);
        assert_eq!(response.name, "get_current_weather");
        assert_eq!(response.response["location"], "Boston");
        assert_eq!(response.response["unit"], "fahrenheit");
        assert!(response.response["temperature"].is_number());
    }

    #[test]
    fn calculator_handles_division_and_zero() {
        let call = build_call(
            "calculate",
            &[
                ("operation", Value::from("divide")),
                ("a", Value::from(10.0)),
                ("b", Value::from(0.0)),
            ],
        );
        let response = simulate(&call);
        assert!(response.response["result"].as_f64().unwrap().is_nan());
    }

    #[test]
    fn unknown_function_returns_error() {
        let call = build_call("unknown", &[]);
        let response = simulate(&call);
        assert!(response.response["error"].as_str().unwrap().contains("Unknown function"));
    }
}
