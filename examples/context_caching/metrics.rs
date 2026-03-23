use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestionOutcome {
    pub question: String,
    pub elapsed: Duration,
    pub preview: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunSummary {
    pub outcomes: Vec<QuestionOutcome>,
}

impl RunSummary {
    #[must_use]
    pub fn total_time(&self) -> Duration {
        self.outcomes.iter().fold(Duration::ZERO, |total, outcome| total + outcome.elapsed)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Savings {
    pub duration: Duration,
    pub percentage: f64,
}

#[must_use]
pub fn compare_runs(without_cache: &RunSummary, with_cache: &RunSummary) -> Option<Savings> {
    let without_time = without_cache.total_time();
    let with_time = with_cache.total_time();

    let duration = without_time.checked_sub(with_time)?;
    if duration.is_zero() || without_time.is_zero() {
        return None;
    }

    let percentage = duration.as_secs_f64() / without_time.as_secs_f64() * 100.0;
    Some(Savings { duration, percentage })
}

#[must_use]
pub fn preview_for(text: Option<String>) -> Option<String> {
    match text {
        Some(body) if body.len() > 100 => Some(format!("{}...", &body[..100])),
        Some(body) => Some(body),
        None => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn summary_with_times(times: &[u64]) -> RunSummary {
        RunSummary {
            outcomes: times
                .iter()
                .enumerate()
                .map(|(i, secs)| QuestionOutcome {
                    question: format!("Q{i}"),
                    elapsed: Duration::from_secs(*secs),
                    preview: None,
                })
                .collect(),
        }
    }

    #[test]
    fn calculates_savings_when_faster() {
        let without_cache = summary_with_times(&[3, 3]);
        let with_cache = summary_with_times(&[1, 1]);

        let result = compare_runs(&without_cache, &with_cache).unwrap();
        assert_eq!(result.duration, Duration::from_secs(4));
        assert!((result.percentage - 66.6).abs() < 0.1);
    }

    #[test]
    fn returns_none_when_not_faster() {
        let without_cache = summary_with_times(&[1, 1]);
        let with_cache = summary_with_times(&[2, 1]);

        assert!(compare_runs(&without_cache, &with_cache).is_none());
    }

    #[test]
    fn returns_none_for_zero_without_cache() {
        let without_cache = summary_with_times(&[]);
        let with_cache = summary_with_times(&[]);

        assert!(compare_runs(&without_cache, &with_cache).is_none());
    }

    #[test]
    fn previews_are_trimmed_when_needed() {
        let short = "ok".repeat(10);
        assert_eq!(preview_for(Some(short.clone())), Some(short));

        let long = "x".repeat(120);
        let preview = preview_for(Some(long));
        assert_eq!(preview.unwrap().len(), 103);
    }
}
