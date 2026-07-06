use super::helpers::{triangle_heat_seed_model, triangle_result_nodes, triangle_source_nodes};
use crate::bridge::{
    bridge_electrostatic_result_to_heat_plane_triangle_model,
    resolve_electrostatic_to_heat_bridge_contract,
};
use crate::{solve, EngineSolveRequest};
use kyuubiki_protocol::{
    AnalysisResult, HeatPlaneNodeInput, HeatPlaneTriangleElementInput,
    SolveElectrostaticPlaneTriangle2dRequest,
};

#[test]
fn bridges_electrostatic_triangle_fields_into_heat_model() {
    let solved = solve(EngineSolveRequest::ElectrostaticPlaneTriangle2d(
        SolveElectrostaticPlaneTriangle2dRequest {
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
                    fix_potential: true,
                    potential: 0.0,
                    charge_density: 0.0,
                },
                kyuubiki_protocol::ElectrostaticPlaneNodeInput {
                    id: "e2".to_string(),
                    x: 0.0,
                    y: 1.0,
                    fix_potential: false,
                    potential: 0.0,
                    charge_density: 0.0,
                },
            ],
            elements: vec![kyuubiki_protocol::ElectrostaticPlaneTriangleElementInput {
                id: "et0".to_string(),
                node_i: 0,
                node_j: 1,
                node_k: 2,
                thickness: 0.05,
                permittivity: 2.5,
            }],
        },
    ))
    .expect("electrostatic triangle should solve");
    let electrostatic_result = match solved {
        AnalysisResult::ElectrostaticPlaneTriangle2d(result) => result,
        _ => unreachable!("expected electrostatic triangle result"),
    };
    let contract = resolve_electrostatic_to_heat_bridge_contract(&serde_json::json!({
        "contract": {
            "version": "kyuubiki.bridge-contract/v1",
            "source": { "field": "electric_field_magnitude", "distribution": "element_to_nodes", "node_index_fields": ["node_i", "node_j", "node_k"] },
            "transform": { "scale": 50.0, "reduction": "mean", "default_value": 0.0 },
            "target": { "field": "heat_load" }
        }
    }))
    .expect("bridge contract should resolve");
    let (bridged, _) = bridge_electrostatic_result_to_heat_plane_triangle_model(
        &electrostatic_result,
        &kyuubiki_protocol::SolveHeatPlaneTriangle2dRequest {
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
                    fix_temperature: true,
                    temperature: 20.0,
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
            ],
            elements: vec![HeatPlaneTriangleElementInput {
                id: "ht0".to_string(),
                node_i: 0,
                node_j: 1,
                node_k: 2,
                thickness: 0.02,
                conductivity: 45.0,
            }],
        },
        &contract,
    )
    .expect("triangle bridge should build");

    assert!(electrostatic_result.max_electric_field > 0.0);
    for node in &bridged.nodes {
        assert!(node.heat_load > 0.0);
    }
}

#[test]
fn bridges_electrostatic_triangle_flux_alias_into_heat_model() {
    let solved = solve(EngineSolveRequest::ElectrostaticPlaneTriangle2d(
        SolveElectrostaticPlaneTriangle2dRequest {
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
                    fix_potential: true,
                    potential: 0.0,
                    charge_density: 0.0,
                },
                kyuubiki_protocol::ElectrostaticPlaneNodeInput {
                    id: "e2".to_string(),
                    x: 0.0,
                    y: 1.0,
                    fix_potential: false,
                    potential: 0.0,
                    charge_density: 0.0,
                },
            ],
            elements: vec![kyuubiki_protocol::ElectrostaticPlaneTriangleElementInput {
                id: "et0".to_string(),
                node_i: 0,
                node_j: 1,
                node_k: 2,
                thickness: 0.05,
                permittivity: 2.5,
            }],
        },
    ))
    .expect("electrostatic triangle should solve");
    let electrostatic_result = match solved {
        AnalysisResult::ElectrostaticPlaneTriangle2d(result) => result,
        _ => unreachable!("expected electrostatic triangle result"),
    };
    let contract = resolve_electrostatic_to_heat_bridge_contract(&serde_json::json!({
        "contract": {
            "source": { "field": "flux_magnitude", "distribution": "element_to_nodes", "node_index_fields": ["node_i", "node_j", "node_k"] },
            "transform": { "scale": 10.0, "reduction": "mean", "default_value": 0.0 },
            "target": { "field": "heat_load" }
        }
    }))
    .expect("triangle flux contract should resolve");
    let (bridged, _) = bridge_electrostatic_result_to_heat_plane_triangle_model(
        &electrostatic_result,
        &kyuubiki_protocol::SolveHeatPlaneTriangle2dRequest {
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
                    x: 0.0,
                    y: 1.0,
                    fix_temperature: false,
                    temperature: 0.0,
                    heat_load: 0.0,
                },
            ],
            elements: vec![HeatPlaneTriangleElementInput {
                id: "ht0".to_string(),
                node_i: 0,
                node_j: 1,
                node_k: 2,
                thickness: 0.02,
                conductivity: 45.0,
            }],
        },
        &contract,
    )
    .expect("triangle flux bridge should build");

    for node in &bridged.nodes {
        assert!(node.heat_load > 0.0);
    }
}

#[test]
fn bridges_electrostatic_triangle_area_weighted_mean_into_heat_model() {
    let contract = resolve_electrostatic_to_heat_bridge_contract(&serde_json::json!({
        "contract": {
            "source": { "field": "electric_field_magnitude", "distribution": "element_to_nodes", "node_index_fields": ["node_i", "node_j", "node_k"] },
            "transform": { "scale": 1.0, "reduction": "area_weighted_mean", "default_value": 0.0 },
            "target": { "field": "heat_load" }
        }
    }))
    .expect("weighted contract should resolve");
    let (bridged, _) = bridge_electrostatic_result_to_heat_plane_triangle_model(
        &kyuubiki_protocol::SolveElectrostaticPlaneTriangle2dResult {
            input: SolveElectrostaticPlaneTriangle2dRequest {
                nodes: triangle_source_nodes(),
                elements: vec![
                    kyuubiki_protocol::ElectrostaticPlaneTriangleElementInput {
                        id: "et0".to_string(),
                        node_i: 0,
                        node_j: 1,
                        node_k: 2,
                        thickness: 0.05,
                        permittivity: 2.5,
                    },
                    kyuubiki_protocol::ElectrostaticPlaneTriangleElementInput {
                        id: "et1".to_string(),
                        node_i: 1,
                        node_j: 3,
                        node_k: 2,
                        thickness: 0.05,
                        permittivity: 2.5,
                    },
                ],
            },
            nodes: triangle_result_nodes(),
            elements: vec![
                kyuubiki_protocol::ElectrostaticPlaneTriangleElementResult {
                    index: 0,
                    id: "et0".to_string(),
                    node_i: 0,
                    node_j: 1,
                    node_k: 2,
                    area: 1.0,
                    average_potential: 4.0,
                    potential_gradient_x: 0.0,
                    potential_gradient_y: 0.0,
                    electric_field_x: 2.0,
                    electric_field_y: 0.0,
                    electric_field_magnitude: 2.0,
                    electric_flux_density_x: 1.0,
                    electric_flux_density_y: 0.0,
                    electric_flux_density_magnitude: 1.0,
                    stored_energy: 0.0,
                },
                kyuubiki_protocol::ElectrostaticPlaneTriangleElementResult {
                    index: 1,
                    id: "et1".to_string(),
                    node_i: 1,
                    node_j: 3,
                    node_k: 2,
                    area: 3.0,
                    average_potential: 8.0,
                    potential_gradient_x: 0.0,
                    potential_gradient_y: 0.0,
                    electric_field_x: 10.0,
                    electric_field_y: 0.0,
                    electric_field_magnitude: 10.0,
                    electric_flux_density_x: 5.0,
                    electric_flux_density_y: 0.0,
                    electric_flux_density_magnitude: 5.0,
                    stored_energy: 0.0,
                },
            ],
            max_potential: 10.0,
            max_electric_field: 10.0,
            max_flux_density: 5.0,
            total_stored_energy: 0.0,
        },
        &triangle_heat_seed_model(),
        &contract,
    )
    .expect("weighted triangle bridge should build");

    assert_eq!(bridged.nodes[0].heat_load, 2.0);
    assert_eq!(bridged.nodes[3].heat_load, 10.0);
    assert_eq!(bridged.nodes[1].heat_load, 8.0);
    assert_eq!(bridged.nodes[2].heat_load, 8.0);
}

#[test]
fn bridges_electrostatic_triangle_stored_energy_into_heat_model() {
    let contract = resolve_electrostatic_to_heat_bridge_contract(&serde_json::json!({
        "contract": {
            "source": { "field": "stored_energy", "distribution": "element_to_nodes", "node_index_fields": ["node_i", "node_j", "node_k"] },
            "transform": { "scale": 2.0, "reduction": "sum", "default_value": 0.0 },
            "target": { "field": "heat_load" }
        }
    }))
    .expect("stored energy contract should resolve");
    let mut result = kyuubiki_protocol::SolveElectrostaticPlaneTriangle2dResult {
        input: SolveElectrostaticPlaneTriangle2dRequest {
            nodes: triangle_source_nodes(),
            elements: vec![],
        },
        nodes: triangle_result_nodes(),
        elements: vec![
            energy_element("et0", 0, 1, 2, 2.0, 6.0),
            energy_element("et1", 1, 3, 2, 1.0, 3.0),
        ],
        max_potential: 10.0,
        max_electric_field: 0.0,
        max_flux_density: 0.0,
        total_stored_energy: 9.0,
    };
    result.input.elements = result
        .elements
        .iter()
        .map(
            |element| kyuubiki_protocol::ElectrostaticPlaneTriangleElementInput {
                id: element.id.clone(),
                node_i: element.node_i,
                node_j: element.node_j,
                node_k: element.node_k,
                thickness: 0.05,
                permittivity: 2.5,
            },
        )
        .collect();

    let (bridged, diagnostics) = bridge_electrostatic_result_to_heat_plane_triangle_model(
        &result,
        &triangle_heat_seed_model(),
        &contract,
    )
    .expect("stored-energy triangle bridge should build");

    assert_eq!(bridged.nodes[0].heat_load, 12.0);
    assert_eq!(bridged.nodes[1].heat_load, 18.0);
    assert_eq!(bridged.nodes[2].heat_load, 18.0);
    assert_eq!(bridged.nodes[3].heat_load, 6.0);
    assert_eq!(diagnostics.source_field, "stored_energy");
    assert_eq!(diagnostics.source_value_max, Some(12.0));
}

fn energy_element(
    id: &str,
    node_i: usize,
    node_j: usize,
    node_k: usize,
    area: f64,
    stored_energy: f64,
) -> kyuubiki_protocol::ElectrostaticPlaneTriangleElementResult {
    kyuubiki_protocol::ElectrostaticPlaneTriangleElementResult {
        index: node_i,
        id: id.to_string(),
        node_i,
        node_j,
        node_k,
        area,
        average_potential: 0.0,
        potential_gradient_x: 0.0,
        potential_gradient_y: 0.0,
        electric_field_x: 0.0,
        electric_field_y: 0.0,
        electric_field_magnitude: 0.0,
        electric_flux_density_x: 0.0,
        electric_flux_density_y: 0.0,
        electric_flux_density_magnitude: 0.0,
        stored_energy,
    }
}
