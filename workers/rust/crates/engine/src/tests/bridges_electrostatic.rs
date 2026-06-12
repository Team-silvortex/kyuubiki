use crate::bridge::{
    bridge_electrostatic_result_to_heat_plane_quad_model,
    bridge_electrostatic_result_to_heat_plane_triangle_model,
    resolve_electrostatic_to_heat_bridge_contract,
};
use crate::{EngineSolveRequest, solve};
use kyuubiki_protocol::{
    AnalysisResult, ElectrostaticPlaneNodeInput, ElectrostaticPlaneNodeResult, HeatPlaneNodeInput,
    HeatPlaneQuadElementInput, HeatPlaneTriangleElementInput, SolveElectrostaticPlaneQuad2dRequest,
    SolveElectrostaticPlaneTriangle2dRequest, SolveHeatPlaneQuad2dRequest,
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
    let bridged = bridge_electrostatic_result_to_heat_plane_quad_model(
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
    let bridged = bridge_electrostatic_result_to_heat_plane_triangle_model(
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
    let bridged = bridge_electrostatic_result_to_heat_plane_quad_model(
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
    let bridged = bridge_electrostatic_result_to_heat_plane_triangle_model(
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
    let bridged = bridge_electrostatic_result_to_heat_plane_triangle_model(
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
                },
            ],
            max_potential: 10.0,
            max_electric_field: 10.0,
            max_flux_density: 5.0,
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

fn triangle_source_nodes() -> Vec<ElectrostaticPlaneNodeInput> {
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
            x: 0.0,
            y: 1.0,
            fix_potential: false,
            potential: 0.0,
            charge_density: 0.0,
        },
        ElectrostaticPlaneNodeInput {
            id: "e3".to_string(),
            x: 1.0,
            y: 1.0,
            fix_potential: true,
            potential: 0.0,
            charge_density: 0.0,
        },
    ]
}

fn triangle_result_nodes() -> Vec<ElectrostaticPlaneNodeResult> {
    vec![
        ElectrostaticPlaneNodeResult {
            index: 0,
            id: "e0".to_string(),
            x: 0.0,
            y: 0.0,
            potential: 10.0,
            charge_density: 0.0,
        },
        ElectrostaticPlaneNodeResult {
            index: 1,
            id: "e1".to_string(),
            x: 1.0,
            y: 0.0,
            potential: 0.0,
            charge_density: 0.0,
        },
        ElectrostaticPlaneNodeResult {
            index: 2,
            id: "e2".to_string(),
            x: 0.0,
            y: 1.0,
            potential: 0.0,
            charge_density: 0.0,
        },
        ElectrostaticPlaneNodeResult {
            index: 3,
            id: "e3".to_string(),
            x: 1.0,
            y: 1.0,
            potential: 0.0,
            charge_density: 0.0,
        },
    ]
}

fn triangle_heat_seed_model() -> kyuubiki_protocol::SolveHeatPlaneTriangle2dRequest {
    kyuubiki_protocol::SolveHeatPlaneTriangle2dRequest {
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
            HeatPlaneNodeInput {
                id: "h3".to_string(),
                x: 1.0,
                y: 1.0,
                fix_temperature: false,
                temperature: 0.0,
                heat_load: 0.0,
            },
        ],
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
                node_i: 1,
                node_j: 3,
                node_k: 2,
                thickness: 0.02,
                conductivity: 45.0,
            },
        ],
    }
}
