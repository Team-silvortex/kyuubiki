use kyuubiki_engine::{EngineSolveRequest, describe_built_in_operator, run_solve_operator, solve};
use kyuubiki_headless_sdk::{
    HeadlessExecutionBatch, build_execution_plan, engine_solver_headless_bridge_manifest,
    project_composite_heat_to_thermal,
};
use kyuubiki_protocol::{
    AnalysisResult, SolveHeatPlaneQuad2dRequest, SolveHeatPlaneQuad2dResult,
    SolveThermalPlaneQuad2dRequest,
};
use serde_json::{Value, json};

fn model() -> Value {
    serde_json::from_str(include_str!(
        "../../protocol/fixtures/heat-plane-contact-quad.json"
    ))
    .unwrap()
}

#[test]
fn headless_plan_and_existing_engine_operator_preserve_the_contact_contract() {
    let batch: HeadlessExecutionBatch = serde_json::from_value(json!({
        "schema_version":"kyuubiki.headless-execution-batch/v1",
        "exported_at":"2026-09-23T00:00:00Z", "language":"rust", "workflow_id":"thermal-contact",
        "steps":[{"index":1, "action":"solve_heat_plane_quad_2d", "risk":"normal", "payload":{"model":model()}}]
    })).unwrap();
    let plan = build_execution_plan(&batch);
    assert!(plan.ok, "{:?}", plan.validation);
    assert_eq!(plan.steps[0].payload["model"], model());
    let manifest = engine_solver_headless_bridge_manifest();
    let route = manifest
        .routes
        .iter()
        .find(|route| route.action == plan.steps[0].action)
        .unwrap();
    assert!(
        describe_built_in_operator(&route.engine_operator_id)
            .unwrap()
            .capability_tags
            .iter()
            .any(|tag| tag == "thermal-contact-resistance")
    );
    let workflow = run_solve_operator(
        &route.engine_operator_id,
        plan.steps[0].payload["model"].clone(),
    )
    .unwrap();
    let request: SolveHeatPlaneQuad2dRequest = serde_json::from_value(model()).unwrap();
    let AnalysisResult::HeatPlaneQuad2d(native) =
        solve(EngineSolveRequest::HeatPlaneQuad2d(request)).unwrap()
    else {
        panic!("incorrect result type");
    };
    assert_eq!(
        workflow["contact_interfaces"],
        serde_json::to_value(&native.contact_interfaces).unwrap()
    );
    assert_eq!(
        workflow["input"]["contact_interfaces"],
        model()["contact_interfaces"]
    );
    let power = workflow["contact_interfaces"][0]["heat_flow_a_to_b_w"]
        .as_f64()
        .unwrap();
    assert!((power - 20.0 / 3.5).abs() < 1e-12);
    assert!(
        (native.nodes[1].temperature - native.nodes[4].temperature - 2.0 * power).abs() < 1e-12
    );
}

#[test]
fn invalid_contact_propagates_an_operator_error_and_clean_retry_succeeds() {
    let mut invalid = model();
    invalid["contact_interfaces"][0]["thermal_resistance_m2_k_w"] = json!(0.0);
    let error = run_solve_operator("solve.heat_plane_quad_2d", invalid).unwrap_err();
    assert!(error.contains("thermal contact"), "{error}");
    let mut typo = model();
    typo["contact_interfaces"][0]["conductance"] = json!(3.0);
    assert!(
        run_solve_operator("solve.heat_plane_quad_2d", typo)
            .unwrap_err()
            .contains("unknown field")
    );
    let result = run_solve_operator("solve.heat_plane_quad_2d", model()).unwrap();
    assert!(
        result["contact_interfaces"][0]["heat_flow_a_to_b_w"]
            .as_f64()
            .unwrap()
            > 0.0
    );
}

#[test]
fn unresolved_contact_heat_balance_is_an_operator_failure_not_a_successful_study() {
    let mut input = model();
    input["contact_interfaces"][0]["thermal_resistance_m2_k_w"] = json!(1e-12);
    let error = run_solve_operator("solve.heat_plane_quad_2d", input).unwrap_err();
    assert!(error.contains("thermal contact heat balance"), "{error}");
    assert!(error.contains("node"), "{error}");
    let result = run_solve_operator("solve.heat_plane_quad_2d", model()).unwrap();
    let flow = result["contact_interfaces"][0]["heat_flow_a_to_b_w"]
        .as_f64()
        .unwrap();
    assert!((flow - 20.0 / 3.5).abs() < 1e-12);
}

#[test]
fn sdk_thermal_transfer_preserves_the_jump_between_coincident_but_independent_nodes() {
    let heat: SolveHeatPlaneQuad2dResult =
        serde_json::from_value(run_solve_operator("solve.heat_plane_quad_2d", model()).unwrap())
            .unwrap();
    let mut seed = model();
    seed.as_object_mut().unwrap().remove("contact_interfaces");
    for node in seed["nodes"].as_array_mut().unwrap() {
        node["fix_x"] = json!(true);
        node["fix_y"] = json!(true);
        node["load_x"] = json!(0.0);
        node["load_y"] = json!(0.0);
        node["temperature_delta"] = json!(0.0);
    }
    for element in seed["elements"].as_array_mut().unwrap() {
        element["youngs_modulus"] = json!(70e9);
        element["poisson_ratio"] = json!(0.3);
        element["thermal_expansion"] = json!(1e-5);
    }
    let seed: SolveThermalPlaneQuad2dRequest = serde_json::from_value(seed).unwrap();
    let (projected, _) = project_composite_heat_to_thermal(&heat, &seed, 0.0).unwrap();
    let jump = projected.nodes[1].temperature_delta - projected.nodes[4].temperature_delta;
    assert!((jump - 40.0 / 3.5).abs() < 1e-12);
    let thermal = run_solve_operator(
        "solve.thermal_plane_quad_2d",
        serde_json::to_value(projected).unwrap(),
    )
    .unwrap();
    for (element, mean_temperature) in [(0, 120.0 / 7.0), (1, 10.0 / 7.0)] {
        let expected_stress = -70e9 * 1e-5 * mean_temperature / 0.7;
        for component in ["stress_x", "stress_y"] {
            let actual = thermal["elements"][element][component].as_f64().unwrap();
            assert!((actual - expected_stress).abs() / expected_stress.abs() < 1e-12);
        }
    }
}
