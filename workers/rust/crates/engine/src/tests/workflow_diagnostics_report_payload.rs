use crate::{
    workflow_bundle_transforms::{
        compose_diagnostics_bundle, compose_diagnostics_report_payload,
        evaluate_diagnostics_bundle_guard,
    },
    workflow_executor::run_transform_operator,
    workflow_focus_chain::{
        compose_focus_bridge_request, compose_focus_chain_input, execute_focus_bridge_execution,
        resolve_focus_bridge_execution, select_focus_payload,
    },
};

fn sample_bundle_and_guard() -> (serde_json::Value, serde_json::Value) {
    let bundle = compose_diagnostics_bundle(
        serde_json::json!({
            "thermal": {
                "diagnostic_contract": "kyuubiki.workflow_diagnostics/v1",
                "diagnostic_domain": "thermal",
                "diagnostic_subject": "thermal_result",
                "diagnostic_prefix": "thermal",
                "diagnostic_node_count": 4,
                "diagnostic_element_count": 1,
                "diagnostic_metric_groups": ["temperature", "flux"],
                "thermal_temperature_max": 125.0
            },
            "thermo": {
                "diagnostic_contract": "kyuubiki.workflow_diagnostics/v1",
                "diagnostic_domain": "thermo_mechanical",
                "diagnostic_subject": "thermo_result",
                "diagnostic_prefix": "thermo",
                "diagnostic_node_count": 4,
                "diagnostic_element_count": 1,
                "diagnostic_metric_groups": ["temperature_delta", "stress", "thermal_strain"],
                "thermo_temperature_delta_max": 125.0,
                "thermo_peak_stress": 220.0,
                "thermo_peak_thermal_strain": 0.00048,
                "peak_element_id": "te1",
                "peak_stress_x": 14.0,
                "peak_stress_y": 9.0,
                "peak_tau_xy": 3.0,
                "peak_element_temperature_delta": 35.0,
                "thermo_peak_thermal_strain_id": "te1"
            }
        }),
        serde_json::json!({}),
    )
    .expect("bundle should compose");
    let guard = evaluate_diagnostics_bundle_guard(
        bundle.clone(),
        serde_json::json!({
            "rules": [
                { "source": "thermal", "field": "thermal_temperature_max", "threshold": 120.0, "severity": "warn", "label": "thermal temperature" }
            ]
        }),
    )
    .expect("guard should evaluate");
    (bundle, guard)
}

#[test]
fn composes_diagnostics_report_payload_with_guard() {
    let (bundle, guard) = sample_bundle_and_guard();
    let report = compose_diagnostics_report_payload(
        serde_json::json!({
            "bundle": bundle,
            "guard": guard
        }),
        serde_json::json!({}),
    )
    .expect("report payload should compose");

    assert_eq!(
        report["report_contract"].as_str(),
        Some("kyuubiki.workflow_report_payload/v1")
    );
    assert_eq!(
        report["report_kind"].as_str(),
        Some("diagnostics_bundle_report_payload")
    );
    assert_eq!(report["report_guard_status"].as_str(), Some("warn"));
    assert_eq!(
        report["report_guard_recommendation"].as_str(),
        Some("review_before_continue")
    );
    assert_eq!(
        report["report_focus_metrics"]["thermal.temperature_max"].as_f64(),
        Some(125.0)
    );
    assert_eq!(
        report["report_focus_metrics"]["thermo.stress_peak"].as_f64(),
        Some(220.0)
    );
    assert_eq!(
        report["report_focus_context"]["thermo.stress_peak"]["peak_stress_x"].as_f64(),
        Some(14.0)
    );
    assert_eq!(
        report["report_focus_context"]["thermo.stress_peak"]["source"].as_str(),
        Some("thermo")
    );
    assert_eq!(
        report["report_focus_payloads"]["thermo.stress_peak"]["focus_contract"].as_str(),
        Some("kyuubiki.workflow_focus_payload/v1")
    );
    assert_eq!(
        report["report_focus_payloads"]["thermo.stress_peak"]["metric_id"].as_str(),
        Some("thermo.stress_peak")
    );
    assert_eq!(
        report["report_focus_payloads"]["thermo.stress_peak"]["value"].as_f64(),
        Some(220.0)
    );
    assert_eq!(
        report["report_focus_payloads"]["thermo.stress_peak"]["context"]["peak_stress_x"].as_f64(),
        Some(14.0)
    );
    let highlights = report["report_highlights"]
        .as_array()
        .expect("report highlights should be an array");
    assert!(highlights.iter().any(|item| {
        item["id"].as_str() == Some("thermal.temperature_max")
            && item["attention"].as_bool() == Some(true)
    }));
    assert!(highlights.iter().any(|item| {
        item["id"].as_str() == Some("thermo.stress_peak")
            && item["attention"].as_bool() == Some(false)
    }));
    assert!(report.get("guard_payload").is_some());
    assert!(report.get("bundle_items").is_some());
}

#[test]
fn composes_diagnostics_report_payload_with_pruned_fields() {
    let (bundle, guard) = sample_bundle_and_guard();
    let report = compose_diagnostics_report_payload(
        serde_json::json!({
            "bundle": bundle,
            "guard": guard
        }),
        serde_json::json!({
            "include_guard": false,
            "include_bundle_items": false
        }),
    )
    .expect("report payload should compose");

    assert!(report.get("guard_payload").is_none());
    assert!(report.get("report_guard_status").is_none());
    assert!(report.get("bundle_items").is_none());
    assert_eq!(
        report["report_focus_metrics"]["thermo.thermal_strain_peak"].as_f64(),
        Some(0.00048)
    );
    assert_eq!(
        report["report_focus_payloads"]["thermo.thermal_strain_peak"]["value"].as_f64(),
        Some(0.00048)
    );
    let sources = report["report_sources"]
        .as_array()
        .expect("report sources should be an array");
    assert_eq!(sources.len(), 2);
}

#[test]
fn runs_diagnostics_report_payload_through_transform_executor() {
    let (bundle, guard) = sample_bundle_and_guard();
    let report = run_transform_operator(
        "transform.compose_diagnostics_report_payload",
        serde_json::json!({
            "bundle": bundle,
            "guard": guard
        }),
        serde_json::json!({
            "include_bundle_items": false
        }),
    )
    .expect("transform report payload should succeed");

    assert_eq!(report["report_guard_status"].as_str(), Some("warn"));
    assert!(report.get("bundle_items").is_none());
}

#[test]
fn selects_focus_payload_from_report() {
    let (bundle, guard) = sample_bundle_and_guard();
    let report = compose_diagnostics_report_payload(
        serde_json::json!({
            "bundle": bundle,
            "guard": guard
        }),
        serde_json::json!({}),
    )
    .expect("report payload should compose");

    let focus = select_focus_payload(
        report,
        serde_json::json!({
            "metric_id": "thermo.stress_peak"
        }),
    )
    .expect("focus payload should select");

    assert_eq!(
        focus["focus_contract"].as_str(),
        Some("kyuubiki.workflow_focus_payload/v1")
    );
    assert_eq!(focus["metric_id"].as_str(), Some("thermo.stress_peak"));
    assert_eq!(focus["value"].as_f64(), Some(220.0));
    assert_eq!(focus["context"]["peak_stress_x"].as_f64(), Some(14.0));
}

#[test]
fn composes_focus_chain_input_from_report() {
    let (bundle, guard) = sample_bundle_and_guard();
    let report = compose_diagnostics_report_payload(
        serde_json::json!({
            "bundle": bundle,
            "guard": guard
        }),
        serde_json::json!({}),
    )
    .expect("report payload should compose");

    let chain = compose_focus_chain_input(
        report,
        serde_json::json!({
            "metric_id": "thermo.stress_peak",
            "target_operator": "bridge.temperature_field_to_thermo_quad_2d",
            "bindings": {
                "seed_model_ref": "thermo_seed"
            },
            "annotations": {
                "label": "stress handoff"
            }
        }),
    )
    .expect("focus chain input should compose");

    assert_eq!(
        chain["chain_contract"].as_str(),
        Some("kyuubiki.workflow_focus_chain_input/v1")
    );
    assert_eq!(chain["metric_id"].as_str(), Some("thermo.stress_peak"));
    assert_eq!(chain["value"].as_f64(), Some(220.0));
    assert_eq!(
        chain["target_operator"].as_str(),
        Some("bridge.temperature_field_to_thermo_quad_2d")
    );
    assert_eq!(
        chain["bindings"]["seed_model_ref"].as_str(),
        Some("thermo_seed")
    );
    assert_eq!(
        chain["annotations"]["label"].as_str(),
        Some("stress handoff")
    );
    assert_eq!(
        chain["focus_payload"]["focus_contract"].as_str(),
        Some("kyuubiki.workflow_focus_payload/v1")
    );
}

#[test]
fn composes_focus_bridge_request_from_chain_input() {
    let (bundle, guard) = sample_bundle_and_guard();
    let report = compose_diagnostics_report_payload(
        serde_json::json!({
            "bundle": bundle,
            "guard": guard
        }),
        serde_json::json!({}),
    )
    .expect("report payload should compose");
    let chain = compose_focus_chain_input(
        report,
        serde_json::json!({
            "metric_id": "thermo.stress_peak",
            "target_operator": "bridge.temperature_field_to_thermo_quad_2d",
            "bindings": {
                "seed_model_ref": "thermo_seed"
            }
        }),
    )
    .expect("focus chain input should compose");

    let bridge_request = compose_focus_bridge_request(
        chain,
        serde_json::json!({
            "seed_model": {
                "nodes": [],
                "elements": []
            },
            "contract": {
                "target": {
                    "field": "temperature_delta"
                }
            },
            "bridge_payload_source": "solve_heat.result"
        }),
    )
    .expect("focus bridge request should compose");

    assert_eq!(
        bridge_request["request_contract"].as_str(),
        Some("kyuubiki.workflow_focus_bridge_request/v1")
    );
    assert_eq!(
        bridge_request["bridge_operator"].as_str(),
        Some("bridge.temperature_field_to_thermo_quad_2d")
    );
    assert_eq!(
        bridge_request["metric_id"].as_str(),
        Some("thermo.stress_peak")
    );
    assert_eq!(bridge_request["focus_value"].as_f64(), Some(220.0));
    assert_eq!(
        bridge_request["bridge_payload_source"].as_str(),
        Some("solve_heat.result")
    );
    assert!(bridge_request["bridge_config"]["seed_model"].is_object());
    assert_eq!(
        bridge_request["focus_chain_input"]["chain_contract"].as_str(),
        Some("kyuubiki.workflow_focus_chain_input/v1")
    );
}

#[test]
fn resolves_focus_bridge_execution_from_request() {
    let bridge_request = serde_json::json!({
        "request_contract": "kyuubiki.workflow_focus_bridge_request/v1",
        "bridge_operator": "bridge.temperature_field_to_thermo_quad_2d",
        "metric_id": "thermo.stress_peak",
        "focus_value": 220.0,
        "bridge_config": {
            "seed_model": {
                "nodes": [],
                "elements": []
            },
            "contract": {
                "target": {
                    "field": "temperature_delta"
                }
            }
        },
        "bindings": {
            "seed_model_ref": "thermo_seed"
        },
        "annotations": {
            "label": "stress handoff"
        }
    });

    let execution = resolve_focus_bridge_execution(
        bridge_request,
        serde_json::json!({
            "bridge_payload": {
                "nodes": [],
                "elements": []
            },
            "bridge_payload_source": "solve_heat.result"
        }),
    )
    .expect("focus bridge execution should resolve");

    assert_eq!(
        execution["execution_contract"].as_str(),
        Some("kyuubiki.workflow_focus_bridge_execution/v1")
    );
    assert_eq!(
        execution["operator_id"].as_str(),
        Some("bridge.temperature_field_to_thermo_quad_2d")
    );
    assert!(execution["bridge_payload"].is_object());
    assert!(execution["bridge_config"].is_object());
    assert_eq!(execution["metric_id"].as_str(), Some("thermo.stress_peak"));
    assert_eq!(execution["focus_value"].as_f64(), Some(220.0));
    assert_eq!(
        execution["bridge_payload_source"].as_str(),
        Some("solve_heat.result")
    );
}

#[test]
fn resolves_focus_bridge_execution_from_named_payload_map() {
    let execution = resolve_focus_bridge_execution(
        serde_json::json!({
            "request": {
                "request_contract": "kyuubiki.workflow_focus_bridge_request/v1",
                "bridge_operator": "bridge.temperature_field_to_thermo_quad_2d",
                "metric_id": "thermo.stress_peak",
                "focus_value": 220.0,
                "bridge_config": {
                    "seed_model": {
                        "nodes": [],
                        "elements": []
                    },
                    "contract": {
                        "target": {
                            "field": "temperature_delta"
                        }
                    }
                }
            },
            "bridge_payload": {
                "nodes": [{ "id": "h0" }],
                "elements": []
            }
        }),
        serde_json::json!({}),
    )
    .expect("focus bridge execution should resolve from named payload");

    assert_eq!(
        execution["execution_contract"].as_str(),
        Some("kyuubiki.workflow_focus_bridge_execution/v1")
    );
    assert_eq!(
        execution["operator_id"].as_str(),
        Some("bridge.temperature_field_to_thermo_quad_2d")
    );
    assert_eq!(
        execution["bridge_payload"]["nodes"][0]["id"].as_str(),
        Some("h0")
    );
}

#[test]
fn executes_focus_bridge_execution_into_bridge_result() {
    let result = execute_focus_bridge_execution(
        serde_json::json!({
            "execution_contract": "kyuubiki.workflow_focus_bridge_execution/v1",
            "operator_id": "bridge.temperature_field_to_thermo_quad_2d",
            "metric_id": "thermo.stress_peak",
            "focus_value": 220.0,
            "bridge_payload": {
                "input": {
                    "nodes": [
                        { "id": "h0", "x": 0.0, "y": 0.0, "fix_temperature": true, "temperature": 100.0, "heat_load": 6.0 },
                        { "id": "h1", "x": 1.0, "y": 0.0, "fix_temperature": true, "temperature": 80.0, "heat_load": 12.0 },
                        { "id": "h2", "x": 1.0, "y": 1.0, "fix_temperature": true, "temperature": 60.0, "heat_load": 18.0 },
                        { "id": "h3", "x": 0.0, "y": 1.0, "fix_temperature": true, "temperature": 40.0, "heat_load": 24.0 }
                    ],
                    "elements": [
                        { "id": "hq0", "node_i": 0, "node_j": 1, "node_k": 2, "node_l": 3, "thickness": 0.02, "conductivity": 45.0 }
                    ]
                },
                "nodes": [
                    { "index": 0, "id": "h0", "x": 0.0, "y": 0.0, "temperature": 100.0, "heat_load": 6.0 },
                    { "index": 1, "id": "h1", "x": 1.0, "y": 0.0, "temperature": 80.0, "heat_load": 12.0 },
                    { "index": 2, "id": "h2", "x": 1.0, "y": 1.0, "temperature": 60.0, "heat_load": 18.0 },
                    { "index": 3, "id": "h3", "x": 0.0, "y": 1.0, "temperature": 40.0, "heat_load": 24.0 }
                ],
                "elements": [
                    {
                        "index": 0,
                        "id": "hq0",
                        "node_i": 0,
                        "node_j": 1,
                        "node_k": 2,
                        "node_l": 3,
                        "area": 1.0,
                        "average_temperature": 70.0,
                        "temperature_gradient_x": -15.0,
                        "temperature_gradient_y": -5.0,
                        "heat_flux_x": 30.0,
                        "heat_flux_y": 10.0,
                        "heat_flux_magnitude": 31.6227766017
                    }
                ],
                "max_temperature": 100.0,
                "max_heat_flux": 31.6227766017
            },
            "bridge_config": {
                "seed_model": {
                    "nodes": [
                        { "id": "t0", "x": 0.0, "y": 0.0, "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0, "temperature_delta": 0.0 },
                        { "id": "t1", "x": 1.0, "y": 0.0, "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0, "temperature_delta": 0.0 },
                        { "id": "t2", "x": 1.0, "y": 1.0, "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0, "temperature_delta": 0.0 },
                        { "id": "t3", "x": 0.0, "y": 1.0, "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0, "temperature_delta": 0.0 }
                    ],
                    "elements": [
                        { "id": "tq0", "node_i": 0, "node_j": 1, "node_k": 2, "node_l": 3, "thickness": 0.02, "youngs_modulus": 70000000000.0, "poisson_ratio": 0.33, "thermal_expansion": 0.000011 }
                    ]
                }
            },
            "bridge_payload_source": "solve_heat.result"
        }),
        serde_json::json!({}),
    )
    .expect("focus bridge execution should execute");

    assert_eq!(
        result["result_contract"].as_str(),
        Some("kyuubiki.workflow_focus_bridge_result/v1")
    );
    assert_eq!(
        result["operator_id"].as_str(),
        Some("bridge.temperature_field_to_thermo_quad_2d")
    );
    assert_eq!(result["metric_id"].as_str(), Some("thermo.stress_peak"));
    assert_eq!(result["focus_value"].as_f64(), Some(220.0));
    assert_eq!(
        result["bridge_payload_source"].as_str(),
        Some("solve_heat.result")
    );
    assert_eq!(
        result["bridge_result"]["nodes"]
            .as_array()
            .expect("bridge result nodes should exist")
            .len(),
        4
    );
}
