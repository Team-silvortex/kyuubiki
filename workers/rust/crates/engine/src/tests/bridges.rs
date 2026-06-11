use crate::bridge::{
    bridge_electrostatic_result_to_heat_plane_quad_model,
    resolve_electrostatic_to_heat_bridge_contract,
};
use crate::{
    EngineSolveRequest, bridge_heat_result_to_thermal_plane_quad_model,
    run_heat_to_thermo_plane_quad_2d_workflow, solve,
};
use kyuubiki_protocol::{
    AnalysisResult, HeatPlaneNodeInput, HeatPlaneQuadElementInput,
    HeatToThermoPlaneQuad2dWorkflowRequest, SolveElectrostaticPlaneQuad2dRequest,
    SolveHeatPlaneQuad2dRequest, SolveThermalPlaneQuad2dRequest, ThermalPlaneNodeInput,
    ThermalPlaneQuadElementInput,
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
            "source": {
                "field": "electric_field_magnitude",
                "distribution": "element_to_nodes",
                "node_index_fields": ["node_i", "node_j", "node_k", "node_l"]
            },
            "transform": {
                "scale": 50.0,
                "reduction": "mean",
                "default_value": 0.0
            },
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
fn bridges_heat_quad_temperatures_into_thermo_model() {
    let solved = solve(EngineSolveRequest::HeatPlaneQuad2d(
        SolveHeatPlaneQuad2dRequest {
            nodes: vec![
                HeatPlaneNodeInput {
                    id: "h0".to_string(),
                    x: 0.0,
                    y: 0.0,
                    fix_temperature: true,
                    temperature: 100.0,
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
                    fix_temperature: true,
                    temperature: 20.0,
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
    ))
    .expect("heat quad should solve");
    let heat_result = match solved {
        AnalysisResult::HeatPlaneQuad2d(result) => result,
        _ => unreachable!("expected heat quad result"),
    };

    let bridged = bridge_heat_result_to_thermal_plane_quad_model(
        &heat_result,
        &SolveThermalPlaneQuad2dRequest {
            nodes: vec![
                ThermalPlaneNodeInput {
                    id: "n0".to_string(),
                    x: 0.0,
                    y: 0.0,
                    fix_x: true,
                    fix_y: true,
                    load_x: 0.0,
                    load_y: 0.0,
                    temperature_delta: 0.0,
                },
                ThermalPlaneNodeInput {
                    id: "n1".to_string(),
                    x: 1.0,
                    y: 0.0,
                    fix_x: true,
                    fix_y: true,
                    load_x: 0.0,
                    load_y: 0.0,
                    temperature_delta: 0.0,
                },
                ThermalPlaneNodeInput {
                    id: "n2".to_string(),
                    x: 1.0,
                    y: 1.0,
                    fix_x: true,
                    fix_y: true,
                    load_x: 0.0,
                    load_y: 0.0,
                    temperature_delta: 0.0,
                },
                ThermalPlaneNodeInput {
                    id: "n3".to_string(),
                    x: 0.0,
                    y: 1.0,
                    fix_x: true,
                    fix_y: true,
                    load_x: 0.0,
                    load_y: 0.0,
                    temperature_delta: 0.0,
                },
            ],
            elements: vec![ThermalPlaneQuadElementInput {
                id: "tq0".to_string(),
                node_i: 0,
                node_j: 1,
                node_k: 2,
                node_l: 3,
                thickness: 0.02,
                youngs_modulus: 70.0e9,
                poisson_ratio: 0.33,
                thermal_expansion: 11.0e-6,
            }],
        },
    )
    .expect("bridge should build");

    assert_eq!(bridged.nodes[0].temperature_delta, 100.0);
    assert_eq!(bridged.nodes[1].temperature_delta, 60.0);
    assert_eq!(bridged.nodes[2].temperature_delta, 20.0);
    assert_eq!(bridged.nodes[3].temperature_delta, 20.0);
}

#[test]
fn runs_heat_to_thermo_plane_quad_workflow() {
    let result =
        run_heat_to_thermo_plane_quad_2d_workflow(HeatToThermoPlaneQuad2dWorkflowRequest {
            heat_model: SolveHeatPlaneQuad2dRequest {
                nodes: vec![
                    HeatPlaneNodeInput {
                        id: "h0".to_string(),
                        x: 0.0,
                        y: 0.0,
                        fix_temperature: true,
                        temperature: 100.0,
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
                        fix_temperature: true,
                        temperature: 20.0,
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
            thermo_seed_model: SolveThermalPlaneQuad2dRequest {
                nodes: vec![
                    ThermalPlaneNodeInput {
                        id: "n0".to_string(),
                        x: 0.0,
                        y: 0.0,
                        fix_x: true,
                        fix_y: true,
                        load_x: 0.0,
                        load_y: 0.0,
                        temperature_delta: 30.0,
                    },
                    ThermalPlaneNodeInput {
                        id: "n1".to_string(),
                        x: 1.0,
                        y: 0.0,
                        fix_x: true,
                        fix_y: true,
                        load_x: 0.0,
                        load_y: 0.0,
                        temperature_delta: 30.0,
                    },
                    ThermalPlaneNodeInput {
                        id: "n2".to_string(),
                        x: 1.0,
                        y: 1.0,
                        fix_x: true,
                        fix_y: true,
                        load_x: 0.0,
                        load_y: 0.0,
                        temperature_delta: 30.0,
                    },
                    ThermalPlaneNodeInput {
                        id: "n3".to_string(),
                        x: 0.0,
                        y: 1.0,
                        fix_x: true,
                        fix_y: true,
                        load_x: 0.0,
                        load_y: 0.0,
                        temperature_delta: 30.0,
                    },
                ],
                elements: vec![ThermalPlaneQuadElementInput {
                    id: "tq0".to_string(),
                    node_i: 0,
                    node_j: 1,
                    node_k: 2,
                    node_l: 3,
                    thickness: 0.02,
                    youngs_modulus: 70.0e9,
                    poisson_ratio: 0.33,
                    thermal_expansion: 11.0e-6,
                }],
            },
        })
        .expect("workflow should run");

    assert_eq!(result.workflow_id, "workflow.heat-to-thermo-quad-2d");
    assert_eq!(result.heat_result.max_temperature, 100.0);
    assert_eq!(result.bridged_model.nodes[1].temperature_delta, 60.0);
    assert_eq!(result.thermo_result.max_temperature_delta, 100.0);
    assert!(result.thermo_result.max_stress > 0.0);
}
