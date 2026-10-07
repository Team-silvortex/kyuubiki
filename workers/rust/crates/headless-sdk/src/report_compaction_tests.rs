use super::*;
use crate::run_batch_dry;
use crate::{
    HeadlessExecutionBatch, HeadlessExecutionBatchStep, HeadlessExecutor, HeadlessExecutorError,
    HeadlessExecutorOutcome, HeadlessRisk, execute_batch_with_executor,
};
use serde_json::json;

fn previous_compaction(value: &Value) -> Value {
    match value {
        Value::Array(items) if items.len() > 128 => json!({
            "$kyuubiki_report_summary":"array","item_count":items.len(),
            "sample":items.iter().take(3).map(previous_compaction).collect::<Vec<_>>(),
            "omitted_item_count":items.len()-3,
        }),
        Value::Array(items) => Value::Array(items.iter().map(previous_compaction).collect()),
        Value::Object(fields) => Value::Object(
            fields
                .iter()
                .map(|(key, value)| (key.clone(), previous_compaction(value)))
                .collect(),
        ),
        Value::String(text) if text.len() > 4096 => json!({
            "$kyuubiki_report_summary":"string","byte_count":text.len(),
            "prefix":text.chars().take(256).collect::<String>(),
        }),
        value => value.clone(),
    }
}

#[test]
fn owned_and_borrowed_compaction_match_previous_shape_at_all_threshold_edges() {
    let mut values = vec![
        json!(null),
        json!(false),
        json!(u64::MAX),
        json!(-0.0),
        json!({"literal":"{{steps.999.result.secret}}","empty":{},"array":[]}),
    ];
    for count in [0, 1, 3, 127, 128, 129, 4096] {
        values.push(json!((0..count).collect::<Vec<_>>()));
    }
    for count in [0, 1, 256, 4095, 4096, 4097, 32768] {
        values.push(json!("x".repeat(count)));
    }
    for tail in ["", "a", "aa"] {
        values.push(json!(format!("{}{tail}", "\u{4e2d}".repeat(1365))));
    }
    let nested = json!({"outer":values.clone(),"nested":(0..129).map(|_| json!({
        "text":"x".repeat(4097),"array":(0..129).collect::<Vec<_>>(),"tail":null
    })).collect::<Vec<_>>()});
    values.push(nested);
    for value in values {
        let expected = previous_compaction(&value);
        assert_eq!(compact_report_value(&value), expected);
        assert_eq!(compact_owned_report_value(value.clone()), expected);
        assert_eq!(compact_report_payload(Cow::Borrowed(&value)), expected);
        assert_eq!(compact_report_payload(Cow::Owned(value)), expected);
    }
}

#[test]
fn owned_small_arrays_strings_and_object_keys_keep_their_original_allocations() {
    let value = json!({"nested-key":{"array":["x".repeat(1024),"literal"]}});
    let array = value["nested-key"]["array"].as_array().unwrap().as_ptr();
    let text = value["nested-key"]["array"][0].as_str().unwrap().as_ptr();
    let key = value.as_object().unwrap().keys().next().unwrap().as_ptr();
    let result = compact_owned_report_value(value);
    assert_eq!(
        result["nested-key"]["array"].as_array().unwrap().as_ptr(),
        array
    );
    assert_eq!(
        result["nested-key"]["array"][0].as_str().unwrap().as_ptr(),
        text
    );
    assert_eq!(
        result.as_object().unwrap().keys().next().unwrap().as_ptr(),
        key
    );
    assert_eq!(
        result["nested-key"]["array"][0].as_str().unwrap().len(),
        1024
    );
}

#[test]
fn large_array_samples_move_values_but_do_not_retain_the_source_vector_capacity() {
    let items = (0..4096)
        .map(|_| json!({"label":"x".repeat(1024)}))
        .collect::<Vec<_>>();
    let pointer = items[0]["label"].as_str().unwrap().as_ptr();
    let original = items.as_ptr();
    let preview = compact_owned_report_value(Value::Array(items));
    let sample = preview["sample"].as_array().unwrap();
    assert_eq!(preview["item_count"], 4096);
    assert_eq!(preview["omitted_item_count"], 4093);
    assert_eq!(sample.len(), 3);
    assert_eq!(sample.capacity(), 3);
    assert_ne!(sample.as_ptr(), original);
    assert_eq!(sample[0]["label"].as_str().unwrap().as_ptr(), pointer);
}

#[test]
fn small_overreserved_arrays_release_excess_capacity_without_losing_values() {
    let mut items = Vec::with_capacity(4096);
    items.push(json!({"label":"keep"}));
    let expected = Value::Array(items.clone());
    let preview = compact_owned_report_value(Value::Array(items));
    assert_eq!(preview, expected);
    assert!(preview.as_array().unwrap().capacity() <= MAX_REPORT_ARRAY_ITEMS);
}

#[test]
fn short_overreserved_strings_release_excess_capacity_without_truncation() {
    let mut text = String::with_capacity(65536);
    text.push_str("\u{4e2d}\u{6587}");
    let preview = compact_owned_report_value(Value::String(text));
    let Value::String(text) = preview else {
        panic!("small text must remain a string")
    };
    assert_eq!(text, "\u{4e2d}\u{6587}");
    assert!(text.capacity() <= MAX_REPORT_STRING_BYTES);
}

#[test]
fn unicode_summary_keeps_byte_count_and_character_prefix_without_splitting_utf8() {
    let text = "\u{4e2d}\u{6587}\u{0628}\u{062d}\u{062b}\u{1f600}\n".repeat(8192);
    let expected = text.chars().take(256).collect::<String>();
    let bytes = text.len();
    let preview = compact_owned_report_value(Value::String(text));
    assert_eq!(preview["byte_count"], bytes);
    assert_eq!(preview["prefix"], expected);
    assert_eq!(preview["prefix"].as_str().unwrap().chars().count(), 256);
    assert!(preview["prefix"].as_str().unwrap().len() <= 1024);
}

#[test]
fn borrowed_payload_report_is_independent_and_never_mutates_the_original() {
    let value = json!({"data":["x".repeat(1024)],"nodes":(0..4096).collect::<Vec<_>>()});
    let original = value.clone();
    let mut preview = compact_report_payload(Cow::Borrowed(&value));
    assert_ne!(
        preview["data"].as_array().unwrap().as_ptr(),
        value["data"].as_array().unwrap().as_ptr()
    );
    assert_ne!(
        preview["data"][0].as_str().unwrap().as_ptr(),
        value["data"][0].as_str().unwrap().as_ptr()
    );
    preview["data"][0] = json!("changed");
    preview["nodes"]["sample"][0] = json!(999);
    assert_eq!(value, original);
}

#[test]
fn owned_payload_report_takes_original_leaf_allocations_instead_of_cloning() {
    let value = json!({"data":["x".repeat(1024)],"literal":"{{steps.1.result.status}}"});
    let pointer = value["data"][0].as_str().unwrap().as_ptr();
    let preview = compact_report_payload(Cow::Owned(value));
    assert_eq!(preview["data"][0].as_str().unwrap().as_ptr(), pointer);
    assert_eq!(preview["literal"], "{{steps.1.result.status}}");
}

#[test]
fn report_compaction_stays_idempotent_for_nested_summaries_and_literal_markers() {
    let source = json!({"nodes":(0..4096).collect::<Vec<_>>(),"text":"x".repeat(4097),
        "literal":{"$kyuubiki_report_summary":"not-an-instruction","sample":[1,2,3]}});
    let preview = compact_owned_report_value(source);
    assert_eq!(compact_report_value(&preview), preview);
    assert_eq!(compact_owned_report_value(preview.clone()), preview);
}

struct MoveExecutor {
    result: Option<Value>,
    status: &'static str,
    calls: usize,
    bound_pointer: Option<usize>,
}

impl HeadlessExecutor for MoveExecutor {
    fn name(&self) -> &'static str {
        "service"
    }

    fn execute_step(
        &mut self,
        _action: &str,
        index: usize,
        payload: &Value,
    ) -> Result<HeadlessExecutorOutcome, HeadlessExecutorError> {
        self.calls += 1;
        if index == 2 {
            self.bound_pointer = Some(payload["name"].as_str().unwrap().as_ptr() as usize);
        }
        Ok(HeadlessExecutorOutcome {
            status: self.status.into(),
            result: self
                .result
                .take()
                .unwrap_or_else(|| json!({"project_id":"next"})),
        })
    }
}

fn document(next_payload: Value) -> HeadlessExecutionBatch {
    HeadlessExecutionBatch {
        schema_version: "kyuubiki.headless-execution-batch/v1".into(),
        exported_at: "2026-10-07T00:00:00Z".into(),
        language: "en".into(),
        workflow_id: "report-ownership".into(),
        template_id: None,
        warnings: vec![],
        steps: vec![
            HeadlessExecutionBatchStep {
                index: 1,
                action: "service_health".into(),
                risk: HeadlessRisk::Normal,
                payload: json!({}),
            },
            HeadlessExecutionBatchStep {
                index: 2,
                action: "project_create".into(),
                risk: HeadlessRisk::Normal,
                payload: next_payload,
            },
        ],
    }
}

#[test]
fn public_batch_moves_unused_report_fields_and_final_bound_payload_but_not_shared_values() {
    let source = json!({"status":"x".repeat(1024),"diagnostics":{"data":["keep"]}});
    let expected = previous_compaction(&source);
    let status = source["status"].as_str().unwrap().as_ptr() as usize;
    let array = source["diagnostics"]["data"].as_array().unwrap().as_ptr();
    let mut executor = MoveExecutor {
        result: Some(source),
        status: "executed",
        calls: 0,
        bound_pointer: None,
    };
    let report = execute_batch_with_executor(
        &document(json!({"name":"{{steps.1.result.status}}"})),
        &mut executor,
        false,
        false,
    );
    assert_eq!(report.status, "ok");
    assert_eq!(report.executed_step_count, 2);
    assert_eq!(report.steps[0].result_preview, expected);
    assert_ne!(
        report.steps[0].result_preview["status"]
            .as_str()
            .unwrap()
            .as_ptr() as usize,
        status
    );
    assert_eq!(
        report.steps[0].result_preview["diagnostics"]["data"]
            .as_array()
            .unwrap()
            .as_ptr(),
        array
    );
    assert_eq!(executor.bound_pointer, Some(status));
    assert_eq!(
        report.steps[1].payload["name"].as_str().unwrap().as_ptr() as usize,
        status
    );
}

#[test]
fn public_batch_noncompletion_moves_diagnostics_without_enabling_bindings_or_later_calls() {
    for status in ["blocked", "failed", "cancelled", "pending"] {
        let source = json!({"status":"x".repeat(1024),"diagnostics":{"data":["keep"]}});
        let pointer = source["diagnostics"]["data"].as_array().unwrap().as_ptr();
        let expected = previous_compaction(&source);
        let mut executor = MoveExecutor {
            result: Some(source),
            status,
            calls: 0,
            bound_pointer: None,
        };
        let report = execute_batch_with_executor(
            &document(json!({"name":"{{steps.1.result.status}}"})),
            &mut executor,
            false,
            false,
        );
        assert_eq!(
            report.status,
            if status == "blocked" {
                "blocked"
            } else {
                "failed"
            }
        );
        assert_eq!(report.executed_step_count, 0);
        assert_eq!(report.steps.len(), 1);
        assert_eq!(executor.calls, 1);
        let preview = if status == "pending" {
            &report.steps[0].result_preview["executor_receipt"]
        } else {
            &report.steps[0].result_preview
        };
        assert_eq!(*preview, expected);
        assert_eq!(
            preview["diagnostics"]["data"].as_array().unwrap().as_ptr(),
            pointer
        );
        if status == "pending" {
            assert_eq!(
                report.execution_summary.failure.unwrap().category,
                "runtime_failure"
            );
        }
    }
}

#[test]
fn dry_run_bound_payload_reports_keep_previous_values_without_mutating_the_batch() {
    let note = "\u{4e2d}\u{6587}\n\u{0628}\u{062d}\u{062b}".repeat(8192);
    let document = document(json!({"name":"{{steps.1.result.status}}",
        "metadata":{"note":note,"nodes":(0..4096).collect::<Vec<_>>()}}));
    let original = serde_json::to_value(&document).unwrap();
    let mut resolved = document.steps[1].payload.clone();
    resolved["name"] = json!("ok");
    let report = run_batch_dry(&document, false, false);
    assert_eq!(report.status, "ok");
    assert_eq!(report.mode, "dry_run");
    assert_eq!(report.executed_step_count, 2);
    assert_eq!(report.steps[1].payload, previous_compaction(&resolved));
    assert_eq!(serde_json::to_value(&document).unwrap(), original);
}
