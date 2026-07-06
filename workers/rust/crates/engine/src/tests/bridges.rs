use crate::workflow_executor::run_transform_operator;
use crate::{
    EngineSolveRequest, bridge_heat_result_to_thermal_plane_quad_model,
    bridge_heat_result_to_thermal_plane_triangle_model, run_heat_to_thermo_plane_quad_2d_workflow,
    run_heat_to_thermo_plane_triangle_2d_workflow, solve,
};
use kyuubiki_protocol::{
    AnalysisResult, HeatPlaneNodeInput, HeatPlaneQuadElementInput, HeatPlaneTriangleElementInput,
    HeatToThermoPlaneQuad2dWorkflowRequest, HeatToThermoPlaneTriangle2dWorkflowRequest,
    SolveHeatPlaneQuad2dRequest, SolveHeatPlaneTriangle2dRequest, SolveThermalPlaneQuad2dRequest,
    SolveThermalPlaneTriangle2dRequest, ThermalPlaneNodeInput, ThermalPlaneQuadElementInput,
    ThermalPlaneTriangleElementInput,
};

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

    let (bridged, _) = bridge_heat_result_to_thermal_plane_quad_model(
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

#[test]
fn bridges_heat_triangle_temperatures_into_thermo_model() {
    let solved = solve(EngineSolveRequest::HeatPlaneTriangle2d(
        SolveHeatPlaneTriangle2dRequest {
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
                    fix_temperature: true,
                    temperature: 20.0,
                    heat_load: 0.0,
                },
                HeatPlaneNodeInput {
                    id: "h2".to_string(),
                    x: 0.0,
                    y: 1.0,
                    fix_temperature: true,
                    temperature: 40.0,
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
    ))
    .expect("heat triangle should solve");
    let heat_result = match solved {
        AnalysisResult::HeatPlaneTriangle2d(result) => result,
        _ => unreachable!("expected heat triangle result"),
    };

    let (bridged, diagnostics) = bridge_heat_result_to_thermal_plane_triangle_model(
        &heat_result,
        &SolveThermalPlaneTriangle2dRequest {
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
                    x: 0.0,
                    y: 1.0,
                    fix_x: true,
                    fix_y: true,
                    load_x: 0.0,
                    load_y: 0.0,
                    temperature_delta: 0.0,
                },
            ],
            elements: vec![ThermalPlaneTriangleElementInput {
                id: "tt0".to_string(),
                node_i: 0,
                node_j: 1,
                node_k: 2,
                thickness: 0.02,
                youngs_modulus: 70.0e9,
                poisson_ratio: 0.33,
                thermal_expansion: 11.0e-6,
            }],
        },
    )
    .expect("triangle bridge should build");

    assert_eq!(diagnostics.bridge_kind, "heat_to_thermo_triangle_2d");
    assert_eq!(bridged.nodes[0].temperature_delta, 100.0);
    assert_eq!(bridged.nodes[1].temperature_delta, 20.0);
    assert_eq!(bridged.nodes[2].temperature_delta, 40.0);
}

#[test]
fn runs_heat_to_thermo_plane_triangle_workflow() {
    let result =
        run_heat_to_thermo_plane_triangle_2d_workflow(HeatToThermoPlaneTriangle2dWorkflowRequest {
            heat_model: SolveHeatPlaneTriangle2dRequest {
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
                        fix_temperature: true,
                        temperature: 20.0,
                        heat_load: 0.0,
                    },
                    HeatPlaneNodeInput {
                        id: "h2".to_string(),
                        x: 0.0,
                        y: 1.0,
                        fix_temperature: true,
                        temperature: 40.0,
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
            thermo_seed_model: SolveThermalPlaneTriangle2dRequest {
                nodes: vec![
                    ThermalPlaneNodeInput {
                        id: "n0".to_string(),
                        x: 0.0,
                        y: 0.0,
                        fix_x: true,
                        fix_y: true,
                        load_x: 0.0,
                        load_y: 0.0,
                        temperature_delta: 5.0,
                    },
                    ThermalPlaneNodeInput {
                        id: "n1".to_string(),
                        x: 1.0,
                        y: 0.0,
                        fix_x: true,
                        fix_y: true,
                        load_x: 0.0,
                        load_y: 0.0,
                        temperature_delta: 5.0,
                    },
                    ThermalPlaneNodeInput {
                        id: "n2".to_string(),
                        x: 0.0,
                        y: 1.0,
                        fix_x: true,
                        fix_y: true,
                        load_x: 0.0,
                        load_y: 0.0,
                        temperature_delta: 5.0,
                    },
                ],
                elements: vec![ThermalPlaneTriangleElementInput {
                    id: "tt0".to_string(),
                    node_i: 0,
                    node_j: 1,
                    node_k: 2,
                    thickness: 0.02,
                    youngs_modulus: 70.0e9,
                    poisson_ratio: 0.33,
                    thermal_expansion: 11.0e-6,
                }],
            },
        })
        .expect("triangle workflow should run");

    assert_eq!(result.workflow_id, "workflow.heat-to-thermo-triangle-2d");
    assert_eq!(result.heat_result.max_temperature, 100.0);
    assert_eq!(result.bridged_model.nodes[0].temperature_delta, 100.0);
    assert_eq!(result.thermo_result.max_temperature_delta, 100.0);
    assert!(result.thermo_result.max_stress > 0.0);
}

#[test]
fn heat_to_thermo_transform_respects_contract_and_seed_model_wrapper() {
    let solved = solve(EngineSolveRequest::HeatPlaneQuad2d(
        SolveHeatPlaneQuad2dRequest {
            nodes: vec![
                HeatPlaneNodeInput {
                    id: "h0".to_string(),
                    x: 0.0,
                    y: 0.0,
                    fix_temperature: true,
                    temperature: 10.0,
                    heat_load: 6.0,
                },
                HeatPlaneNodeInput {
                    id: "h1".to_string(),
                    x: 1.0,
                    y: 0.0,
                    fix_temperature: true,
                    temperature: 20.0,
                    heat_load: 12.0,
                },
                HeatPlaneNodeInput {
                    id: "h2".to_string(),
                    x: 1.0,
                    y: 1.0,
                    fix_temperature: true,
                    temperature: 30.0,
                    heat_load: 18.0,
                },
                HeatPlaneNodeInput {
                    id: "h3".to_string(),
                    x: 0.0,
                    y: 1.0,
                    fix_temperature: true,
                    temperature: 40.0,
                    heat_load: 24.0,
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

    let bridged = run_transform_operator(
        "bridge.temperature_field_to_thermo_quad_2d",
        serde_json::to_value(heat_result).expect("heat result should serialize"),
        serde_json::json!({
            "seed_model": {
                "nodes": [
                    { "id": "n0", "x": 0.0, "y": 0.0, "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0, "temperature_delta": 0.0 },
                    { "id": "n1", "x": 1.0, "y": 0.0, "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0, "temperature_delta": 0.0 },
                    { "id": "n2", "x": 1.0, "y": 1.0, "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0, "temperature_delta": 0.0 },
                    { "id": "n3", "x": 0.0, "y": 1.0, "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0, "temperature_delta": 0.0 }
                ],
                "elements": [
                    { "id": "tq0", "node_i": 0, "node_j": 1, "node_k": 2, "node_l": 3, "thickness": 0.02, "youngs_modulus": 70000000000.0, "poisson_ratio": 0.33, "thermal_expansion": 0.000011 }
                ]
            },
            "contract": {
                "source": { "field": "heat_load" },
                "transform": { "scale": 0.5, "default_value": 0.0 },
                "target": { "field": "temperature_delta" }
            }
        }),
    )
    .expect("transform operator should apply contract");

    let nodes = bridged["nodes"]
        .as_array()
        .expect("bridged nodes should exist");
    assert_eq!(nodes[0]["temperature_delta"], serde_json::json!(3.0));
    assert_eq!(nodes[1]["temperature_delta"], serde_json::json!(6.0));
    assert_eq!(nodes[2]["temperature_delta"], serde_json::json!(9.0));
    assert_eq!(nodes[3]["temperature_delta"], serde_json::json!(12.0));
    assert_eq!(
        bridged["__bridge_diagnostics"]["source_field"],
        serde_json::json!("heat_load")
    );
    assert_eq!(
        bridged["__bridge_diagnostics"]["scale"],
        serde_json::json!(0.5)
    );
}
