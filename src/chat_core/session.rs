use crate::chat_core::config::ChatConfig;
use crate::{Content, GenerateContentRequest, GenerationConfig, Part};

pub struct ChatSession {
    history: Vec<Content>,
    system_instruction: Option<Content>,
    temperature: f32,
    max_tokens: i32,
}

#[derive(Clone, Copy)]
pub struct SessionStats {
    pub messages: usize,
    pub temperature: f32,
}

impl ChatSession {
    pub fn new(config: &ChatConfig) -> Self {
        let system_instruction = config.system.as_ref().map(|text| Content {
            role: "system".to_string(),
            parts: vec![Part::Text { text: text.clone() }],
        });

        Self {
            history: Vec::new(),
            system_instruction,
            temperature: config.temperature,
            max_tokens: config.max_tokens,
        }
    }

    pub fn add_user_message(&mut self, text: String) {
        self.history.push(Content { role: "user".to_string(), parts: vec![Part::Text { text }] });
    }

    pub fn add_model_message(&mut self, text: String) {
        self.history.push(Content { role: "model".to_string(), parts: vec![Part::Text { text }] });
    }

    pub fn rollback_last(&mut self) {
        self.history.pop();
    }

    pub fn clear(&mut self) {
        self.history.clear();
    }

    pub const fn stats(&self) -> SessionStats {
        SessionStats { messages: self.history.len(), temperature: self.temperature }
    }

    pub const fn set_temperature(&mut self, value: f32) {
        self.temperature = value;
    }

    #[cfg(test)]
    pub const fn temperature(&self) -> f32 {
        self.temperature
    }

    pub fn build_request(&self) -> GenerateContentRequest {
        GenerateContentRequest {
            contents: self.history.clone(),
            system_instruction: self.system_instruction.clone(),
            generation_config: Some(GenerationConfig {
                temperature: Some(self.temperature),
                max_output_tokens: Some(self.max_tokens),
                top_p: Some(0.95),
                top_k: Some(40),
                ..Default::default()
            }),
            safety_settings: None,
            tools: None,
            tool_config: None,
            cached_content: None,
            metadata: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chat_core::config::DEFAULT_SYSTEM_PROMPT;

    fn base_config() -> ChatConfig {
        ChatConfig {
            project: "p".to_string(),
            location: "l".to_string(),
            model: "m".to_string(),
            temperature: 1.0,
            max_tokens: 10,
            system: Some(DEFAULT_SYSTEM_PROMPT.to_string()),
            debug: false,
        }
    }

    #[test]
    fn builds_request_with_system_and_history() {
        let mut session = ChatSession::new(&base_config());
        session.add_user_message("hi".to_string());
        let request = session.build_request();
        assert_eq!(request.contents.len(), 1);
        assert!(request.system_instruction.is_some());
        let config = request.generation_config.unwrap();
        assert_eq!(config.temperature, Some(1.0));
        assert_eq!(config.max_output_tokens, Some(10));
    }

    #[test]
    fn tracks_stats_and_temperature() {
        let mut session = ChatSession::new(&base_config());
        session.add_user_message("hi".to_string());
        let stats = session.stats();
        assert_eq!(stats.messages, 1);
        assert!((stats.temperature - 1.0).abs() < f32::EPSILON);

        session.set_temperature(0.5);
        assert!((session.temperature() - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn rolls_back_last_message() {
        let mut session = ChatSession::new(&base_config());
        session.add_user_message("hi".to_string());
        session.add_model_message("there".to_string());
        session.rollback_last();
        assert_eq!(session.history.len(), 1);
    }
}
