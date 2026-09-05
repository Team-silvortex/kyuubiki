use crate::workflow_executor::run_transform_operator;
use serde_json::{Value, json};

#[test]
fn numeric_column_overflow_cannot_emit_null_or_reset_the_accumulator() {
    let error = summarize(
        json!([
            {"id": "a", "summary": {"mass": 1e308}}, {"id": "b", "summary": {"mass": 1e308}},
            {"id": "c", "summary": {"mass": 1}}
        ]),
        Value::Null,
    )
    .unwrap_err();
    assert!(
        error.contains("mass") && error.contains('b') && error.contains("non-finite"),
        "{error}"
    );
}

#[test]
fn summary_field_selections_must_be_unique_nonblank_strings() {
    for fields in [
        Value::Null,
        json!("mass"),
        json!([]),
        json!(["mass", "mass"]),
        json!(["mass", 2]),
        json!([""]),
        json!([" \t "]),
    ] {
        assert!(summarize(json!([{"summary": {"mass": 2}}]), json!({"fields": fields})).is_err());
    }
}

#[test]
fn summary_metrics_cannot_replace_case_identity_or_metadata() {
    for field in ["case_id", "parameters", "metadata"] {
        for config in [
            Value::Null,
            json!({"fields": [field], "include_metadata": false, "include_parameters": false}),
        ] {
            assert!(
                summarize(
                    json!([{"id": "actual", "summary": {field: "forged"}}]),
                    config
                )
                .is_err()
            );
        }
    }
}

#[test]
fn missing_selected_metrics_require_explicit_diagnostic_mode() {
    let cases = json!([{"id": "a", "summary": {"mass": 1, "temperature": 10}},
                      {"id": "b", "summary": {"mass": 2}}]);
    let error = summarize(cases.clone(), Value::Null).unwrap_err();
    assert!(
        error.contains("temperature") && error.contains('b'),
        "{error}"
    );
    let summary = summarize(cases, json!({"fail_on_missing": false})).unwrap();
    assert_eq!(summary["summary_complete"], false);
    assert_eq!(summary["missing_field_count"], 1);
    assert_eq!(summary["missing_fields"][0]["case_id"], "b");
    assert_eq!(summary["missing_fields"][0]["field"], "temperature");
    assert_eq!(summary["numeric_columns"]["temperature"]["count"], 1);
    assert!(score(summary).unwrap_err().contains("incomplete"));
}

#[test]
fn mixed_numeric_and_text_metrics_cannot_silently_shrink_statistics() {
    let cases = json!([{"id": "a", "summary": {"mass": 1}}, {"id": "b", "summary": {"mass": "2"}}]);
    assert!(summarize(cases.clone(), Value::Null).is_err());
    let summary = summarize(cases, json!({"fail_on_missing": false})).unwrap();
    assert_eq!(summary["missing_fields"][0]["reason"], "non_numeric");
    assert_eq!(summary["summary_complete"], false);
}

#[test]
fn duplicate_or_malformed_case_ids_are_not_rankable_rows() {
    for cases in [
        json!([{"id": "a", "summary": {}}, {"id": "a", "summary": {}}]),
        json!([{"summary": {}}, {"id": "case_0", "summary": {}}]),
        json!([{"id": null, "summary": {"mass": 1}}]),
        json!([{"id": 1, "summary": {"mass": 1}}]),
    ] {
        assert!(summarize(cases, Value::Null).is_err());
    }
}

#[test]
fn unusable_result_status_cannot_be_hidden_by_a_valid_summary() {
    for status in [json!("failed"), json!("pending"), json!(null), json!(1)] {
        assert!(
            summarize(
                json!([{"id": "a", "result_status": status, "summary": {"mass": 0}}]),
                Value::Null
            )
            .is_err()
        );
    }
    assert!(summarize(json!([{"id": "a", "result_status": "ok", "result_error": "bad", "summary": {"mass": 0}}]), Value::Null).is_err());
}

#[test]
fn summary_options_do_not_accept_malformed_boolean_defaults() {
    for config in [
        json!([]),
        json!(false),
        json!({"include_parameters": "false"}),
        json!({"include_metadata": null}),
        json!({"fail_on_missing": "false"}),
    ] {
        assert!(summarize(json!([{"summary": {"mass": 1}}]), config).is_err());
    }
}

#[test]
fn finite_signed_statistics_and_text_rows_keep_their_meaning() {
    let summary = summarize(
        json!([
            {"id": "a", "summary": {"mass": -2, "note": "cold"}},
            {"id": "b", "summary": {"mass": 0, "note": "neutral"}},
            {"id": "c", "summary": {"mass": 2, "note": "warm"}}
        ]),
        Value::Null,
    )
    .unwrap();
    assert_eq!(summary["summary_complete"], true);
    assert_eq!(
        summary["numeric_columns"]["mass"],
        json!({"count": 3, "sum": 0.0, "mean": 0.0, "min": -2.0, "max": 2.0})
    );
    assert_eq!(summary["rows"][1]["note"], "neutral");
    assert!(summary["numeric_columns"].get("note").is_none());
}

#[test]
fn contradictory_summary_completeness_cannot_enter_scoring() {
    for fields in [
        json!({"summary_complete": false}),
        json!({"summary_complete": "true"}),
        json!({"summary_complete": true, "missing_field_count": 1}),
        json!({"summary_complete": true, "missing_fields": [{"case_id": "a", "field": "temperature"}]}),
        json!({"missing_field_count": null}),
        json!({"missing_fields": "none"}),
    ] {
        let mut payload = fields;
        payload["rows"] = json!([{"case_id": "a", "mass": 1}]);
        assert!(score(payload).is_err());
    }
}

fn summarize(cases: Value, config: Value) -> Result<Value, String> {
    run_transform_operator(
        "transform.summarize_parameter_sweep",
        json!({"cases": cases}),
        config,
    )
}

#[test]
fn null_metrics_and_blank_automatic_keys_are_not_complete_evidence() {
    assert!(summarize(json!([{"summary": {"mass": null}}]), Value::Null).is_err());
    assert!(summarize(json!([{"summary": {"": 1}}]), Value::Null).is_err());
    let summary = summarize(
        json!([{"summary": {"mass": null}}]),
        json!({"fail_on_missing": false}),
    )
    .unwrap();
    assert_eq!(summary["missing_fields"][0]["reason"], "null");
    assert_eq!(summary["summary_complete"], false);
    assert!(score(summary).is_err());
}

#[test]
fn declared_result_and_row_counts_must_match_the_actual_batch() {
    for field in ["case_count", "joined_summary_count"] {
        for count in [json!(2), Value::Null, json!("1")] {
            assert!(
                run_transform_operator(
                    "transform.summarize_parameter_sweep",
                    json!({
                        field: count, "cases": [{"id": "a", "summary": {"mass": 1}}]
                    }),
                    Value::Null
                )
                .is_err()
            );
        }
    }
    assert!(score(json!({"row_count": 2, "rows": [{"case_id": "a", "mass": 1}]})).is_err());
}

#[test]
fn complete_flags_do_not_override_missing_or_unmatched_result_evidence() {
    for fields in [
        json!({"join_complete": false}),
        json!({"join_complete": "true"}),
        json!({"join_complete": true, "missing_summary_count": 1}),
        json!({"join_complete": true, "unmatched_result_ids": ["other"]}),
        json!({"join_complete": true, "rejected_result_count": 1}),
    ] {
        let mut payload = fields;
        payload["cases"] = json!([{"id": "a", "summary": {"mass": 1}}]);
        assert!(
            run_transform_operator("transform.summarize_parameter_sweep", payload, Value::Null)
                .is_err()
        );
    }
}

fn score(payload: Value) -> Result<Value, String> {
    run_transform_operator(
        "transform.score_parameter_sweep",
        payload,
        json!({"objectives": [{"field": "mass"}]}),
    )
}
