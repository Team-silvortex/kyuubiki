use super::*;
use crate::service_executor_ack_loss_tests::{observe, observe_sequence_with_limit, run};

fn response(value: &Value) -> Vec<u8> {
    let body = value.to_string();
    format!(
        "HTTP/1.1 201 Created\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    )
    .into_bytes()
}

fn receipt(artifact: &TemporaryModelArtifact) -> Value {
    json!({"artifact":{"schema_version":MODEL_ARTIFACT_SCHEMA,
        "artifact_id":artifact.sha256,"sha256":artifact.sha256,"size_bytes":artifact.size_bytes,
        "media_type":MODEL_ARTIFACT_MEDIA_TYPE,"immutable":true}})
}

#[test]
fn upload_receipts_require_exact_typed_digest_size_schema_and_media_fields() {
    let artifact = TemporaryModelArtifact::serialize(&json!({"tip_force":1000.0})).unwrap();
    let original = receipt(&artifact);
    let expected = original["artifact"].clone();
    assert_eq!(
        validate_upload_receipt(&original, &artifact.sha256, artifact.size_bytes as u64).unwrap(),
        expected
    );
    for field in expected.as_object().unwrap().keys() {
        let mut missing = original.clone();
        missing["artifact"].as_object_mut().unwrap().remove(field);
        assert!(
            validate_upload_receipt(&missing, &artifact.sha256, artifact.size_bytes as u64)
                .is_err(),
            "{field}"
        );
        for value in [
            Value::Null,
            json!(false),
            json!(0),
            json!("rejected-secret"),
        ] {
            let mut wrong = original.clone();
            wrong["artifact"][field] = value;
            let error =
                validate_upload_receipt(&wrong, &artifact.sha256, artifact.size_bytes as u64)
                    .unwrap_err();
            assert!(error.message.starts_with(INVALID_UPLOAD_RECEIPT));
            assert!(!error.message.contains("rejected-secret"));
        }
    }
    for (field, value) in [
        ("sha256", json!(artifact.sha256.to_ascii_uppercase())),
        ("artifact_id", json!("f".repeat(64))),
        ("size_bytes", json!(artifact.size_bytes as f64)),
        ("size_bytes", json!(artifact.size_bytes + 1)),
        ("media_type", json!("application/vnd.kyuubiki.result+json")),
        ("schema_version", json!("kyuubiki.model-artifact-ref/v2")),
    ] {
        let mut wrong = original.clone();
        wrong["artifact"][field] = value;
        assert!(
            validate_upload_receipt(&wrong, &artifact.sha256, artifact.size_bytes as u64).is_err(),
            "{field}"
        );
    }
    for value in [
        Value::Null,
        json!([]),
        json!({}),
        json!({"artifact":false}),
        expected.clone(),
    ] {
        assert!(
            validate_upload_receipt(&value, &artifact.sha256, artifact.size_bytes as u64).is_err()
        );
    }
    let mut extended = original;
    extended["artifact"]["untrusted_endpoint"] = json!("rejected-secret");
    assert_eq!(
        validate_upload_receipt(&extended, &artifact.sha256, artifact.size_bytes as u64).unwrap(),
        expected
    );
}

#[test]
fn temporary_artifact_hash_covers_exact_numbers_escapes_and_unsampled_content() {
    let model = json!({"tiny":1e-30,"nearby":f64::from_bits(1.0_f64.to_bits()+1),
        "escaped":"quote\" newline\n\\", "nodes":(0..4096).collect::<Vec<_>>()});
    let artifact = TemporaryModelArtifact::serialize(&model).unwrap();
    let expected = serde_json::to_vec(&model).unwrap();
    assert_eq!(artifact.size_bytes, expected.len());
    assert_eq!(artifact.sha256, format!("{:x}", Sha256::digest(&expected)));
    assert_eq!(fs::read(&artifact.path).unwrap(), expected);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(&artifact.path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
    let mut changed = model;
    changed["nodes"][4095] = json!(-1);
    assert_ne!(
        TemporaryModelArtifact::serialize(&changed).unwrap().sha256,
        artifact.sha256
    );
}

#[test]
fn modified_temporary_content_cannot_be_rebound_to_a_valid_upload_acknowledgement() {
    let model = json!({"force":1000.0});
    let artifact = TemporaryModelArtifact::serialize(&model).unwrap();
    let changed = serde_json::to_vec(&json!({"force":2000.0})).unwrap();
    assert_eq!(changed.len(), artifact.size_bytes);
    fs::write(&artifact.path, &changed).unwrap();
    let digest = format!("{:x}", Sha256::digest(&changed));
    let acknowledgement = json!({"artifact":{"schema_version":MODEL_ARTIFACT_SCHEMA,
        "artifact_id":digest,"sha256":digest,"size_bytes":changed.len(),
        "media_type":MODEL_ARTIFACT_MEDIA_TYPE,"immutable":true}});
    let (result, query) = observe(&response(&acknowledgement), |url| {
        upload_model_artifact(url, None, &artifact)
    });
    assert_eq!(query.split_once("\r\n\r\n").unwrap().1.as_bytes(), changed);
    assert!(
        result
            .unwrap_err()
            .message
            .starts_with(INVALID_UPLOAD_RECEIPT)
    );
}

#[test]
fn invalid_large_upload_receipts_halt_the_batch_before_solver_or_downstream_post() {
    let model = json!({"large-model-padding":"x".repeat(MAX_INLINE_JSON_BYTES)});
    let artifact = TemporaryModelArtifact::serialize(&model).unwrap();
    let mut acknowledgement = receipt(&artifact);
    acknowledgement["artifact"]["size_bytes"] = json!(artifact.size_bytes + 1);
    let ((report, calls), queries) = observe_sequence_with_limit(
        vec![response(&acknowledgement)],
        MAX_INLINE_JSON_BYTES + 64 * 1024,
        |url| run(url, "solve_bar_1d", json!({"model":model})),
    );
    assert_eq!(queries.len(), 1);
    assert!(queries[0].starts_with("POST /api/v1/model-artifacts HTTP/1.1"));
    assert_eq!(calls, ["solve_bar_1d"]);
    assert_eq!(report.status, "failed");
    assert_eq!(report.executed_step_count, 0);
    assert_eq!(report.steps.len(), 1);
    let failure = report.execution_summary.failure.unwrap();
    assert_eq!(failure.category, "contract_failure");
    assert_eq!(failure.stage, "artifact_upload");
    assert!(!failure.retryable);
    assert_eq!(failure.retry_strategy, "none");
}

#[test]
fn mismatched_upload_content_cannot_supply_a_solver_reference() {
    let model = json!({"tip_force":1000.0,"tiny":1e-30});
    let artifact = TemporaryModelArtifact::serialize(&model).unwrap();
    let receipt = json!({"artifact":{
        "schema_version":"kyuubiki.model-artifact-ref/v1",
        "artifact_id":"0".repeat(64),"sha256":"0".repeat(64),
        "size_bytes":artifact.size_bytes,"immutable":true,
        "media_type":MODEL_ARTIFACT_MEDIA_TYPE
    }})
    .to_string();
    let response = format!(
        "HTTP/1.1 201 Created\r\nContent-Length: {}\r\n\r\n{receipt}",
        receipt.len()
    );
    let (result, request) =
        crate::service_executor_ack_loss_tests::observe(response.as_bytes(), |url| {
            upload_model_artifact(url, None, &artifact)
        });
    assert!(request.starts_with("POST /api/v1/model-artifacts HTTP/1.1"));
    let error =
        result.expect_err("another content-addressed object must not become the solver input");
    assert!(error.message.starts_with("model_artifact_receipt_invalid:"));
}

#[test]
fn keeps_small_models_inline() {
    let model = json!({"nodes": [], "elements": []});
    let prepared = prepare_direct_fem_request_body("http://127.0.0.1:3000", None, &model)
        .expect("small model does not need a live service");
    assert_eq!(prepared.body, model);
    assert!(prepared.upload.is_none());
}

#[test]
fn large_artifacts_fail_fast_on_known_local_frontend_proxies() {
    for base_url in ["http://127.0.0.1:3000", "http://localhost:3000/"] {
        let error = reject_known_frontend_proxy(base_url, "size_bytes=42000000".to_string())
            .expect_err("known GUI proxy must not receive large artifacts");
        assert!(error.message.contains("frontend_proxy_artifact_limit"));
        assert!(error.message.contains("size_bytes=42000000"));
        assert!(error.message.contains("127.0.0.1:4000"));
    }
    assert!(
        reject_known_frontend_proxy("http://127.0.0.1:4000", "size_bytes=42000000".to_string())
            .is_ok()
    );
    assert!(
        reject_known_frontend_proxy(
            "http://runtime.example:3000",
            "size_bytes=42000000".to_string()
        )
        .is_ok()
    );
}

#[test]
fn counts_large_model_entities_without_serializing_the_model() {
    let model = json!({"nodes": [1, 2, 3], "elements": [4, 5]});
    assert_eq!(model_entity_count(&model), 5);
}

#[test]
fn streaming_size_matches_canonical_json_serialization() {
    let model = json!({"nodes": [{"x": 1.0}], "elements": [], "label": "test"});
    assert_eq!(
        serialized_json_size(&model).expect("measure model"),
        serde_json::to_vec(&model).expect("serialize model").len()
    );

    let artifact = TemporaryModelArtifact::serialize(&model).expect("temporary artifact");
    let path = artifact.path.clone();
    assert_eq!(artifact.size_bytes, serialized_json_size(&model).unwrap());
    assert!(path.is_file());
    drop(artifact);
    assert!(!path.exists());
}
