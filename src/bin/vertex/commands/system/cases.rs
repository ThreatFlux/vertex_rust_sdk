#[derive(Debug, Clone, Copy)]
pub struct SystemTestCase {
    pub name: &'static str,
    pub system_instruction: &'static str,
    pub prompt: &'static str,
}

pub const fn cases() -> &'static [SystemTestCase] {
    &[
        SystemTestCase {
            name: "JSON Response Format",
            system_instruction: "Always respond in JSON format with a 'message' field",
            prompt: "Tell me about the weather",
        },
        SystemTestCase {
            name: "Pirate Personality",
            system_instruction:
                "You are a friendly pirate. Always respond in pirate speak with 'Ahoy matey!' and use pirate vocabulary",
            prompt: "How do I learn programming?",
        },
        SystemTestCase {
            name: "Response Length Constraint",
            system_instruction: "Keep all responses under 30 words",
            prompt: "Explain quantum computing",
        },
        SystemTestCase {
            name: "Spanish Language",
            system_instruction: "Always respond in Spanish",
            prompt: "What are the benefits of exercise?",
        },
        SystemTestCase {
            name: "Haiku Format",
            system_instruction: "Always respond in haiku format (5-7-5 syllable pattern)",
            prompt: "Describe artificial intelligence",
        },
        SystemTestCase {
            name: "Technical Expert Role",
            system_instruction: "You are a senior software engineer specializing in Rust. Always provide technical, detailed explanations with code examples when relevant",
            prompt: "How do I handle errors in Rust?",
        },
    ]
}

pub const fn comparison_prompt() -> &'static str {
    "Explain machine learning"
}

pub const fn comparison_system_instruction() -> &'static str {
    "You are an expert teacher who explains complex topics in simple terms using analogies"
}
