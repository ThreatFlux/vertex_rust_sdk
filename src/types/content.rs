use serde::{Deserialize, Serialize};

use super::code_execution::{CodeExecutionResult, ExecutableCode, Language, Outcome};
use super::function_calling::{FunctionCall, FunctionResponse};

/// Thinking content from models with thinking mode enabled.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThinkingPart {
    /// The thinking content/process text.
    pub content: String,
}

/// Content part - can be text, file data, function call, code execution, or thinking.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Part {
    /// Text content.
    Text { text: String },

    /// Inline data content.
    InlineData {
        #[serde(rename = "inlineData")]
        inline_data: InlineData,
    },

    /// File data content.
    FileData {
        #[serde(rename = "fileData")]
        file_data: FileData,
    },

    /// Function call.
    FunctionCall {
        #[serde(rename = "functionCall")]
        function_call: FunctionCall,
    },

    /// Function response.
    FunctionResponse {
        #[serde(rename = "functionResponse")]
        function_response: FunctionResponse,
    },

    /// Executable code.
    ExecutableCode {
        #[serde(rename = "executableCode")]
        executable_code: ExecutableCode,
    },

    /// Code execution result.
    CodeExecutionResult {
        #[serde(rename = "codeExecutionResult")]
        code_execution_result: CodeExecutionResult,
    },

    /// Thinking process content.
    Thinking {
        #[serde(rename = "thought")]
        thought: String,
    },
}

impl Part {
    /// Create a text part.
    #[must_use]
    pub fn text<S: Into<String>>(text: S) -> Self {
        Self::Text { text: text.into() }
    }

    /// Create an inline data part.
    #[must_use]
    pub fn inline_data<D: Into<String>, M: Into<String>>(data: D, mime_type: M) -> Self {
        Self::InlineData {
            inline_data: InlineData { data: data.into(), mime_type: mime_type.into() },
        }
    }

    /// Create a file data part.
    #[must_use]
    pub const fn file_data(file_uri: String, mime_type: String) -> Self {
        Self::FileData { file_data: FileData { file_uri, mime_type } }
    }

    /// Create an executable code part.
    #[must_use]
    pub const fn executable_code(language: Language, code: String) -> Self {
        Self::ExecutableCode { executable_code: ExecutableCode { language, code } }
    }

    /// Create a code execution result part.
    #[must_use]
    pub const fn code_execution_result(outcome: Outcome, output: String) -> Self {
        Self::CodeExecutionResult { code_execution_result: CodeExecutionResult { outcome, output } }
    }

    /// Create a thinking part.
    #[must_use]
    pub fn thinking<S: Into<String>>(thought: S) -> Self {
        Self::Thinking { thought: thought.into() }
    }
}

/// File data for multimodal content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileData {
    #[serde(rename = "fileUri")]
    pub file_uri: String,
    #[serde(rename = "mimeType")]
    pub mime_type: String,
}

/// Inline data for multimodal content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InlineData {
    #[serde(rename = "mimeType")]
    pub mime_type: String,
    pub data: String,
}

/// Content with role and parts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Content {
    /// Role of the content (user, model, system).
    pub role: String,
    /// Parts of the content.
    pub parts: Vec<Part>,
}

impl Content {
    /// Create user content with text.
    #[must_use]
    pub fn user_text<S: Into<String>>(text: S) -> Self {
        Self { role: "user".to_string(), parts: vec![Part::text(text)] }
    }

    /// Create model content with text.
    #[must_use]
    pub fn model_text<S: Into<String>>(text: S) -> Self {
        Self { role: "model".to_string(), parts: vec![Part::text(text)] }
    }

    /// Create system content with text.
    #[must_use]
    pub fn system_text<S: Into<String>>(text: S) -> Self {
        Self { role: "system".to_string(), parts: vec![Part::text(text)] }
    }
}
