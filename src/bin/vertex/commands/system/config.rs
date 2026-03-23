#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SystemTestConfig {
    pub fast_mode: bool,
    pub case_limit: usize,
}

impl SystemTestConfig {
    #[must_use]
    pub fn from_env() -> Self {
        let fast_mode = std::env::var("VERTEX_TEST_FAST").is_ok();
        let case_limit = std::env::var("VERTEX_TEST_CASE_LIMIT")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(usize::MAX);

        Self { fast_mode, case_limit }
    }
}
