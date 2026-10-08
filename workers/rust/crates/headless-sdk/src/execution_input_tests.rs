use super::*;
use crate::{
    HeadlessRisk, MockHeadlessExecutor, build_template_document, execute_batch_with_executor,
    headless_batch_content_sha256, list_templates, normalize_workflow_document, run_batch_dry,
};
use serde_json::json;

fn batch(payload: Value) -> HeadlessExecutionBatch {
    HeadlessExecutionBatch {
        schema_version: "kyuubiki.headless-execution-batch/v1".into(),
        exported_at: "2026-10-08T00:00:00Z".into(),
        language: "en".into(),
        workflow_id: "input-identity".into(),
        template_id: None,
        steps: vec![HeadlessExecutionBatchStep {
            index: 1,
            action: "service_health".into(),
            risk: HeadlessRisk::Normal,
            payload,
        }],
        warnings: vec![],
    }
}

// Independent, allocating reference used only for small test fixtures.
fn reference_json(value: &Value) -> String {
    match value {
        Value::Array(values) => format!(
            "[{}]",
            values
                .iter()
                .map(reference_json)
                .collect::<Vec<_>>()
                .join(",")
        ),
        Value::Object(values) => {
            let sorted = values.iter().collect::<std::collections::BTreeMap<_, _>>();
            format!(
                "{{{}}}",
                sorted
                    .iter()
                    .map(|(key, value)| format!(
                        "{}:{}",
                        serde_json::to_string(key).unwrap(),
                        reference_json(value)
                    ))
                    .collect::<Vec<_>>()
                    .join(",")
            )
        }
        _ => serde_json::to_string(value).unwrap(),
    }
}

fn reference_fingerprint(batch: &HeadlessExecutionBatch) -> String {
    let mut value = serde_json::to_value(batch).unwrap();
    value["warnings"] = json!([]);
    let mut digest = Sha256::new();
    digest.update(HEADLESS_EXECUTION_INPUT_SCHEMA_VERSION.as_bytes());
    digest.update([0]);
    digest.update(reference_json(&value));
    format!("{:x}", digest.finalize())
}

#[test]
fn streaming_fingerprint_matches_all_normalized_template_inputs() {
    for template in list_templates() {
        let document = build_template_document(template.id, None).unwrap();
        let batch = normalize_workflow_document(&document).unwrap();
        assert_eq!(
            headless_execution_input_fingerprint(&batch).unwrap().sha256,
            reference_fingerprint(&batch),
            "{}",
            template.id
        );
    }
}

#[test]
fn fingerprint_preserves_nested_json_strings_and_numeric_tokens() {
    let source = batch(json!({
        "quoted":"\"\\\n\t", "unicode":"\u{6750}\u{6599} / \u{3b4}\u{3bf}\u{3ba}\u{3b9}\u{3bc}\u{3ae}", "nested":[
            null, true, false, {"z":1e-30,"a":-0.0}, u64::MAX, i64::MIN,
            f64::MAX, f64::MIN_POSITIVE, f64::from_bits(1), 1.0, 1
        ]
    }));
    let fingerprint = headless_execution_input_fingerprint(&source).unwrap();
    assert_eq!(fingerprint.sha256, reference_fingerprint(&source));
    let reloaded: HeadlessExecutionBatch =
        serde_json::from_str(&serde_json::to_string(&source).unwrap()).unwrap();
    assert_eq!(
        fingerprint,
        headless_execution_input_fingerprint(&reloaded).unwrap()
    );
}

#[test]
fn every_non_warning_batch_field_is_bound() {
    let source = batch(json!({"input":10.0}));
    let before = headless_execution_input_fingerprint(&source).unwrap();
    for (pointer, value) in [
        ("/schema_version", json!("different-schema")),
        ("/exported_at", json!("2026-10-09T00:00:00Z")),
        ("/language", json!("zh-CN")),
        ("/workflow_id", json!("different-workflow")),
        ("/steps/0/index", json!(2)),
        ("/steps/0/action", json!("project_list")),
        ("/steps/0/risk", json!("sensitive")),
        ("/steps/0/payload/input", json!(12.0)),
    ] {
        let mut value_batch = serde_json::to_value(&source).unwrap();
        *value_batch.pointer_mut(pointer).unwrap() = value;
        let changed = serde_json::from_value(value_batch).unwrap();
        assert_ne!(
            before,
            headless_execution_input_fingerprint(&changed).unwrap(),
            "{pointer}"
        );
    }
    let mut changed = source.clone();
    changed.template_id = Some("different-template".into());
    assert_ne!(
        before,
        headless_execution_input_fingerprint(&changed).unwrap()
    );
    assert_eq!(
        headless_execution_input_fingerprint(&changed)
            .unwrap()
            .sha256,
        reference_fingerprint(&changed)
    );
    changed = source;
    changed.warnings.push("diagnostic only".into());
    assert_eq!(
        before,
        headless_execution_input_fingerprint(&changed).unwrap()
    );
}

#[test]
fn nearby_floats_and_tiny_values_do_not_collide_like_legacy_lineage_hashes() {
    for (a, b) in [
        (json!(1e-30), json!(2e-30)),
        (json!(1.0), json!(f64::from_bits(1.0_f64.to_bits() + 1))),
        (json!(0.0), json!(-0.0)),
        (json!(1), json!(1.0)),
        (json!(u64::MAX - 1), json!(u64::MAX)),
    ] {
        assert_ne!(
            headless_execution_input_fingerprint(&batch(a)).unwrap(),
            headless_execution_input_fingerprint(&batch(b)).unwrap(),
        );
    }
    assert_eq!(
        headless_batch_content_sha256(&batch(json!(1e-30))).unwrap(),
        headless_batch_content_sha256(&batch(json!(2e-30))).unwrap(),
        "the legacy digest stays unchanged; it is not the exact identity gate"
    );
}

#[test]
fn object_order_is_irrelevant_but_array_and_step_order_are_bound() {
    let a = batch(serde_json::from_str(r#"{"z":[1,2],"a":{"b":1,"a":2}}"#).unwrap());
    let mut b = batch(serde_json::from_str(r#"{"a":{"a":2,"b":1},"z":[1,2]}"#).unwrap());
    let before = headless_execution_input_fingerprint(&a).unwrap();
    assert_eq!(before, headless_execution_input_fingerprint(&b).unwrap());
    b.steps[0].payload["z"] = json!([2, 1]);
    assert_ne!(before, headless_execution_input_fingerprint(&b).unwrap());
    b.steps.push(b.steps[0].clone());
    b.steps[1].index = 2;
    let before = headless_execution_input_fingerprint(&b).unwrap();
    b.steps.reverse();
    assert_ne!(before, headless_execution_input_fingerprint(&b).unwrap());
}

#[test]
fn both_report_producers_capture_inputs_before_compaction() {
    let a = batch(json!({"nodes":vec![0; 1000], "name":"a".repeat(5000)}));
    let mut b = a.clone();
    b.steps[0].payload["nodes"][999] = json!(1);
    let mut executor = MockHeadlessExecutor;
    let dry_a = run_batch_dry(&a, false, false);
    let dry_b = run_batch_dry(&b, false, false);
    let executed_a = execute_batch_with_executor(&a, &mut executor, false, false);
    let executed_b = execute_batch_with_executor(&b, &mut executor, false, false);
    for (first, second) in [(&dry_a, &dry_b), (&executed_a, &executed_b)] {
        assert_eq!(first.status, "ok");
        assert_eq!(first.steps[0].payload, second.steps[0].payload);
        assert_ne!(first.execution_input, second.execution_input);
        assert_eq!(
            first.execution_input.as_ref().unwrap(),
            &headless_execution_input_fingerprint(&a).unwrap()
        );
    }
    assert_eq!(dry_a.execution_input, executed_a.execution_input);
    assert_eq!(a.steps[0].payload["nodes"].as_array().unwrap().len(), 1000);
}

#[test]
fn invalid_batches_do_not_claim_a_captured_execution_input() {
    let mut invalid = batch(json!({}));
    invalid.steps[0].action = "nonexistent_action".into();
    let dry = run_batch_dry(&invalid, false, false);
    let executed = execute_batch_with_executor(&invalid, &mut MockHeadlessExecutor, false, false);
    assert_eq!(dry.status, "invalid");
    assert_eq!(executed.status, "invalid");
    assert!(dry.execution_input.is_none());
    assert!(executed.execution_input.is_none());
    assert!(executed.steps.is_empty());
}

#[test]
fn fingerprint_schema_is_explicit_and_closed() {
    let schema: Value = serde_json::from_str(include_str!(
        "../../../../../schemas/headless-execution-input.schema.json"
    ))
    .unwrap();
    assert_eq!(
        schema["properties"]["schema_version"]["const"],
        HEADLESS_EXECUTION_INPUT_SCHEMA_VERSION
    );
    assert_eq!(schema["additionalProperties"], false);
    let mut fingerprint =
        serde_json::to_value(headless_execution_input_fingerprint(&batch(json!({}))).unwrap())
            .unwrap();
    fingerprint["unknown"] = json!(true);
    assert!(serde_json::from_value::<HeadlessExecutionInputFingerprint>(fingerprint).is_err());
}
