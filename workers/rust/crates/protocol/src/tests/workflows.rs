use super::prelude::*;

#[test]
fn serializes_heat_to_thermo_plane_quad_workflow_round_trip() {
    let request = HeatToThermoPlaneQuad2dWorkflowRequest {
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
                    fix_temperature: true,
                    temperature: 20.0,
                    heat_load: 0.0,
                },
            ],
            elements: vec![HeatPlaneQuadElementInput {
                id: "hq0".to_string(),
                node_i: 0,
                node_j: 1,
                node_k: 1,
                node_l: 0,
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
            ],
            elements: vec![ThermalPlaneQuadElementInput {
                id: "tq0".to_string(),
                node_i: 0,
                node_j: 1,
                node_k: 1,
                node_l: 0,
                thickness: 0.02,
                youngs_modulus: 70.0e9,
                poisson_ratio: 0.33,
                thermal_expansion: 11.0e-6,
            }],
        },
    };

    let json = serde_json::to_string(&request).expect("workflow request should serialize");
    let decoded: HeatToThermoPlaneQuad2dWorkflowRequest =
        serde_json::from_str(&json).expect("workflow request should decode");
    assert_eq!(decoded.heat_model.nodes.len(), 2);

    let result = HeatToThermoPlaneQuad2dWorkflowResult {
        workflow_id: "workflow.heat-to-thermo-quad-2d".to_string(),
        heat_result: SolveHeatPlaneQuad2dResult {
            input: decoded.heat_model.clone(),
            nodes: vec![HeatPlaneNodeResult {
                index: 0,
                id: "h0".to_string(),
                x: 0.0,
                y: 0.0,
                temperature: 100.0,
                heat_load: 0.0,
            }],
            elements: vec![HeatPlaneQuadElementResult {
                index: 0,
                id: "hq0".to_string(),
                node_i: 0,
                node_j: 1,
                node_k: 1,
                node_l: 0,
                area: 0.02,
                average_temperature: 60.0,
                temperature_gradient_x: -40.0,
                temperature_gradient_y: 0.0,
                heat_flux_x: 1800.0,
                heat_flux_y: 0.0,
                heat_flux_magnitude: 1800.0,
            }],
            max_temperature: 100.0,
            max_heat_flux: 1800.0,
        },
        bridged_model: decoded.thermo_seed_model.clone(),
        thermo_result: SolveThermalPlaneQuad2dResult {
            input: decoded.thermo_seed_model,
            nodes: vec![ThermalPlaneNodeResult {
                index: 0,
                id: "n0".to_string(),
                x: 0.0,
                y: 0.0,
                ux: 0.0,
                uy: 0.0,
                displacement_magnitude: 0.0,
                temperature_delta: 80.0,
            }],
            elements: vec![ThermalPlaneQuadElementResult {
                index: 0,
                id: "tq0".to_string(),
                node_i: 0,
                node_j: 1,
                node_k: 1,
                node_l: 0,
                area: 0.02,
                average_temperature_delta: 80.0,
                thermal_strain: 8.8e-4,
                mechanical_strain_x: 0.0,
                mechanical_strain_y: 0.0,
                total_strain_x: 0.0,
                total_strain_y: 0.0,
                gamma_xy: 0.0,
                stress_x: -1.0,
                stress_y: -1.0,
                tau_xy: 0.0,
                principal_stress_1: -1.0,
                principal_stress_2: -1.0,
                max_in_plane_shear: 0.0,
                von_mises: 1.0,
            }],
            max_displacement: 0.0,
            max_stress: 1.0,
            max_temperature_delta: 80.0,
        },
    };

    let json = serde_json::to_string(&result).expect("workflow result should serialize");
    let decoded: HeatToThermoPlaneQuad2dWorkflowResult =
        serde_json::from_str(&json).expect("workflow result should decode");
    assert_eq!(decoded.workflow_id, "workflow.heat-to-thermo-quad-2d");
    assert_eq!(decoded.thermo_result.max_temperature_delta, 80.0);
}

#[test]
fn serializes_heat_to_thermo_plane_triangle_workflow_round_trip() {
    let request = HeatToThermoPlaneTriangle2dWorkflowRequest {
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
    };

    let json = serde_json::to_string(&request).expect("workflow request should serialize");
    let decoded: HeatToThermoPlaneTriangle2dWorkflowRequest =
        serde_json::from_str(&json).expect("workflow request should decode");
    assert_eq!(decoded.heat_model.nodes.len(), 3);

    let result = HeatToThermoPlaneTriangle2dWorkflowResult {
        workflow_id: "workflow.heat-to-thermo-triangle-2d".to_string(),
        heat_result: SolveHeatPlaneTriangle2dResult {
            input: decoded.heat_model.clone(),
            nodes: vec![HeatPlaneNodeResult {
                index: 0,
                id: "h0".to_string(),
                x: 0.0,
                y: 0.0,
                temperature: 100.0,
                heat_load: 0.0,
            }],
            elements: vec![HeatPlaneTriangleElementResult {
                index: 0,
                id: "ht0".to_string(),
                node_i: 0,
                node_j: 1,
                node_k: 2,
                area: 0.5,
                average_temperature: 53.333333333333336,
                temperature_gradient_x: -80.0,
                temperature_gradient_y: -60.0,
                heat_flux_x: 3600.0,
                heat_flux_y: 2700.0,
                heat_flux_magnitude: 4500.0,
            }],
            max_temperature: 100.0,
            max_heat_flux: 4500.0,
        },
        bridged_model: decoded.thermo_seed_model.clone(),
        thermo_result: SolveThermalPlaneTriangle2dResult {
            input: decoded.thermo_seed_model,
            nodes: vec![ThermalPlaneNodeResult {
                index: 0,
                id: "n0".to_string(),
                x: 0.0,
                y: 0.0,
                ux: 0.0,
                uy: 0.0,
                displacement_magnitude: 0.0,
                temperature_delta: 80.0,
            }],
            elements: vec![ThermalPlaneTriangleElementResult {
                index: 0,
                id: "tt0".to_string(),
                node_i: 0,
                node_j: 1,
                node_k: 2,
                area: 0.5,
                average_temperature_delta: 80.0,
                thermal_strain: 8.8e-4,
                mechanical_strain_x: 0.0,
                mechanical_strain_y: 0.0,
                total_strain_x: 0.0,
                total_strain_y: 0.0,
                gamma_xy: 0.0,
                stress_x: -1.0,
                stress_y: -1.0,
                tau_xy: 0.0,
                principal_stress_1: -1.0,
                principal_stress_2: -1.0,
                max_in_plane_shear: 0.0,
                von_mises: 1.0,
            }],
            max_displacement: 0.0,
            max_stress: 1.0,
            max_temperature_delta: 80.0,
        },
    };

    let json = serde_json::to_string(&result).expect("workflow result should serialize");
    let decoded: HeatToThermoPlaneTriangle2dWorkflowResult =
        serde_json::from_str(&json).expect("workflow result should decode");
    assert_eq!(decoded.workflow_id, "workflow.heat-to-thermo-triangle-2d");
    assert_eq!(decoded.thermo_result.max_temperature_delta, 80.0);
}

#[test]
fn serializes_workflow_graph_run_request_round_trip() {
    let dataset_contract = WorkflowDatasetContract {
        id: "dataset.heat_to_thermo_quad/v1".to_string(),
        version: "1.0.0".to_string(),
        values: vec![
            WorkflowDatasetValueInfo {
                id: "heat_model".to_string(),
                data_class: "study_model".to_string(),
                element_type: "json_object".to_string(),
                shape: WorkflowDatasetShape {
                    axes: vec![WorkflowDatasetAxis {
                        id: "elements".to_string(),
                        label: Some("quad elements".to_string()),
                        size: None,
                        semantic: Some("mesh_element".to_string()),
                    }],
                },
                semantic_type: Some("study_model/heat_plane_quad_2d".to_string()),
                unit: None,
                encoding: Some(WorkflowDatasetEncoding::Json),
                schema_ref: Some(OperatorSchemaRef {
                    schema: "kyuubiki.operator.solve.heat_plane_quad_2d.input".to_string(),
                    version: "1".to_string(),
                }),
            },
            WorkflowDatasetValueInfo {
                id: "thermo_result".to_string(),
                data_class: "result".to_string(),
                element_type: "json_object".to_string(),
                shape: WorkflowDatasetShape::default(),
                semantic_type: Some("result/thermal_plane_quad_2d".to_string()),
                unit: None,
                encoding: Some(WorkflowDatasetEncoding::Json),
                schema_ref: Some(OperatorSchemaRef {
                    schema: "kyuubiki.operator.solve.thermal_plane_quad_2d.output".to_string(),
                    version: "1".to_string(),
                }),
            },
        ],
        metadata: std::collections::BTreeMap::from([(
            "philosophy".to_string(),
            "onnx_like_cross_operator_contract".to_string(),
        )]),
    };

    let graph = WorkflowGraph {
        schema_version: "kyuubiki.workflow-graph/v1".to_string(),
        id: "workflow.heat-to-thermo-quad-2d".to_string(),
        name: "Heat to thermo-mechanical quad".to_string(),
        version: "1.0.0".to_string(),
        description: Some("Reference headless graph".to_string()),
        dataset_contract: Some(dataset_contract),
        entry_nodes: vec!["heat_model".to_string()],
        output_nodes: vec!["thermo_summary".to_string()],
        defaults: WorkflowDefaults {
            cache_policy: Some(WorkflowCachePolicy::Cached),
            orchestrated: Some(true),
        },
        nodes: vec![
            WorkflowNode {
                id: "heat_model".to_string(),
                kind: WorkflowNodeKind::Input,
                operator_id: None,
                name: Some("Heat input".to_string()),
                description: None,
                config: None,
                cache_policy: None,
                inputs: vec![],
                outputs: vec![WorkflowPort {
                    id: "model".to_string(),
                    artifact_type: "study_model/heat_plane_quad_2d".to_string(),
                    name: None,
                    required: None,
                    cardinality: None,
                    dataset_value: Some("heat_model".to_string()),
                }],
            },
            WorkflowNode {
                id: "thermo_summary".to_string(),
                kind: WorkflowNodeKind::Output,
                operator_id: None,
                name: Some("Thermo summary".to_string()),
                description: None,
                config: None,
                cache_policy: None,
                inputs: vec![WorkflowPort {
                    id: "result".to_string(),
                    artifact_type: "result/thermal_plane_quad_2d".to_string(),
                    name: None,
                    required: None,
                    cardinality: None,
                    dataset_value: Some("thermo_result".to_string()),
                }],
                outputs: vec![],
            },
        ],
        edges: vec![WorkflowEdge {
            id: "edge-1".to_string(),
            from: WorkflowNodePortRef {
                node: "heat_model".to_string(),
                port: "model".to_string(),
            },
            to: WorkflowNodePortRef {
                node: "thermo_summary".to_string(),
                port: "result".to_string(),
            },
            artifact_type: "result/thermal_plane_quad_2d".to_string(),
            dataset_value: Some("thermo_result".to_string()),
        }],
    };

    let request = WorkflowGraphRunRequest {
        graph,
        input_artifacts: std::collections::BTreeMap::from([(
            "heat_model".to_string(),
            serde_json::json!({"kind": "heat_plane_quad_2d"}),
        )]),
    };

    let json = serde_json::to_string(&request).expect("workflow graph request should serialize");
    let decoded: WorkflowGraphRunRequest =
        serde_json::from_str(&json).expect("workflow graph request should decode");
    assert_eq!(decoded.graph.id, "workflow.heat-to-thermo-quad-2d");
    assert_eq!(decoded.input_artifacts.len(), 1);
    assert_eq!(
        decoded
            .graph
            .dataset_contract
            .as_ref()
            .expect("dataset contract")
            .values
            .len(),
        2
    );

    let result = WorkflowGraphRunResult {
        workflow_id: decoded.graph.id,
        completed_nodes: vec!["heat_model".to_string(), "thermo_summary".to_string()],
        skipped_nodes: vec![],
        progress_events: vec![],
        branch_decisions: vec![],
        node_runs: vec![],
        artifact_lineage: vec![],
        artifacts: std::collections::BTreeMap::from([(
            "thermo_summary.result".to_string(),
            serde_json::json!({"max_stress": 123.0}),
        )]),
    };
    let json = serde_json::to_string(&result).expect("workflow graph result should serialize");
    let decoded: WorkflowGraphRunResult =
        serde_json::from_str(&json).expect("workflow graph result should decode");
    assert_eq!(decoded.completed_nodes.len(), 2);
    assert_eq!(decoded.skipped_nodes.len(), 0);
    assert_eq!(decoded.progress_events.len(), 0);
    assert_eq!(decoded.node_runs.len(), 0);
}
