//! Token counting example

use std::convert::TryFrom;
use threatflux_vertex_rust_sdk::{Content, CountTokensRequest, VertexClient};

fn chars_to_f64(len: usize) -> f64 {
    u32::try_from(len).map(f64::from).unwrap_or(f64::MAX)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    // Get project and location from environment
    let project_id = std::env::var("GOOGLE_CLOUD_PROJECT")
        .expect("Set GOOGLE_CLOUD_PROJECT environment variable");
    let location =
        std::env::var("GOOGLE_CLOUD_LOCATION").unwrap_or_else(|_| "us-central1".to_string());

    // Create client
    let client = VertexClient::new_legacy(&project_id, &location).await?;

    // Example 1: Simple text token counting
    println!("=== Example 1: Simple Text ===");
    let simple_text = "How many tokens are in this sentence?";
    let request1 = CountTokensRequest::new(simple_text);

    let response1 = client.count_tokens("gemini-2.0-flash-001", &request1).await?;
    println!("Text: \"{simple_text}\"");
    println!("Token count: {}\n", response1.total_tokens);

    // Example 2: Longer text
    println!("=== Example 2: Longer Text ===");
    let long_text = "Artificial intelligence (AI) is intelligence demonstrated by machines, in contrast to the natural intelligence displayed by humans and animals. Leading AI textbooks define the field as the study of 'intelligent agents': any device that perceives its environment and takes actions that maximize its chance of successfully achieving its goals.";
    let request2 = CountTokensRequest::new(long_text);

    let response2 = client.count_tokens("gemini-2.0-flash-001", &request2).await?;
    println!("Text length: {} characters", long_text.len());
    println!("Token count: {}", response2.total_tokens);
    println!(
        "Chars per token: {:.2}\n",
        chars_to_f64(long_text.len()) / f64::from(response2.total_tokens)
    );

    // Example 3: Multi-turn conversation
    println!("=== Example 3: Multi-turn Conversation ===");
    let conversation = vec![
        Content::user_text("What is machine learning?"),
        Content::model_text("Machine learning is a subset of artificial intelligence (AI) that focuses on developing algorithms and statistical models that enable computer systems to improve their performance on a specific task through experience, without being explicitly programmed for that task."),
        Content::user_text("Can you give me a simple example?"),
        Content::model_text("Sure! A simple example is email spam filtering. Instead of programming specific rules to identify spam, a machine learning algorithm learns from thousands of examples of spam and non-spam emails. Over time, it gets better at recognizing patterns that indicate whether a new email is likely to be spam or not."),
        Content::user_text("How does it learn from examples?"),
    ];

    let request3 = CountTokensRequest::with_contents(conversation);
    let response3 = client.count_tokens("gemini-2.0-flash-001", &request3).await?;
    println!("Conversation turns: 5 (3 user + 2 model + 1 pending user)");
    println!("Total conversation tokens: {}\n", response3.total_tokens);

    // Example 4: Code token counting
    println!("=== Example 4: Code ===");
    let code_text = r#"
def fibonacci(n):
    """Generate Fibonacci sequence up to n terms."""
    if n <= 0:
        return []
    elif n == 1:
        return [0]
    elif n == 2:
        return [0, 1]
    
    fib_sequence = [0, 1]
    for i in range(2, n):
        next_fib = fib_sequence[i-1] + fib_sequence[i-2]
        fib_sequence.append(next_fib)
    
    return fib_sequence

# Example usage
print(fibonacci(10))
"#;
    let request4 = CountTokensRequest::new(code_text);
    let response4 = client.count_tokens("gemini-2.0-flash-001", &request4).await?;
    println!("Code length: {} characters", code_text.len());
    println!("Token count: {}", response4.total_tokens);
    println!(
        "Chars per token: {:.2}\n",
        chars_to_f64(code_text.len()) / f64::from(response4.total_tokens)
    );

    // Example 5: Different languages
    println!("=== Example 5: Different Languages ===");

    let texts = vec![
        ("English", "Hello, how are you today?"),
        ("Spanish", "Hola, ¿cómo estás hoy?"),
        ("French", "Bonjour, comment allez-vous aujourd'hui?"),
        ("German", "Hallo, wie geht es dir heute?"),
        ("Japanese", "こんにちは、今日はいかがですか？"),
        ("Chinese", "你好，你今天怎么样？"),
    ];

    for (language, text) in texts {
        let request = CountTokensRequest::new(text);
        let response = client.count_tokens("gemini-2.0-flash-001", &request).await?;
        println!("{language}: \"{text}\" - {} tokens", response.total_tokens);
    }

    println!("\n=== Token Counting Tips ===");
    println!("1. Tokens roughly correspond to words, but punctuation and spaces also count");
    println!("2. Different languages may have different tokenization patterns");
    println!("3. Code typically has more tokens per character due to special characters");
    println!("4. Use token counting to estimate costs and stay within model limits");
    println!("5. Gemini models have different context windows (token limits)");

    Ok(())
}
