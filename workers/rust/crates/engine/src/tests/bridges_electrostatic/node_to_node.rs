use crate::bridge::{
    bridge_electrostatic_result_to_heat_plane_quad_model,
    bridge_electrostatic_result_to_heat_plane_triangle_model,
    resolve_electrostatic_to_heat_bridge_contract,
};
use kyuubiki_protocol::{
    ElectrostaticPlaneNodeInput, HeatPlaneNodeInput, HeatPlaneQuadElementInput,
    HeatPlaneTriangleElementInput, SolveElectrostaticPlaneQuad2dRequest,
    SolveElectrostaticPlaneQuad2dResult, SolveElectrostaticPlaneTriangle2dRequest,
    SolveElectrostaticPlaneTriangle2dResult, SolveHeatPlaneQuad2dRequest,
    SolveHeatPlaneTriangle2dRequest,
};

#[test]
fn bridges_electrostatic_quad_potential_node_to_node_into_heat_temperature() {
    let contract = resolve_electrostatic_to_heat_bridge_contract(&serde_json::json!({
        "contract": {
            "source": { "field": "potential", "distribution": "node_to_node" },
            "transform": { "scale": 2.0, "default_value": 0.0 },
            "target": { "field": "temperature" }
        }
    }))
    .expect("node_to_node quad contract should resolve");

    let (bridged, diagnostics) = bridge_electrostatic_result_to_heat_plane_quad_model(
        &SolveElectrostaticPlaneQuad2dResult {
            input: SolveElectrostaticPlaneQuad2dRequest {
                nodes: quad_nodes(),
                elements: vec![],
            },
            nodes: vec![
                node_result("e0", 0.0, 0.0, 10.0, 1.0),
                node_result("e1", 1.0, 0.0, 7.5, 2.0),
                node_result("e2", 1.0, 1.0, 5.0, 3.0),
                node_result("e3", 0.0, 1.0, 2.5, 4.0),
            ],
            elements: vec![],
            max_potential: 10.0,
            max_electric_field: 0.0,
            max_flux_density: 0.0,
            max_electric_energy_density: 0.0,
            total_stored_energy: 0.0,
        },
        &SolveHeatPlaneQuad2dRequest {
            nodes: heat_quad_nodes(),
            elements: vec![
                HeatPlaneQuadElementInput {
                    id: "hq0".to_string(),
                    node_i: 0,
                    node_j: 1,
                    node_k: 2,
                    node_l: 3,
                    thickness: 0.02,
                    conductivity: 45.0,
                },
                HeatPlaneQuadElementInput {
                    id: "hq1".to_string(),
                    node_i: 0,
                    node_j: 1,
                    node_k: 2,
                    node_l: 3,
                    thickness: 0.02,
                    conductivity: 45.0,
                },
            ],
        },
        &contract,
    )
    .expect("quad node_to_node bridge should build");

    assert_eq!(bridged.nodes[0].temperature, 20.0);
    assert_eq!(bridged.nodes[1].temperature, 15.0);
    assert_eq!(bridged.nodes[2].temperature, 10.0);
    assert_eq!(bridged.nodes[3].temperature, 5.0);
    assert_eq!(bridged.nodes[0].heat_load, 0.0);
    assert_eq!(diagnostics.source_field, "potential");
    assert_eq!(diagnostics.target_field, "temperature");
}

#[test]
fn bridges_electrostatic_triangle_charge_density_node_to_node_into_heat_load() {
    let contract = resolve_electrostatic_to_heat_bridge_contract(&serde_json::json!({
        "contract": {
            "source": { "field": "charge_density", "distribution": "node_to_node" },
            "transform": { "scale": 3.0, "default_value": 0.0 },
            "target": { "field": "heat_load" }
        }
    }))
    .expect("node_to_node triangle contract should resolve");

    let (bridged, diagnostics) = bridge_electrostatic_result_to_heat_plane_triangle_model(
        &SolveElectrostaticPlaneTriangle2dResult {
            input: SolveElectrostaticPlaneTriangle2dRequest {
                nodes: triangle_nodes(),
                elements: vec![],
            },
            nodes: vec![
                node_result("t0", 0.0, 0.0, 9.0, 0.5),
                node_result("t1", 1.0, 0.0, 4.0, 1.0),
                node_result("t2", 0.0, 1.0, 1.0, 1.5),
            ],
            elements: vec![],
            max_potential: 9.0,
            max_electric_field: 0.0,
            max_flux_density: 0.0,
            max_electric_energy_density: 0.0,
            total_stored_energy: 0.0,
        },
        &SolveHeatPlaneTriangle2dRequest {
            nodes: heat_triangle_nodes(),
            elements: vec![
                HeatPlaneTriangleElementInput {
                    id: "ht0".to_string(),
                    node_i: 0,
                    node_j: 1,
                    node_k: 2,
                    thickness: 0.02,
                    conductivity: 45.0,
                },
                HeatPlaneTriangleElementInput {
                    id: "ht1".to_string(),
                    node_i: 0,
                    node_j: 1,
                    node_k: 2,
                    thickness: 0.02,
                    conductivity: 45.0,
                },
            ],
        },
        &contract,
    )
    .expect("triangle node_to_node bridge should build");

    assert_eq!(bridged.nodes[0].heat_load, 1.5);
    assert_eq!(bridged.nodes[1].heat_load, 3.0);
    assert_eq!(bridged.nodes[2].heat_load, 4.5);
    assert_eq!(diagnostics.source_field, "charge_density");
    assert_eq!(diagnostics.target_field, "heat_load");
}

fn node_result(
    id: &str,
    x: f64,
    y: f64,
    potential: f64,
    charge_density: f64,
) -> kyuubiki_protocol::ElectrostaticPlaneNodeResult {
    kyuubiki_protocol::ElectrostaticPlaneNodeResult {
        index: 0,
        id: id.to_string(),
        x,
        y,
        potential,
        charge_density,
    }
}

fn quad_nodes() -> Vec<ElectrostaticPlaneNodeInput> {
    vec![
        ElectrostaticPlaneNodeInput {
            id: "e0".to_string(),
            x: 0.0,
            y: 0.0,
            fix_potential: true,
            potential: 10.0,
            charge_density: 0.0,
        },
        ElectrostaticPlaneNodeInput {
            id: "e1".to_string(),
            x: 1.0,
            y: 0.0,
            fix_potential: false,
            potential: 0.0,
            charge_density: 0.0,
        },
        ElectrostaticPlaneNodeInput {
            id: "e2".to_string(),
            x: 1.0,
            y: 1.0,
            fix_potential: false,
            potential: 0.0,
            charge_density: 0.0,
        },
        ElectrostaticPlaneNodeInput {
            id: "e3".to_string(),
            x: 0.0,
            y: 1.0,
            fix_potential: true,
            potential: 0.0,
            charge_density: 0.0,
        },
    ]
}

fn triangle_nodes() -> Vec<ElectrostaticPlaneNodeInput> {
    vec![
        ElectrostaticPlaneNodeInput {
            id: "t0".to_string(),
            x: 0.0,
            y: 0.0,
            fix_potential: true,
            potential: 9.0,
            charge_density: 0.0,
        },
        ElectrostaticPlaneNodeInput {
            id: "t1".to_string(),
            x: 1.0,
            y: 0.0,
            fix_potential: false,
            potential: 0.0,
            charge_density: 0.0,
        },
        ElectrostaticPlaneNodeInput {
            id: "t2".to_string(),
            x: 0.0,
            y: 1.0,
            fix_potential: false,
            potential: 0.0,
            charge_density: 0.0,
        },
    ]
}

fn heat_quad_nodes() -> Vec<HeatPlaneNodeInput> {
    vec![
        HeatPlaneNodeInput {
            id: "h0".to_string(),
            x: 0.0,
            y: 0.0,
            fix_temperature: false,
            temperature: 0.0,
            heat_load: 0.0,
        },
        HeatPlaneNodeInput {
            id: "h1".to_string(),
            x: 1.0,
            y: 0.0,
            fix_temperature: false,
            temperature: 0.0,
            heat_load: 0.0,
        },
        HeatPlaneNodeInput {
            id: "h2".to_string(),
            x: 1.0,
            y: 1.0,
            fix_temperature: false,
            temperature: 0.0,
            heat_load: 0.0,
        },
        HeatPlaneNodeInput {
            id: "h3".to_string(),
            x: 0.0,
            y: 1.0,
            fix_temperature: false,
            temperature: 0.0,
            heat_load: 0.0,
        },
    ]
}

fn heat_triangle_nodes() -> Vec<HeatPlaneNodeInput> {
    vec![
        HeatPlaneNodeInput {
            id: "h0".to_string(),
            x: 0.0,
            y: 0.0,
            fix_temperature: false,
            temperature: 0.0,
            heat_load: 0.0,
        },
        HeatPlaneNodeInput {
            id: "h1".to_string(),
            x: 1.0,
            y: 0.0,
            fix_temperature: false,
            temperature: 0.0,
            heat_load: 0.0,
        },
        HeatPlaneNodeInput {
            id: "h2".to_string(),
            x: 0.0,
            y: 1.0,
            fix_temperature: false,
            temperature: 0.0,
            heat_load: 0.0,
        },
    ]
}
