use crate::workflow_executor::run_transform_operator;
use serde_json::{Value, json};

const DOMAINS: &[(&str, &str, &str)] = &[
    ("structural", "max_displacement", "max_stress"),
    (
        "thermal",
        "thermal_temperature_max",
        "thermal_flux_peak_magnitude",
    ),
    (
        "acoustic",
        "max_sound_pressure_level_db",
        "max_acoustic_intensity",
    ),
    ("modal", "total_mass", "frequency_span_hz"),
    ("dynamic", "max_displacement", "max_force"),
    (
        "electrostatic",
        "electrostatic_field_peak_magnitude",
        "electrostatic_peak_energy_density",
    ),
    (
        "magnetostatic",
        "magnetostatic_field_peak_magnitude",
        "magnetostatic_flux_peak_magnitude",
    ),
    (
        "cfd",
        "cfd_divergence_error_peak",
        "cfd_reynolds_number_peak",
    ),
    (
        "transport",
        "transport_total_flux_peak_magnitude",
        "transport_peclet_peak",
    ),
];

fn score(domain: &str, payload: Value, config: Value) -> Result<Value, String> {
    run_transform_operator(
        &format!("transform.score_{domain}_quality"),
        payload,
        config,
    )
}

fn config(field: &str) -> Value {
    json!({"enabled_terms":[field], "targets":{field:1.0}, "weights":{field:1.0}})
}

fn reject(domain: &str, payload: Value, config: Value, path: &str) {
    let error = score(domain, payload, config).expect_err("invalid evidence must not be scored");
    assert!(
        error.contains(&format!("transform.score_{domain}_quality")),
        "{error}"
    );
    assert!(error.contains(path), "{error}");
}

#[test]
fn malformed_config_containers_cannot_activate_defaults() {
    for &(domain, field, _) in DOMAINS {
        for config in [json!([]), json!(false), json!("default"), json!(0)] {
            reject(domain, json!({field:1.0}), config, "config");
        }
    }
}

#[test]
fn explicit_empty_or_malformed_term_lists_do_not_fall_back() {
    for &(domain, field, _) in DOMAINS {
        for terms in [
            Value::Null,
            json!([]),
            json!(field),
            json!(false),
            json!({}),
        ] {
            reject(
                domain,
                json!({field:1.0}),
                json!({"enabled_terms":terms}),
                "config.enabled_terms",
            );
        }
    }
}

#[test]
fn unknown_or_malformed_terms_cannot_be_silently_dropped() {
    for &(domain, field, _) in DOMAINS {
        for term in [
            json!("typo_metric"),
            Value::Null,
            json!(7),
            json!(" "),
            json!({}),
        ] {
            reject(
                domain,
                json!({field:1.0}),
                json!({"enabled_terms":[field, term]}),
                "config.enabled_terms[1]",
            );
        }
    }
}

#[test]
fn repeated_terms_cannot_double_count_a_metric() {
    for &(domain, field, _) in DOMAINS {
        reject(
            domain,
            json!({field:1.0}),
            json!({"enabled_terms":[field, field]}),
            "config.enabled_terms[1]",
        );
    }
}

#[test]
fn target_and_weight_maps_must_be_objects_when_present() {
    for &(domain, field, _) in DOMAINS {
        for group in ["targets", "weights"] {
            for value in [Value::Null, json!([]), json!(1), json!("default")] {
                let mut config = config(field);
                config[group] = value;
                reject(
                    domain,
                    json!({field:1.0}),
                    config,
                    &format!("config.{group}"),
                );
            }
        }
    }
}

#[test]
fn invalid_targets_cannot_be_clamped_or_defaulted() {
    for &(domain, field, _) in DOMAINS {
        for value in [
            Value::Null,
            json!("1"),
            json!(false),
            json!(-1.0),
            json!(0.0),
        ] {
            let mut config = config(field);
            config["targets"][field] = value;
            reject(
                domain,
                json!({field:1.0}),
                config,
                &format!("config.targets.{field}"),
            );
        }
    }
}

#[test]
fn invalid_weights_cannot_be_clamped_or_defaulted() {
    for &(domain, field, _) in DOMAINS {
        for value in [Value::Null, json!("1"), json!(false), json!(-1.0)] {
            let mut config = config(field);
            config["weights"][field] = value;
            reject(
                domain,
                json!({field:1.0}),
                config,
                &format!("config.weights.{field}"),
            );
        }
    }
}

#[test]
fn override_typos_and_invalid_inactive_overrides_are_rejected() {
    for &(domain, field, other) in DOMAINS {
        for group in ["targets", "weights"] {
            for (key, value) in [("typo_metric", json!(1.0)), (other, Value::Null)] {
                let mut config = config(field);
                config[group][key] = value;
                reject(
                    domain,
                    json!({field:1.0}),
                    config,
                    &format!("config.{group}.{key}"),
                );
            }
        }
    }
}

#[test]
fn invalid_ready_thresholds_cannot_use_the_default() {
    for &(domain, field, _) in DOMAINS {
        for value in [Value::Null, json!("8"), json!(false), json!(-1.0)] {
            let mut config = config(field);
            config["max_ready_score"] = value;
            reject(domain, json!({field:1.0}), config, "config.max_ready_score");
        }
    }
}

#[test]
fn explicit_invalid_metrics_are_not_treated_as_absent() {
    for &(domain, field, _) in DOMAINS {
        for value in [Value::Null, json!("1"), json!(false), json!([]), json!({})] {
            reject(
                domain,
                json!({field:value}),
                config(field),
                &format!("payload.{field}"),
            );
        }
    }
}

#[test]
fn invalid_canonical_metric_cannot_hide_behind_a_healthy_alias() {
    for value in [Value::Null, json!("bad"), json!(false)] {
        reject(
            "structural",
            json!({"max_displacement":value, "peak_displacement":0.001}),
            config("max_displacement"),
            "payload.max_displacement",
        );
    }
}

#[test]
fn overflowed_penalties_cannot_disappear_from_the_score() {
    for &(domain, field, _) in DOMAINS {
        let mut config = config(field);
        config["weights"][field] = json!(2.0);
        reject(
            domain,
            json!({field:1e308}),
            config,
            &format!("{field}.penalty"),
        );
    }
}

#[test]
fn overflowed_ratios_are_rejected_before_serialization() {
    for &(domain, field, _) in DOMAINS {
        let mut config = config(field);
        config["targets"][field] = json!(0.1);
        reject(
            domain,
            json!({field:1e308}),
            config,
            &format!("{field}.ratio"),
        );
    }
}

#[test]
fn finite_penalties_cannot_overflow_the_total() {
    for &(domain, field, other) in DOMAINS {
        reject(
            domain,
            json!({field:1e308, other:1e308}),
            json!({
                "enabled_terms":[field, other], "targets":{field:1.0, other:1.0},
                "weights":{field:1.0, other:1.0}
            }),
            "quality_score",
        );
    }
}

#[test]
fn nonfinite_derived_values_are_not_downgraded_to_missing() {
    reject(
        "cfd",
        json!({"cfd_velocity_min":-1e308,"cfd_velocity_max":1e308}),
        config("cfd_velocity_span"),
        "payload.cfd_velocity_span",
    );
}

#[test]
fn valid_small_positive_targets_are_used_without_a_hidden_floor() {
    for &(domain, field, _) in DOMAINS {
        let mut config = config(field);
        config["targets"][field] = json!(1e-15);
        let result = score(domain, json!({field:1e-15}), config).unwrap();
        assert_eq!(result[format!("{domain}_quality_score")], 1.0);
        assert_eq!(
            result[format!("{domain}_quality_terms")][0]["target"],
            1e-15
        );
    }
}

#[test]
fn missing_metrics_still_return_an_explicit_blocking_assessment() {
    for &(domain, field, _) in DOMAINS {
        let result = score(domain, json!({}), config(field)).unwrap();
        assert_eq!(result[format!("{domain}_quality_ready")], false);
        assert_eq!(result[format!("{domain}_quality_grade")], "block");
        assert_eq!(result[format!("{domain}_quality_missing_metric_count")], 1);
        assert_eq!(
            result[format!("{domain}_quality_blocking_terms")][0]["status"],
            "missing"
        );
    }
}

#[test]
fn zero_weights_remain_explicit_and_do_not_skip_metric_validation() {
    for &(domain, field, _) in DOMAINS {
        let mut config = config(field);
        config["weights"][field] = json!(0.0);
        config["targets"][field] = json!(0.1);
        config["max_ready_score"] = json!(0.0);
        let result = score(domain, json!({field:1e308}), config.clone()).unwrap();
        assert_eq!(result[format!("{domain}_quality_score")], 0.0);
        assert_eq!(result[format!("{domain}_quality_ready")], true);
        assert_eq!(result[format!("{domain}_quality_watch_count")], 1);
        let missing = score(domain, json!({}), config.clone()).unwrap();
        assert_eq!(missing[format!("{domain}_quality_ready")], false);
        reject(
            domain,
            json!({field:"bad"}),
            config,
            &format!("payload.{field}"),
        );
    }
}

#[test]
fn valid_aliases_optional_terms_and_null_config_remain_supported() {
    let direct = score(
        "structural",
        json!({"max_displacement":0.1}),
        config("max_displacement"),
    )
    .unwrap();
    let alias = score(
        "structural",
        json!({"peak_displacement":0.1}),
        config("max_displacement"),
    )
    .unwrap();
    assert_eq!(direct, alias);
    for (domain, field) in [
        ("thermal", "thermal_total_energy"),
        ("dynamic", "max_velocity"),
        ("electrostatic", "electrostatic_total_stored_energy"),
        ("magnetostatic", "magnetostatic_total_stored_energy"),
    ] {
        let result = score(domain, json!({field:0.5}), config(field)).unwrap();
        assert_eq!(result[format!("{domain}_quality_score")], 0.5);
        assert_eq!(result[format!("{domain}_quality_term_count")], 1);
    }
    for (domain, _, _) in DOMAINS {
        assert_eq!(
            score(domain, json!({}), Value::Null).unwrap(),
            score(domain, json!({}), json!({})).unwrap()
        );
    }
}

#[test]
fn valid_weighted_scores_grades_and_term_order_are_preserved() {
    for &(domain, field, other) in DOMAINS {
        let mut config = json!({
            "enabled_terms":[other, field], "targets":{field:2.0, other:2.0},
            "weights":{field:2.0, other:4.0}, "max_ready_score":8.0
        });
        for (value, expected_grade) in [
            (0.5, "excellent"),
            (1.0, "good"),
            (2.0, "review"),
            (3.0, "block"),
        ] {
            config["on_error"] = json!("skip");
            let result = score(domain, json!({field:value, other:value}), config.clone()).unwrap();
            assert_eq!(result[format!("{domain}_quality_score")], 3.0 * value);
            assert_eq!(result[format!("{domain}_quality_grade")], expected_grade);
            assert_eq!(result[format!("{domain}_quality_terms")][0]["field"], other);
            assert_eq!(
                result[format!("{domain}_quality_dominant_term")]["field"],
                other
            );
        }
    }
}

#[test]
fn inverse_goal_scoring_preserves_finite_results_and_rejects_overflow() {
    for (domain, field) in [
        ("structural", "stiffness_margin"),
        ("modal", "min_frequency_hz"),
        ("dynamic", "peak_frequency_hz"),
        ("acoustic", "total_damping_loss"),
        ("electrostatic", "electrostatic_potential_span"),
    ] {
        let mut config = config(field);
        config["targets"][field] = json!(20.0);
        config["weights"][field] = json!(2.0);
        let result = score(domain, json!({field:40.0}), config.clone()).unwrap();
        assert_eq!(result[format!("{domain}_quality_score")], 1.0);
        assert_eq!(result[format!("{domain}_quality_terms")][0]["goal"], "max");
        config["targets"][field] = json!(1e308);
        reject(
            domain,
            json!({field:1e-12}),
            config,
            &format!("{field}.ratio"),
        );
    }
}

#[test]
fn trimmed_selection_keeps_order_and_reusable_valid_overrides() {
    for &(domain, field, other) in DOMAINS {
        let mut config = config(field);
        config["enabled_terms"] = json!([format!(" {field} ")]);
        config["targets"][other] = json!(2.0);
        config["weights"][other] = json!(3.0);
        let result = score(domain, json!({field:1.0}), config.clone()).unwrap();
        assert_eq!(result[format!("{domain}_quality_term_count")], 1);
        assert_eq!(result[format!("{domain}_quality_score")], 1.0);
        config["enabled_terms"] = json!([field, format!(" {field} ")]);
        reject(
            domain,
            json!({field:1.0}),
            config,
            "config.enabled_terms[1]",
        );
    }
}
