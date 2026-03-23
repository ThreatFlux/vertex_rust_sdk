use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use threatflux_vertex_rust_sdk::models::GenerateContentRequest;

use super::{
    cases,
    client::ContentGenerator,
    config::SystemTestConfig,
    reporter::{ComparisonMode, Reporter, ResponseSummary},
};

const SLEEP_BETWEEN_CASES: Duration = Duration::from_millis(500);

#[async_trait]
pub trait Sleeper: Send + Sync {
    async fn sleep(&self, duration: Duration);
}

pub struct TokioSleeper;

#[async_trait]
impl Sleeper for TokioSleeper {
    async fn sleep(&self, duration: Duration) {
        tokio::time::sleep(duration).await;
    }
}

pub async fn run_system_suite<G, R, S>(
    model: &str,
    config: &SystemTestConfig,
    generator: &G,
    reporter: &mut R,
    sleeper: &S,
) -> Result<()>
where
    G: ContentGenerator + Send + Sync,
    R: Reporter + ?Sized,
    S: Sleeper + Sync,
{
    reporter.suite_start(model, config.fast_mode, config.case_limit);

    for (index, case) in cases::cases().iter().enumerate() {
        if index >= config.case_limit {
            break;
        }

        let request =
            GenerateContentRequest::new(case.prompt).with_system_text(case.system_instruction);
        reporter.case_start(index + 1, case);

        match generator.generate(model, &request).await {
            Ok(response) => {
                let summary = ResponseSummary::from_response(&response);
                if summary.text.is_some() {
                    reporter.case_success(index + 1, &summary);
                } else {
                    reporter.case_missing_text(index + 1);
                }
            }
            Err(error) => reporter.case_error(index + 1, &error.to_string()),
        }

        reporter.after_case(index + 1);

        if !config.fast_mode {
            sleeper.sleep(SLEEP_BETWEEN_CASES).await;
        }
    }

    if !config.fast_mode {
        run_comparison(model, generator, reporter).await?;
    }

    reporter.suite_end(config.fast_mode);

    Ok(())
}

pub async fn run_comparison<G, R>(model: &str, generator: &G, reporter: &mut R) -> Result<()>
where
    G: ContentGenerator + Send + Sync,
    R: Reporter + ?Sized,
{
    let prompt = cases::comparison_prompt();
    let system_instruction = cases::comparison_system_instruction();

    reporter.comparison_start(prompt, system_instruction);

    let request_without = GenerateContentRequest::new(prompt);
    match generator.generate(model, &request_without).await {
        Ok(response) => {
            let summary = ResponseSummary::from_response(&response);
            if summary.text.is_some() {
                reporter.comparison_success(ComparisonMode::WithoutSystem, &summary);
            } else {
                reporter.comparison_missing_text(ComparisonMode::WithoutSystem);
            }
        }
        Err(error) => reporter.comparison_error(ComparisonMode::WithoutSystem, &error.to_string()),
    }

    let request_with = GenerateContentRequest::new(prompt).with_system_text(system_instruction);
    match generator.generate(model, &request_with).await {
        Ok(response) => {
            let summary = ResponseSummary::from_response(&response);
            if summary.text.is_some() {
                reporter.comparison_success(ComparisonMode::WithSystem, &summary);
            } else {
                reporter.comparison_missing_text(ComparisonMode::WithSystem);
            }
        }
        Err(error) => reporter.comparison_error(ComparisonMode::WithSystem, &error.to_string()),
    }

    reporter.comparison_end();

    Ok(())
}
