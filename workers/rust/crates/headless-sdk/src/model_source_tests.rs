use super::*;
use serde_json::json;
use sha2::{Digest, Sha256};

fn record() -> Value {
    json!({"project_id":"p", "model_id":"m", "version_id":"v",
        "latest_version_id":"v", "kind":"axial_bar_1d", "material":"steel",
        "model_schema_version":"kyuubiki.model/v1",
        "payload":{"tip_force":1000.0,"tiny":1e-30,"nodes":[0,1,2]}})
}

#[test]
fn saved_source_streaming_hash_matches_the_versioned_lossless_projection() {
    let record = record();
    let source =
        headless_saved_model_source(&record, HeadlessModelSourceKind::ModelVersion).unwrap();
    let projection = json!({"schema_version":HEADLESS_MODEL_SOURCE_SCHEMA_VERSION,
        "source_kind":"model_version", "project_id":"p", "model_id":"m",
        "model_version_id":"v", "kind":"axial_bar_1d", "payload":record["payload"],
        "material":record["material"], "model_schema_version":record["model_schema_version"]});
    let mut digest = Sha256::new();
    digest.update(HEADLESS_MODEL_SOURCE_SCHEMA_VERSION.as_bytes());
    digest.update([0]);
    digest.update(serde_json::to_vec(&projection).unwrap());
    assert_eq!(source.sha256, format!("{:x}", digest.finalize()));
    source.validate().unwrap();
    let encoded = serde_json::to_value(&source).unwrap();
    assert!(encoded.get("payload").is_none());
    assert_eq!(HeadlessModelSource::decode(&encoded).unwrap(), source);
}

#[test]
fn saved_source_binds_content_identity_kind_and_latest_snapshot_but_not_display_metadata() {
    for kind in [
        HeadlessModelSourceKind::Model,
        HeadlessModelSourceKind::ModelVersion,
    ] {
        let original = record();
        let expected = headless_saved_model_source(&original, kind).unwrap();
        for (pointer, value) in [
            ("/project_id", json!("other")),
            ("/model_id", json!("other")),
            ("/kind", json!("heat_bar_1d")),
            ("/material", json!("aluminum")),
            ("/model_schema_version", json!("kyuubiki.model/v2")),
            ("/payload/tip_force", json!(2000.0)),
            ("/payload/tiny", json!(2e-30)),
            ("/payload/nodes", json!([2, 1, 0])),
            (
                if kind == HeadlessModelSourceKind::Model {
                    "/latest_version_id"
                } else {
                    "/version_id"
                },
                json!("other"),
            ),
        ] {
            let mut changed = original.clone();
            *changed.pointer_mut(pointer).unwrap() = value;
            assert_ne!(
                headless_saved_model_source(&changed, kind).unwrap(),
                expected,
                "{pointer}"
            );
        }
        let mut metadata = original.clone();
        metadata["name"] = json!("renamed model");
        metadata["updated_at"] = json!("2026-10-09T00:00:00Z");
        assert_eq!(
            headless_saved_model_source(&metadata, kind).unwrap(),
            expected
        );
        assert_ne!(
            headless_saved_model_source(&original, HeadlessModelSourceKind::Model).unwrap(),
            headless_saved_model_source(&original, HeadlessModelSourceKind::ModelVersion).unwrap()
        );
    }
}

#[test]
fn saved_source_rejects_invalid_records_and_closed_descriptor_fields() {
    let original = record();
    for key in ["project_id", "model_id", "version_id", "kind", "payload"] {
        for value in [Value::Null, json!(true), json!("../escape")] {
            let mut invalid = original.clone();
            invalid[key] = if key == "kind" && value.is_string() {
                json!(" kind ")
            } else {
                value
            };
            assert!(
                headless_saved_model_source(&invalid, HeadlessModelSourceKind::ModelVersion)
                    .is_err(),
                "{key}"
            );
        }
    }
    let source =
        headless_saved_model_source(&original, HeadlessModelSourceKind::ModelVersion).unwrap();
    let encoded = serde_json::to_value(source).unwrap();
    for (key, value) in [
        ("schema_version", json!("unknown")),
        ("sha256", json!("A".repeat(64))),
        ("sha256", json!("a".repeat(63))),
        ("model_version_id", Value::Null),
        ("source_kind", json!("inline")),
        ("project_id", json!(" p")),
        ("kind", json!("axial_bar_1d\n")),
        ("unknown", json!(true)),
    ] {
        let mut invalid = encoded.clone();
        invalid[key] = value;
        assert!(HeadlessModelSource::decode(&invalid).is_err(), "{key}");
    }
}

#[test]
fn mutable_model_sources_allow_missing_latest_version_without_inventing_a_version() {
    let mut record = record();
    record.as_object_mut().unwrap().remove("latest_version_id");
    let source = headless_saved_model_source(&record, HeadlessModelSourceKind::Model).unwrap();
    assert!(source.model_version_id.is_none());
    source.validate().unwrap();
    record["latest_version_id"] = Value::Null;
    assert_eq!(
        headless_saved_model_source(&record, HeadlessModelSourceKind::Model).unwrap(),
        source
    );
    record["latest_version_id"] = json!(42);
    assert!(headless_saved_model_source(&record, HeadlessModelSourceKind::Model).is_err());
}
