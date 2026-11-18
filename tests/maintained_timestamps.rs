//! Wire-format regressions for the actual public cache and Claude response models.

use serde_json::json;
use threatflux_vertex_rust_sdk::{claude::types::MessageResponse, CachedContent, Timestamp};

#[test]
fn cache_response_preserves_offset_instants_nanoseconds_and_optional_fields() {
    let record: CachedContent = serde_json::from_value(json!({
        "contents": [], "expireTime": "2026-11-18T02:00:00.123456789+02:00",
        "createTime": "1969-12-31T23:59:59.5Z", "updateTime": null,
    }))
    .unwrap();
    let expected: Timestamp = "2026-11-18T00:00:00.123456789Z".parse().unwrap();
    assert_eq!(record.expire_time, Some(expected));
    let before_epoch: Timestamp = "1969-12-31T23:59:59.5Z".parse().unwrap();
    assert_eq!(record.create_time, Some(before_epoch));
    assert!(record.update_time.is_none());
    let wire = serde_json::to_value(record).unwrap();
    assert_eq!(wire["expireTime"], json!("2026-11-18T00:00:00.123456789Z"));
    assert_eq!(wire["createTime"], json!("1969-12-31T23:59:59.5Z"));
    assert!(!wire.as_object().unwrap().contains_key("updateTime"));
}

#[test]
fn invalid_cache_timestamps_fail_and_expired_ttl_stays_zero() {
    for invalid in ["2026-02-30T00:00:00Z", "2026-11-18T00:00:00", "not-a-date"] {
        assert!(serde_json::from_value::<CachedContent>(
            json!({"contents": [], "expireTime": invalid})
        )
        .is_err());
    }
    let mut cache = CachedContent::new(Vec::new());
    assert!(!cache.is_expired());
    assert_eq!(cache.remaining_ttl_seconds(), None);
    cache.expire_time = Some(Timestamp::from_second(0).unwrap());
    assert!(cache.is_expired());
    assert_eq!(cache.remaining_ttl_seconds(), Some(0));
}

#[test]
fn claude_response_uses_the_public_type_and_documents_leap_second_normalization() {
    let response: MessageResponse = serde_json::from_value(json!({
        "id": "fixture", "type": "message", "role": "assistant", "content": [],
        "created_at": "2016-12-31T23:59:60.5Z",
    }))
    .unwrap();
    let expected: Timestamp = "2016-12-31T23:59:59.5Z".parse().unwrap();
    assert_eq!(response.created_at, Some(expected));
    assert_eq!(
        serde_json::to_value(response).unwrap()["created_at"],
        json!("2016-12-31T23:59:59.5Z")
    );
}
