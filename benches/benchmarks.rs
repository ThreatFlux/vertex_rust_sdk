//! Benchmarks for Vertex Rust SDK

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use std::hint::black_box;
use threatflux_vertex_rust_sdk::{
    ChatMessage, Content, CountTokensRequest, GenerateContentRequest, GenerationConfig, Part,
};

/// Benchmark request serialization
fn bench_request_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("serialization");

    // Simple request
    let simple_request = GenerateContentRequest::new("Hello world");
    group.bench_function("simple_request", |b| {
        b.iter(|| serde_json::to_string(black_box(&simple_request)).unwrap());
    });

    // Complex request with config
    let complex_request = GenerateContentRequest::new("Complex prompt with multiple parameters")
        .with_generation_config(GenerationConfig {
            temperature: Some(0.7),
            top_p: Some(0.95),
            top_k: Some(40),
            max_output_tokens: Some(1024),
            stop_sequences: Some(vec!["END".to_string(), "STOP".to_string()]),
            candidate_count: Some(1),
            response_mime_type: None,
            response_schema: None,
            thinking_config: None,
        });

    group.bench_function("complex_request", |b| {
        b.iter(|| serde_json::to_string(black_box(&complex_request)).unwrap());
    });

    // Multi-turn conversation
    let conversation = vec![
        Content::user_text("What is machine learning?"),
        Content::model_text("Machine learning is a subset of AI..."),
        Content::user_text("Can you give examples?"),
        Content::model_text("Sure, here are some examples..."),
        Content::user_text("What about deep learning?"),
    ];
    let conversation_request = GenerateContentRequest::with_contents(conversation);

    group.bench_function("conversation_request", |b| {
        b.iter(|| serde_json::to_string(black_box(&conversation_request)).unwrap());
    });

    group.finish();
}

/// Benchmark content creation
fn bench_content_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("content_creation");

    group.bench_function("user_text", |b| {
        b.iter(|| Content::user_text(black_box("This is a test message")));
    });

    group.bench_function("model_text", |b| {
        b.iter(|| Content::model_text(black_box("This is a response message")));
    });

    group.bench_function("text_part", |b| {
        b.iter(|| Part::text(black_box("This is a text part")));
    });

    group.finish();
}

/// Benchmark different request sizes
fn bench_request_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("request_sizes");

    // Different text sizes
    let sizes = [100, 500, 1000, 5000, 10000];

    for size in &sizes {
        let text = "a".repeat(*size);
        let request = GenerateContentRequest::new(&text);

        group.bench_with_input(BenchmarkId::new("serialize", size), size, |b, _| {
            b.iter(|| serde_json::to_string(black_box(&request)).unwrap());
        });
    }

    group.finish();
}

/// Benchmark token counting requests
fn bench_token_counting(c: &mut Criterion) {
    let mut group = c.benchmark_group("token_counting");

    let simple_request = CountTokensRequest::new("Count these tokens");
    group.bench_function("simple_count_request", |b| {
        b.iter(|| serde_json::to_string(black_box(&simple_request)).unwrap());
    });

    let conversation = vec![
        Content::user_text("What is AI?"),
        Content::model_text("Artificial Intelligence is..."),
        Content::user_text("Tell me more about machine learning"),
    ];
    let conversation_request = CountTokensRequest::with_contents(conversation);

    group.bench_function("conversation_count_request", |b| {
        b.iter(|| serde_json::to_string(black_box(&conversation_request)).unwrap());
    });

    group.finish();
}

/// Benchmark chat message conversions
fn bench_chat_messages(c: &mut Criterion) {
    let mut group = c.benchmark_group("chat_messages");

    let user_msg = ChatMessage::user("Hello");
    group.bench_function("user_message_creation", |b| {
        b.iter(|| ChatMessage::user(black_box("Hello world")));
    });

    group.bench_function("message_to_content", |b| {
        b.iter(|| {
            let content: Content = black_box(user_msg.clone()).into();
            content
        });
    });

    // Batch conversion
    let messages = vec![
        ChatMessage::system("You are helpful"),
        ChatMessage::user("Hello"),
        ChatMessage::assistant("Hi there!"),
        ChatMessage::user("How are you?"),
        ChatMessage::assistant("I'm doing well, thanks!"),
    ];

    group.bench_function("batch_message_conversion", |b| {
        b.iter(|| {
            let contents: Vec<Content> =
                black_box(&messages).iter().cloned().map(Into::into).collect();
            contents
        });
    });

    group.finish();
}

/// Benchmark URL construction (simulated)
fn bench_url_construction(c: &mut Criterion) {
    let mut group = c.benchmark_group("url_construction");

    let project_id = "test-project";
    let location = "us-central1";
    let model = "gemini-2.0-flash-001";

    group.bench_function("generate_content_url", |b| {
        b.iter(|| {
            format!(
                "https://{}-aiplatform.googleapis.com/v1/projects/{}/locations/{}/publishers/google/models/{}:generateContent",
                black_box(location),
                black_box(project_id),
                black_box(location),
                black_box(model)
            )
        });
    });

    group.bench_function("stream_generate_url", |b| {
        b.iter(|| {
            format!(
                "https://{}-aiplatform.googleapis.com/v1/projects/{}/locations/{}/publishers/google/models/{}:streamGenerateContent",
                black_box(location),
                black_box(project_id),
                black_box(location),
                black_box(model)
            )
        });
    });

    group.bench_function("count_tokens_url", |b| {
        b.iter(|| {
            format!(
                "https://{}-aiplatform.googleapis.com/v1/projects/{}/locations/{}/publishers/google/models/{}:countTokens",
                black_box(location),
                black_box(project_id),
                black_box(location),
                black_box(model)
            )
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_request_serialization,
    bench_content_creation,
    bench_request_sizes,
    bench_token_counting,
    bench_chat_messages,
    bench_url_construction
);
criterion_main!(benches);
