use kyuubiki_engine::run_solve_operator;
use kyuubiki_headless_sdk::{
    HeadlessExecutionBatch, build_execution_plan, engine_solver_headless_bridge_manifest,
};
use serde_json::{Value, json};

fn model(space: bool) -> Value {
    let nodes: Vec<_> = (0..4)
        .map(|index| {
            let mut node = json!({"id":format!("n{index}"), "x":(index % 2) as f64,
            "y":(index / 2) as f64, "fix_x":index % 2 == 0, "fix_y":true, "fix_rz":true,
            "load_x":0.0, "load_y":0.0, "moment_z":0.0});
            if space {
                node["z"] = json!(0.0);
                for key in ["load_z", "moment_x", "moment_y"] {
                    node[key] = json!(0.0);
                }
                for key in ["fix_z", "fix_rx", "fix_ry"] {
                    node[key] = json!(true);
                }
            }
            node
        })
        .collect();
    let elements: Vec<_> = (0..2)
        .map(|index| {
            let modulus = if index == 0 { 1.0 } else { 1.0e14 };
            let mut element = json!({"id":format!("beam{index}"), "node_i":index * 2,
            "node_j":index * 2 + 1, "area":1.0, "youngs_modulus":modulus, "density":1.0});
            if space {
                element["shear_modulus"] = json!(modulus * 0.4);
                for key in [
                    "torsion_constant",
                    "moment_of_inertia_y",
                    "moment_of_inertia_z",
                ] {
                    element[key] = json!(0.01);
                }
            } else {
                element["moment_of_inertia"] = json!(0.01);
                element["section_modulus"] = json!(0.1);
            }
            element
        })
        .collect();
    json!({"nodes":nodes, "elements":elements, "mode_count":2})
}

fn planned_solve(space: bool, model: Value) -> Result<Value, String> {
    let action = if space {
        "solve_modal_frame_3d"
    } else {
        "solve_modal_frame_2d"
    };
    let batch: HeadlessExecutionBatch = serde_json::from_value(json!({
        "schema_version":"kyuubiki.headless-execution-batch/v1",
        "exported_at":"2026-09-23T00:00:00Z", "language":"rust", "workflow_id":"modal-spectrum",
        "steps":[{"index":1, "action":action, "risk":"normal", "payload":{"model":model}}]
    }))
    .unwrap();
    let plan = build_execution_plan(&batch);
    assert!(plan.ok, "{:?}", plan.validation);
    let manifest = engine_solver_headless_bridge_manifest();
    let route = manifest
        .routes
        .iter()
        .find(|route| route.action == plan.steps[0].action)
        .unwrap();
    run_solve_operator(
        &route.engine_operator_id,
        plan.steps[0].payload["model"].clone(),
    )
}

fn assert_spectrum(result: Value) {
    assert_eq!(result["modes"].as_array().unwrap().len(), 2);
    for (index, expected) in [2.0, 2.0e14].into_iter().enumerate() {
        let mode = &result["modes"][index];
        let actual = mode["eigenvalue_rad_s_squared"].as_f64().unwrap();
        assert!((actual / expected - 1.0).abs() < 1.0e-10);
        for key in [
            "natural_frequency_rad_s",
            "natural_frequency_hz",
            "period_s",
            "participation_norm",
        ] {
            let value = mode[key]
                .as_f64()
                .expect("successful mode must contain finite numbers");
            assert!(value.is_finite() && value > 0.0, "{key}: {value}");
        }
    }
}

#[test]
fn headless_plan_preserves_both_soft_and_stiff_modal_branches() {
    for space in [false, true] {
        assert_spectrum(planned_solve(space, model(space)).unwrap());
    }
}

#[test]
fn engine_returns_restraint_errors_and_the_corrected_model_can_replay() {
    for space in [false, true] {
        let mut floating = model(space);
        floating["nodes"][2]["fix_x"] = json!(false);
        let error = planned_solve(space, floating).unwrap_err();
        assert!(error.contains("unrestrained rigid-body motion"), "{error}");
        assert_spectrum(planned_solve(space, model(space)).unwrap());
    }
}
