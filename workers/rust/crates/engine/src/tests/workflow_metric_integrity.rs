use crate::workflow_executor::{run_extract_operator, run_transform_operator};
use serde_json::{Value, json};

fn quality(domain: &str, field: &str, payload: Value) -> Result<Value, String> {
    run_transform_operator(
        &format!("transform.score_{domain}_quality"),
        payload,
        json!({"enabled_terms":[field]}),
    )
}

fn rejected(result: Result<Value, String>, path: &str) {
    let error = result.expect_err("corrupt samples must not produce partial successful evidence");
    assert!(error.contains(path), "{error}");
}

#[test]
fn corrupt_first_alias_cannot_fall_through_to_a_healthy_later_alias() {
    for (domain, field, first, next) in [
        ("structural", "max_stress", "peak_stress", "von_mises_peak"),
        (
            "thermal",
            "thermal_temperature_max",
            "max_temperature",
            "temperature_max",
        ),
        (
            "acoustic",
            "max_sound_pressure_level_db",
            "peak_spl_db",
            "spl_max_db",
        ),
        (
            "modal",
            "min_frequency_hz",
            "first_frequency_hz",
            "natural_frequency_min_hz",
        ),
        (
            "dynamic",
            "max_displacement",
            "peak_displacement",
            "displacement_amplitude_peak",
        ),
        (
            "electrostatic",
            "electrostatic_field_peak_magnitude",
            "max_electric_field",
            "peak_electric_field",
        ),
        (
            "magnetostatic",
            "magnetostatic_flux_peak_magnitude",
            "max_flux_density",
            "peak_flux_density",
        ),
        (
            "transport",
            "transport_peclet_peak",
            "max_peclet",
            "peclet_max",
        ),
        (
            "cfd",
            "cfd_divergence_error_peak",
            "max_divergence_error",
            "divergence_peak",
        ),
    ] {
        for bad in [Value::Null, json!("bad"), json!(true), json!([])] {
            let payload = json!({first:bad,next:1.0});
            rejected(
                quality(domain, field, payload.clone()),
                &format!("payload.{first}"),
            );
            rejected(
                run_transform_operator(
                    &format!("transform.evaluate_{domain}_guard"),
                    payload.clone(),
                    json!({"rules":[{"field":field,"threshold":100.0}]}),
                ),
                &format!("payload.{first}"),
            );
            let pair_domain = if domain == "thermal" {
                "coupled_heat"
            } else {
                domain
            };
            rejected(
                run_transform_operator(
                    &format!("transform.benchmark_{pair_domain}_pair"),
                    json!({"left":payload,"right":{field:2.0}}),
                    json!({"criteria":[{"field":field}]}),
                ),
                &format!("payload.left.{first}"),
            );
        }
    }
}

#[test]
fn transient_node_reduction_rejects_every_bad_or_missing_sample() {
    for (field, component) in [
        ("max_displacement", "ux"),
        ("max_velocity", "vx"),
        ("max_acceleration", "ax"),
    ] {
        for bad in [Value::Null, json!("0"), json!(true)] {
            rejected(
                quality(
                    "dynamic",
                    field,
                    json!({"nodes":[{component:1.0},{component:bad}]}),
                ),
                &format!("payload.nodes[1].{component}"),
            );
        }
        rejected(
            quality("dynamic", field, json!({"nodes":[{component:1.0},{}]})),
            &format!("payload.nodes[1].{component}"),
        );
        for bad in [Value::Null, json!(false), json!("sample")] {
            rejected(
                quality("dynamic", field, json!({"nodes":[{component:1.0},bad]})),
                "payload.nodes[1]",
            );
        }
    }
}

#[test]
fn frequency_reduction_rejects_incomplete_rows_instead_of_lowering_peaks() {
    for (field, alias) in [
        ("max_displacement", "displacement_amplitude"),
        ("max_velocity", "velocity_amplitude"),
        ("max_acceleration", "acceleration_amplitude"),
        ("max_force", "force_amplitude"),
    ] {
        for row in [
            json!({alias:null}),
            json!({}),
            json!({alias:"bad"}),
            Value::Null,
        ] {
            rejected(
                quality("dynamic", field, json!({"frequencies":[{alias:1.0},row]})),
                "payload.frequencies[1]",
            );
        }
    }
}

#[test]
fn peak_frequency_requires_frequency_and_displacement_from_every_row() {
    for row in [
        json!({"frequency_hz":20.0}),
        json!({"displacement_amplitude":100.0}),
        json!({"frequency_hz":null,"freq_hz":20.0,"displacement_amplitude":100.0}),
        json!({"frequency_hz":20.0,"max_displacement":null,"displacement_amplitude":100.0}),
    ] {
        rejected(
            quality(
                "dynamic",
                "peak_frequency_hz",
                json!({"frequencies":[
            {"frequency_hz":30.0,"max_displacement":1.0},row]}),
            ),
            "payload.frequencies[1]",
        );
    }
}

#[test]
fn malformed_frequency_source_cannot_fall_back_to_healthy_nodes() {
    for frequencies in [Value::Null, json!({}), json!(false), json!("bad")] {
        rejected(
            quality(
                "dynamic",
                "max_displacement",
                json!({"frequencies":frequencies,"nodes":[{"ux":1.0}]}),
            ),
            "payload.frequencies",
        );
    }
}

#[test]
fn modal_bounds_require_complete_frequency_evidence() {
    for field in ["min_frequency_hz", "frequency_span_hz"] {
        for row in [
            Value::Null,
            json!({}),
            json!({"frequency_hz":null}),
            json!({"frequency_hz":"bad"}),
        ] {
            rejected(
                quality("modal", field, json!({"modes":[{"frequency_hz":30.0},row]})),
                "payload.modes[1]",
            );
        }
    }
}

#[test]
fn malformed_modal_source_or_first_participation_is_not_missing_evidence() {
    for modes in [Value::Null, json!({}), json!(false)] {
        rejected(
            quality("modal", "min_frequency_hz", json!({"modes":modes})),
            "payload.modes",
        );
    }
    rejected(
        quality(
            "modal",
            "mode_1_participation_norm",
            json!({"modes":[{"participation_norm":null}]}),
        ),
        "payload.modes[0].participation_norm",
    );
}

#[test]
fn corrupt_bounds_cannot_use_later_aliases() {
    rejected(
        quality(
            "thermal",
            "thermo_temperature_delta_max",
            json!({
        "temperature_max":null,"max_temperature":30.0,"temperature_min":10.0}),
        ),
        "payload.temperature_max",
    );
}

#[test]
fn valid_selected_sources_do_not_validate_unselected_metadata() {
    let result = quality(
        "dynamic",
        "max_displacement",
        json!({
        "max_displacement":0.001,"peak_displacement":"ignored","frequencies":"unused"}),
    )
    .unwrap();
    assert_eq!(result["dynamic_quality_ready"], true);
    let result = quality(
        "dynamic",
        "max_velocity",
        json!({"nodes":[
        {"vx":0.1,"unused":null},{"vx":-0.2,"ux":"not selected"}]}),
    )
    .unwrap();
    assert_eq!(result["dynamic_quality_max_velocity"], 0.2);
}

#[test]
fn completely_absent_and_empty_sample_sources_remain_visibly_not_ready() {
    for payload in [json!({}), json!({"nodes":[]}), json!({"frequencies":[]})] {
        let result = quality("dynamic", "max_velocity", payload).unwrap();
        assert_eq!(result["dynamic_quality_ready"], false);
        assert_eq!(result["dynamic_quality_missing_metric_count"], 1);
    }
}

fn cfd() -> Value {
    json!({"nodes":[{"id":"a","vx":0.0,"vy":0.0,"p":1.0},{"id":"b","vx":3.0,"vy":4.0,"p":-3.0}],
        "elements":[{"id":"e0","div_u":0.01,"re":1.0,"dissipation":0.2},
                    {"id":"e1","div_u":0.02,"re":2.0,"dissipation":0.3}]})
}

fn extract(payload: Value) -> Result<Value, String> {
    run_extract_operator(
        "extract.stokes_flow_result_diagnostics",
        payload,
        Value::Null,
    )
}

#[test]
fn cfd_rejects_malformed_node_and_element_entries() {
    for collection in ["nodes", "elements"] {
        for row in [Value::Null, json!(false), json!(42)] {
            let mut payload = cfd();
            payload[collection][1] = row;
            rejected(extract(payload), &format!("payload.{collection}[1]"));
        }
    }
}

#[test]
fn cfd_rejects_corrupt_or_missing_fields_in_every_consumed_sample() {
    for (collection, field) in [
        ("nodes", "vx"),
        ("nodes", "vy"),
        ("nodes", "p"),
        ("elements", "div_u"),
        ("elements", "re"),
        ("elements", "dissipation"),
    ] {
        for bad in [Value::Null, json!("bad"), json!(false)] {
            let mut payload = cfd();
            payload[collection][1][field] = bad;
            rejected(extract(payload), &format!("payload.{collection}[1]"));
        }
        let mut payload = cfd();
        payload[collection][1]
            .as_object_mut()
            .unwrap()
            .remove(field);
        rejected(extract(payload), &format!("payload.{collection}[1]"));
    }
}

#[test]
fn cfd_empty_collections_cannot_invent_zero_measurements() {
    for collection in ["nodes", "elements"] {
        let mut payload = cfd();
        payload[collection] = json!([]);
        rejected(extract(payload), collection);
    }
}

#[test]
fn cfd_rejects_nonconverged_results_before_erasing_the_marker() {
    let mut payload = cfd();
    payload["converged"] = json!(false);
    rejected(extract(payload), "payload.converged");
}

#[test]
fn representable_velocity_norm_avoids_intermediate_square_overflow_and_underflow() {
    for scale in [1e308, 1e-300] {
        let mut payload = cfd();
        payload["nodes"][1]["vx"] = json!(0.3 * scale);
        payload["nodes"][1]["vy"] = json!(0.4 * scale);
        let summary = extract(payload).unwrap();
        let actual = summary["cfd_velocity_max"].as_f64().expect("finite norm");
        assert!((actual / (0.5 * scale) - 1.0).abs() < 1e-14, "{actual}");
    }
}

#[test]
fn cfd_rejects_unrepresentable_norm_span_and_total() {
    let mut norm = cfd();
    norm["nodes"][1]["vx"] = json!(f64::MAX);
    norm["nodes"][1]["vy"] = json!(f64::MAX);
    rejected(extract(norm), "payload.nodes[1]");
    let mut span = cfd();
    span["nodes"][0]["p"] = json!(-1e308);
    span["nodes"][1]["p"] = json!(1e308);
    rejected(extract(span), "cfd_pressure_span");
    let mut total = cfd();
    total["elements"][0]["dissipation"] = json!(1e308);
    total["elements"][1]["dissipation"] = json!(1e308);
    rejected(extract(total), "cfd_viscous_dissipation_total");
}

#[test]
fn representable_cfd_mean_does_not_require_a_representable_unused_sum() {
    let mut payload = cfd();
    for node in payload["nodes"].as_array_mut().unwrap() {
        node["p"] = json!(1e308);
    }
    let summary = extract(payload).unwrap();
    assert_eq!(summary["cfd_pressure_mean"], 1e308);
    assert_eq!(summary["cfd_pressure_span"], 0.0);
}

#[test]
fn valid_cfd_summary_preserves_peak_identity_and_signed_pressure_statistics() {
    let summary = extract(cfd()).unwrap();
    assert_eq!(summary["diagnostic_node_count"], 2);
    assert_eq!(summary["cfd_velocity_max"], 5.0);
    assert_eq!(summary["cfd_pressure_mean"], -1.0);
    assert_eq!(summary["cfd_pressure_span"], 4.0);
    assert_eq!(summary["cfd_divergence_error_peak_element_id"], "e1");
    assert_eq!(summary["cfd_viscous_dissipation_total"], 0.5);
}

#[test]
fn a_corrupt_tail_sample_is_not_hidden_by_a_healthy_prefix() {
    let mut nodes = vec![json!({"vx": 0.1}); 4096];
    nodes[4095]["vx"] = Value::Null;
    rejected(
        quality("dynamic", "max_velocity", json!({"nodes":nodes})),
        "payload.nodes[4095].vx",
    );
}

#[test]
fn empty_selected_frequency_table_does_not_change_to_a_different_source() {
    let result = quality(
        "dynamic",
        "max_velocity",
        json!({
            "frequencies":[], "nodes":[{"vx":0.1}]
        }),
    )
    .unwrap();
    assert_eq!(result["dynamic_quality_ready"], false);
    assert_eq!(result["dynamic_quality_missing_metric_count"], 1);
}

#[test]
fn streaming_cfd_statistics_match_independent_reductions() {
    for count in [1, 2, 3, 64, 257] {
        let nodes = (0..count)
            .map(|index| {
                let velocity = index as f64 * 0.125;
                json!({"id":index,"vx":velocity * 0.6,"vy":velocity * 0.8,"p":index as f64 - 17.0})
            })
            .collect::<Vec<_>>();
        let mut payload = cfd();
        payload["nodes"] = json!(nodes);
        let summary = run_extract_operator(
            "extract.stokes_flow_result_diagnostics",
            payload,
            json!({"output_prefix":"probe"}),
        )
        .unwrap();
        let expected_mean = (0..count).map(|i| i as f64 - 17.0).sum::<f64>() / count as f64;
        let actual = summary["probe_pressure_mean"].as_f64().unwrap();
        assert!((actual - expected_mean).abs() < 1e-12);
        assert_eq!(summary["probe_pressure_span"], (count - 1) as f64);
        let peak = summary["probe_velocity_max"].as_f64().unwrap();
        assert!((peak - (count - 1) as f64 * 0.125).abs() < 1e-12);
        assert_eq!(summary["diagnostic_node_count"], count);
    }
}

#[test]
fn cfd_peak_ties_keep_the_last_sample_and_its_signed_value() {
    let mut payload = cfd();
    payload["elements"][0]["div_u"] = json!(0.2);
    payload["elements"][1]["div_u"] = json!(-0.2);
    let summary = extract(payload).unwrap();
    assert_eq!(summary["cfd_divergence_error_peak"], -0.2);
    assert_eq!(summary["cfd_divergence_error_peak_element_id"], "e1");
}
