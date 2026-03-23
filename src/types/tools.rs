use serde::{Deserialize, Serialize};

use super::code_execution::CodeExecutionTool;
use super::function_calling::FunctionDeclaration;
use super::grounding::GroundingConfig;

/// Function calling mode.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FunctionCallingMode {
    #[serde(rename = "AUTO")]
    Auto,
    #[serde(rename = "ANY")]
    Any,
    #[serde(rename = "NONE")]
    None,
    #[serde(rename = "VALIDATED")]
    Validated,
}

/// Function calling configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCallingConfig {
    pub mode: FunctionCallingMode,
    #[serde(rename = "allowedFunctionNames")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_function_names: Option<Vec<String>>,
}

/// Tool configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolConfig {
    #[serde(rename = "functionCallingConfig")]
    pub function_calling_config: FunctionCallingConfig,
}

/// Tool specification.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Tool {
    /// Function calling tool.
    FunctionCalling {
        #[serde(rename = "functionDeclarations")]
        function_declarations: Vec<FunctionDeclaration>,
    },
    /// Code execution tool.
    CodeExecution {
        #[serde(rename = "codeExecution")]
        code_execution: CodeExecutionTool,
    },
    /// Google Search retrieval tool.
    GoogleSearchRetrieval {
        #[serde(rename = "googleSearchRetrieval")]
        google_search_retrieval: GroundingConfig,
    },
}

impl FunctionCallingConfig {
    /// Create AUTO mode configuration.
    #[must_use]
    pub const fn auto() -> Self {
        Self { mode: FunctionCallingMode::Auto, allowed_function_names: None }
    }

    /// Create ANY mode configuration (force function calling).
    #[must_use]
    pub const fn any() -> Self {
        Self { mode: FunctionCallingMode::Any, allowed_function_names: None }
    }

    /// Create ANY mode with specific allowed functions.
    #[must_use]
    pub const fn any_with_functions(function_names: Vec<String>) -> Self {
        Self { mode: FunctionCallingMode::Any, allowed_function_names: Some(function_names) }
    }

    /// Create NONE mode configuration (disable function calling).
    #[must_use]
    pub const fn none() -> Self {
        Self { mode: FunctionCallingMode::None, allowed_function_names: None }
    }

    /// Create VALIDATED mode configuration.
    #[must_use]
    pub const fn validated() -> Self {
        Self { mode: FunctionCallingMode::Validated, allowed_function_names: None }
    }

    /// Create VALIDATED mode with specific allowed functions.
    #[must_use]
    pub const fn validated_with_functions(function_names: Vec<String>) -> Self {
        Self { mode: FunctionCallingMode::Validated, allowed_function_names: Some(function_names) }
    }
}

impl ToolConfig {
    /// Create tool config with AUTO mode.
    #[must_use]
    pub const fn auto() -> Self {
        Self { function_calling_config: FunctionCallingConfig::auto() }
    }

    /// Create tool config with ANY mode (force function calling).
    #[must_use]
    pub const fn any() -> Self {
        Self { function_calling_config: FunctionCallingConfig::any() }
    }

    /// Create tool config with NONE mode (disable function calling).
    #[must_use]
    pub const fn none() -> Self {
        Self { function_calling_config: FunctionCallingConfig::none() }
    }

    /// Create tool config with VALIDATED mode.
    #[must_use]
    pub const fn validated() -> Self {
        Self { function_calling_config: FunctionCallingConfig::validated() }
    }

    /// Create tool config with custom function calling config.
    #[must_use]
    pub const fn with_config(config: FunctionCallingConfig) -> Self {
        Self { function_calling_config: config }
    }
}

impl Tool {
    /// Create a function calling tool.
    #[must_use]
    pub const fn function_calling(function_declarations: Vec<FunctionDeclaration>) -> Self {
        Self::FunctionCalling { function_declarations }
    }

    /// Create a code execution tool.
    #[must_use]
    pub const fn code_execution() -> Self {
        Self::CodeExecution { code_execution: CodeExecutionTool::new() }
    }

    /// Create a Google Search retrieval tool.
    #[must_use]
    pub fn google_search() -> Self {
        Self::GoogleSearchRetrieval { google_search_retrieval: GroundingConfig::default() }
    }

    /// Create a Google Search retrieval tool with custom config.
    #[must_use]
    pub const fn google_search_with_config(config: GroundingConfig) -> Self {
        Self::GoogleSearchRetrieval { google_search_retrieval: config }
    }
}
