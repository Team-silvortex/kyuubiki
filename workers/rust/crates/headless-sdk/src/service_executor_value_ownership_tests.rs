use crate::service_executor::{normalize_job_state_result, normalize_job_submission_result};
use crate::service_executor_job_receipt::JOB_STATUSES;
use crate::service_executor_result::{
    normalize_preferred_job_result, normalize_result_fetch_result,
};
use crate::service_executor_solve::combine_solve_and_wait_results;
use serde_json::{Map, Value, json};

fn large_result() -> Value {
    json!({"nodes":(0..4096).collect::<Vec<_>>(), "text":"x".repeat(1024 * 1024),
        "literal":"{{steps.999.result.secret}}"})
}

fn job() -> Value {
    json!({"job_id":"owned-job","status":"completed","progress":1.0,
        "project_id":"owned-project","model_version_id":"owned-version",
        "diagnostics":{"samples":(0..4096).collect::<Vec<_>>()}})
}

fn envelope() -> Value {
    Value::Object(Map::from_iter([
        ("job".into(), job()),
        ("result".into(), large_result()),
        ("unused".into(), json!({"text":"y".repeat(4096)})),
    ]))
}

fn array_pointer(value: &Value) -> *const Value {
    value.as_array().unwrap().as_ptr()
}

fn string_pointer(value: &Value) -> *const u8 {
    value.as_str().unwrap().as_ptr()
}

// These test-only references preserve the former public shapes and defaults.
fn previous_job_state(result: Value, include_result: bool) -> Value {
    let Some(job) = result.get("job").and_then(Value::as_object) else {
        return result;
    };
    let mut normalized = json!({
        "job_id":job.get("job_id").cloned().unwrap_or(Value::Null),
        "status":job.get("status").cloned().unwrap_or(Value::Null),
        "progress":job.get("progress").cloned().unwrap_or(Value::Null),
        "job":result.get("job").cloned().unwrap_or(Value::Null),
        "raw":result,
    });
    if include_result {
        normalized["result"] = result.get("result").cloned().unwrap_or(Value::Null);
    }
    normalized
}

fn previous_preferred_result(job_id: &str, mut envelope: Value) -> Value {
    let result = envelope.as_object_mut().unwrap().remove("result").unwrap();
    json!({"job_id":job_id, "status":envelope.pointer("/job/status"),
        "job":envelope.get("job"), "result":result})
}

fn previous_result_fetch(job_id: &str, result: Value) -> Value {
    let result = match result {
        Value::Object(mut envelope) => envelope.remove("result").unwrap_or(Value::Object(envelope)),
        value => value,
    };
    json!({"job_id":job_id,"result":result})
}

fn previous_combined(job_id: &str, solved: Value, waited: Value, fetched: Value) -> Value {
    json!({"job_id":job_id,
        "status":waited.get("status").cloned().unwrap_or_else(||json!("completed")),
        "model_version_id":solved.get("model_version_id").cloned().unwrap_or(Value::Null),
        "endpoint":solved.get("endpoint").cloned().unwrap_or(Value::Null),
        "solve":solved,"wait":waited,"result":fetched})
}

#[test]
fn combined_saved_solve_retains_the_small_source_descriptor_and_moves_large_results() {
    let source =
        json!({"schema_version":"kyuubiki.headless-model-source/v1","sha256":"a".repeat(64)});
    let upload = json!({"artifact_id":"b".repeat(64),"size_bytes":8_000_001});
    let solved = json!({"job_id":"owned-job","model_version_id":"owned-version","model_source":source,
            "model_artifact_upload":upload});
    let fetched = envelope();
    let pointer = array_pointer(&fetched["result"]["nodes"]);
    let combined = combine_solve_and_wait_results("owned-job", solved, job(), fetched);
    assert_eq!(combined["model_source"], source);
    assert_eq!(combined["solve"]["model_source"], source);
    assert_eq!(combined["model_artifact_upload"], upload);
    assert_eq!(combined["solve"]["model_artifact_upload"], upload);
    assert_eq!(
        array_pointer(&combined["result"]["result"]["nodes"]),
        pointer
    );
}

#[test]
fn job_observation_moves_original_raw_arrays_and_strings_without_changing_shape() {
    let source = envelope();
    let pointer = array_pointer(&source["result"]["nodes"]);
    let text_pointer = string_pointer(&source["result"]["text"]);
    let diagnostic_pointer = array_pointer(&source["job"]["diagnostics"]["samples"]);
    let expected = previous_job_state(source.clone(), true);
    let actual = normalize_job_state_result(source);
    assert_eq!(actual, expected);
    assert_eq!(array_pointer(&actual["raw"]["result"]["nodes"]), pointer);
    assert_eq!(
        string_pointer(&actual["raw"]["result"]["text"]),
        text_pointer
    );
    assert_eq!(
        array_pointer(&actual["raw"]["job"]["diagnostics"]["samples"]),
        diagnostic_pointer
    );
    assert_ne!(array_pointer(&actual["result"]["nodes"]), pointer);
    assert_ne!(
        array_pointer(&actual["job"]["diagnostics"]["samples"]),
        diagnostic_pointer
    );
}

#[test]
fn public_observation_mirrors_remain_independent_mutable_values() {
    let mut actual = normalize_job_state_result(envelope());
    actual["result"]["nodes"][0] = json!(9999);
    actual["job"]["diagnostics"]["samples"][0] = json!(8888);
    assert_eq!(actual["raw"]["result"]["nodes"][0], 0);
    assert_eq!(actual["raw"]["job"]["diagnostics"]["samples"][0], 0);
    actual["raw"]["result"]["nodes"][1] = json!(7777);
    assert_eq!(actual["result"]["nodes"][1], 1);
    assert_eq!(actual["result"]["literal"], "{{steps.999.result.secret}}");
}

#[test]
fn observation_normalization_matches_previous_optional_and_diagnostic_shapes() {
    for &status in JOB_STATUSES {
        for progress in [None, Some(Value::Null), Some(json!(0.5))] {
            for result in [
                None,
                Some(Value::Null),
                Some(json!([1, 2, 3])),
                Some(json!({"x":17})),
            ] {
                let mut source = json!({"job":{"job_id":"owned-job","status":status},
                    "unknown":{"kept":"only in raw"}});
                if let Some(progress) = &progress {
                    source["job"]["progress"] = progress.clone();
                }
                if let Some(result) = result {
                    source["result"] = result;
                }
                assert_eq!(
                    normalize_job_state_result(source.clone()),
                    previous_job_state(source, true)
                );
            }
        }
    }
}

#[test]
fn non_job_normalizer_fallback_keeps_the_original_allocation_and_shape() {
    for source in [
        large_result(),
        json!("x".repeat(4096)),
        json!([1, 2]),
        Value::Null,
        json!({"job":null}),
        json!({"job":false}),
    ] {
        let expected = source.clone();
        let pointer = source.get("nodes").map(array_pointer);
        let actual = normalize_job_state_result(source);
        assert_eq!(actual, expected);
        assert_eq!(actual.get("nodes").map(array_pointer), pointer);
    }
}

#[test]
fn submission_moves_original_raw_payload_and_keeps_its_existing_public_fields() {
    let source = envelope();
    let pointer = array_pointer(&source["result"]["nodes"]);
    let text_pointer = string_pointer(&source["result"]["text"]);
    let expected = previous_job_state(source.clone(), false);
    let actual = normalize_job_submission_result(source).unwrap();
    assert_eq!(actual, expected);
    assert_eq!(array_pointer(&actual["raw"]["result"]["nodes"]), pointer);
    assert_eq!(
        string_pointer(&actual["raw"]["result"]["text"]),
        text_pointer
    );
    assert!(actual.get("result").is_none());
}

#[test]
fn submission_states_and_receipt_rejection_keep_the_existing_gates() {
    for &status in JOB_STATUSES {
        let source = json!({"job":{"job_id":"owned-job","status":status}});
        let actual = normalize_job_submission_result(source.clone());
        if matches!(status, "failed" | "cancelled") {
            let marker = if status == "failed" {
                "job_terminal_failed:"
            } else {
                "job_terminal_cancelled:"
            };
            assert!(actual.unwrap_err().message.starts_with(marker));
        } else {
            assert_eq!(actual.unwrap(), previous_job_state(source, false));
        }
    }
    for source in [
        json!({}),
        json!({"job":{"job_id":"foreign","status":"completed"},"job_id":"owned"}),
        json!({"job":{"job_id":"owned-job","status":"completed"},"status":"solving"}),
    ] {
        let error = normalize_job_submission_result(source).unwrap_err();
        assert!(
            error
                .message
                .starts_with("service_request_outcome_unknown:")
        );
    }
}

#[test]
fn preferred_result_moves_original_result_and_job_without_deep_copies() {
    let source = envelope();
    let pointer = array_pointer(&source["result"]["nodes"]);
    let text_pointer = string_pointer(&source["result"]["text"]);
    let diagnostic_pointer = array_pointer(&source["job"]["diagnostics"]["samples"]);
    let expected = previous_preferred_result("owned-job", source.clone());
    let actual = normalize_preferred_job_result("owned-job", source);
    assert_eq!(actual, expected);
    assert_eq!(array_pointer(&actual["result"]["nodes"]), pointer);
    assert_eq!(string_pointer(&actual["result"]["text"]), text_pointer);
    assert_eq!(
        array_pointer(&actual["job"]["diagnostics"]["samples"]),
        diagnostic_pointer
    );
    assert!(actual.get("raw").is_none());
    assert!(actual.get("unused").is_none());
}

#[test]
fn separate_result_moves_the_original_object_without_cloning_its_arrays() {
    let source = envelope();
    let pointer = array_pointer(&source["result"]["nodes"]);
    let text_pointer = string_pointer(&source["result"]["text"]);
    let expected = previous_result_fetch("owned-job", source.clone());
    let actual = normalize_result_fetch_result("owned-job", source);
    assert_eq!(actual, expected);
    assert_eq!(array_pointer(&actual["result"]["nodes"]), pointer);
    assert_eq!(string_pointer(&actual["result"]["text"]), text_pointer);
    assert_eq!(actual.as_object().unwrap().len(), 2);
}

#[test]
fn separate_result_fallback_and_optional_values_match_previous_normalization() {
    for source in [
        large_result(),
        json!({}),
        json!({"result":null,"unused":17}),
        json!({"result":[1,2]}),
        json!("x".repeat(4096)),
        json!(false),
        json!(7),
        json!([]),
        Value::Null,
    ] {
        assert_eq!(
            normalize_result_fetch_result("owned-job", source.clone()),
            previous_result_fetch("owned-job", source)
        );
    }
}

#[test]
fn combined_solve_moves_all_three_owned_envelopes_without_changing_public_shape() {
    let solved = json!({"job_id":"owned-job","model_version_id":"owned-version",
        "endpoint":"127.0.0.1:5001","raw":{"data":(0..4096).collect::<Vec<_>>()}});
    let waited = json!({"job_id":"owned-job","status":"completed",
        "wait":{"samples":(0..4096).collect::<Vec<_>>()}});
    let fetched = normalize_result_fetch_result("owned-job", envelope());
    let solve_pointer = array_pointer(&solved["raw"]["data"]);
    let wait_pointer = array_pointer(&waited["wait"]["samples"]);
    let result_pointer = array_pointer(&fetched["result"]["nodes"]);
    let text_pointer = string_pointer(&fetched["result"]["text"]);
    let expected = previous_combined("owned-job", solved.clone(), waited.clone(), fetched.clone());
    let actual = combine_solve_and_wait_results("owned-job", solved, waited, fetched);
    assert_eq!(actual, expected);
    assert_eq!(
        array_pointer(&actual["solve"]["raw"]["data"]),
        solve_pointer
    );
    assert_eq!(
        array_pointer(&actual["wait"]["wait"]["samples"]),
        wait_pointer
    );
    assert_eq!(
        array_pointer(&actual["result"]["result"]["nodes"]),
        result_pointer
    );
    assert_eq!(
        string_pointer(&actual["result"]["result"]["text"]),
        text_pointer
    );
}

#[test]
fn combined_solve_optional_defaults_and_public_aliases_match_previous_values() {
    for solved in [
        json!({}),
        json!({"model_version_id":null,"endpoint":null}),
        json!({"model_version_id":"owned-version","endpoint":"127.0.0.1:5001"}),
    ] {
        for waited in [
            json!({}),
            json!({"status":null}),
            json!({"status":"completed"}),
        ] {
            let fetched = json!({"job_id":"owned-job","result":{"tip_displacement":1e-7}});
            let expected =
                previous_combined("owned-job", solved.clone(), waited.clone(), fetched.clone());
            let mut actual =
                combine_solve_and_wait_results("owned-job", solved.clone(), waited, fetched);
            assert_eq!(actual, expected);
            actual["model_version_id"] = json!("changed");
            assert_eq!(actual["solve"], solved);
        }
    }
}
