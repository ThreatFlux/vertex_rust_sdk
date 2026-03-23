use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex,
    },
    time::Duration,
};

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use std::sync::LazyLock;
use threatflux_vertex_rust_sdk::{
    models::GenerateContentResponse,
    types::{Candidate, Content, UsageMetadata},
};

use super::{
    cases,
    client::ContentGenerator,
    config::SystemTestConfig,
    reporter::{ComparisonMode, Reporter, ResponseSummary, StdoutReporter},
    runner::{run_system_suite, Sleeper},
};

static ENV_GUARD: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

#[test]
fn config_defaults_without_env() {
    let _guard = ENV_GUARD.lock().unwrap();
    clear_test_env();

    let config = SystemTestConfig::from_env();
    assert!(!config.fast_mode);
    assert_eq!(config.case_limit, usize::MAX);
}

#[test]
fn config_reads_env_overrides() {
    let _guard = ENV_GUARD.lock().unwrap();
    clear_test_env();
    std::env::set_var("VERTEX_TEST_FAST", "1");
    std::env::set_var("VERTEX_TEST_CASE_LIMIT", "2");

    let config = SystemTestConfig::from_env();
    assert!(config.fast_mode);
    assert_eq!(config.case_limit, 2);

    clear_test_env();
}

#[test]
fn stdout_reporter_accepts_custom_writer() {
    let mut reporter = StdoutReporter::with_writer(Vec::new());
    reporter.suite_start("demo-model", false, usize::MAX);

    let output = String::from_utf8(reporter.into_inner()).unwrap();
    assert!(output.contains("System Instructions Test Suite"));
    assert!(output.contains("demo-model"));
}

#[tokio::test]
async fn suite_respects_fast_mode_and_case_limit() {
    let mut reporter = RecordingReporter::default();
    let generator = StubGenerator::new(vec![Ok(response_with_text("one", true))]);
    let sleeper = RecordingSleeper::default();
    let config = SystemTestConfig { fast_mode: true, case_limit: 1 };

    run_system_suite("gemini-1.5-flash", &config, &generator, &mut reporter, &sleeper)
        .await
        .unwrap();

    let events = reporter.events();
    assert!(events.iter().any(|event| event == "case_start:1:JSON Response Format"));
    assert!(events.iter().any(|event| event == "case_success:1:text"));
    assert!(events.iter().any(|event| event == "suite_end:fast"));
    assert!(events.iter().all(|event| !event.starts_with("comparison_start")));
    assert_eq!(sleeper.calls(), 0);
}

#[tokio::test]
async fn suite_runs_comparison_when_not_fast() {
    let mut reporter = RecordingReporter::default();
    let generator = StubGenerator::new(vec![
        Ok(response_with_text("first", false)),
        Ok(response_with_text("second", false)),
        Ok(response_with_text("without", true)),
        Ok(response_with_text("with", true)),
    ]);
    let sleeper = RecordingSleeper::default();
    let config = SystemTestConfig { fast_mode: false, case_limit: 2 };

    run_system_suite("gemini-1.5-flash", &config, &generator, &mut reporter, &sleeper)
        .await
        .unwrap();

    let events = reporter.events();
    assert!(events.iter().any(|event| event == "comparison_start"));
    assert!(events.iter().any(|event| event == "comparison_success:Without System Instruction"));
    assert!(events.iter().any(|event| event == "comparison_success:With System Instruction"));
    assert!(events.iter().any(|event| event == "suite_end:complete"));
    assert_eq!(sleeper.calls(), 2);
}

#[tokio::test]
async fn suite_reports_errors_and_missing_text() {
    let mut reporter = RecordingReporter::default();
    let generator = StubGenerator::new(vec![
        Err(anyhow!("boom")),
        Ok(GenerateContentResponse {
            candidates: Vec::new(),
            usage_metadata: None,
            grounding_metadata: None,
        }),
    ]);
    let sleeper = RecordingSleeper::default();
    let config = SystemTestConfig { fast_mode: true, case_limit: 2 };

    run_system_suite("gemini-1.5-flash", &config, &generator, &mut reporter, &sleeper)
        .await
        .unwrap();

    let events = reporter.events();
    assert!(events.iter().any(|event| event == "case_error:1:boom"));
    assert!(events.iter().any(|event| event == "case_missing_text:2"));
    assert_eq!(sleeper.calls(), 0);
}

fn response_with_text(text: &str, include_usage: bool) -> GenerateContentResponse {
    GenerateContentResponse {
        candidates: vec![Candidate {
            content: Content::model_text(text),
            finish_reason: None,
            safety_ratings: Vec::new(),
            index: None,
        }],
        usage_metadata: include_usage.then_some(UsageMetadata {
            prompt_token_count: 1,
            candidates_token_count: Some(1),
            total_token_count: 2,
            traffic_type: None,
            modality_token_count: None,
        }),
        grounding_metadata: None,
    }
}

fn clear_test_env() {
    for key in ["VERTEX_TEST_FAST", "VERTEX_TEST_CASE_LIMIT"] {
        std::env::remove_var(key);
    }
}

#[derive(Default)]
struct RecordingReporter {
    events: Mutex<Vec<String>>,
}

impl RecordingReporter {
    fn record(&self, event: impl Into<String>) {
        self.events.lock().unwrap().push(event.into());
    }

    fn events(&self) -> Vec<String> {
        self.events.lock().unwrap().clone()
    }
}

impl Reporter for RecordingReporter {
    fn suite_start(&mut self, _model: &str, fast_mode: bool, _case_limit: usize) {
        self.record(if fast_mode { "suite_start:fast" } else { "suite_start:normal" });
    }

    fn case_start(&mut self, index: usize, case: &cases::SystemTestCase) {
        self.record(format!("case_start:{index}:{name}", name = case.name));
    }

    fn case_success(&mut self, index: usize, summary: &ResponseSummary) {
        let kind = if summary.text.is_some() { "text" } else { "no_text" };
        self.record(format!("case_success:{index}:{kind}"));
    }

    fn case_missing_text(&mut self, index: usize) {
        self.record(format!("case_missing_text:{index}"));
    }

    fn case_error(&mut self, index: usize, error: &str) {
        self.record(format!("case_error:{index}:{error}"));
    }

    fn after_case(&mut self, index: usize) {
        self.record(format!("after_case:{index}"));
    }

    fn comparison_start(&mut self, _prompt: &str, _system_instruction: &str) {
        self.record("comparison_start");
    }

    fn comparison_success(&mut self, mode: ComparisonMode, _summary: &ResponseSummary) {
        self.record(format!("comparison_success:{}", mode.label()));
    }

    fn comparison_missing_text(&mut self, mode: ComparisonMode) {
        self.record(format!("comparison_missing_text:{}", mode.label()));
    }

    fn comparison_error(&mut self, mode: ComparisonMode, error: &str) {
        self.record(format!("comparison_error:{}:{}", mode.label(), error));
    }

    fn comparison_end(&mut self) {
        self.record("comparison_end");
    }

    fn suite_end(&mut self, fast_mode: bool) {
        self.record(if fast_mode { "suite_end:fast" } else { "suite_end:complete" });
    }
}

#[derive(Default)]
struct RecordingSleeper {
    calls: AtomicUsize,
}

impl RecordingSleeper {
    fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl Sleeper for RecordingSleeper {
    async fn sleep(&self, _duration: Duration) {
        self.calls.fetch_add(1, Ordering::SeqCst);
    }
}

struct StubGenerator {
    responses: Mutex<VecDeque<Result<GenerateContentResponse>>>,
}

impl StubGenerator {
    fn new(responses: Vec<Result<GenerateContentResponse>>) -> Self {
        Self { responses: Mutex::new(responses.into()) }
    }
}

#[async_trait]
impl ContentGenerator for StubGenerator {
    async fn generate(
        &self,
        _model: &str,
        _request: &threatflux_vertex_rust_sdk::models::GenerateContentRequest,
    ) -> Result<GenerateContentResponse> {
        self.responses
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or_else(|| Err(anyhow!("no stubbed response available")))
    }
}
