use serde::{Deserialize, Serialize};

/// Code execution tool configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeExecutionTool {}

impl CodeExecutionTool {
    /// Create a new code execution tool.
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }
}

impl Default for CodeExecutionTool {
    fn default() -> Self {
        Self::new()
    }
}

/// Programming language for executable code.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Language {
    #[serde(rename = "PYTHON")]
    Python,
    #[serde(rename = "LANGUAGE_UNSPECIFIED")]
    Unspecified,
}

/// Executable code part.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutableCode {
    /// Programming language.
    pub language: Language,
    /// Code to execute.
    pub code: String,
}

/// Code execution outcome.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Outcome {
    #[serde(rename = "OUTCOME_OK")]
    Ok,
    #[serde(rename = "OUTCOME_FAILED")]
    Failed,
    #[serde(rename = "OUTCOME_DEADLINE_EXCEEDED")]
    DeadlineExceeded,
    #[serde(rename = "OUTCOME_UNSPECIFIED")]
    Unspecified,
}

/// Code execution result part.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeExecutionResult {
    /// Execution outcome.
    pub outcome: Outcome,
    /// Output from code execution.
    pub output: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_and_language_round_trip() {
        let tool = CodeExecutionTool::new();
        let encoded = serde_json::to_string(&tool).unwrap();
        assert_eq!(encoded, "{}");

        let code = ExecutableCode { language: Language::Python, code: "print('hi')".to_string() };
        let json = serde_json::to_string(&code).unwrap();
        let decoded: ExecutableCode = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.code, "print('hi')");
        matches!(decoded.language, Language::Python);
    }

    #[test]
    fn outcomes_deserialize() {
        let result: CodeExecutionResult =
            serde_json::from_str(r#"{"outcome":"OUTCOME_FAILED","output":"boom"}"#).unwrap();
        assert!(matches!(result.outcome, Outcome::Failed));
        assert_eq!(result.output, "boom");

        let unspecified: Outcome = serde_json::from_str(r#""OUTCOME_UNSPECIFIED""#).unwrap();
        assert!(matches!(unspecified, Outcome::Unspecified));
    }
}
