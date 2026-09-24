use crate::workflow_executor::run_transform_operator;
use serde_json::{Value, json};

const GUARD: &str = "transform.evaluate_structural_guard";
const PAIR: &str = "transform.benchmark_structural_pair";

fn guard(payload: Value, rule: Value) -> Result<Value, String> {
    run_transform_operator(GUARD, payload, json!({"rules":[rule]}))
}

fn pair(payload: Value, criterion: Value) -> Result<Value, String> {
    run_transform_operator(PAIR, payload, json!({"criteria":[criterion]}))
}

fn candidates() -> Value {
    json!({"left":{"max_stress":1.0},"right":{"max_stress":2.0}})
}

fn rejection(result: Result<Value, String>, operator: &str, path: &str) -> String {
    let error = result.expect_err("incomplete or invalid evidence cannot yield an assessment");
    assert!(error.contains(operator), "{error}");
    assert!(error.contains(path), "{error}");
    error
}

#[test]
fn missing_guard_metric_cannot_pass_or_count_as_checked() {
    rejection(
        run_transform_operator(
            GUARD,
            json!({"max_stress":1.0}),
            json!({"rules":[
                {"field":"max_stress","threshold":10.0},
                {"field":"max_displacement","threshold":10.0}
            ]}),
        ),
        GUARD,
        "payload.max_displacement",
    );
}

#[test]
fn malformed_guard_rule_is_not_filtered_out() {
    for rule in [Value::Null, json!(3), json!("rule"), json!([])] {
        rejection(
            guard(json!({"max_stress":1.0}), rule),
            GUARD,
            "config.rules[0]",
        );
    }
}

#[test]
fn guard_field_must_be_a_nonblank_string() {
    for field in [Value::Null, json!(3), json!(true), json!(""), json!("  ")] {
        rejection(
            guard(
                json!({"max_stress":1.0}),
                json!({"field":field,"threshold":10.0}),
            ),
            GUARD,
            "config.rules[0].field",
        );
    }
    rejection(
        guard(json!({}), json!({"threshold":10.0})),
        GUARD,
        "config.rules[0].field",
    );
}

#[test]
fn missing_or_invalid_guard_threshold_is_not_a_passing_rule() {
    for key in ["threshold", "value"] {
        for value in [Value::Null, json!(true), json!("10"), json!({})] {
            rejection(
                guard(
                    json!({"max_stress":1.0}),
                    json!({"field":"max_stress",key:value}),
                ),
                GUARD,
                &format!("config.rules[0].{key}"),
            );
        }
    }
    rejection(
        guard(json!({"max_stress":1.0}), json!({"field":"max_stress"})),
        GUARD,
        "threshold",
    );
}

#[test]
fn threshold_alias_cannot_hide_invalid_or_conflicting_canonical_value() {
    for threshold in [Value::Null, json!("broken"), json!(12.0)] {
        rejection(
            guard(
                json!({"max_stress":1.0}),
                json!({
                    "field":"max_stress","threshold":threshold,"value":10.0
                }),
            ),
            GUARD,
            "config.rules[0].threshold",
        );
    }
}

#[test]
fn invalid_guard_options_do_not_fall_back_even_when_rule_would_not_trigger() {
    for (field, value) in [
        ("comparison", json!("grater")),
        ("comparison", Value::Null),
        ("comparison", json!(1)),
        ("severity", json!("blok")),
        ("severity", Value::Null),
        ("severity", json!(false)),
        ("label", json!(3)),
        ("label", json!(" ")),
    ] {
        for metric in [1.0, 11.0] {
            let mut rule = json!({"field":"max_stress","threshold":10.0});
            rule[field] = value.clone();
            rejection(
                guard(json!({"max_stress":metric}), rule),
                GUARD,
                &format!("config.rules[0].{field}"),
            );
        }
    }
}

#[test]
fn guard_defaults_threshold_alias_and_resolved_metric_alias_remain_supported() {
    for rule in [
        json!({"field":"max_stress","value":10.0}),
        json!({"field":"max_stress","threshold":10.0,"value":10.0}),
    ] {
        let result = guard(json!({"von_mises_peak":10.0}), rule).unwrap();
        assert_eq!(result["guard_status"], "warn");
        assert_eq!(result["guard_checked_rule_count"], 1);
        assert_eq!(result["guard_triggers"][0]["comparison"], "gte");
        assert_eq!(result["guard_triggers"][0]["label"], "max_stress");
    }
}

#[test]
fn valid_comparison_boundary_semantics_are_preserved() {
    for (comparison, expected) in [
        ("gt", [false, false, true]),
        ("gte", [false, true, true]),
        ("lt", [true, false, false]),
        ("lte", [true, true, false]),
        ("eq", [false, true, false]),
    ] {
        for (value, triggered) in [9.0, 10.0, 11.0].into_iter().zip(expected) {
            let result = guard(
                json!({"max_stress":value}),
                json!({
                    "field":"max_stress","threshold":10.0,"comparison":comparison,"severity":"block"
                }),
            )
            .unwrap();
            assert_eq!(result["guard_block_count"], usize::from(triggered));
            assert_eq!(result["guard_passed"], !triggered);
        }
    }
}

#[test]
fn invalid_explicit_metric_cannot_be_replaced_by_an_alias() {
    for value in [Value::Null, json!("1"), json!(true), json!([])] {
        let payload = json!({"max_stress":value,"von_mises_peak":1.0});
        rejection(
            guard(
                payload.clone(),
                json!({"field":"max_stress","threshold":10.0}),
            ),
            GUARD,
            "payload.max_stress",
        );
        rejection(
            pair(
                json!({"left":payload,"right":{"max_stress":2.0}}),
                json!({"field":"max_stress"}),
            ),
            PAIR,
            "payload.left.max_stress",
        );
    }
}

#[test]
fn overflowing_derived_metric_is_not_a_guard_or_comparison_value() {
    let payload = json!({"temperature_min":-1e308,"temperature_max":1e308});
    rejection(
        guard(
            payload.clone(),
            json!({"field":"temperature_span","comparison":"lt","threshold":1.0}),
        ),
        GUARD,
        "payload.temperature_span",
    );
    rejection(
        pair(
            json!({"left":payload,"right":{"temperature_span":1.0}}),
            json!({"field":"temperature_span"}),
        ),
        PAIR,
        "payload.left.temperature_span",
    );
}

#[test]
fn missing_pair_criterion_cannot_reduce_the_denominator_and_declare_a_winner() {
    for side in ["left", "right"] {
        let mut payload =
            json!({"left":{"max_stress":1.0,"mass":4.0},"right":{"max_stress":2.0,"mass":1.0}});
        payload[side].as_object_mut().unwrap().remove("mass");
        let error = rejection(
            run_transform_operator(
                PAIR,
                payload,
                json!({"criteria":[
                    {"field":"max_stress"},{"field":"mass","weight":3.0}
                ]}),
            ),
            PAIR,
            &format!("payload.{side}.mass"),
        );
        assert!(error.contains("config.criteria[1]"), "{error}");
    }
}

#[test]
fn malformed_pair_criterion_cannot_be_skipped_beside_a_valid_one() {
    for bad in [
        Value::Null,
        json!(false),
        json!("mass"),
        json!([]),
        json!({}),
    ] {
        rejection(
            run_transform_operator(
                PAIR,
                candidates(),
                json!({"criteria":[{"field":"max_stress"},bad]}),
            ),
            PAIR,
            "config.criteria[1]",
        );
    }
}

#[test]
fn invalid_explicit_side_fields_do_not_fall_back_to_common_field() {
    for key in ["field", "left_field", "right_field"] {
        for value in [Value::Null, json!(3), json!(""), json!("  ")] {
            let mut criterion =
                json!({"field":"max_stress","left_field":"max_stress","right_field":"max_stress"});
            criterion[key] = value;
            rejection(
                pair(candidates(), criterion),
                PAIR,
                &format!("config.criteria[0].{key}"),
            );
        }
    }
}

#[test]
fn invalid_pair_goal_never_silently_switches_to_minimization() {
    for goal in [
        Value::Null,
        json!("maximize"),
        json!(true),
        json!(0),
        json!(""),
    ] {
        rejection(
            pair(candidates(), json!({"field":"max_stress","goal":goal})),
            PAIR,
            "config.criteria[0].goal",
        );
    }
}

#[test]
fn invalid_pair_weight_is_not_replaced_by_one() {
    for weight in [
        Value::Null,
        json!(0.0),
        json!(-1.0),
        json!("2"),
        json!(true),
    ] {
        rejection(
            pair(candidates(), json!({"field":"max_stress","weight":weight})),
            PAIR,
            "config.criteria[0].weight",
        );
    }
}

#[test]
fn comparison_labels_cannot_collide_or_use_the_tie_sentinel() {
    for (left, right) in [
        (json!("same"), json!("same")),
        (json!(" same "), json!("same")),
        (json!("tie"), json!("other")),
        (json!("other"), json!("tie")),
    ] {
        rejection(
            run_transform_operator(
                PAIR,
                candidates(),
                json!({
                    "left_label":left,"right_label":right,"criteria":[{"field":"max_stress"}]
                }),
            ),
            PAIR,
            "label",
        );
    }
    for key in ["left_label", "right_label"] {
        for label in [Value::Null, json!(3), json!("  ")] {
            rejection(
                run_transform_operator(
                    PAIR,
                    candidates(),
                    json!({key:label,"criteria":[{"field":"max_stress"}]}),
                ),
                PAIR,
                &format!("config.{key}"),
            );
        }
    }
}

#[test]
fn pair_delta_overflow_is_not_serialized_as_successful_null() {
    for sign in [-1.0, 1.0] {
        rejection(
            pair(
                json!({"left":{"metric":-sign*1e308},"right":{"metric":sign*1e308}}),
                json!({"field":"metric"}),
            ),
            PAIR,
            "config.criteria[0].delta",
        );
    }
}

#[test]
fn pair_total_score_overflow_fails_before_serializing_a_winner() {
    for side in ["left", "right"] {
        let mut payload = json!({"left":{"metric":2.0},"right":{"metric":2.0}});
        payload[side]["metric"] = json!(1.0);
        rejection(
            run_transform_operator(
                PAIR,
                payload,
                json!({"criteria":[
                    {"field":"metric","weight":1e308}, {"field":"metric","weight":1e308}
                ]}),
            ),
            PAIR,
            &format!("{side}_score"),
        );
    }
}

#[test]
fn valid_pair_defaults_distinct_side_fields_and_weighted_ties_are_preserved() {
    let result = run_transform_operator(
        PAIR,
        json!({
            "left":{"max_stress":1.0,"mass":3.0,"life":5.0},
            "right":{"von_mises_peak":2.0,"mass":3.0,"duration":6.0}
        }),
        json!({"left_label":" baseline ","right_label":"candidate","criteria":[
            {"field":"max_stress"}, {"field":"mass","weight":2.0},
            {"left_field":"life","right_field":"duration","goal":"max","weight":3.0}
        ]}),
    )
    .unwrap();
    assert_eq!(result["benchmark_winner"], "candidate");
    assert_eq!(result["benchmark_criteria_count"], 3);
    assert_eq!(result["baseline_score"], 2.0);
    assert_eq!(result["candidate_score"], 4.0);
    assert_eq!(result["benchmark_tie_count"], 1);
    assert_eq!(result["benchmark_breakdown"][0]["weight"], 1.0);
    assert_eq!(result["benchmark_breakdown"][2]["right_field"], "duration");
}

#[test]
fn all_domain_guard_and_pair_registrations_require_every_configured_metric() {
    for domain in [
        "structural",
        "thermal",
        "acoustic",
        "modal",
        "dynamic",
        "electrostatic",
        "magnetostatic",
        "cfd",
        "transport",
    ] {
        let operator = format!("transform.evaluate_{domain}_guard");
        rejection(
            run_transform_operator(
                &operator,
                json!({}),
                json!({"rules":[{"field":"missing","threshold":10.0}]}),
            ),
            &operator,
            "payload.missing",
        );
        let pair_domain = if domain == "thermal" {
            "coupled_heat"
        } else {
            domain
        };
        let operator = format!("transform.benchmark_{pair_domain}_pair");
        rejection(
            run_transform_operator(
                &operator,
                candidates(),
                json!({"criteria":[
                    {"field":"max_stress"},{"field":"missing"}
                ]}),
            ),
            &operator,
            "payload.left.missing",
        );
    }
}
