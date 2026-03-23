use thiserror::Error;
use threatflux_vertex_rust_sdk::{VertexClient, VertexError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExampleEnvironment {
    pub project_id: String,
    pub location: String,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum EnvError {
    #[error("GOOGLE_CLOUD_PROJECT is not set")]
    MissingProjectId,
    #[error("GOOGLE_CLOUD_LOCATION cannot be empty")]
    EmptyLocation,
}

impl ExampleEnvironment {
    pub fn from_env() -> Result<Self, EnvError> {
        let project_id =
            std::env::var("GOOGLE_CLOUD_PROJECT").map_err(|_| EnvError::MissingProjectId)?;

        let location = match std::env::var("GOOGLE_CLOUD_LOCATION") {
            Ok(value) if value.trim().is_empty() => return Err(EnvError::EmptyLocation),
            Ok(value) => value,
            Err(_) => "us-central1".to_string(),
        };

        Ok(Self { project_id, location })
    }

    pub async fn new_client(&self) -> Result<VertexClient, VertexError> {
        VertexClient::new_legacy(&self.project_id, &self.location).await
    }
}

pub fn init_logging() -> Result<(), log::SetLoggerError> {
    env_logger::Builder::from_default_env().filter_level(log::LevelFilter::Info).try_init()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::LazyLock;
    use std::sync::Mutex;

    static ENV_MUTEX: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

    struct EnvGuard {
        project: Option<String>,
        location: Option<String>,
    }

    impl EnvGuard {
        fn new(project: Option<&str>, location: Option<&str>) -> Self {
            let guard = Self {
                project: std::env::var("GOOGLE_CLOUD_PROJECT").ok(),
                location: std::env::var("GOOGLE_CLOUD_LOCATION").ok(),
            };

            if let Some(value) = project {
                std::env::set_var("GOOGLE_CLOUD_PROJECT", value);
            } else {
                std::env::remove_var("GOOGLE_CLOUD_PROJECT");
            }

            if let Some(value) = location {
                std::env::set_var("GOOGLE_CLOUD_LOCATION", value);
            } else {
                std::env::remove_var("GOOGLE_CLOUD_LOCATION");
            }

            guard
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            if let Some(value) = &self.project {
                std::env::set_var("GOOGLE_CLOUD_PROJECT", value);
            } else {
                std::env::remove_var("GOOGLE_CLOUD_PROJECT");
            }

            if let Some(value) = &self.location {
                std::env::set_var("GOOGLE_CLOUD_LOCATION", value);
            } else {
                std::env::remove_var("GOOGLE_CLOUD_LOCATION");
            }
        }
    }

    #[test]
    fn loads_env_with_defaults() {
        let _lock = ENV_MUTEX.lock().unwrap();
        let _guard = EnvGuard::new(Some("test-project"), None);

        let env = ExampleEnvironment::from_env().expect("should load env");

        assert_eq!(env.project_id, "test-project");
        assert_eq!(env.location, "us-central1");
    }

    #[test]
    fn errors_when_project_missing() {
        let _lock = ENV_MUTEX.lock().unwrap();
        let _guard = EnvGuard::new(None, None);

        let result = ExampleEnvironment::from_env();

        assert!(matches!(result, Err(EnvError::MissingProjectId)));
    }

    #[test]
    fn errors_when_location_empty() {
        let _lock = ENV_MUTEX.lock().unwrap();
        let _guard = EnvGuard::new(Some("project"), Some("   "));

        let result = ExampleEnvironment::from_env();

        assert!(matches!(result, Err(EnvError::EmptyLocation)));
    }
}
