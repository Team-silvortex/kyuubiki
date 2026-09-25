use crate::workflow_executor::run_transform_operator;
use serde_json::{Value, json};

const COMPOSE: &str = "transform.compose_diagnostics_bundle";
const GUARD: &str = "transform.evaluate_diagnostics_bundle_guard";

fn diagnostic(value: f64) -> Value {
    json!({"diagnostic_contract":"kyuubiki.workflow_diagnostics/v1","diagnostic_domain":"thermal",
        "diagnostic_prefix":"thermal","diagnostic_subject":"thermal_result","diagnostic_metric_groups":["temperature"],
        "diagnostic_node_count":2,"diagnostic_element_count":1,"thermal_temperature_max":value})
}

fn bundle() -> Value {
    run_transform_operator(
        COMPOSE,
        json!({"left":diagnostic(10.0),"right":diagnostic(20.0)}),
        json!({}),
    )
    .unwrap()
}

fn rule() -> Value {
    json!({"source":"left","field":"thermal_temperature_max","threshold":30.0})
}

fn reject(operator: &str, input: Value, config: Value, path: &str) {
    let error = run_transform_operator(operator, input, config).unwrap_err();
    assert!(error.contains(path), "expected {path}, got {error}");
}

#[test]
fn missing_bundle_guard_metrics_and_sources_cannot_pass() {
    for source in ["left", "missing"] {
        let mut rule = rule();
        rule["source"] = json!(source);
        rule["field"] = json!("missing_metric");
        reject(GUARD, bundle(), json!({"rules":[rule]}), "config.rules[0]");
    }
    reject(
        GUARD,
        bundle(),
        json!({"rules":[{"field":"missing_total","threshold":1.0}]}),
        "missing_total",
    );
}

#[test]
fn invalid_guard_rule_is_not_counted_as_successfully_checked() {
    for bad in [
        Value::Null,
        json!([]),
        json!({}),
        json!({"field":"bundle_source_count"}),
    ] {
        reject(
            GUARD,
            bundle(),
            json!({"rules":[rule(),bad]}),
            "config.rules[1]",
        );
    }
}

#[test]
fn invalid_bundle_guard_options_cannot_become_default_comparisons() {
    for (key, bads) in [
        (
            "comparison",
            vec![Value::Null, json!(false), json!("greater")],
        ),
        ("severity", vec![Value::Null, json!("fatal")]),
        ("source", vec![Value::Null, json!(false), json!(" ")]),
        ("field", vec![Value::Null, json!("")]),
        ("label", vec![Value::Null, json!("")]),
        ("threshold", vec![Value::Null, json!("30")]),
    ] {
        for bad in bads {
            let mut rule = rule();
            rule[key] = bad;
            reject(
                GUARD,
                bundle(),
                json!({"rules":[rule]}),
                &format!("config.rules[0].{key}"),
            );
        }
    }
}

#[test]
fn bundle_guard_threshold_aliases_must_agree_and_be_valid() {
    let mut conflict = rule();
    conflict["value"] = json!(31.0);
    reject(GUARD, bundle(), json!({"rules":[conflict]}), "conflicts");
    let mut invalid = rule();
    invalid["value"] = Value::Null;
    reject(
        GUARD,
        bundle(),
        json!({"rules":[invalid]}),
        "config.rules[0].value",
    );
}

#[test]
fn invalid_direct_bundle_metric_cannot_be_skipped_or_resolved_from_an_alias() {
    for bad in [Value::Null, json!("10"), json!(false)] {
        let mut input = bundle();
        input["bundle_payloads"]["left"]["thermal_temperature_max"] = bad;
        input["bundle_payloads"]["left"]["max_temperature"] = json!(10.0);
        reject(
            GUARD,
            input,
            json!({"rules":[rule()]}),
            "payload.bundle_payloads.left.thermal_temperature_max",
        );
    }
}

#[test]
fn payload_free_bundle_cannot_claim_source_rules_passed() {
    let input = run_transform_operator(
        COMPOSE,
        json!({"left":diagnostic(10.0)}),
        json!({"include_payloads":false}),
    )
    .unwrap();
    reject(GUARD, input, json!({"rules":[rule()]}), "bundle_payloads");
}

#[test]
fn diagnostic_bundle_composition_cannot_erase_failed_sources() {
    for marker in [json!(false), Value::Null, json!("true")] {
        let mut input = json!({"left":diagnostic(10.0),"right":diagnostic(20.0)});
        input["right"]["converged"] = marker;
        reject(
            COMPOSE,
            input,
            json!({"include_payloads":false}),
            "payload.right.converged",
        );
    }
}

#[test]
fn bundle_guard_checks_all_retained_source_admission_not_just_the_rule_target() {
    let mut input = bundle();
    input["bundle_payloads"]["right"]["converged"] = json!(false);
    reject(
        GUARD,
        input,
        json!({"rules":[rule()]}),
        "payload.bundle_payloads.right.converged",
    );
    for operator in [COMPOSE, GUARD] {
        let mut input = if operator == COMPOSE {
            json!({"left":diagnostic(10.0)})
        } else {
            bundle()
        };
        input["converged"] = json!(false);
        reject(
            operator,
            input,
            json!({"rules":[rule()]}),
            "payload.converged",
        );
    }
}

#[test]
fn malformed_bundle_flags_or_claimed_contracts_cannot_silently_drop_a_source() {
    for key in [
        "include_payloads",
        "include_numeric_fields",
        "include_non_diagnostics",
    ] {
        for bad in [Value::Null, json!("true"), json!(1)] {
            reject(
                COMPOSE,
                json!({"left":diagnostic(10.0)}),
                json!({key:bad}),
                &format!("config.{key}"),
            );
        }
    }
    for bad in [
        Value::Null,
        json!(false),
        json!("kyuubiki.workflow_diagnostics/v99"),
    ] {
        let mut right = diagnostic(20.0);
        right["diagnostic_contract"] = bad;
        reject(
            COMPOSE,
            json!({"left":diagnostic(10.0),"right":right}),
            json!({}),
            "payload.right.diagnostic_contract",
        );
    }
}

#[test]
fn malformed_bundle_count_metadata_cannot_disappear_from_totals() {
    for key in ["diagnostic_node_count", "diagnostic_element_count"] {
        for bad in [Value::Null, json!(-1), json!(0.5), json!("2")] {
            let mut right = diagnostic(20.0);
            right[key] = bad;
            reject(
                COMPOSE,
                json!({"left":diagnostic(10.0),"right":right}),
                json!({}),
                &format!("payload.right.{key}"),
            );
        }
    }
}

#[test]
fn bundle_count_overflow_returns_an_error_not_a_panic_or_wrapped_total() {
    for (field, total) in [
        ("diagnostic_node_count", "bundle_total_node_count"),
        ("diagnostic_element_count", "bundle_total_element_count"),
    ] {
        let mut left = diagnostic(10.0);
        left[field] = json!(u64::MAX);
        let mut right = diagnostic(20.0);
        right[field] = json!(1);
        reject(
            COMPOSE,
            json!({"left":left,"right":right}),
            json!({}),
            total,
        );
    }
}

#[test]
fn absent_bundle_counts_remain_unknown_not_zero_measurements() {
    let mut left = diagnostic(10.0);
    left.as_object_mut()
        .unwrap()
        .remove("diagnostic_node_count");
    let input = run_transform_operator(
        COMPOSE,
        json!({"left":left,"right":diagnostic(20.0)}),
        json!({}),
    )
    .unwrap();
    assert_eq!(input["bundle_items"][0]["node_count"], Value::Null);
    assert_eq!(input["bundle_total_node_count"], Value::Null);
    assert_eq!(input["bundle_total_element_count"], 2);
    reject(
        GUARD,
        input,
        json!({"rules":[{"field":"bundle_total_node_count","threshold":100.0}]}),
        "bundle_total_node_count",
    );
}

#[test]
fn metadata_only_diagnostics_do_not_count_as_bundle_measurement_evidence() {
    let mut left = diagnostic(10.0);
    left.as_object_mut()
        .unwrap()
        .remove("thermal_temperature_max");
    reject(
        COMPOSE,
        json!({"left":left,"right":diagnostic(20.0)}),
        json!({}),
        "payload.left",
    );
}

#[test]
fn invalid_domain_and_metric_group_metadata_is_not_filtered() {
    for (key, bad) in [
        ("diagnostic_domain", json!(false)),
        ("diagnostic_prefix", json!(" ")),
        ("diagnostic_subject", Value::Null),
        ("diagnostic_metric_groups", json!("temperature")),
        ("diagnostic_metric_groups", json!(["temperature", null])),
    ] {
        let mut left = diagnostic(10.0);
        left[key] = bad;
        reject(
            COMPOSE,
            json!({"left":left}),
            json!({}),
            &format!("payload.left.{key}"),
        );
    }
}

#[test]
fn valid_rule_defaults_legacy_threshold_and_root_counts_remain_usable() {
    let input = bundle();
    let result = run_transform_operator(
        GUARD,
        input.clone(),
        json!({"rules":[
        {"source":"left","field":"thermal_temperature_max","value":10.0},
        {"field":"bundle_source_count","comparison":"eq","threshold":2.0,"severity":"block"}]}),
    )
    .unwrap();
    assert_eq!(result["guard_status"], "block");
    assert_eq!(result["guard_checked_rule_count"], 2);
    assert_eq!(result["guard_warn_count"], 1);
    assert_eq!(result["guard_block_count"], 1);
    let passed = run_transform_operator(GUARD, input, json!({"rules":[rule()]})).unwrap();
    assert_eq!(passed["guard_passed"], true);
}

#[test]
fn plain_numeric_sources_are_opt_in_and_zero_counts_are_real_evidence() {
    let mut left = diagnostic(10.0);
    left["diagnostic_node_count"] = json!(0);
    let result = run_transform_operator(
        COMPOSE,
        json!({"left":left,"ignored":{"metric":20.0}}),
        json!({}),
    )
    .unwrap();
    assert_eq!(result["bundle_source_count"], 1);
    assert_eq!(result["bundle_total_node_count"], 0);
    let result = run_transform_operator(
        COMPOSE,
        json!({"plain":{"metric":20.0}}),
        json!({"include_non_diagnostics":true,"include_numeric_fields":false}),
    )
    .unwrap();
    assert_eq!(result["bundle_source_count"], 1);
    assert_eq!(result["bundle_payloads"]["plain"]["metric"], 20.0);
    assert!(result.get("bundle_numeric_fields").is_none());
}

#[test]
fn prefixed_counts_are_only_a_fallback_for_absent_canonical_counts() {
    let mut left = diagnostic(10.0);
    left["thermal_node_count"] = json!(7);
    left["thermal_element_count"] = json!(3);
    let compose = |left| run_transform_operator(COMPOSE, json!({"left":left}), json!({}));
    let canonical = compose(left.clone()).unwrap();
    assert_eq!(canonical["bundle_total_node_count"], 2);
    assert_eq!(canonical["bundle_total_element_count"], 1);
    for key in ["diagnostic_node_count", "diagnostic_element_count"] {
        let mut invalid = left.clone();
        invalid[key] = Value::Null;
        assert!(compose(invalid).unwrap_err().contains(key));
        left.as_object_mut().unwrap().remove(key);
    }
    let fallback = compose(left.clone()).unwrap();
    assert_eq!(fallback["bundle_total_node_count"], 7);
    assert_eq!(fallback["bundle_total_element_count"], 3);
    for key in ["thermal_node_count", "thermal_element_count"] {
        let mut invalid = left.clone();
        invalid[key] = json!("7");
        assert!(compose(invalid).unwrap_err().contains(key));
    }
}

#[test]
fn malformed_selected_sources_and_retained_payload_shapes_are_not_ignored() {
    for bad in [Value::Null, json!([]), json!(false), json!(7)] {
        reject(
            COMPOSE,
            json!({"left":diagnostic(10.0),"right":bad}),
            json!({"include_non_diagnostics":true}),
            "payload.right",
        );
        let mut input = bundle();
        input["bundle_payloads"] = bad.clone();
        reject(
            GUARD,
            input,
            json!({"rules":[rule()]}),
            "payload.bundle_payloads",
        );
        let mut input = bundle();
        input["bundle_payloads"]["right"] = bad;
        reject(
            GUARD,
            input,
            json!({"rules":[rule()]}),
            "payload.bundle_payloads.right",
        );
    }
    let mut unmarked = diagnostic(10.0);
    unmarked
        .as_object_mut()
        .unwrap()
        .remove("diagnostic_contract");
    reject(
        COMPOSE,
        json!({"left":unmarked}),
        json!({}),
        "payload.left.diagnostic_contract",
    );
}
