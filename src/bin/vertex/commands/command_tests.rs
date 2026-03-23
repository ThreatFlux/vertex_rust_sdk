use super::{cache, config, generation, models, system};
use mockito::Matcher;
use serde_json::json;
use std::env;
use std::sync::LazyLock;
use tokio::sync::Mutex;

pub(super) static ENV_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

pub(super) fn set_common_env(base_url: &str) {
    env::set_var("VERTEX_PROJECT_ID", "test-project");
    env::set_var("VERTEX_REGION", "us-central1");
    env::set_var("GOOGLE_ACCESS_TOKEN", "test-token");
    env::set_var("VERTEX_BASE_URL", base_url);
}

#[tokio::test]
#[allow(clippy::significant_drop_tightening)]
async fn generation_and_cache_commands_use_mock_server() {
    let _guard = ENV_LOCK.lock().await;
    let mut server = mockito::Server::new_async().await;
    reset_env();
    set_common_env(&server.url());

    let generate_path = "/v1/projects/test-project/locations/us-central1/publishers/google/models/gemini-1.5-flash:generateContent";
    let generate_mock = server
        .mock("POST", generate_path)
        .match_header("authorization", "Bearer test-token")
        .expect(1)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            json!({
                "candidates": [{
                    "content": {
                        "role": "model",
                        "parts": [{"text": "hi"}]
                    },
                    "finishReason": "STOP",
                    "safetyRatings": []
                }],
                "usageMetadata": {
                    "promptTokenCount": 1,
                    "totalTokenCount": 2
                }
            })
            .to_string(),
        )
        .create();

    let cache_path = "/v1/projects/test-project/locations/us-central1/cachedContents";
    let cache_list_mock = server
        .mock("GET", cache_path)
        .match_query(Matcher::Any)
        .expect(1)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            json!({
                "cachedContents": [{
                    "name": format!("{cache_path}/demo-cache"),
                    "displayName": "Demo",
                    "ttl": "60s",
                    "contents": [{
                        "role": "user",
                        "parts": [{"text": "cached"}]
                    }]
                }],
                "nextPageToken": null
            })
            .to_string(),
        )
        .create();

    let cache_create_mock = server
        .mock("POST", cache_path)
        .expect(1)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            json!({
                "name": format!("{cache_path}/created-cache"),
                "displayName": "created-cache",
                "ttl": "120s",
                "contents": [{
                    "role": "user",
                    "parts": [{"text": "hello"}]
                }]
            })
            .to_string(),
        )
        .create();

    generation::generate("hello", "gemini-1.5-flash", 0.0, 128, None).await.unwrap();
    cache::cache_list(Some(5)).await.unwrap();
    cache::cache_create(Some("hello"), None, Some("demo-cache"), 120, None).await.unwrap();

    generate_mock.assert();
    cache_list_mock.assert();
    cache_create_mock.assert();
}

#[tokio::test]
#[allow(clippy::significant_drop_tightening)]
async fn models_and_system_commands_use_mock_server() {
    let _guard = ENV_LOCK.lock().await;
    let mut server = mockito::Server::new_async().await;
    reset_env();
    set_common_env(&server.url());
    env::set_var("VERTEX_TEST_FAST", "1");
    env::set_var("VERTEX_TEST_CASE_LIMIT", "1");

    let models_path = "/v1beta1/publishers/google/models";
    let models_mock = server
        .mock("GET", models_path)
        .expect(1)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            json!({
                "models": [{
                    "name": "publishers/google/models/gemini-1.5-flash",
                    "displayName": "Gemini",
                    "supportedGenerationMethods": ["generateContent"]
                }]
            })
            .to_string(),
        )
        .create();

    let locations_path = "/v1/projects/test-project/locations";
    let locations_mock = server
        .mock("GET", locations_path)
        .expect(1)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            json!({
                "locations": [{
                    "name": "projects/test-project/locations/us-central1",
                    "locationId": "us-central1",
                    "displayName": "US Central"
                }]
            })
            .to_string(),
        )
        .create();

    let generate_path = "/v1/projects/test-project/locations/us-central1/publishers/google/models/gemini-1.5-flash:generateContent";
    let system_mock = server
        .mock("POST", generate_path)
        .expect(1)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            json!({
                "candidates": [{
                    "content": {
                        "role": "model",
                        "parts": [{"text": "system reply"}]
                    },
                    "finishReason": "STOP",
                    "safetyRatings": []
                }]
            })
            .to_string(),
        )
        .create();

    models::list_models(true, None).await.unwrap();
    models::list_locations(None).await.unwrap();
    system::system_test_with_config(
        "gemini-1.5-flash",
        system::SystemTestConfig { fast_mode: true, case_limit: 1 },
    )
    .await
    .unwrap();

    models_mock.assert();
    locations_mock.assert();
    system_mock.assert();
}

#[tokio::test]
async fn config_commands_work_with_env_token() {
    let _guard = ENV_LOCK.lock().await;
    reset_env();
    env::set_var("VERTEX_PROJECT_ID", "demo");
    env::set_var("VERTEX_REGION", "us-central1");
    env::set_var("GOOGLE_ACCESS_TOKEN", "token");

    config::show_config().unwrap();
    config::check_config().await.unwrap();
}

pub(super) fn reset_env() {
    for key in [
        "VERTEX_PROJECT_ID",
        "VERTEX_REGION",
        "VERTEX_BASE_URL",
        "GOOGLE_ACCESS_TOKEN",
        "VERTEX_TEST_FAST",
        "VERTEX_TEST_CASE_LIMIT",
    ] {
        env::remove_var(key);
    }
}
