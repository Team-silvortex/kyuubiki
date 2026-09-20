use kyuubiki_engine::{EngineSolveRequest, run_solve_operator, solve};
use kyuubiki_protocol::{
    AnalysisResult, ElectricConductionPlaneNodeInput, ElectricConductionPlaneQuadElementInput,
    SolveElectricConductionPlaneQuad2dRequest,
};

#[test]
fn reference_shift_preserves_current_driven_joule_to_heat_chain() {
    for offset in [0.0, -2.0_f64.powi(40), 2.0_f64.powi(40)] {
        let mut conductor = request();
        for (index, node) in conductor.nodes.iter_mut().enumerate() {
            node.electric_potential_v = offset;
            if index == 1 || index == 2 {
                node.fix_electric_potential = false;
                node.current_source_a = 1.0;
            }
        }
        let direct = solve(EngineSolveRequest::ElectricConductionPlaneQuad2d(
            conductor.clone(),
        ))
        .unwrap();
        let workflow = run_solve_operator(
            "solve.electric_conduction_plane_quad_2d",
            serde_json::to_value(conductor).unwrap(),
        )
        .unwrap();
        let AnalysisResult::ElectricConductionPlaneQuad2d(direct) = direct else {
            panic!("unexpected conduction result");
        };
        let power = workflow["total_joule_power_w"].as_f64().unwrap();
        assert!((power - 6.72e-5).abs() < 1.0e-15);
        assert!((power - direct.total_joule_power_w).abs() < 1.0e-15);
        assert!(
            workflow["source_power_balance_relative_error"]
                .as_f64()
                .unwrap()
                < 1.0e-12
        );
        // Explicit conservative nodal power transfer, not a volumetric-source
        // projection or a temperature-dependent material feedback model.
        let heat = run_solve_operator("solve.heat_plane_quad_2d", serde_json::json!({
            "nodes": [
                {"id": "h0", "x": 0.0, "y": 0.0, "fix_temperature": true, "temperature": 20.0},
                {"id": "h1", "x": 1.0, "y": 0.0, "fix_temperature": false, "heat_load": power / 2.0},
                {"id": "h2", "x": 1.0, "y": 1.0, "fix_temperature": false, "heat_load": power / 2.0},
                {"id": "h3", "x": 0.0, "y": 1.0, "fix_temperature": true, "temperature": 20.0}
            ],
            "elements": [{"id": "heat", "node_i": 0, "node_j": 1, "node_k": 2, "node_l": 3,
                "thickness": 1.0, "conductivity": 1.0}]
        })).unwrap();
        for index in [1, 2] {
            let temperature = heat["nodes"][index]["temperature"].as_f64().unwrap();
            assert!((temperature - (20.0 + power)).abs() < 1.0e-12);
        }
        let flux = heat["elements"][0]["heat_flux_x"].as_f64().unwrap();
        assert!((flux + power).abs() < 1.0e-12);
    }
}

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
    assert!(direct.current_balance_relative_error < 1.0e-12);
    assert!(direct.free_current_residual_relative_error < 1.0e-12);
    assert!(direct.power_balance_relative_error < 1.0e-12);
    assert!(
        (workflow["total_joule_power_w"].as_f64().unwrap_or_default() - 6.72e-5).abs() < 1.0e-15
    );
    assert!(
        workflow["power_balance_relative_error"]
            .as_f64()
            .is_some_and(|error| error < 1.0e-12)
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
        contact_interfaces: vec![],
        terminals: vec![],
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
