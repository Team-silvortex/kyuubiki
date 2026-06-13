use crate::bridge::{
    bridge_electrostatic_result_to_heat_plane_quad_model,
    resolve_electrostatic_to_heat_bridge_contract,
};
use crate::{EngineSolveRequest, solve};
use kyuubiki_protocol::{
    AnalysisResult, HeatPlaneNodeInput, HeatPlaneQuadElementInput,
    SolveElectrostaticPlaneQuad2dRequest, SolveHeatPlaneQuad2dRequest,
};

#[test]
fn bridges_electrostatic_quad_fields_into_heat_model() {
    let solved = solve(EngineSolveRequest::ElectrostaticPlaneQuad2d(
        SolveElectrostaticPlaneQuad2dRequest {
            nodes: vec![
                kyuubiki_protocol::ElectrostaticPlaneNodeInput {
                    id: "e0".to_string(),
                    x: 0.0,
                    y: 0.0,
                    fix_potential: true,
                    potential: 10.0,
                    charge_density: 0.0,
                },
                kyuubiki_protocol::ElectrostaticPlaneNodeInput {
                    id: "e1".to_string(),
                    x: 1.0,
                    y: 0.0,
                    fix_potential: false,
                    potential: 0.0,
                    charge_density: 0.0,
                },
                kyuubiki_protocol::ElectrostaticPlaneNodeInput {
                    id: "e2".to_string(),
                    x: 1.0,
                    y: 1.0,
                    fix_potential: true,
                    potential: 0.0,
                    charge_density: 0.0,
                },
                kyuubiki_protocol::ElectrostaticPlaneNodeInput {
                    id: "e3".to_string(),
                    x: 0.0,
                    y: 1.0,
                    fix_potential: true,
                    potential: 0.0,
                    charge_density: 0.0,
                },
            ],
            elements: vec![kyuubiki_protocol::ElectrostaticPlaneQuadElementInput {
                id: "eq0".to_string(),
                node_i: 0,
                node_j: 1,
                node_k: 2,
                node_l: 3,
                thickness: 0.05,
                permittivity: 2.5,
            }],
        },
    ))
    .expect("electrostatic quad should solve");
    let electrostatic_result = match solved {
        AnalysisResult::ElectrostaticPlaneQuad2d(result) => result,
        _ => unreachable!("expected electrostatic quad result"),
    };
    let contract = resolve_electrostatic_to_heat_bridge_contract(&serde_json::json!({
        "contract": {
            "version": "kyuubiki.bridge-contract/v1",
            "source": { "field": "electric_field_magnitude", "distribution": "element_to_nodes", "node_index_fields": ["node_i", "node_j", "node_k", "node_l"] },
            "transform": { "scale": 50.0, "reduction": "mean", "default_value": 0.0 },
            "target": { "field": "heat_load" }
        }
    }))
    .expect("bridge contract should resolve");
    let (bridged, _) = bridge_electrostatic_result_to_heat_plane_quad_model(
        &electrostatic_result,
        &SolveHeatPlaneQuad2dRequest {
            nodes: vec![
                HeatPlaneNodeInput {
                    id: "h0".to_string(),
                    x: 0.0,
                    y: 0.0,
                    fix_temperature: true,
                    temperature: 20.0,
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
                    fix_temperature: true,
                    temperature: 20.0,
                    heat_load: 0.0,
                },
            ],
            elements: vec![HeatPlaneQuadElementInput {
                id: "hq0".to_string(),
                node_i: 0,
                node_j: 1,
                node_k: 2,
                node_l: 3,
                thickness: 0.02,
                conductivity: 45.0,
            }],
        },
        &contract,
    )
    .expect("bridge should build");

    assert!(electrostatic_result.max_electric_field > 0.0);
    for node in &bridged.nodes {
        assert!(node.heat_load > 0.0);
    }
    assert_eq!(bridged.nodes[0].heat_load, bridged.nodes[1].heat_load);
    assert_eq!(bridged.nodes[1].heat_load, bridged.nodes[2].heat_load);
    assert_eq!(bridged.nodes[2].heat_load, bridged.nodes[3].heat_load);
}

#[test]
fn bridges_electrostatic_quad_average_potential_into_heat_temperature() {
    let solved = solve(EngineSolveRequest::ElectrostaticPlaneQuad2d(
        SolveElectrostaticPlaneQuad2dRequest {
            nodes: vec![
                kyuubiki_protocol::ElectrostaticPlaneNodeInput {
                    id: "e0".to_string(),
                    x: 0.0,
                    y: 0.0,
                    fix_potential: true,
                    potential: 10.0,
                    charge_density: 0.0,
                },
                kyuubiki_protocol::ElectrostaticPlaneNodeInput {
                    id: "e1".to_string(),
                    x: 1.0,
                    y: 0.0,
                    fix_potential: false,
                    potential: 0.0,
                    charge_density: 0.0,
                },
                kyuubiki_protocol::ElectrostaticPlaneNodeInput {
                    id: "e2".to_string(),
                    x: 1.0,
                    y: 1.0,
                    fix_potential: true,
                    potential: 0.0,
                    charge_density: 0.0,
                },
                kyuubiki_protocol::ElectrostaticPlaneNodeInput {
                    id: "e3".to_string(),
                    x: 0.0,
                    y: 1.0,
                    fix_potential: true,
                    potential: 0.0,
                    charge_density: 0.0,
                },
            ],
            elements: vec![kyuubiki_protocol::ElectrostaticPlaneQuadElementInput {
                id: "eq0".to_string(),
                node_i: 0,
                node_j: 1,
                node_k: 2,
                node_l: 3,
                thickness: 0.05,
                permittivity: 2.5,
            }],
        },
    ))
    .expect("electrostatic quad should solve");
    let electrostatic_result = match solved {
        AnalysisResult::ElectrostaticPlaneQuad2d(result) => result,
        _ => unreachable!("expected electrostatic quad result"),
    };
    let contract = resolve_electrostatic_to_heat_bridge_contract(&serde_json::json!({
        "contract": {
            "source": { "field": "average_potential", "distribution": "element_to_nodes", "node_index_fields": ["node_i", "node_j", "node_k", "node_l"] },
            "transform": { "scale": 2.0, "reduction": "mean", "default_value": 0.0 },
            "target": { "field": "temperature" }
        }
    }))
    .expect("bridge contract should resolve");
    let (bridged, _) = bridge_electrostatic_result_to_heat_plane_quad_model(
        &electrostatic_result,
        &SolveHeatPlaneQuad2dRequest {
            nodes: vec![
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
            ],
            elements: vec![HeatPlaneQuadElementInput {
                id: "hq0".to_string(),
                node_i: 0,
                node_j: 1,
                node_k: 2,
                node_l: 3,
                thickness: 0.02,
                conductivity: 45.0,
            }],
        },
        &contract,
    )
    .expect("average potential bridge should build");

    for node in &bridged.nodes {
        assert!(node.temperature > 0.0);
        assert_eq!(node.heat_load, 0.0);
    }
}
