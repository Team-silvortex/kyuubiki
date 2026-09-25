use crate::workflow_executor::run_extract_operator;
use serde_json::{Value, json};

const OPERATORS: [&str; 2] = ["extract.field_statistics", "extract.field_hotspots"];

fn payload() -> Value {
    json!({"nodes":[{"id":"a","v":1.0},{"id":"b","v":2.0},{"id":"c","v":3.0}]})
}

fn config() -> Value {
    json!({"source":"nodes","field":"v","threshold":1.0})
}

fn reject(operator: &str, input: Value, config: Value, path: &str) {
    let error = run_extract_operator(operator, input, config).unwrap_err();
    assert!(error.contains(path), "expected {path}, got {error}");
}

#[test]
fn field_reductions_do_not_filter_malformed_or_missing_samples() {
    for operator in OPERATORS {
        for index in [0, 2] {
            for bad in [
                Value::Null,
                json!(false),
                json!({}),
                json!({"v":null}),
                json!({"v":"3"}),
            ] {
                let mut input = payload();
                input["nodes"][index] = bad;
                reject(
                    operator,
                    input,
                    config(),
                    &format!("payload.nodes[{index}]"),
                );
            }
        }
    }
}

#[test]
fn selected_field_collections_must_exist_and_contain_samples() {
    for operator in OPERATORS {
        for source in [Value::Null, json!({}), json!([])] {
            reject(operator, json!({"nodes":source}), config(), "payload.nodes");
        }
    }
}

#[test]
fn generic_field_reduction_cannot_erase_nonconvergence() {
    for operator in OPERATORS {
        for marker in [json!(false), json!("true"), Value::Null] {
            let mut input = payload();
            input["converged"] = marker;
            reject(operator, input, config(), "payload.converged");
        }
    }
}

#[test]
fn field_configuration_cannot_silently_select_defaults() {
    for operator in OPERATORS {
        for key in ["source", "field", "output_prefix"] {
            for bad in [Value::Null, json!(true), json!(" ")] {
                let mut config = config();
                config[key] = bad;
                reject(operator, payload(), config, &format!("config.{key}"));
            }
        }
        reject(operator, payload(), json!([]), "config");
    }
}

#[test]
fn requested_percentiles_are_all_valid_and_unique() {
    for bad in [
        Value::Null,
        json!(90),
        json!([50, "90"]),
        json!([null]),
        json!([-1]),
        json!([101]),
        json!([50, 50]),
        json!([0, -0.0]),
    ] {
        let mut config = config();
        config["percentiles"] = bad;
        reject(OPERATORS[0], payload(), config, "config.percentiles");
    }
}

#[test]
fn hotspot_threshold_and_percentile_cannot_fall_back_on_invalid_options() {
    for bad in [Value::Null, json!("2"), json!(false)] {
        let mut config = config();
        config["threshold"] = bad;
        reject(OPERATORS[1], payload(), config, "config.threshold");
    }
    for bad in [Value::Null, json!("90"), json!(-1), json!(101)] {
        let mut config = config();
        config["percentile"] = bad;
        reject(OPERATORS[1], payload(), config, "config.percentile");
    }
}

#[test]
fn hotspot_sampling_options_are_not_silently_replaced() {
    for bad in [Value::Null, json!(-1), json!(1.5), json!("2"), json!(false)] {
        let mut config = config();
        config["sample_limit"] = bad;
        reject(OPERATORS[1], payload(), config, "config.sample_limit");
    }
    for bad in [Value::Null, json!(false), json!(" "), json!("descending")] {
        let mut config = config();
        config["sample_sort"] = bad;
        reject(OPERATORS[1], payload(), config, "config.sample_sort");
    }
}

#[test]
fn overflowing_exported_field_sum_is_not_successful_json_null() {
    reject(
        OPERATORS[0],
        json!({"nodes":[{"v":1e308},{"v":1e308}]}),
        config(),
        "v_sum",
    );
}

#[test]
fn population_stddev_preserves_representable_large_and_small_scales() {
    for scale in [1e200, 1e-200] {
        let result = run_extract_operator(
            OPERATORS[0],
            json!({"nodes":[{"v":-scale},{"v":scale}]}),
            config(),
        )
        .unwrap();
        let stddev = result["v_stddev"]
            .as_f64()
            .expect("finite standard deviation");
        assert!((stddev / scale - 1.0).abs() < 1e-14, "{result}");
    }
}

#[test]
fn stddev_does_not_require_unscaled_deviations_to_fit_the_numeric_range() {
    let result = run_extract_operator(
        OPERATORS[0],
        json!({"nodes":[{"v":f64::MAX},{"v":-f64::MAX},{"v":-f64::MAX}]}),
        config(),
    )
    .unwrap();
    let expected = f64::MAX * (2.0 * 2.0_f64.sqrt() / 3.0);
    assert!((result["v_stddev"].as_f64().unwrap() / expected - 1.0).abs() < 1e-14);
}

#[test]
fn percentile_interpolation_handles_endpoints_and_opposite_large_values() {
    let result = run_extract_operator(
        OPERATORS[0],
        json!({"nodes":[{"v":-1e308},{"v":1e308}]}),
        json!({"field":"v","percentiles":[0,25,50,75,100]}),
    )
    .unwrap();
    for (key, expected) in [
        ("p0", -1e308),
        ("p25", -5e307),
        ("p50", 0.0),
        ("p75", 5e307),
        ("p100", 1e308),
    ] {
        let actual = result[format!("v_{key}")].as_f64().unwrap();
        assert!((actual - expected).abs() <= 1e293, "{key}: {actual}");
    }
}

#[test]
fn field_statistics_match_independent_linear_series_reductions() {
    for count in [1, 2, 3, 64, 257] {
        let nodes = (1..=count)
            .rev()
            .map(|i| json!({"v":i}))
            .collect::<Vec<_>>();
        let result = run_extract_operator(
            OPERATORS[0],
            json!({"nodes":nodes}),
            json!({"field":"v","percentiles":[0,50,90,100]}),
        )
        .unwrap();
        assert_eq!(result["v_count"], count);
        assert_eq!(result["v_min"], 1.0);
        assert_eq!(result["v_max"], count as f64);
        assert_eq!(result["v_sum"], (count * (count + 1) / 2) as f64);
        assert_eq!(result["v_mean"], (count + 1) as f64 / 2.0);
        let expected = (((count * count - 1) as f64) / 12.0).sqrt();
        assert!((result["v_stddev"].as_f64().unwrap() - expected).abs() < 1e-12);
        assert!(
            (result["v_p90"].as_f64().unwrap() - (1.0 + 0.9 * (count - 1) as f64)).abs() < 1e-12
        );
    }
}

#[test]
fn hotspot_mean_does_not_need_an_unexported_sum() {
    let result = run_extract_operator(
        OPERATORS[1],
        json!({"nodes":[{"v":1e308},{"v":1e308}]}),
        config(),
    )
    .unwrap();
    assert_eq!(result["v_hotspot_mean"], 1e308);
    let result = run_extract_operator(
        OPERATORS[1],
        json!({"nodes":[{"v":f64::MAX},{"v":f64::MAX},{"v":-f64::MAX}]}),
        json!({"source":"nodes","field":"v","threshold":-f64::MAX}),
    )
    .unwrap();
    assert!((result["v_hotspot_mean"].as_f64().unwrap() / (f64::MAX / 3.0) - 1.0).abs() < 1e-14);
}

#[test]
fn hotspot_sort_ties_threshold_and_sampling_caps_preserve_the_contract() {
    let input = json!({"cells":[{"id":"a","v":2.0},{"id":"b","v":3.0},{"id":"c","v":3.0},{"id":"d","v":1.0}]});
    for (sort, ids) in [
        ("value_asc", json!(["a", "b", "c"])),
        ("value_desc", json!(["b", "c", "a"])),
    ] {
        let result = run_extract_operator(OPERATORS[1], input.clone(), json!({"source":"cells","field":"v","output_prefix":"custom.value","threshold":2.0,"sample_sort":sort,"sample_limit":2})).unwrap();
        assert_eq!(result["custom.value_hotspot_ids"], ids);
        assert_eq!(result["custom.value_hotspot_fraction"], 0.75);
        assert_eq!(result["custom.value_hotspot_count"], 3);
        assert_eq!(
            result["custom.value_hotspot_samples"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
    }
    let nodes = (0..100)
        .map(|i| json!({"id":i,"v":1.0}))
        .collect::<Vec<_>>();
    for (limit, expected) in [(0, 0), (1000, 32)] {
        let result = run_extract_operator(
            OPERATORS[1],
            json!({"nodes":nodes}),
            json!({"source":"nodes","field":"v","threshold":1.0,"sample_limit":limit}),
        )
        .unwrap();
        assert_eq!(result["v_hotspot_ids"].as_array().unwrap().len(), 100);
        assert_eq!(
            result["v_hotspot_samples"].as_array().unwrap().len(),
            expected
        );
    }
}

#[test]
fn absolute_threshold_retains_precedence_and_no_matches_remains_an_error() {
    let result = run_extract_operator(
        OPERATORS[1],
        payload(),
        json!({"source":"nodes","field":"v","threshold":1.0,"percentile":100}),
    )
    .unwrap();
    assert_eq!(result["v_hotspot_count"], 3);
    reject(
        OPERATORS[1],
        payload(),
        json!({"source":"nodes","field":"v","threshold":4.0}),
        "did not find any values meeting",
    );
}

#[test]
fn corrupt_tail_cannot_disappear_below_an_absolute_hotspot_threshold() {
    for operator in OPERATORS {
        let mut nodes = vec![json!({"v":2.0}); 4096];
        nodes[4095] = json!({"v":null});
        reject(
            operator,
            json!({"nodes":nodes}),
            config(),
            "payload.nodes[4095].v",
        );
    }
}
