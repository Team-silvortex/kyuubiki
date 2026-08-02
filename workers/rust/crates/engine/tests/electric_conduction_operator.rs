use kyuubiki_engine::{EngineSolveRequest, run_solve_operator, solve};
use kyuubiki_protocol::{
    AnalysisResult, ElectricConductionPlaneNodeInput, ElectricConductionPlaneQuadElementInput,
    SolveElectricConductionPlaneQuad2dRequest,
};

#[test]
fn engine_and_workflow_operator_execute_electric_conduction() {
    let request = request();
    let direct = solve(EngineSolveRequest::ElectricConductionPlaneQuad2d(
        request.clone(),
    ))
    .expect("engine solve");
    let workflow = run_solve_operator(
        "solve.electric_conduction_plane_quad_2d",
        serde_json::to_value(request).expect("request payload"),
    )
    .expect("workflow solve");

    let AnalysisResult::ElectricConductionPlaneQuad2d(direct) = direct else {
        panic!("unexpected engine result")
    };
    assert!((direct.total_joule_power_w - 6.72e-5).abs() < 1.0e-15);
    assert!(
        (workflow["total_joule_power_w"].as_f64().unwrap_or_default() - 6.72e-5).abs() < 1.0e-15
    );
}

fn request() -> SolveElectricConductionPlaneQuad2dRequest {
    let voltage = 3.36e-5;
    SolveElectricConductionPlaneQuad2dRequest {
        nodes: vec![
            node("n0", 0.0, 0.0, 0.0),
            node("n1", 0.03, 0.0, voltage),
            node("n2", 0.03, 0.03, voltage),
            node("n3", 0.0, 0.03, 0.0),
        ],
        elements: vec![ElectricConductionPlaneQuadElementInput {
            id: "conductor".to_string(),
            node_i: 0,
            node_j: 1,
            node_k: 2,
            node_l: 3,
            thickness: 0.001,
            electrical_conductivity_s_m: 1.0 / 1.68e-8,
        }],
    }
}

fn node(id: &str, x: f64, y: f64, potential: f64) -> ElectricConductionPlaneNodeInput {
    ElectricConductionPlaneNodeInput {
        id: id.to_string(),
        x,
        y,
        fix_electric_potential: true,
        electric_potential_v: potential,
        current_source_a: 0.0,
    }
}
