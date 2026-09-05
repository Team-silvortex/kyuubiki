use crate::workflow_executor::run_transform_operator;
use serde_json::{Map, Value, json};

#[test]
fn integral_float_case_limits_are_enforced_in_direct_expansion() {
    let error = expand(direct(), json!({"max_cases": 2.0}))
        .expect_err("a floating representation must not discard the budget");
    assert!(error.contains("above max_cases 2"), "{error}");
    assert_eq!(
        expand(direct(), json!({"max_cases": 3.0})).unwrap()["case_count"],
        3
    );
}

#[test]
fn materialization_rechecks_a_lower_effective_budget() {
    let plan = plan(json!({"model.x": [1, 2, 3]}), json!({"max_cases": 8})).unwrap();
    let expansion = materialize(plan, json!({"max_cases": 2.0})).unwrap();
    assert_eq!(expansion["config"]["max_cases"].as_u64(), Some(2));
    assert_eq!(expansion["expansion_budget_ready"], false);
    assert_eq!(expansion["sweep_budget"]["max_cases"], 2);
    assert_budget_block(expand(expansion, Value::Null).unwrap());
}

#[test]
fn expansion_rechecks_a_lower_budget_override_before_allocation() {
    let plan = plan(json!({"model.x": [1, 2, 3]}), json!({"max_cases": 8})).unwrap();
    let expansion = materialize(plan, Value::Null).unwrap();
    assert_budget_block(expand(expansion, json!({"max_cases": 2.0})).unwrap());
}

#[test]
fn upstream_budget_blocks_require_replanning_not_a_downstream_override() {
    let plan = plan(json!({"model.x": [1, 2, 3]}), json!({"max_cases": 1})).unwrap();
    let expansion = materialize(plan, json!({"max_cases": 100})).unwrap();
    assert_eq!(expansion["expansion_budget_ready"], false);
    assert_budget_block(expand(expansion, json!({"max_cases": 100})).unwrap());
}

#[test]
fn malformed_case_limits_cannot_fall_back_to_larger_defaults() {
    for value in [Value::Null, json!("2"), json!(-1), json!(1.5), json!(true)] {
        let config = json!({"max_cases": value});
        assert!(expand(direct(), config.clone()).is_err());
        assert!(plan(json!({"model.x": [1, 2, 3]}), config.clone()).is_err());
        assert!(
            materialize(
                json!({"sweep_enabled": true, "base": direct()["base"],
            "axes": direct()["axes"]}),
                config
            )
            .is_err()
        );
    }
}

#[test]
fn malformed_sampling_options_are_not_rounded_or_clamped() {
    for field in ["samples_per_axis", "max_axes"] {
        for value in [Value::Null, json!("2"), json!(0), json!(1.5), json!(-1)] {
            let error = plan(
                json!({"model.x": {"min": 1.0, "max": 2.0}}),
                json!({field: value}),
            )
            .expect_err("invalid counts must not silently change the sweep");
            assert!(error.contains(field), "{error}");
        }
    }
}

#[test]
fn planner_checks_cartesian_product_overflow_before_materializing_values() {
    let space: Map<String, Value> = (0..65)
        .map(|i| (format!("model.p{i}"), json!([0, 1])))
        .collect();
    let error =
        plan(Value::Object(space), Value::Null).expect_err("product overflow must not panic");
    assert!(error.contains("overflow"), "{error}");
}

#[test]
fn malformed_axes_cannot_silently_shrink_search_coverage() {
    for spec in [
        Value::Null,
        json!([]),
        json!({"values": []}),
        json!({"min": 0.0}),
        json!({"min": "0", "max": 1.0}),
    ] {
        let error = plan(
            json!({"model.good": [1, 2], "model.bad": spec}),
            Value::Null,
        )
        .expect_err("missing axes must not be silently skipped");
        assert!(error.contains("model.bad"), "{error}");
    }
}

#[test]
fn over_budget_ranges_are_described_without_allocating_samples() {
    let plan = plan(
        json!({"model.x": {"min": 0.0, "max": 1.0}}),
        json!({"samples_per_axis": 1_000_000_000, "max_cases": 2}),
    )
    .unwrap();
    assert_eq!(plan["case_count_estimate"], 1_000_000_000);
    assert_eq!(plan["sweep_budget"]["case_budget_exceeded"], true);
    assert_eq!(plan["axes"][0]["value_count"], 1_000_000_000);
    assert_eq!(
        plan["sweep_budget"]["recommendation"],
        "reduce_samples_per_axis"
    );
    assert_eq!(plan["axes"][0]["values_deferred"], true);
    assert!(plan["axes"][0].get("values").is_none());
    assert_budget_block(expand(materialize(plan, Value::Null).unwrap(), Value::Null).unwrap());
}

#[test]
fn interpolation_preserves_finite_extreme_endpoints_and_midpoints() {
    for (min, max, middle) in [
        (-1.0e308, 1.0e308, 0.0),
        (1.0e308, -1.0e308, 0.0),
        (1.0e308, 1.0e308, 1.0e308),
    ] {
        let plan = plan(json!({"model.x": {"min": min, "max": max}}), Value::Null).unwrap();
        assert_eq!(plan["axes"][0]["values"], json!([min, middle, max]));
    }
    let plan = plan(json!({"model.x": {"min": 1.0, "max": 3.0}}), Value::Null).unwrap();
    assert_eq!(plan["axes"][0]["values"], json!([1.0, 2.0, 3.0]));
}

#[test]
fn materialization_uses_actual_axes_instead_of_stale_budget_claims() {
    let expansion = materialize(
        json!({
            "sweep_enabled": true, "case_count_estimate": 1, "max_cases": 2,
            "sweep_budget": {"case_budget_exceeded": false, "status": "ok"},
            "base": direct()["base"], "axes": direct()["axes"]
        }),
        Value::Null,
    )
    .unwrap();
    assert_eq!(expansion["case_count_estimate"], 3);
    assert_eq!(expansion["expansion_budget_ready"], false);
    assert_budget_block(expand(expansion, Value::Null).unwrap());
}

#[test]
fn malformed_execution_flags_do_not_enable_a_sweep() {
    for value in [Value::Null, json!("false"), json!(0)] {
        assert!(
            materialize(
                json!({"sweep_enabled": value,
            "base": direct()["base"], "axes": direct()["axes"]}),
                Value::Null
            )
            .is_err()
        );
        for field in ["expansion_enabled", "expansion_budget_ready"] {
            let mut wrapper = json!({
                "quality_sweep_expansion_contract": "kyuubiki.quality_sweep_expansion/v1",
                "expansion_enabled": true, "payload": direct(), "config": {"max_cases": 3}
            });
            wrapper[field] = value.clone();
            assert!(expand(wrapper, Value::Null).is_err(), "{field}");
        }
    }
}

#[test]
fn a_zero_budget_remains_zero_through_the_whole_chain() {
    assert!(expand(direct(), json!({"max_cases": 0.0})).is_err());
    let plan = plan(json!({"model.x": [1, 2]}), json!({"max_cases": 0.0})).unwrap();
    assert_eq!(plan["max_cases"].as_u64(), Some(0));
    assert_budget_block(expand(materialize(plan, Value::Null).unwrap(), Value::Null).unwrap());
}

#[test]
fn out_of_range_counts_never_saturate_or_fall_back() {
    for value in [json!(1.0e308), json!(usize::MAX as f64 + 1.0)] {
        for field in ["samples_per_axis", "max_axes", "max_cases"] {
            let error = plan(json!({"model.x": [1, 2]}), json!({field: value}))
                .expect_err("unrepresentable counts must be rejected");
            assert!(error.contains(field), "{error}");
        }
        assert!(expand(direct(), json!({"max_cases": value})).is_err());
    }
    assert!(plan(json!({"model.x": [1, 2]}), json!({"samples_per_axis": 1})).is_err());
    assert_eq!(
        crate::workflow_sweep_contract::count_option(Some(&json!(usize::MAX)), "max_cases", 0, 0,)
            .unwrap(),
        usize::MAX
    );
}

#[test]
fn unselected_ranges_are_not_materialized_and_remain_reported() {
    let plan = plan(
        json!({"model.x": [1, 2], "model.y": {"min": 0, "max": 1}}),
        json!({"samples_per_axis": 1_000_000_000, "max_axes": 1.0, "max_cases": 2.0}),
    )
    .unwrap();
    assert_eq!(plan["case_count_estimate"], 2);
    assert_eq!(plan["sweep_budget"]["usable_axis_count"], 2);
    assert_eq!(plan["sweep_budget"]["axis_budget_truncated"], true);
    assert_eq!(
        plan["sweep_budget"]["recommendation"],
        "schedule_followup_axis_batch"
    );
    assert_eq!(
        expand(materialize(plan, Value::Null).unwrap(), Value::Null).unwrap()["case_count"],
        2
    );
}

#[test]
fn malformed_configs_and_budget_flags_cannot_restore_defaults() {
    for config in [json!([]), json!("default"), json!(false)] {
        assert!(plan(json!({"model.x": [1, 2]}), config.clone()).is_err());
        assert!(materialize(direct(), config.clone()).is_err());
        assert!(expand(direct(), config.clone()).is_err());
        assert!(
            expand(
                json!({
                    "quality_sweep_expansion_contract": "kyuubiki.quality_sweep_expansion/v1",
                    "payload": direct(), "config": config
                }),
                Value::Null
            )
            .is_err()
        );
    }
    for value in [Value::Null, json!("false"), json!(0)] {
        let mut plan = direct();
        plan["sweep_budget"] = json!({"case_budget_exceeded": value});
        assert!(materialize(plan.clone(), Value::Null).is_err());
        assert!(
            expand(
                json!({
                    "quality_sweep_expansion_contract": "kyuubiki.quality_sweep_expansion/v1",
                    "sweep_budget": plan["sweep_budget"], "payload": direct()
                }),
                Value::Null
            )
            .is_err()
        );
    }
}

#[test]
fn unknown_wrapper_contracts_and_conflicting_readiness_are_not_executable() {
    for contract in [
        Value::Null,
        json!(1),
        json!("kyuubiki.quality_sweep_expansion/v999"),
    ] {
        assert!(
            expand(
                json!({"quality_sweep_expansion_contract": contract, "payload": direct()}),
                Value::Null
            )
            .is_err()
        );
    }
    assert_budget_block(
        expand(
            json!({
                "quality_sweep_expansion_contract": "kyuubiki.quality_sweep_expansion/v1",
                "expansion_budget_ready": true, "sweep_budget": {"case_budget_exceeded": true},
                "payload": direct(), "config": {"max_cases": 100}
            }),
            Value::Null,
        )
        .unwrap(),
    );
}

#[test]
fn actual_axis_product_overflow_is_rejected_at_each_execution_boundary() {
    let axes: Vec<Value> = (0..65)
        .map(|i| json!({"path": format!("model.p{i}"), "values": [0, 1]}))
        .collect();
    let payload = json!({"base": {}, "axes": axes, "case_count_estimate": 1});
    assert!(
        materialize(payload.clone(), Value::Null)
            .unwrap_err()
            .contains("overflow")
    );
    assert!(
        expand(payload, Value::Null)
            .unwrap_err()
            .contains("overflow")
    );
}

#[test]
fn serialized_mixed_sweeps_preserve_limits_and_case_values() {
    let plan = plan(
        json!({"model.x": {"min": 1, "max": 3}, "model.y": [10, 20]}),
        json!({"samples_per_axis": 3.0, "max_axes": 2.0, "max_cases": 6.0}),
    )
    .unwrap();
    let plan: Value = serde_json::from_str(&serde_json::to_string(&plan).unwrap()).unwrap();
    let expansion = materialize(plan, Value::Null).unwrap();
    assert_eq!(expansion["config"]["max_cases"].as_u64(), Some(6));
    assert_eq!(expansion["sweep_budget"]["case_count_estimate"], 6);
    let expansion: Value =
        serde_json::from_str(&serde_json::to_string(&expansion).unwrap()).unwrap();
    let result = expand(expansion, Value::Null).unwrap();
    assert_eq!(result["case_count"], 6);
    for (index, (x, y)) in [
        (1.0, 10),
        (1.0, 20),
        (2.0, 10),
        (2.0, 20),
        (3.0, 10),
        (3.0, 20),
    ]
    .iter()
    .enumerate()
    {
        assert_eq!(
            result["cases"][index]["model"]["model"],
            json!({"x": x, "y": y})
        );
        assert_eq!(
            result["cases"][index]["metadata"]["sweep_budget"]["max_cases"],
            6
        );
    }
}

fn plan(space: Value, config: Value) -> Result<Value, String> {
    run_transform_operator(
        "transform.build_quality_parameter_sweep_plan",
        json!({
            "action": "continue", "request_payload": {"search_space": space},
            "base": {"model": {"x": 0}}
        }),
        config,
    )
}

fn materialize(plan: Value, config: Value) -> Result<Value, String> {
    run_transform_operator(
        "transform.materialize_quality_sweep_expansion",
        plan,
        config,
    )
}

fn expand(payload: Value, config: Value) -> Result<Value, String> {
    run_transform_operator("transform.expand_parameter_sweep", payload, config)
}

fn direct() -> Value {
    json!({"base": {"model": {"x": 0}}, "axes": [{"path": "model.x", "values": [1, 2, 3]}]})
}

fn assert_budget_block(result: Value) {
    assert_eq!(result["case_count"], 0);
    assert_eq!(result["expansion_budget_ready"], false);
    assert_eq!(result["expansion_blocking_reason"], "case_budget_exceeded");
}
