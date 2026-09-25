use crate::workflow_executor::run_extract_operator;
use serde_json::{Value, json};

#[derive(Clone, Copy)]
struct Case {
    domain: &'static str,
    scalar: &'static str,
    scalar_config: &'static str,
    vector: &'static str,
    x: &'static str,
    y: &'static str,
    magnitude: &'static str,
}

const CASES: [Case; 3] = [
    Case {
        domain: "thermal",
        scalar: "temperature",
        scalar_config: "temperature_field",
        vector: "flux",
        x: "heat_flux_x",
        y: "heat_flux_y",
        magnitude: "heat_flux_magnitude",
    },
    Case {
        domain: "electrostatic",
        scalar: "potential",
        scalar_config: "potential_field",
        vector: "field",
        x: "electric_field_x",
        y: "electric_field_y",
        magnitude: "electric_field_magnitude",
    },
    Case {
        domain: "magnetostatic",
        scalar: "vector_potential",
        scalar_config: "vector_potential_field",
        vector: "field",
        x: "magnetic_field_strength_x",
        y: "magnetic_field_strength_y",
        magnitude: "magnetic_field_strength_magnitude",
    },
];

impl Case {
    fn payload(self) -> Value {
        json!({
            "nodes":[{"id":"n0",self.scalar:1.0},{"id":"n1",self.scalar:3.0}],
            "elements":[{"id":"e0",self.x:3.0,self.y:4.0},{"id":"e1",self.x:5.0,self.y:12.0}]
        })
    }

    fn run(self, payload: Value, config: Value) -> Result<Value, String> {
        run_extract_operator(
            &format!("extract.{}_result_diagnostics", self.domain),
            payload,
            config,
        )
    }

    fn rejects(self, payload: Value, config: Value, path: &str) {
        let error = self.run(payload, config).expect_err(self.domain);
        assert!(
            error.contains(path),
            "{}: {error}, expected {path}",
            self.domain
        );
    }
}

#[test]
fn metadata_and_empty_collections_are_not_measurement_evidence() {
    for case in CASES {
        for payload in [
            json!({"nodes":[],"elements":[]}),
            json!({"nodes":[{}],"elements":[{}]}),
        ] {
            case.rejects(payload, json!({}), "did not find any diagnostic fields");
        }
    }
}

#[test]
fn every_record_must_be_an_object_even_when_other_metrics_are_available() {
    for case in CASES {
        for source in ["nodes", "elements"] {
            for invalid in [Value::Null, json!(false), json!(3), json!([])] {
                let mut payload = case.payload();
                payload[source][1] = invalid;
                case.rejects(payload, json!({}), &format!("payload.{source}[1]"));
            }
        }
    }
}

#[test]
fn diagnostic_reduction_cannot_erase_explicit_solver_failure() {
    for case in CASES {
        for marker in [json!(false), json!("true"), Value::Null] {
            let mut payload = case.payload();
            payload["converged"] = marker;
            case.rejects(payload, json!({}), "payload.converged");
        }
        let mut payload = case.payload();
        payload["stability_result"] = json!({"converged":false});
        case.rejects(payload, json!({}), "payload.stability_result.converged");
    }
}

#[test]
fn invalid_scalar_samples_cannot_be_filtered_out() {
    for case in CASES {
        for invalid in [Value::Null, json!("3"), json!(false), json!([]), json!({})] {
            let mut payload = case.payload();
            payload["nodes"][1][case.scalar] = invalid;
            case.rejects(
                payload,
                json!({}),
                &format!("payload.nodes[1].{}", case.scalar),
            );
        }
    }
}

#[test]
fn incomplete_scalar_groups_do_not_shrink_the_sample_count() {
    for case in CASES {
        for index in [0, 1] {
            let mut payload = case.payload();
            payload["nodes"][index]
                .as_object_mut()
                .unwrap()
                .remove(case.scalar);
            case.rejects(
                payload,
                json!({}),
                &format!("payload.nodes[{index}].{}", case.scalar),
            );
        }
    }
}

#[test]
fn corrupt_canonical_values_cannot_hide_behind_valid_aliases() {
    for (case, alias) in [(CASES[1], "phi"), (CASES[2], "a")] {
        let mut payload = case.payload();
        payload["nodes"][1][case.scalar] = Value::Null;
        payload["nodes"][1][alias] = json!(3.0);
        case.rejects(
            payload,
            json!({}),
            &format!("payload.nodes[1].{}", case.scalar),
        );
    }
}

#[test]
fn mixed_energy_aliases_are_resolved_per_element_before_selecting_a_peak() {
    for (case, canonical, alias) in [
        (CASES[1], "energy_density", "electric_energy_density"),
        (CASES[2], "energy_area_density", "energy_density"),
        (CASES[2], "energy_density", "magnetic_energy_density"),
    ] {
        let mut payload = case.payload();
        payload["elements"][0][canonical] = json!(1.0);
        payload["elements"][1][alias] = json!(50.0);
        let result = case.run(payload, json!({})).unwrap();
        assert_eq!(result[format!("{}_energy_density_peak", case.domain)], 50.0);
        assert_eq!(
            result[format!("{}_energy_density_peak_element_id", case.domain)],
            "e1"
        );
    }
}

#[test]
fn corrupt_energy_aliases_and_missing_energy_rows_are_rejected() {
    for (case, field, alias) in [
        (CASES[1], "energy_density", "electric_energy_density"),
        (CASES[2], "energy_area_density", "stored_energy"),
    ] {
        let mut payload = case.payload();
        payload["elements"][0][field] = json!(1.0);
        payload["elements"][1][field] = Value::Null;
        payload["elements"][1][alias] = json!(10.0);
        case.rejects(
            payload.clone(),
            json!({}),
            &format!("payload.elements[1].{field}"),
        );
        payload["elements"][1]
            .as_object_mut()
            .unwrap()
            .remove(field);
        payload["elements"][1]
            .as_object_mut()
            .unwrap()
            .remove(alias);
        case.rejects(payload, json!({}), "payload.elements[1]");
    }
}

#[test]
fn invalid_vector_magnitude_cannot_fall_back_to_healthy_components() {
    for case in CASES {
        for invalid in [Value::Null, json!("13"), json!(-1.0)] {
            let mut payload = case.payload();
            payload["elements"][1][case.magnitude] = invalid;
            case.rejects(
                payload,
                json!({}),
                &format!("payload.elements[1].{}", case.magnitude),
            );
        }
    }
}

#[test]
fn partially_missing_vectors_cannot_be_reduced_as_complete_evidence() {
    for case in CASES {
        let mut payload = case.payload();
        payload["elements"][1]
            .as_object_mut()
            .unwrap()
            .remove(case.y);
        case.rejects(
            payload,
            json!({}),
            &format!("payload.elements[1].{}", case.y),
        );
        let mut payload = case.payload();
        payload["elements"][1] = json!({"id":"missing"});
        case.rejects(payload, json!({}), "payload.elements[1]");
    }
}

#[test]
fn explicit_configuration_is_not_silently_replaced_with_defaults() {
    for case in CASES {
        for config in [json!([]), json!(false), json!("defaults")] {
            case.rejects(case.payload(), config, "config");
        }
        for key in [
            "node_source",
            "element_source",
            "output_prefix",
            case.scalar_config,
        ] {
            for invalid in [Value::Null, json!(false), json!(" ")] {
                case.rejects(
                    case.payload(),
                    json!({key:invalid}),
                    &format!("config.{key}"),
                );
            }
        }
        case.rejects(
            case.payload(),
            json!({"output_prefix":"---"}),
            "config.output_prefix",
        );
    }
}

#[test]
fn explicit_missing_field_mappings_fail_instead_of_disappearing() {
    for case in CASES {
        case.rejects(
            case.payload(),
            json!({case.scalar_config:"requested"}),
            "payload.nodes[0].requested",
        );
        let magnitude_config = format!("{}_magnitude_field", case.vector);
        case.rejects(
            case.payload(),
            json!({magnitude_config:"requested"}),
            "payload.elements[0].requested",
        );
    }
}

#[test]
fn overflowing_distribution_outputs_are_not_serialized_as_successful_nulls() {
    for case in CASES {
        let mut payload = case.payload();
        payload["nodes"][0][case.scalar] = json!(1e308);
        payload["nodes"][1][case.scalar] = json!(1e308);
        case.rejects(
            payload,
            json!({}),
            &format!("{}_{}_sum", case.domain, case.scalar),
        );
        let mut payload = case.payload();
        payload["nodes"][0][case.scalar] = json!(-1e308);
        payload["nodes"][1][case.scalar] = json!(1e308);
        case.rejects(
            payload,
            json!({}),
            &format!("{}_{}_span", case.domain, case.scalar),
        );
    }
}

#[test]
fn vector_norms_preserve_representable_large_and_small_values() {
    for case in CASES {
        for scale in [1e200, 1e-200] {
            let mut payload = case.payload();
            for index in [0, 1] {
                payload["elements"][index][case.x] = json!(3.0 * scale);
                payload["elements"][index][case.y] = json!(4.0 * scale);
            }
            let result = case.run(payload, json!({})).unwrap();
            let norm = result[format!("{}_{}_peak_magnitude", case.domain, case.vector)]
                .as_f64()
                .unwrap();
            assert!((norm / (5.0 * scale) - 1.0).abs() < 1e-14, "{norm}");
        }
    }
}

#[test]
fn unrepresentable_vector_norms_fail_before_publishing_a_peak() {
    for case in CASES {
        let mut payload = case.payload();
        payload["elements"][1][case.x] = json!(f64::MAX);
        payload["elements"][1][case.y] = json!(f64::MAX);
        case.rejects(payload, json!({}), "payload.elements[1]");
    }
}

#[test]
fn explicit_third_components_are_required_and_included_in_norms() {
    for case in CASES {
        let config = json!({format!("{}_z_field", case.vector):"z"});
        case.rejects(case.payload(), config.clone(), "payload.elements[0].z");
        let mut payload = case.payload();
        for index in [0, 1] {
            payload["elements"][index][case.x] = json!(3.0);
            payload["elements"][index][case.y] = json!(4.0);
            payload["elements"][index]["z"] = json!(12.0);
        }
        let result = case.run(payload, config).unwrap();
        assert_eq!(
            result[format!("{}_{}_peak_magnitude", case.domain, case.vector)],
            13.0
        );
        assert_eq!(
            result[format!("{}_{}_peak_z", case.domain, case.vector)],
            12.0
        );
    }
}

#[test]
fn absent_optional_groups_and_magnitude_only_records_remain_supported() {
    for case in CASES {
        let payload = json!({"nodes":[],"elements":[{"id":"only",case.magnitude:7.0}]});
        let result = case.run(payload, Value::Null).unwrap();
        assert_eq!(
            result[format!("{}_{}_peak_magnitude", case.domain, case.vector)],
            7.0
        );
        assert!(
            result
                .get(format!("{}_{}_mean", case.domain, case.scalar))
                .is_none()
        );
    }
}

#[test]
fn complete_scalar_only_evidence_does_not_invent_unavailable_vectors() {
    for case in CASES {
        let mut payload = case.payload();
        payload["elements"] = json!([]);
        let result = case.run(payload, json!({})).unwrap();
        assert_eq!(result[format!("{}_{}_count", case.domain, case.scalar)], 2);
        assert_eq!(result[format!("{}_{}_mean", case.domain, case.scalar)], 2.0);
        assert!(
            result
                .get(format!("{}_{}_peak_magnitude", case.domain, case.vector))
                .is_none()
        );
    }
}

#[test]
fn custom_sources_fields_prefix_and_last_peak_ties_remain_supported() {
    for case in CASES {
        let result = case.run(
            json!({"points":[{"value":-3.0},{"value":1.0}],"cells":[{"id":"a","x":3.0,"y":4.0},{"id":"b","x":3.0,"y":4.0}]}),
            json!({"node_source":"points","element_source":"cells","output_prefix":"sample A",
                case.scalar_config:"value",format!("{}_x_field",case.vector):"x",format!("{}_y_field",case.vector):"y"}),
        ).unwrap();
        assert_eq!(result["diagnostic_prefix"], "sample_A");
        assert_eq!(result[format!("sample_A_{}_min", case.scalar)], -3.0);
        assert_eq!(result[format!("sample_A_{}_sum", case.scalar)], -2.0);
        assert_eq!(
            result[format!("sample_A_{}_peak_element_id", case.vector)],
            "b"
        );
    }
}

#[test]
fn unused_aliases_and_unconfigured_dimensions_do_not_override_selected_evidence() {
    for case in CASES {
        let mut payload = case.payload();
        payload["elements"][0][case.magnitude] = json!(20.0);
        payload["elements"][0]["unselected_z"] = Value::Null;
        payload["nodes"][0]["phi"] = Value::Null;
        payload["nodes"][0]["a"] = Value::Null;
        let result = case.run(payload, json!({})).unwrap();
        assert_eq!(
            result[format!("{}_{}_peak_magnitude", case.domain, case.vector)],
            20.0
        );
    }
}

#[test]
fn scalar_energy_peaks_do_not_require_an_unused_representable_sum() {
    for case in [CASES[1], CASES[2]] {
        let mut payload = case.payload();
        for index in [0, 1] {
            payload["elements"][index]["energy_density"] = json!(1e308);
        }
        let result = case.run(payload, json!({})).unwrap();
        assert_eq!(
            result[format!("{}_energy_density_peak", case.domain)],
            1e308
        );
        assert_eq!(
            result[format!("{}_energy_density_peak_element_id", case.domain)],
            "e1"
        );
    }
}

#[test]
fn magnetic_total_energy_fallback_cannot_fill_partial_density_evidence() {
    let case = CASES[2];
    let mut payload = case.payload();
    payload["elements"][0]["energy_area_density"] = json!(1.0);
    payload["elements"][1]["stored_energy"] = json!(100.0);
    case.rejects(
        payload,
        json!({}),
        "payload.elements[1].energy_area_density",
    );
}

#[test]
fn tail_corruption_is_not_hidden_by_a_large_healthy_prefix() {
    for case in CASES {
        let mut payload = case.payload();
        payload["nodes"] = json!(vec![json!({case.scalar:1.0}); 4096]);
        payload["nodes"][4095][case.scalar] = Value::Null;
        case.rejects(
            payload,
            json!({}),
            &format!("payload.nodes[4095].{}", case.scalar),
        );
    }
}

#[test]
fn secondary_metric_groups_validate_every_consumed_sample() {
    for (case, field, alias) in [
        (CASES[0], "heat_load", "heat_load"),
        (CASES[1], "charge_density", "rho_e"),
        (CASES[2], "current_density", "j"),
    ] {
        let mut payload = case.payload();
        for index in [0, 1] {
            payload["nodes"][index][alias] = json!(1.0);
        }
        assert!(case.run(payload.clone(), json!({})).is_ok());
        payload["nodes"][1][field] = Value::Null;
        case.rejects(payload, json!({}), &format!("payload.nodes[1].{field}"));
    }
    for (case, x, y) in [
        (CASES[0], "temperature_gradient_x", "temperature_gradient_y"),
        (
            CASES[2],
            "magnetic_flux_density_x",
            "magnetic_flux_density_y",
        ),
    ] {
        let mut payload = case.payload();
        for index in [0, 1] {
            payload["elements"][index][x] = json!(3.0);
            payload["elements"][index][y] = json!(4.0);
        }
        assert!(case.run(payload.clone(), json!({})).is_ok());
        payload["elements"][1].as_object_mut().unwrap().remove(y);
        case.rejects(payload, json!({}), &format!("payload.elements[1].{y}"));
    }
}

#[test]
fn explicit_mappings_never_silently_revert_to_default_aliases() {
    for (case, alias) in [(CASES[1], "phi"), (CASES[2], "a")] {
        let mut payload = case.payload();
        for index in [0, 1] {
            payload["nodes"][index]
                .as_object_mut()
                .unwrap()
                .remove(case.scalar);
            payload["nodes"][index][alias] = json!(2.0);
        }
        assert!(case.run(payload.clone(), json!({})).is_ok());
        case.rejects(
            payload,
            json!({case.scalar_config:"requested"}),
            "payload.nodes[0].requested",
        );
    }
}

#[test]
fn complete_reductions_match_independent_distribution_and_peak_references() {
    for case in CASES {
        for count in [1, 2, 3, 64, 257] {
            let values = (0..count).map(|i| i as f64 - 2.0).collect::<Vec<_>>();
            let payload = json!({
                "nodes":values.iter().map(|v| json!({case.scalar:v})).collect::<Vec<_>>(),
                "elements":[{"id":"peak",case.x:3.0,case.y:4.0}]
            });
            let result = case.run(payload, json!({})).unwrap();
            let field = format!("{}_{}", case.domain, case.scalar);
            assert_eq!(result[format!("{field}_count")], count);
            assert_eq!(result[format!("{field}_sum")], values.iter().sum::<f64>());
            assert_eq!(result[format!("{field}_min")], values[0]);
            assert_eq!(result[format!("{field}_max")], values[count - 1]);
            assert_eq!(
                result[format!("{field}_mean")],
                values.iter().sum::<f64>() / count as f64
            );
        }
    }
}
