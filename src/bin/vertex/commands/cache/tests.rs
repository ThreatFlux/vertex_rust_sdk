use super::{context::CacheContext, formatting, input, ops, render};
use crate::commands::command_tests::{reset_env, set_common_env, ENV_LOCK};
use mockito::Matcher;
use serde_json::json;
use std::io::Write;
use tempfile::NamedTempFile;

#[tokio::test]
async fn build_cached_content_handles_text_and_file() {
    // Text path
    let text_build =
        input::build_cached_content(Some("hello"), None, Some("demo"), 60, Some("sys")).unwrap();
    assert_eq!(text_build.ttl_seconds, 60);
    assert_eq!(text_build.system_preview.as_deref(), Some("sys"));
    assert_eq!(text_build.cached_content.display_name.as_deref(), Some("demo"));

    // File path
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "from file").unwrap();
    let file_path = file.path().to_str().unwrap().to_string();
    let file_build = input::build_cached_content(None, Some(&file_path), None, 120, None).unwrap();
    assert!(matches!(
        file_build.source,
        input::ContentSource::File(path) if path == file_path
    ));
}

#[test]
fn formatting_helpers_truncate_and_format() {
    assert_eq!(formatting::preview_text("short", 10), "short");
    assert_eq!(formatting::preview_text("truncate-me", 4), "trun...");

    let (seconds, hours) = formatting::format_remaining_ttl(7200);
    assert_eq!(seconds, "7200");
    assert_eq!(hours, "2.00");
}

#[tokio::test]
#[allow(clippy::too_many_lines, clippy::significant_drop_tightening)]
async fn cache_ops_flow_uses_mock_server() {
    let _guard = ENV_LOCK.lock().await;
    let mut server = mockito::Server::new_async().await;
    reset_env();
    set_common_env(&server.url());

    let cache_base = "/v1/projects/test-project/locations/us-central1/cachedContents";

    let list_mock = server
        .mock("GET", cache_base)
        .match_query(Matcher::Any)
        .match_header("authorization", "Bearer test-token")
        .expect(1)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            json!({
                "cachedContents": [{
                    "name": format!("{cache_base}/demo-cache"),
                    "displayName": "Demo",
                    "ttl": "120s",
                    "expireTime": "2099-01-01T00:00:00Z",
                    "contents": [{
                        "role": "user",
                        "parts": [{"text": "cached"}]
                    }]
                }]
            })
            .to_string(),
        )
        .create();

    let create_mock = server
        .mock("POST", cache_base)
        .match_header("authorization", "Bearer test-token")
        .expect(1)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            json!({
                "name": format!("{cache_base}/demo-cache"),
                "displayName": "Demo",
                "ttl": "120s",
                "expireTime": "2099-01-01T00:00:00Z",
                "createTime": "2099-01-01T00:00:00Z",
                "contents": [{
                    "role": "user",
                    "parts": [{"text": "hello"}]
                }]
            })
            .to_string(),
        )
        .create();

    let cache_path = format!("{cache_base}/demo-cache");
    let get_mock = server
        .mock("GET", cache_path.as_str())
        .match_header("authorization", "Bearer test-token")
        .expect(1)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            json!({
                "name": cache_path,
                "displayName": "Demo",
                "ttl": "120s",
                "expireTime": "2099-01-02T00:00:00Z",
                "updateTime": "2099-01-02T00:00:00Z",
                "contents": [{
                    "role": "user",
                    "parts": [{"text": "hello"}]
                }],
                "usageMetadata": {
                    "totalTokenCount": 4
                }
            })
            .to_string(),
        )
        .create();

    let update_mock = server
        .mock("PATCH", cache_path.as_str())
        .match_query(Matcher::Any)
        .match_header("authorization", "Bearer test-token")
        .expect(1)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            json!({
                "name": cache_path,
                "displayName": "Demo",
                "ttl": "300s",
                "expireTime": "2099-01-03T00:00:00Z",
                "contents": [{
                    "role": "user",
                    "parts": [{"text": "hello"}]
                }]
            })
            .to_string(),
        )
        .create();

    let delete_mock = server
        .mock("DELETE", cache_path.as_str())
        .match_header("authorization", "Bearer test-token")
        .expect(1)
        .with_status(200)
        .with_header("content-type", "application/json")
        .create();

    let context = CacheContext::new().await.unwrap();

    let build = input::build_cached_content(Some("hello"), None, Some("Demo"), 120, None).unwrap();
    render::print_create_intro(&build);
    let created = ops::create(&context, build.cached_content.clone()).await.unwrap();
    render::print_create_success(&created);

    render::print_list_intro();
    let list = ops::list(&context, Some(5)).await.unwrap();
    render::print_list(&list);

    render::print_get_intro("demo-cache");
    let fetched = ops::get(&context, "demo-cache").await.unwrap();
    render::print_cache_details(&fetched);

    render::print_update_intro("demo-cache", 300);
    let updated = ops::update_ttl(&context, "demo-cache", 300).await.unwrap();
    render::print_update_success(&updated);

    render::print_delete_intro("demo-cache");
    ops::delete(&context, "demo-cache").await.unwrap();
    render::print_delete_success();

    create_mock.assert();
    list_mock.assert();
    get_mock.assert();
    update_mock.assert();
    delete_mock.assert();
}
