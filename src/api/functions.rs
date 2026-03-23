//! Function/tool calling API

use crate::client::VertexClient;
use crate::error::{Result, VertexError};
use crate::models::{GenerateContentRequest, GenerateContentResponse};
use crate::types::{Content, FunctionCall, FunctionDeclaration, FunctionResponse, Part, Tool};
use serde_json::Value;

impl VertexClient {
    /// Generate content with function calling capabilities
    ///
    /// This method enables the model to call predefined functions based on the
    /// conversation context. The model will decide when and how to call functions
    /// to help answer the user's request.
    ///
    /// # Arguments
    ///
    /// * `model` - The model ID to use
    /// * `request` - The generation request with tools/functions defined
    ///
    /// # Example
    ///
    /// ```no_run
    /// use threatflux_vertex_rust_sdk::{config::Config, FunctionDeclaration, GenerateContentRequest, Tool, VertexClient};
    /// use serde_json::json;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = Config {
    ///     project_id: "project-id".into(),
    ///     region: "us-central1".into(),
    ///     ..Config::default()
    /// };
    /// let client = VertexClient::new(config).await?;
    ///
    /// let weather_fn = FunctionDeclaration {
    ///     name: "get_weather".to_string(),
    ///     description: "Get current weather".to_string(),
    ///     parameters: json!({
    ///         "type": "object",
    ///         "properties": {
    ///             "location": {"type": "string"}
    ///         },
    ///         "required": ["location"]
    ///     }),
    /// };
    ///
    /// let tool = Tool::function_calling(vec![weather_fn]);
    /// let request = GenerateContentRequest::new("What's the weather in Paris?")
    ///     .with_tools(vec![tool]);
    ///
    /// let response = client.generate_with_functions("gemini-2.0-flash-001", &request).await?;
    ///
    /// for function_call in response.function_calls() {
    ///     println!("Function called: {}", function_call.name);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error when no tools are configured on the request or when
    /// the underlying API call fails.
    pub async fn generate_with_functions(
        &self,
        model: &str,
        request: &GenerateContentRequest,
    ) -> Result<GenerateContentResponse> {
        // Ensure we have tools defined
        if request.tools.as_ref().is_none_or(Vec::is_empty) {
            return Err(VertexError::configuration(
                "No tools defined in request for function calling",
            ));
        }

        self.generate_content(model, request).await
    }

    /// Execute a complete function calling flow
    ///
    /// This is a higher-level method that handles the complete function calling flow:
    /// 1. Send initial request with function definitions
    /// 2. Execute any function calls returned by the model
    /// 3. Send function responses back to get the final answer
    ///
    /// # Arguments
    ///
    /// * `model` - The model ID to use
    /// * `request` - The generation request with tools defined
    /// * `executor` - Function to execute function calls
    ///
    /// # Example
    ///
    /// ```no_run
    /// use threatflux_vertex_rust_sdk::{
    ///     config::Config,
    ///     FunctionCall,
    ///     FunctionDeclaration,
    ///     GenerateContentRequest,
    ///     Tool,
    ///     VertexClient,
    /// };
    /// use serde_json::{json, Value};
    /// use std::io;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = Config {
    ///     project_id: "project-id".into(),
    ///     region: "us-central1".into(),
    ///     ..Config::default()
    /// };
    /// let client = VertexClient::new(config).await?;
    ///
    /// let add_function = FunctionDeclaration {
    ///     name: "add".to_string(),
    ///     description: "Add two numbers".to_string(),
    ///     parameters: json!({
    ///         "type": "object",
    ///         "properties": {
    ///             "a": {"type": "number"},
    ///             "b": {"type": "number"}
    ///         },
    ///         "required": ["a", "b"]
    ///     }),
    /// };
    ///
    /// let request = GenerateContentRequest::new("Calculate 2 + 3")
    ///     .with_tools(vec![Tool::function_calling(vec![add_function])]);
    ///
    /// let executor = |call: &FunctionCall| -> Result<Value, io::Error> {
    ///     match call.name.as_str() {
    ///         "add" => {
    ///             let a = call.args.get("a").and_then(|v| v.as_f64()).unwrap_or(0.0);
    ///             let b = call.args.get("b").and_then(|v| v.as_f64()).unwrap_or(0.0);
    ///             Ok(json!({"result": a + b}))
    ///         }
    ///         _ => Err(io::Error::new(io::ErrorKind::Other, "Unknown function")),
    ///     }
    /// };
    ///
    /// let _response = client
    ///     .execute_function_calling_flow("gemini-2.0-flash-001", &request, executor)
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error when the model invocation fails or when the executor
    /// reports a failure while calling an external function.
    pub async fn execute_function_calling_flow<F, E>(
        &self,
        model: &str,
        request: &GenerateContentRequest,
        executor: F,
    ) -> Result<GenerateContentResponse>
    where
        F: Fn(&FunctionCall) -> std::result::Result<Value, E> + Send + Sync,
        E: std::error::Error + Send + Sync + 'static,
    {
        // Step 1: Initial request
        let initial_response = self.generate_content(model, request).await?;

        let function_calls = initial_response.function_calls();
        if function_calls.is_empty() {
            // No function calls, return the response as-is
            return Ok(initial_response);
        }

        // Step 2: Build conversation with function calls and responses
        let mut conversation = request.contents.clone();

        // Add the model's response with function calls
        if let Some(candidate) = initial_response.candidates.first() {
            conversation.push(candidate.content.clone());
        }

        // Step 3: Execute function calls and add responses
        for function_call in &function_calls {
            let result = executor(function_call)
                .map_err(|e| VertexError::generic(format!("Function execution failed: {e}")))?;

            let function_response_part = Part::FunctionResponse {
                function_response: FunctionResponse {
                    name: function_call.name.clone(),
                    response: result,
                },
            };

            conversation
                .push(Content { role: "user".to_string(), parts: vec![function_response_part] });
        }

        // Step 4: Send back with function responses to get final answer
        let final_request = GenerateContentRequest {
            contents: conversation,
            generation_config: request.generation_config.clone(),
            safety_settings: request.safety_settings.clone(),
            tools: request.tools.clone(),
            system_instruction: request.system_instruction.clone(),
            cached_content: request.cached_content.clone(),
            tool_config: request.tool_config.clone(),
            metadata: request.metadata.clone(),
        };

        self.generate_content(model, &final_request).await
    }

    /// Create a simple function calling request
    ///
    /// Convenience method to create a request with a single function.
    #[must_use]
    pub fn create_function_request(
        prompt: &str,
        function: FunctionDeclaration,
    ) -> GenerateContentRequest {
        let tool = Tool::function_calling(vec![function]);

        GenerateContentRequest::new(prompt).with_tools(vec![tool]).with_generation_config(
            crate::types::GenerationConfig {
                temperature: Some(0.0), // Use low temperature for deterministic function calling
                ..Default::default()
            },
        )
    }
}

/// Function calling utilities
pub mod utils {
    use super::{FunctionCall, FunctionDeclaration, Result, Value, VertexError};
    use serde_json::json;

    /// Create a simple function declaration for basic operations
    #[must_use]
    pub fn create_calculator_function() -> FunctionDeclaration {
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

    /// Create a weather function declaration
    #[must_use]
    pub fn create_weather_function() -> FunctionDeclaration {
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

    /// Execute a calculator function call
    ///
    /// # Errors
    ///
    /// Returns an error when operands are missing/invalid, the operation is
    /// unknown, or when attempting to divide by zero.
    pub fn execute_calculator(call: &FunctionCall) -> Result<Value> {
        let operation = call
            .args
            .get("operation")
            .and_then(|v| v.as_str())
            .ok_or_else(|| VertexError::generic("Missing operation parameter"))?;

        let a = call
            .args
            .get("a")
            .and_then(serde_json::Value::as_f64)
            .ok_or_else(|| VertexError::generic("Missing or invalid parameter 'a'"))?;

        let b = call
            .args
            .get("b")
            .and_then(serde_json::Value::as_f64)
            .ok_or_else(|| VertexError::generic("Missing or invalid parameter 'b'"))?;

        let result = match operation {
            "add" => a + b,
            "subtract" => a - b,
            "multiply" => a * b,
            "divide" => {
                if b == 0.0 {
                    return Err(VertexError::generic("Division by zero"));
                }
                a / b
            }
            _ => return Err(VertexError::generic("Unknown operation")),
        };

        Ok(json!({
            "operation": operation,
            "operand_a": a,
            "operand_b": b,
            "result": result
        }))
    }

    /// Mock weather function execution (for testing)
    ///
    /// # Errors
    ///
    /// Returns an error if the required `location` parameter is absent.
    pub fn execute_mock_weather(call: &FunctionCall) -> Result<Value> {
        let location = call
            .args
            .get("location")
            .and_then(|v| v.as_str())
            .ok_or_else(|| VertexError::generic("Missing location parameter"))?;

        let unit = call.args.get("unit").and_then(|v| v.as_str()).unwrap_or("fahrenheit");

        // Mock weather data
        let temperature = if unit == "celsius" { 22 } else { 72 };

        Ok(json!({
            "location": location,
            "temperature": temperature,
            "unit": unit,
            "description": "Sunny with light clouds",
            "humidity": 65,
            "wind_speed": 8
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::utils::*;
    use super::*;
    use serde_json::json;
    use std::collections::HashMap;

    #[test]
    fn test_create_function_request() {
        let function = create_calculator_function();
        let request = VertexClient::create_function_request("Calculate 2 + 2", function);

        assert_eq!(request.contents.len(), 1);
        assert!(request.tools.is_some());
        assert_eq!(request.tools.unwrap().len(), 1);
        assert!(request.generation_config.is_some());
        assert_eq!(request.generation_config.unwrap().temperature, Some(0.0));
    }

    #[test]
    fn test_calculator_function_creation() {
        let func = create_calculator_function();
        assert_eq!(func.name, "calculate");
        assert!(!func.description.is_empty());
        assert!(!func.parameters.is_null());
    }

    #[test]
    fn test_weather_function_creation() {
        let func = create_weather_function();
        assert_eq!(func.name, "get_current_weather");
        assert!(!func.description.is_empty());
        assert!(!func.parameters.is_null());
    }

    #[test]
    fn test_calculator_execution() {
        let mut args = HashMap::new();
        args.insert("operation".to_string(), json!("add"));
        args.insert("a".to_string(), json!(5.0));
        args.insert("b".to_string(), json!(3.0));

        let call = FunctionCall { name: "calculate".to_string(), args };
        let result = execute_calculator(&call).unwrap();

        assert_eq!(result["result"], 8.0);
        assert_eq!(result["operation"], "add");
    }

    #[test]
    fn test_mock_weather_execution() {
        let mut args = HashMap::new();
        args.insert("location".to_string(), json!("New York"));
        args.insert("unit".to_string(), json!("fahrenheit"));

        let call = FunctionCall { name: "get_weather".to_string(), args };
        let result = execute_mock_weather(&call).unwrap();

        assert_eq!(result["location"], "New York");
        assert_eq!(result["temperature"], 72);
        assert_eq!(result["unit"], "fahrenheit");
    }
}
