use kyuubiki_protocol::{
    AnalysisResult, HeatToThermoPlaneQuad2dWorkflowRequest,
    HeatToThermoPlaneQuad2dWorkflowResult, OperatorDescriptor, OperatorKind, OperatorOrigin,
    OperatorSchemaRef, ResultChunkKind, ResultChunkRequest, ResultChunkResponse, SolveBarRequest,
    SolveBeam1dRequest, SolveFrame2dRequest, SolveFrame3dRequest, SolveHeatBar1dRequest,
    SolveHeatPlaneQuad2dRequest, SolveHeatPlaneTriangle2dRequest, SolvePlaneQuad2dRequest,
    SolvePlaneTriangle2dRequest, SolveSpring1dRequest, SolveSpring2dRequest,
    SolveSpring3dRequest, SolveThermalBar1dRequest, SolveThermalBeam1dRequest,
    SolveThermalFrame2dRequest, SolveThermalFrame3dRequest, SolveThermalPlaneQuad2dRequest,
    SolveThermalPlaneTriangle2dRequest, SolveThermalTruss2dRequest, SolveThermalTruss3dRequest,
    SolveTorsion1dRequest, SolveTruss2dRequest, SolveTruss3dRequest, WorkflowGraph,
    WorkflowGraphRunRequest, WorkflowGraphRunResult, WorkflowNodeKind,
};
use std::collections::{BTreeMap, HashMap, HashSet};
use kyuubiki_solver::{
    solve_bar_1d, solve_beam_1d, solve_frame_2d, solve_frame_3d, solve_heat_bar_1d, solve_heat_plane_quad_2d,
    solve_heat_plane_triangle_2d, solve_plane_quad_2d, solve_plane_triangle_2d, solve_spring_1d,
    solve_spring_2d, solve_spring_3d, solve_thermal_bar_1d, solve_thermal_beam_1d, solve_thermal_frame_2d, solve_thermal_frame_3d,
    solve_thermal_plane_quad_2d, solve_thermal_plane_triangle_2d, solve_thermal_truss_2d,
    solve_thermal_truss_3d, solve_torsion_1d, solve_truss_2d, solve_truss_3d,
};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
pub enum EngineSolveRequest {
    Bar1d(SolveBarRequest),
    ThermalBar1d(SolveThermalBar1dRequest),
    HeatBar1d(SolveHeatBar1dRequest),
    HeatPlaneTriangle2d(SolveHeatPlaneTriangle2dRequest),
    HeatPlaneQuad2d(SolveHeatPlaneQuad2dRequest),
    ThermalTruss2d(SolveThermalTruss2dRequest),
    ThermalTruss3d(SolveThermalTruss3dRequest),
    Spring1d(SolveSpring1dRequest),
    Spring2d(SolveSpring2dRequest),
    Spring3d(SolveSpring3dRequest),
    Beam1d(SolveBeam1dRequest),
    ThermalBeam1d(SolveThermalBeam1dRequest),
    ThermalFrame2d(SolveThermalFrame2dRequest),
    ThermalFrame3d(SolveThermalFrame3dRequest),
    Torsion1d(SolveTorsion1dRequest),
    Truss2d(SolveTruss2dRequest),
    Truss3d(SolveTruss3dRequest),
    Frame3d(SolveFrame3dRequest),
    PlaneTriangle2d(SolvePlaneTriangle2dRequest),
    ThermalPlaneTriangle2d(SolveThermalPlaneTriangle2dRequest),
    PlaneQuad2d(SolvePlaneQuad2dRequest),
    ThermalPlaneQuad2d(SolveThermalPlaneQuad2dRequest),
    Frame2d(SolveFrame2dRequest),
}

pub fn built_in_operator_descriptors() -> Vec<OperatorDescriptor> {
    vec![
        built_in_solver_descriptor(
            "solve.frame_3d",
            "mechanical",
            "frame_3d",
            "Solve a 3D frame model with six-DOF nodes and verified baseline coverage.",
            &["verified", "mechanical", "frame", "3d"],
        ),
        built_in_solver_descriptor(
            "solve.thermal_frame_3d",
            "thermo_mechanical",
            "thermal_frame_3d",
            "Solve a thermal 3D frame model with restrained expansion and temperature gradients.",
            &["verified", "thermo_mechanical", "frame", "3d"],
        ),
        built_in_solver_descriptor(
            "solve.heat_plane_quad_2d",
            "thermal",
            "heat_plane_quad_2d",
            "Solve a 2D heat-conduction quad model and expose verified temperature/flux fields.",
            &["verified", "thermal", "heat", "plane", "2d"],
        ),
        built_in_solver_descriptor(
            "solve.thermal_truss_3d",
            "thermo_mechanical",
            "thermal_truss_3d",
            "Solve a thermal 3D truss model with expansion-driven axial response.",
            &["verified", "thermo_mechanical", "truss", "3d"],
        ),
        built_in_bridge_descriptor(
            "bridge.temperature_field_to_thermo_quad_2d",
            "thermo_mechanical",
            "thermal_plane_quad_2d",
            "Bridge a heat quad temperature field into a thermal quad structural model.",
            &["workflow_bridge", "temperature_field", "quad", "2d"],
        ),
        built_in_extract_descriptor(
            "extract.result_summary",
            "multi_domain",
            "result_summary",
            "Extract a compact summary from a solver result artifact.",
            &["extract", "summary", "headless_safe"],
        ),
        built_in_export_descriptor(
            "export.summary_json",
            "multi_domain",
            "summary_json",
            "Export a compact summary artifact as structured JSON content.",
            &["export", "json", "summary", "headless_safe"],
        ),
        built_in_export_descriptor(
            "export.summary_csv",
            "multi_domain",
            "summary_csv",
            "Export a compact summary artifact as CSV text for downstream delivery.",
            &["export", "csv", "summary", "headless_safe"],
        ),
    ]
}

pub fn describe_built_in_operator(id: &str) -> Option<OperatorDescriptor> {
    built_in_operator_descriptors()
        .into_iter()
        .find(|descriptor| descriptor.id == id)
}

pub fn solve(request: EngineSolveRequest) -> Result<AnalysisResult, String> {
    match request {
        EngineSolveRequest::Bar1d(request) => solve_bar_1d(&request).map(AnalysisResult::Bar1d),
        EngineSolveRequest::ThermalBar1d(request) => {
            solve_thermal_bar_1d(&request).map(AnalysisResult::ThermalBar1d)
        }
        EngineSolveRequest::HeatBar1d(request) => {
            solve_heat_bar_1d(&request).map(AnalysisResult::HeatBar1d)
        }
        EngineSolveRequest::HeatPlaneTriangle2d(request) => {
            solve_heat_plane_triangle_2d(&request).map(AnalysisResult::HeatPlaneTriangle2d)
        }
        EngineSolveRequest::HeatPlaneQuad2d(request) => {
            solve_heat_plane_quad_2d(&request).map(AnalysisResult::HeatPlaneQuad2d)
        }
        EngineSolveRequest::ThermalTruss2d(request) => {
            solve_thermal_truss_2d(&request).map(AnalysisResult::ThermalTruss2d)
        }
        EngineSolveRequest::ThermalTruss3d(request) => {
            solve_thermal_truss_3d(&request).map(AnalysisResult::ThermalTruss3d)
        }
        EngineSolveRequest::Spring1d(request) => {
            solve_spring_1d(&request).map(AnalysisResult::Spring1d)
        }
        EngineSolveRequest::Spring2d(request) => {
            solve_spring_2d(&request).map(AnalysisResult::Spring2d)
        }
        EngineSolveRequest::Spring3d(request) => {
            solve_spring_3d(&request).map(AnalysisResult::Spring3d)
        }
        EngineSolveRequest::Beam1d(request) => solve_beam_1d(&request).map(AnalysisResult::Beam1d),
        EngineSolveRequest::ThermalBeam1d(request) => {
            solve_thermal_beam_1d(&request).map(AnalysisResult::ThermalBeam1d)
        }
        EngineSolveRequest::ThermalFrame2d(request) => {
            solve_thermal_frame_2d(&request).map(AnalysisResult::ThermalFrame2d)
        }
        EngineSolveRequest::ThermalFrame3d(request) => {
            solve_thermal_frame_3d(&request).map(AnalysisResult::ThermalFrame3d)
        }
        EngineSolveRequest::Torsion1d(request) => {
            solve_torsion_1d(&request).map(AnalysisResult::Torsion1d)
        }
        EngineSolveRequest::Truss2d(request) => {
            solve_truss_2d(&request).map(AnalysisResult::Truss2d)
        }
        EngineSolveRequest::Truss3d(request) => {
            solve_truss_3d(&request).map(AnalysisResult::Truss3d)
        }
        EngineSolveRequest::Frame3d(request) => {
            solve_frame_3d(&request).map(AnalysisResult::Frame3d)
        }
        EngineSolveRequest::PlaneTriangle2d(request) => {
            solve_plane_triangle_2d(&request).map(AnalysisResult::PlaneTriangle2d)
        }
        EngineSolveRequest::ThermalPlaneTriangle2d(request) => {
            solve_thermal_plane_triangle_2d(&request).map(AnalysisResult::ThermalPlaneTriangle2d)
        }
        EngineSolveRequest::PlaneQuad2d(request) => {
            solve_plane_quad_2d(&request).map(AnalysisResult::PlaneQuad2d)
        }
        EngineSolveRequest::ThermalPlaneQuad2d(request) => {
            solve_thermal_plane_quad_2d(&request).map(AnalysisResult::ThermalPlaneQuad2d)
        }
        EngineSolveRequest::Frame2d(request) => {
            solve_frame_2d(&request).map(AnalysisResult::Frame2d)
        }
    }
}

pub fn bridge_heat_result_to_thermal_plane_quad_model(
    heat_result: &kyuubiki_protocol::SolveHeatPlaneQuad2dResult,
    thermo_seed_model: &SolveThermalPlaneQuad2dRequest,
) -> Result<SolveThermalPlaneQuad2dRequest, String> {
    if heat_result.nodes.len() != thermo_seed_model.nodes.len() {
        return Err("heat and thermo quad models must have the same node count".to_string());
    }
    if heat_result.input.elements.len() != thermo_seed_model.elements.len() {
        return Err("heat and thermo quad models must have the same element count".to_string());
    }

    let mut bridged = thermo_seed_model.clone();
    for (heat_node, thermo_node) in heat_result.nodes.iter().zip(bridged.nodes.iter_mut()) {
        if (heat_node.x - thermo_node.x).abs() > 1.0e-9
            || (heat_node.y - thermo_node.y).abs() > 1.0e-9
        {
            return Err(format!(
                "heat node {} does not align with thermo node {}",
                heat_node.id, thermo_node.id
            ));
        }
        thermo_node.temperature_delta = heat_node.temperature;
    }

    Ok(bridged)
}

pub fn run_heat_to_thermo_plane_quad_2d_workflow(
    request: HeatToThermoPlaneQuad2dWorkflowRequest,
) -> Result<HeatToThermoPlaneQuad2dWorkflowResult, String> {
    let heat_result = match solve(EngineSolveRequest::HeatPlaneQuad2d(request.heat_model))? {
        AnalysisResult::HeatPlaneQuad2d(result) => result,
        _ => return Err("heat-plane-quad workflow produced an unexpected heat result".to_string()),
    };

    let bridged_model =
        bridge_heat_result_to_thermal_plane_quad_model(&heat_result, &request.thermo_seed_model)?;
    let thermo_result = match solve(EngineSolveRequest::ThermalPlaneQuad2d(bridged_model.clone()))?
    {
        AnalysisResult::ThermalPlaneQuad2d(result) => result,
        _ => {
            return Err(
                "heat-to-thermo workflow produced an unexpected thermo result".to_string()
            )
        }
    };

    Ok(HeatToThermoPlaneQuad2dWorkflowResult {
        workflow_id: "workflow.heat-to-thermo-quad-2d".to_string(),
        heat_result,
        bridged_model,
        thermo_result,
    })
}

pub fn run_workflow_graph(
    request: WorkflowGraphRunRequest,
) -> Result<WorkflowGraphRunResult, String> {
    let graph = request.graph;
    validate_workflow_dataset_contract(&graph)?;
    let node_map = graph
        .nodes
        .iter()
        .map(|node| (node.id.clone(), node))
        .collect::<HashMap<_, _>>();
    let mut completed = HashSet::new();
    let mut ordered_completed = Vec::new();
    let mut artifacts = BTreeMap::new();

    loop {
        let mut progressed = false;

        for node in &graph.nodes {
            if completed.contains(&node.id) {
                continue;
            }

            let incoming = graph
                .edges
                .iter()
                .filter(|edge| edge.to.node == node.id)
                .collect::<Vec<_>>();
            let ready = incoming.iter().all(|edge| {
                artifacts.contains_key(&artifact_key(&edge.from.node, &edge.from.port))
            });

            if node.kind != WorkflowNodeKind::Input && !ready {
                continue;
            }

            match node.kind {
                WorkflowNodeKind::Input => {
                    let value = request
                        .input_artifacts
                        .get(&node.id)
                        .cloned()
                        .ok_or_else(|| format!("missing workflow input artifact for node {}", node.id))?;
                    for output in &node.outputs {
                        artifacts.insert(artifact_key(&node.id, &output.id), value.clone());
                    }
                }
                WorkflowNodeKind::Solve => {
                    let operator_id = node
                        .operator_id
                        .as_deref()
                        .ok_or_else(|| format!("workflow solve node {} is missing operator_id", node.id))?;
                    let payload = resolve_single_input_payload(node, &incoming, &artifacts)?;
                    let output_value = run_solve_operator(operator_id, payload)?;
                    for output in &node.outputs {
                        artifacts.insert(artifact_key(&node.id, &output.id), output_value.clone());
                    }
                }
                WorkflowNodeKind::Transform => {
                    let operator_id = node.operator_id.as_deref().ok_or_else(|| {
                        format!("workflow transform node {} is missing operator_id", node.id)
                    })?;
                    let payload = resolve_single_input_payload(node, &incoming, &artifacts)?;
                    let output_value = run_transform_operator(
                        operator_id,
                        payload,
                        node.config.clone().unwrap_or(Value::Null),
                    )?;
                    for output in &node.outputs {
                        artifacts.insert(artifact_key(&node.id, &output.id), output_value.clone());
                    }
                }
                WorkflowNodeKind::Extract => {
                    let operator_id = node.operator_id.as_deref().ok_or_else(|| {
                        format!("workflow extract node {} is missing operator_id", node.id)
                    })?;
                    let payload = resolve_single_input_payload(node, &incoming, &artifacts)?;
                    let output_value = run_extract_operator(
                        operator_id,
                        payload,
                        node.config.clone().unwrap_or(Value::Null),
                    )?;
                    for output in &node.outputs {
                        artifacts.insert(artifact_key(&node.id, &output.id), output_value.clone());
                    }
                }
                WorkflowNodeKind::Export => {
                    let operator_id = node.operator_id.as_deref().ok_or_else(|| {
                        format!("workflow export node {} is missing operator_id", node.id)
                    })?;
                    let payload = resolve_single_input_payload(node, &incoming, &artifacts)?;
                    let output_value = run_export_operator(
                        operator_id,
                        payload,
                        node.config.clone().unwrap_or(Value::Null),
                    )?;
                    for output in &node.outputs {
                        artifacts.insert(artifact_key(&node.id, &output.id), output_value.clone());
                    }
                }
                WorkflowNodeKind::Output => {
                    for edge in incoming {
                        let value = artifacts
                            .get(&artifact_key(&edge.from.node, &edge.from.port))
                            .cloned()
                            .ok_or_else(|| {
                                format!(
                                    "workflow output node {} could not read {}.{}",
                                    node.id, edge.from.node, edge.from.port
                                )
                            })?;
                        artifacts.insert(artifact_key(&node.id, &edge.to.port), value);
                    }
                }
                _ => {
                    return Err(format!(
                        "workflow node kind {:?} is not supported by the first headless executor",
                        node.kind
                    ))
                }
            }

            completed.insert(node.id.clone());
            ordered_completed.push(node.id.clone());
            progressed = true;
        }

        if completed.len() == graph.nodes.len() {
            break;
        }
        if !progressed {
            let pending = graph
                .nodes
                .iter()
                .filter(|node| !completed.contains(&node.id))
                .map(|node| node.id.clone())
                .collect::<Vec<_>>();
            return Err(format!(
                "workflow graph could not make progress; pending nodes: {}",
                pending.join(", ")
            ));
        }
    }

    for node_id in &graph.output_nodes {
        if !node_map.contains_key(node_id) {
            return Err(format!("workflow output node {} is not defined", node_id));
        }
    }

    Ok(WorkflowGraphRunResult {
        workflow_id: graph.id,
        completed_nodes: ordered_completed,
        artifacts,
    })
}

fn validate_workflow_dataset_contract(graph: &WorkflowGraph) -> Result<(), String> {
    let Some(contract) = graph.dataset_contract.as_ref() else {
        return Ok(());
    };

    let value_map = contract
        .values
        .iter()
        .map(|value| (value.id.as_str(), value))
        .collect::<HashMap<_, _>>();

    for node in &graph.nodes {
        for port in node.inputs.iter().chain(node.outputs.iter()) {
            let Some(dataset_value) = port.dataset_value.as_deref() else {
                continue;
            };
            let value = value_map.get(dataset_value).ok_or_else(|| {
                format!(
                    "workflow port {}.{} references unknown dataset value {}",
                    node.id, port.id, dataset_value
                )
            })?;

            if let Some(semantic_type) = value.semantic_type.as_deref() {
                if semantic_type != port.artifact_type {
                    return Err(format!(
                        "workflow port {}.{} declares artifact_type {} but dataset value {} uses semantic_type {}",
                        node.id, port.id, port.artifact_type, dataset_value, semantic_type
                    ));
                }
            }
        }
    }

    for edge in &graph.edges {
        let from_node = graph
            .nodes
            .iter()
            .find(|node| node.id == edge.from.node)
            .ok_or_else(|| format!("workflow edge {} references unknown from node {}", edge.id, edge.from.node))?;
        let to_node = graph
            .nodes
            .iter()
            .find(|node| node.id == edge.to.node)
            .ok_or_else(|| format!("workflow edge {} references unknown to node {}", edge.id, edge.to.node))?;
        let from_port = from_node
            .outputs
            .iter()
            .find(|port| port.id == edge.from.port)
            .ok_or_else(|| {
                format!(
                    "workflow edge {} references unknown output port {}.{}",
                    edge.id, edge.from.node, edge.from.port
                )
            })?;
        let to_port = to_node
            .inputs
            .iter()
            .find(|port| port.id == edge.to.port)
            .ok_or_else(|| {
                format!(
                    "workflow edge {} references unknown input port {}.{}",
                    edge.id, edge.to.node, edge.to.port
                )
            })?;

        if from_port.artifact_type != edge.artifact_type {
            return Err(format!(
                "workflow edge {} artifact_type {} does not match from port {}.{} artifact_type {}",
                edge.id, edge.artifact_type, edge.from.node, edge.from.port, from_port.artifact_type
            ));
        }
        if to_port.artifact_type != edge.artifact_type {
            return Err(format!(
                "workflow edge {} artifact_type {} does not match to port {}.{} artifact_type {}",
                edge.id, edge.artifact_type, edge.to.node, edge.to.port, to_port.artifact_type
            ));
        }

        let referenced_dataset_value = edge
            .dataset_value
            .as_deref()
            .or(from_port.dataset_value.as_deref())
            .or(to_port.dataset_value.as_deref());

        if let Some(dataset_value) = referenced_dataset_value {
            let value = value_map.get(dataset_value).ok_or_else(|| {
                format!(
                    "workflow edge {} references unknown dataset value {}",
                    edge.id, dataset_value
                )
            })?;

            if let Some(from_dataset_value) = from_port.dataset_value.as_deref() {
                if from_dataset_value != dataset_value {
                    return Err(format!(
                        "workflow edge {} dataset value {} does not match from port dataset value {}",
                        edge.id, dataset_value, from_dataset_value
                    ));
                }
            }
            if let Some(to_dataset_value) = to_port.dataset_value.as_deref() {
                if to_dataset_value != dataset_value {
                    return Err(format!(
                        "workflow edge {} dataset value {} does not match to port dataset value {}",
                        edge.id, dataset_value, to_dataset_value
                    ));
                }
            }

            if let Some(semantic_type) = value.semantic_type.as_deref() {
                if semantic_type != edge.artifact_type {
                    return Err(format!(
                        "workflow edge {} artifact_type {} does not match dataset value {} semantic_type {}",
                        edge.id, edge.artifact_type, dataset_value, semantic_type
                    ));
                }
            }
        }
    }

    Ok(())
}

fn built_in_solver_descriptor(
    id: &str,
    domain: &str,
    family: &str,
    summary: &str,
    capability_tags: &[&str],
) -> OperatorDescriptor {
    OperatorDescriptor {
        id: id.to_string(),
        version: "1.0.0".to_string(),
        domain: domain.to_string(),
        family: family.to_string(),
        kind: OperatorKind::Solver,
        summary: summary.to_string(),
        capability_tags: capability_tags.iter().map(|tag| (*tag).to_string()).collect(),
        origin: OperatorOrigin::BuiltIn,
        input_schema: OperatorSchemaRef {
            schema: format!("kyuubiki.operator.{family}.input"),
            version: "1".to_string(),
        },
        output_schema: OperatorSchemaRef {
            schema: format!("kyuubiki.operator.{family}.output"),
            version: "1".to_string(),
        },
    }
}

fn built_in_bridge_descriptor(
    id: &str,
    domain: &str,
    family: &str,
    summary: &str,
    capability_tags: &[&str],
) -> OperatorDescriptor {
    OperatorDescriptor {
        id: id.to_string(),
        version: "1.0.0".to_string(),
        domain: domain.to_string(),
        family: family.to_string(),
        kind: OperatorKind::WorkflowBridge,
        summary: summary.to_string(),
        capability_tags: capability_tags.iter().map(|tag| (*tag).to_string()).collect(),
        origin: OperatorOrigin::BuiltIn,
        input_schema: OperatorSchemaRef {
            schema: format!("kyuubiki.operator.{family}.bridge_input"),
            version: "1".to_string(),
        },
        output_schema: OperatorSchemaRef {
            schema: format!("kyuubiki.operator.{family}.bridge_output"),
            version: "1".to_string(),
        },
    }
}

fn built_in_extract_descriptor(
    id: &str,
    domain: &str,
    family: &str,
    summary: &str,
    capability_tags: &[&str],
) -> OperatorDescriptor {
    OperatorDescriptor {
        id: id.to_string(),
        version: "1.0.0".to_string(),
        domain: domain.to_string(),
        family: family.to_string(),
        kind: OperatorKind::Extract,
        summary: summary.to_string(),
        capability_tags: capability_tags.iter().map(|tag| (*tag).to_string()).collect(),
        origin: OperatorOrigin::BuiltIn,
        input_schema: OperatorSchemaRef {
            schema: format!("kyuubiki.operator.{family}.extract_input"),
            version: "1".to_string(),
        },
        output_schema: OperatorSchemaRef {
            schema: format!("kyuubiki.operator.{family}.extract_output"),
            version: "1".to_string(),
        },
    }
}

fn built_in_export_descriptor(
    id: &str,
    domain: &str,
    family: &str,
    summary: &str,
    capability_tags: &[&str],
) -> OperatorDescriptor {
    OperatorDescriptor {
        id: id.to_string(),
        version: "1.0.0".to_string(),
        domain: domain.to_string(),
        family: family.to_string(),
        kind: OperatorKind::Export,
        summary: summary.to_string(),
        capability_tags: capability_tags.iter().map(|tag| (*tag).to_string()).collect(),
        origin: OperatorOrigin::BuiltIn,
        input_schema: OperatorSchemaRef {
            schema: format!("kyuubiki.operator.{family}.export_input"),
            version: "1".to_string(),
        },
        output_schema: OperatorSchemaRef {
            schema: format!("kyuubiki.operator.{family}.export_output"),
            version: "1".to_string(),
        },
    }
}

fn artifact_key(node_id: &str, port_id: &str) -> String {
    format!("{node_id}.{port_id}")
}

fn resolve_single_input_payload(
    node: &kyuubiki_protocol::WorkflowNode,
    incoming: &[&kyuubiki_protocol::WorkflowEdge],
    artifacts: &BTreeMap<String, Value>,
) -> Result<Value, String> {
    let first = incoming.first().ok_or_else(|| {
        format!(
            "workflow node {} requires at least one input artifact in the first executor",
            node.id
        )
    })?;
    artifacts
        .get(&artifact_key(&first.from.node, &first.from.port))
        .cloned()
        .ok_or_else(|| {
            format!(
                "workflow node {} could not resolve input from {}.{}",
                node.id, first.from.node, first.from.port
            )
        })
}

fn run_solve_operator(operator_id: &str, payload: Value) -> Result<Value, String> {
    match operator_id {
        "solve.heat_plane_quad_2d" => {
            let request: SolveHeatPlaneQuad2dRequest =
                serde_json::from_value(payload).map_err(|err| err.to_string())?;
            let result = match solve(EngineSolveRequest::HeatPlaneQuad2d(request))? {
                AnalysisResult::HeatPlaneQuad2d(result) => result,
                _ => unreachable!("solve.heat_plane_quad_2d returned unexpected result"),
            };
            serde_json::to_value(result).map_err(|err| err.to_string())
        }
        "solve.thermal_plane_quad_2d" => {
            let request: SolveThermalPlaneQuad2dRequest =
                serde_json::from_value(payload).map_err(|err| err.to_string())?;
            let result = match solve(EngineSolveRequest::ThermalPlaneQuad2d(request))? {
                AnalysisResult::ThermalPlaneQuad2d(result) => result,
                _ => unreachable!("solve.thermal_plane_quad_2d returned unexpected result"),
            };
            serde_json::to_value(result).map_err(|err| err.to_string())
        }
        _ => Err(format!("unsupported solve operator in first executor: {operator_id}")),
    }
}

fn run_transform_operator(operator_id: &str, payload: Value, config: Value) -> Result<Value, String> {
    match operator_id {
        "bridge.temperature_field_to_thermo_quad_2d" => {
            let heat_result = serde_json::from_value(payload).map_err(|err| err.to_string())?;
            let thermo_seed_model: SolveThermalPlaneQuad2dRequest =
                serde_json::from_value(config).map_err(|err| err.to_string())?;
            let bridged =
                bridge_heat_result_to_thermal_plane_quad_model(&heat_result, &thermo_seed_model)?;
            serde_json::to_value(bridged).map_err(|err| err.to_string())
        }
        _ => Err(format!(
            "unsupported transform operator in first executor: {operator_id}"
        )),
    }
}

fn run_extract_operator(operator_id: &str, payload: Value, config: Value) -> Result<Value, String> {
    match operator_id {
        "extract.result_summary" => extract_result_summary(payload, config),
        _ => Err(format!(
            "unsupported extract operator in first executor: {operator_id}"
        )),
    }
}

fn run_export_operator(operator_id: &str, payload: Value, config: Value) -> Result<Value, String> {
    match operator_id {
        "export.summary_json" => export_summary_json(payload),
        "export.summary_csv" => export_summary_csv(payload, config),
        _ => Err(format!(
            "unsupported export operator in first executor: {operator_id}"
        )),
    }
}

fn extract_result_summary(payload: Value, config: Value) -> Result<Value, String> {
    let object = payload
        .as_object()
        .ok_or_else(|| "extract.result_summary expects an object payload".to_string())?;

    let requested_fields = config
        .get("fields")
        .and_then(Value::as_array)
        .map(|fields| {
            fields
                .iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        });

    let mut summary = serde_json::Map::new();
    if let Some(fields) = requested_fields {
        for field in fields {
            if let Some(value) = object.get(&field) {
                summary.insert(field, value.clone());
            }
        }
    } else {
        for (key, value) in object {
            if key.starts_with("max_") {
                summary.insert(key.clone(), value.clone());
            }
        }
    }

    if summary.is_empty() {
        return Err("extract.result_summary did not find any summary fields".to_string());
    }

    Ok(Value::Object(summary))
}

fn export_summary_json(payload: Value) -> Result<Value, String> {
    if !payload.is_object() {
        return Err("export.summary_json expects an object payload".to_string());
    }
    let content = serde_json::to_string_pretty(&payload).map_err(|err| err.to_string())?;
    Ok(serde_json::json!({
        "format": "json",
        "content_type": "application/json",
        "content": content
    }))
}

fn export_summary_csv(payload: Value, config: Value) -> Result<Value, String> {
    let object = payload
        .as_object()
        .ok_or_else(|| "export.summary_csv expects an object payload".to_string())?;

    let requested_fields = config
        .get("fields")
        .and_then(Value::as_array)
        .map(|fields| {
            fields
                .iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        });

    let mut rows = vec!["key,value".to_string()];
    if let Some(fields) = requested_fields {
        for field in fields {
            if let Some(value) = object.get(&field) {
                rows.push(format!("{},{}", field, csv_cell(value)));
            }
        }
    } else {
        for (key, value) in object {
            rows.push(format!("{},{}", key, csv_cell(value)));
        }
    }

    if rows.len() == 1 {
        return Err("export.summary_csv did not find any exportable fields".to_string());
    }

    Ok(serde_json::json!({
        "format": "csv",
        "content_type": "text/csv",
        "content": rows.join("\n")
    }))
}

fn csv_cell(value: &Value) -> String {
    match value {
        Value::Null => "".to_string(),
        Value::Bool(boolean) => boolean.to_string(),
        Value::Number(number) => number.to_string(),
        Value::String(string) => {
            if string.contains([',', '"', '\n']) {
                format!("\"{}\"", string.replace('"', "\"\""))
            } else {
                string.clone()
            }
        }
        other => serde_json::to_string(other).unwrap_or_else(|_| "\"<invalid>\"".to_string()),
    }
}

pub fn chunk_result(
    result: &AnalysisResult,
    request: &ResultChunkRequest,
) -> Result<ResultChunkResponse, String> {
    let items = match (result, request.kind) {
        (AnalysisResult::Bar1d(result), ResultChunkKind::Nodes) => encode_slice(&result.nodes)?,
        (AnalysisResult::Bar1d(result), ResultChunkKind::Elements) => {
            encode_slice(&result.elements)?
        }
        (AnalysisResult::ThermalBar1d(result), ResultChunkKind::Nodes) => {
            encode_slice(&result.nodes)?
        }
        (AnalysisResult::ThermalBar1d(result), ResultChunkKind::Elements) => {
            encode_slice(&result.elements)?
        }
        (AnalysisResult::HeatBar1d(result), ResultChunkKind::Nodes) => {
            encode_slice(&result.nodes)?
        }
        (AnalysisResult::HeatBar1d(result), ResultChunkKind::Elements) => {
            encode_slice(&result.elements)?
        }
        (AnalysisResult::HeatPlaneTriangle2d(result), ResultChunkKind::Nodes) => {
            encode_slice(&result.nodes)?
        }
        (AnalysisResult::HeatPlaneTriangle2d(result), ResultChunkKind::Elements) => {
            encode_slice(&result.elements)?
        }
        (AnalysisResult::HeatPlaneQuad2d(result), ResultChunkKind::Nodes) => {
            encode_slice(&result.nodes)?
        }
        (AnalysisResult::HeatPlaneQuad2d(result), ResultChunkKind::Elements) => {
            encode_slice(&result.elements)?
        }
        (AnalysisResult::ThermalTruss2d(result), ResultChunkKind::Nodes) => {
            encode_slice(&result.nodes)?
        }
        (AnalysisResult::ThermalTruss2d(result), ResultChunkKind::Elements) => {
            encode_slice(&result.elements)?
        }
        (AnalysisResult::ThermalTruss3d(result), ResultChunkKind::Nodes) => {
            encode_slice(&result.nodes)?
        }
        (AnalysisResult::ThermalTruss3d(result), ResultChunkKind::Elements) => {
            encode_slice(&result.elements)?
        }
        (AnalysisResult::Spring1d(result), ResultChunkKind::Nodes) => encode_slice(&result.nodes)?,
        (AnalysisResult::Spring1d(result), ResultChunkKind::Elements) => {
            encode_slice(&result.elements)?
        }
        (AnalysisResult::Spring2d(result), ResultChunkKind::Nodes) => encode_slice(&result.nodes)?,
        (AnalysisResult::Spring2d(result), ResultChunkKind::Elements) => {
            encode_slice(&result.elements)?
        }
        (AnalysisResult::Spring3d(result), ResultChunkKind::Nodes) => encode_slice(&result.nodes)?,
        (AnalysisResult::Spring3d(result), ResultChunkKind::Elements) => {
            encode_slice(&result.elements)?
        }
        (AnalysisResult::Beam1d(result), ResultChunkKind::Nodes) => encode_slice(&result.nodes)?,
        (AnalysisResult::Beam1d(result), ResultChunkKind::Elements) => {
            encode_slice(&result.elements)?
        }
        (AnalysisResult::ThermalBeam1d(result), ResultChunkKind::Nodes) => {
            encode_slice(&result.nodes)?
        }
        (AnalysisResult::ThermalBeam1d(result), ResultChunkKind::Elements) => {
            encode_slice(&result.elements)?
        }
        (AnalysisResult::ThermalFrame2d(result), ResultChunkKind::Nodes) => {
            encode_slice(&result.nodes)?
        }
        (AnalysisResult::ThermalFrame2d(result), ResultChunkKind::Elements) => {
            encode_slice(&result.elements)?
        }
        (AnalysisResult::ThermalFrame3d(result), ResultChunkKind::Nodes) => {
            encode_slice(&result.nodes)?
        }
        (AnalysisResult::ThermalFrame3d(result), ResultChunkKind::Elements) => {
            encode_slice(&result.elements)?
        }
        (AnalysisResult::Torsion1d(result), ResultChunkKind::Nodes) => encode_slice(&result.nodes)?,
        (AnalysisResult::Torsion1d(result), ResultChunkKind::Elements) => {
            encode_slice(&result.elements)?
        }
        (AnalysisResult::Truss2d(result), ResultChunkKind::Nodes) => encode_slice(&result.nodes)?,
        (AnalysisResult::Truss2d(result), ResultChunkKind::Elements) => {
            encode_slice(&result.elements)?
        }
        (AnalysisResult::Truss3d(result), ResultChunkKind::Nodes) => encode_slice(&result.nodes)?,
        (AnalysisResult::Truss3d(result), ResultChunkKind::Elements) => {
            encode_slice(&result.elements)?
        }
        (AnalysisResult::Frame3d(result), ResultChunkKind::Nodes) => encode_slice(&result.nodes)?,
        (AnalysisResult::Frame3d(result), ResultChunkKind::Elements) => {
            encode_slice(&result.elements)?
        }
        (AnalysisResult::PlaneTriangle2d(result), ResultChunkKind::Nodes) => {
            encode_slice(&result.nodes)?
        }
        (AnalysisResult::PlaneTriangle2d(result), ResultChunkKind::Elements) => {
            encode_slice(&result.elements)?
        }
        (AnalysisResult::ThermalPlaneTriangle2d(result), ResultChunkKind::Nodes) => {
            encode_slice(&result.nodes)?
        }
        (AnalysisResult::ThermalPlaneTriangle2d(result), ResultChunkKind::Elements) => {
            encode_slice(&result.elements)?
        }
        (AnalysisResult::PlaneQuad2d(result), ResultChunkKind::Nodes) => {
            encode_slice(&result.nodes)?
        }
        (AnalysisResult::PlaneQuad2d(result), ResultChunkKind::Elements) => {
            encode_slice(&result.elements)?
        }
        (AnalysisResult::ThermalPlaneQuad2d(result), ResultChunkKind::Nodes) => {
            encode_slice(&result.nodes)?
        }
        (AnalysisResult::ThermalPlaneQuad2d(result), ResultChunkKind::Elements) => {
            encode_slice(&result.elements)?
        }
        (AnalysisResult::Frame2d(result), ResultChunkKind::Nodes) => encode_slice(&result.nodes)?,
        (AnalysisResult::Frame2d(result), ResultChunkKind::Elements) => {
            encode_slice(&result.elements)?
        }
    };

    let offset = request.offset.min(items.len());
    let limit = request.limit.max(1);
    let end_index = (offset + limit).min(items.len());
    let chunk = items[offset..end_index].to_vec();

    Ok(ResultChunkResponse {
        kind: request.kind,
        offset,
        limit,
        returned: chunk.len(),
        total: items.len(),
        items: chunk,
    })
}

fn encode_slice<T>(items: &[T]) -> Result<Vec<Value>, String>
where
    T: serde::Serialize,
{
    items
        .iter()
        .map(|item| serde_json::to_value(item).map_err(|error| error.to_string()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{
        EngineSolveRequest, bridge_heat_result_to_thermal_plane_quad_model,
        built_in_operator_descriptors, chunk_result, describe_built_in_operator,
        run_heat_to_thermo_plane_quad_2d_workflow, run_workflow_graph, solve,
    };
    use kyuubiki_protocol::{
        AnalysisResult, HeatToThermoPlaneQuad2dWorkflowRequest, HeatPlaneNodeInput,
        HeatPlaneQuadElementInput, OperatorKind, ResultChunkKind, ResultChunkRequest,
        SolveBarRequest, SolveHeatPlaneQuad2dRequest, SolveThermalPlaneQuad2dRequest,
        SolveTruss2dRequest, ThermalPlaneNodeInput, ThermalPlaneQuadElementInput,
        TrussElementInput, TrussNodeInput, WorkflowCachePolicy, WorkflowDatasetContract,
        WorkflowDatasetShape, WorkflowDatasetValueInfo, WorkflowDefaults, WorkflowEdge,
        WorkflowGraph, WorkflowGraphRunRequest, WorkflowNode, WorkflowNodeKind,
        WorkflowNodePortRef, WorkflowPort,
    };
    use std::collections::BTreeMap;

    #[test]
    fn solves_through_engine_facade() {
        let result = solve(EngineSolveRequest::Bar1d(SolveBarRequest {
            length: 1.0,
            area: 0.01,
            youngs_modulus: 210.0e9,
            elements: 1,
            tip_force: 1000.0,
        }))
        .expect("bar should solve");

        assert!(matches!(result, AnalysisResult::Bar1d(_)));
    }

    #[test]
    fn chunks_result_items() {
        let result = solve(EngineSolveRequest::Truss2d(SolveTruss2dRequest {
            nodes: vec![
                TrussNodeInput {
                    id: "n0".to_string(),
                    x: 0.0,
                    y: 0.0,
                    fix_x: true,
                    fix_y: true,
                    load_x: 0.0,
                    load_y: 0.0,
                },
                TrussNodeInput {
                    id: "n1".to_string(),
                    x: 1.0,
                    y: 0.0,
                    fix_x: false,
                    fix_y: true,
                    load_x: 0.0,
                    load_y: 0.0,
                },
                TrussNodeInput {
                    id: "n2".to_string(),
                    x: 0.5,
                    y: 0.75,
                    fix_x: false,
                    fix_y: false,
                    load_x: 0.0,
                    load_y: -1000.0,
                },
            ],
            elements: vec![
                TrussElementInput {
                    id: "e0".to_string(),
                    node_i: 0,
                    node_j: 2,
                    area: 0.01,
                    youngs_modulus: 70.0e9,
                },
                TrussElementInput {
                    id: "e1".to_string(),
                    node_i: 1,
                    node_j: 2,
                    area: 0.01,
                    youngs_modulus: 70.0e9,
                },
                TrussElementInput {
                    id: "e2".to_string(),
                    node_i: 0,
                    node_j: 1,
                    area: 0.01,
                    youngs_modulus: 70.0e9,
                },
            ],
        }))
        .expect("truss should solve");

        let chunk = chunk_result(
            &result,
            &ResultChunkRequest {
                kind: ResultChunkKind::Nodes,
                offset: 1,
                limit: 1,
            },
        )
        .expect("chunk should build");

        assert_eq!(chunk.total, 3);
        assert_eq!(chunk.returned, 1);
        assert_eq!(chunk.offset, 1);
    }

    #[test]
    fn exposes_verified_built_in_operator_descriptors() {
        let descriptors = built_in_operator_descriptors();
        assert!(descriptors.len() >= 4);
        assert!(
            descriptors
                .iter()
                .any(|descriptor| descriptor.id == "solve.frame_3d")
        );

        let descriptor =
            describe_built_in_operator("solve.thermal_frame_3d").expect("descriptor");
        assert_eq!(descriptor.kind, OperatorKind::Solver);
        assert_eq!(descriptor.family, "thermal_frame_3d");
        assert!(descriptor.capability_tags.iter().any(|tag| tag == "verified"));
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
        let result = run_heat_to_thermo_plane_quad_2d_workflow(
            HeatToThermoPlaneQuad2dWorkflowRequest {
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
            },
        )
        .expect("workflow should run");

        assert_eq!(result.workflow_id, "workflow.heat-to-thermo-quad-2d");
        assert_eq!(result.heat_result.max_temperature, 100.0);
        assert_eq!(result.bridged_model.nodes[1].temperature_delta, 60.0);
        assert_eq!(result.thermo_result.max_temperature_delta, 100.0);
        assert!(result.thermo_result.max_stress > 0.0);
    }

    #[test]
    fn runs_minimal_generic_workflow_graph() {
        let graph = WorkflowGraph {
            schema_version: "kyuubiki.workflow-graph/v1".to_string(),
            id: "workflow.heat-to-thermo-quad-2d".to_string(),
            name: "Heat to thermo-mechanical quad".to_string(),
            version: "1.0.0".to_string(),
            description: Some("Reference graph".to_string()),
            dataset_contract: None,
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
                    name: Some("Heat model input".to_string()),
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
                        dataset_value: None,
                    }],
                },
                WorkflowNode {
                    id: "solve_heat".to_string(),
                    kind: WorkflowNodeKind::Solve,
                    operator_id: Some("solve.heat_plane_quad_2d".to_string()),
                    name: Some("Solve heat".to_string()),
                    description: None,
                    config: None,
                    cache_policy: None,
                    inputs: vec![WorkflowPort {
                        id: "model".to_string(),
                        artifact_type: "study_model/heat_plane_quad_2d".to_string(),
                        name: None,
                        required: None,
                        cardinality: None,
                        dataset_value: None,
                    }],
                    outputs: vec![WorkflowPort {
                        id: "result".to_string(),
                        artifact_type: "result/heat_plane_quad_2d".to_string(),
                        name: None,
                        required: None,
                        cardinality: None,
                        dataset_value: None,
                    }],
                },
                WorkflowNode {
                    id: "bridge_temperature".to_string(),
                    kind: WorkflowNodeKind::Transform,
                    operator_id: Some("bridge.temperature_field_to_thermo_quad_2d".to_string()),
                    name: Some("Bridge temperature field".to_string()),
                    description: None,
                    config: Some(serde_json::json!({
                        "nodes": [
                            { "id": "n0", "x": 0.0, "y": 0.0, "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0, "temperature_delta": 30.0 },
                            { "id": "n1", "x": 1.0, "y": 0.0, "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0, "temperature_delta": 30.0 },
                            { "id": "n2", "x": 1.0, "y": 1.0, "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0, "temperature_delta": 30.0 },
                            { "id": "n3", "x": 0.0, "y": 1.0, "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0, "temperature_delta": 30.0 }
                        ],
                        "elements": [
                            { "id": "tq0", "node_i": 0, "node_j": 1, "node_k": 2, "node_l": 3, "thickness": 0.02, "youngs_modulus": 70000000000.0, "poisson_ratio": 0.33, "thermal_expansion": 0.000011 }
                        ]
                    })),
                    cache_policy: None,
                    inputs: vec![WorkflowPort {
                        id: "heat_result".to_string(),
                        artifact_type: "result/heat_plane_quad_2d".to_string(),
                        name: None,
                        required: None,
                        cardinality: None,
                        dataset_value: None,
                    }],
                    outputs: vec![WorkflowPort {
                        id: "thermo_model".to_string(),
                        artifact_type: "study_model/thermal_plane_quad_2d".to_string(),
                        name: None,
                        required: None,
                        cardinality: None,
                        dataset_value: None,
                    }],
                },
                WorkflowNode {
                    id: "solve_thermo".to_string(),
                    kind: WorkflowNodeKind::Solve,
                    operator_id: Some("solve.thermal_plane_quad_2d".to_string()),
                    name: Some("Solve thermo".to_string()),
                    description: None,
                    config: None,
                    cache_policy: None,
                    inputs: vec![WorkflowPort {
                        id: "model".to_string(),
                        artifact_type: "study_model/thermal_plane_quad_2d".to_string(),
                        name: None,
                        required: None,
                        cardinality: None,
                        dataset_value: None,
                    }],
                    outputs: vec![WorkflowPort {
                        id: "result".to_string(),
                        artifact_type: "result/thermal_plane_quad_2d".to_string(),
                        name: None,
                        required: None,
                        cardinality: None,
                        dataset_value: None,
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
                        dataset_value: None,
                    }],
                    outputs: vec![],
                },
            ],
            edges: vec![
                WorkflowEdge {
                    id: "edge-heat-input".to_string(),
                    from: WorkflowNodePortRef {
                        node: "heat_model".to_string(),
                        port: "model".to_string(),
                    },
                    to: WorkflowNodePortRef {
                        node: "solve_heat".to_string(),
                        port: "model".to_string(),
                    },
                    artifact_type: "study_model/heat_plane_quad_2d".to_string(),
                    dataset_value: None,
                },
                WorkflowEdge {
                    id: "edge-heat-result".to_string(),
                    from: WorkflowNodePortRef {
                        node: "solve_heat".to_string(),
                        port: "result".to_string(),
                    },
                    to: WorkflowNodePortRef {
                        node: "bridge_temperature".to_string(),
                        port: "heat_result".to_string(),
                    },
                    artifact_type: "result/heat_plane_quad_2d".to_string(),
                    dataset_value: None,
                },
                WorkflowEdge {
                    id: "edge-thermo-model".to_string(),
                    from: WorkflowNodePortRef {
                        node: "bridge_temperature".to_string(),
                        port: "thermo_model".to_string(),
                    },
                    to: WorkflowNodePortRef {
                        node: "solve_thermo".to_string(),
                        port: "model".to_string(),
                    },
                    artifact_type: "study_model/thermal_plane_quad_2d".to_string(),
                    dataset_value: None,
                },
                WorkflowEdge {
                    id: "edge-thermo-result".to_string(),
                    from: WorkflowNodePortRef {
                        node: "solve_thermo".to_string(),
                        port: "result".to_string(),
                    },
                    to: WorkflowNodePortRef {
                        node: "thermo_summary".to_string(),
                        port: "result".to_string(),
                    },
                    artifact_type: "result/thermal_plane_quad_2d".to_string(),
                    dataset_value: None,
                },
            ],
        };

        let run = run_workflow_graph(WorkflowGraphRunRequest {
            graph,
            input_artifacts: BTreeMap::from([(
                "heat_model".to_string(),
                serde_json::json!({
                    "nodes": [
                        { "id": "h0", "x": 0, "y": 0, "fix_temperature": true, "temperature": 100, "heat_load": 0 },
                        { "id": "h1", "x": 1, "y": 0, "fix_temperature": false, "temperature": 0, "heat_load": 0 },
                        { "id": "h2", "x": 1, "y": 1, "fix_temperature": true, "temperature": 20, "heat_load": 0 },
                        { "id": "h3", "x": 0, "y": 1, "fix_temperature": true, "temperature": 20, "heat_load": 0 }
                    ],
                    "elements": [
                        { "id": "hq0", "node_i": 0, "node_j": 1, "node_k": 2, "node_l": 3, "thickness": 0.02, "conductivity": 45 }
                    ]
                }),
            )]),
        })
        .expect("generic workflow graph should run");

        assert_eq!(run.workflow_id, "workflow.heat-to-thermo-quad-2d");
        assert_eq!(run.completed_nodes.len(), 5);
        let summary = run
            .artifacts
            .get("thermo_summary.result")
            .expect("output artifact");
        let thermo_result: SolveThermalPlaneQuad2dRequest = serde_json::from_value(
            run.artifacts
                .get("bridge_temperature.thermo_model")
                .cloned()
                .expect("bridged thermo model"),
        )
        .expect("bridged model should decode");
        assert_eq!(thermo_result.nodes[1].temperature_delta, 60.0);
        assert!(summary.is_object());
    }

    #[test]
    fn runs_solve_extract_output_graph() {
        let graph = WorkflowGraph {
            schema_version: "kyuubiki.workflow-graph/v1".to_string(),
            id: "workflow.heat-summary-quad-2d".to_string(),
            name: "Heat summary quad".to_string(),
            version: "1.0.0".to_string(),
            description: Some("Solve then extract summary".to_string()),
            dataset_contract: None,
            entry_nodes: vec!["heat_model".to_string()],
            output_nodes: vec!["summary_output".to_string()],
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
                        dataset_value: None,
                    }],
                },
                WorkflowNode {
                    id: "solve_heat".to_string(),
                    kind: WorkflowNodeKind::Solve,
                    operator_id: Some("solve.heat_plane_quad_2d".to_string()),
                    name: Some("Solve heat".to_string()),
                    description: None,
                    config: None,
                    cache_policy: None,
                    inputs: vec![WorkflowPort {
                        id: "model".to_string(),
                        artifact_type: "study_model/heat_plane_quad_2d".to_string(),
                        name: None,
                        required: None,
                        cardinality: None,
                        dataset_value: None,
                    }],
                    outputs: vec![WorkflowPort {
                        id: "result".to_string(),
                        artifact_type: "result/heat_plane_quad_2d".to_string(),
                        name: None,
                        required: None,
                        cardinality: None,
                        dataset_value: None,
                    }],
                },
                WorkflowNode {
                    id: "extract_summary".to_string(),
                    kind: WorkflowNodeKind::Extract,
                    operator_id: Some("extract.result_summary".to_string()),
                    name: Some("Extract result summary".to_string()),
                    description: None,
                    config: Some(serde_json::json!({
                        "fields": ["max_temperature", "max_heat_flux"]
                    })),
                    cache_policy: None,
                    inputs: vec![WorkflowPort {
                        id: "result".to_string(),
                        artifact_type: "result/heat_plane_quad_2d".to_string(),
                        name: None,
                        required: None,
                        cardinality: None,
                        dataset_value: None,
                    }],
                    outputs: vec![WorkflowPort {
                        id: "summary".to_string(),
                        artifact_type: "report/summary".to_string(),
                        name: None,
                        required: None,
                        cardinality: None,
                        dataset_value: None,
                    }],
                },
                WorkflowNode {
                    id: "summary_output".to_string(),
                    kind: WorkflowNodeKind::Output,
                    operator_id: None,
                    name: Some("Summary output".to_string()),
                    description: None,
                    config: None,
                    cache_policy: None,
                    inputs: vec![WorkflowPort {
                        id: "summary".to_string(),
                        artifact_type: "report/summary".to_string(),
                        name: None,
                        required: None,
                        cardinality: None,
                        dataset_value: None,
                    }],
                    outputs: vec![],
                },
            ],
            edges: vec![
                WorkflowEdge {
                    id: "edge-heat-input".to_string(),
                    from: WorkflowNodePortRef {
                        node: "heat_model".to_string(),
                        port: "model".to_string(),
                    },
                    to: WorkflowNodePortRef {
                        node: "solve_heat".to_string(),
                        port: "model".to_string(),
                    },
                    artifact_type: "study_model/heat_plane_quad_2d".to_string(),
                    dataset_value: None,
                },
                WorkflowEdge {
                    id: "edge-heat-result".to_string(),
                    from: WorkflowNodePortRef {
                        node: "solve_heat".to_string(),
                        port: "result".to_string(),
                    },
                    to: WorkflowNodePortRef {
                        node: "extract_summary".to_string(),
                        port: "result".to_string(),
                    },
                    artifact_type: "result/heat_plane_quad_2d".to_string(),
                    dataset_value: None,
                },
                WorkflowEdge {
                    id: "edge-summary".to_string(),
                    from: WorkflowNodePortRef {
                        node: "extract_summary".to_string(),
                        port: "summary".to_string(),
                    },
                    to: WorkflowNodePortRef {
                        node: "summary_output".to_string(),
                        port: "summary".to_string(),
                    },
                    artifact_type: "report/summary".to_string(),
                    dataset_value: None,
                },
            ],
        };

        let run = run_workflow_graph(WorkflowGraphRunRequest {
            graph,
            input_artifacts: BTreeMap::from([(
                "heat_model".to_string(),
                serde_json::json!({
                    "nodes": [
                        { "id": "h0", "x": 0, "y": 0, "fix_temperature": true, "temperature": 100, "heat_load": 0 },
                        { "id": "h1", "x": 1, "y": 0, "fix_temperature": false, "temperature": 0, "heat_load": 0 },
                        { "id": "h2", "x": 1, "y": 1, "fix_temperature": true, "temperature": 20, "heat_load": 0 },
                        { "id": "h3", "x": 0, "y": 1, "fix_temperature": true, "temperature": 20, "heat_load": 0 }
                    ],
                    "elements": [
                        { "id": "hq0", "node_i": 0, "node_j": 1, "node_k": 2, "node_l": 3, "thickness": 0.02, "conductivity": 45 }
                    ]
                }),
            )]),
        })
        .expect("solve -> extract -> output graph should run");

        let summary = run
            .artifacts
            .get("summary_output.summary")
            .cloned()
            .expect("summary artifact should exist");
        assert_eq!(run.completed_nodes.len(), 4);
        assert_eq!(summary["max_temperature"], serde_json::json!(100.0));
        assert!(summary.get("max_heat_flux").is_some());
    }

    #[test]
    fn runs_solve_extract_export_output_graph() {
        let graph = WorkflowGraph {
            schema_version: "kyuubiki.workflow-graph/v1".to_string(),
            id: "workflow.heat-summary-export-csv".to_string(),
            name: "Heat summary export csv".to_string(),
            version: "1.0.0".to_string(),
            description: Some("Solve then extract summary and export CSV".to_string()),
            dataset_contract: None,
            entry_nodes: vec!["heat_model".to_string()],
            output_nodes: vec!["csv_output".to_string()],
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
                        dataset_value: None,
                    }],
                },
                WorkflowNode {
                    id: "solve_heat".to_string(),
                    kind: WorkflowNodeKind::Solve,
                    operator_id: Some("solve.heat_plane_quad_2d".to_string()),
                    name: Some("Solve heat".to_string()),
                    description: None,
                    config: None,
                    cache_policy: None,
                    inputs: vec![WorkflowPort {
                        id: "model".to_string(),
                        artifact_type: "study_model/heat_plane_quad_2d".to_string(),
                        name: None,
                        required: None,
                        cardinality: None,
                        dataset_value: None,
                    }],
                    outputs: vec![WorkflowPort {
                        id: "result".to_string(),
                        artifact_type: "result/heat_plane_quad_2d".to_string(),
                        name: None,
                        required: None,
                        cardinality: None,
                        dataset_value: None,
                    }],
                },
                WorkflowNode {
                    id: "extract_summary".to_string(),
                    kind: WorkflowNodeKind::Extract,
                    operator_id: Some("extract.result_summary".to_string()),
                    name: Some("Extract result summary".to_string()),
                    description: None,
                    config: Some(serde_json::json!({
                        "fields": ["max_temperature", "max_heat_flux"]
                    })),
                    cache_policy: None,
                    inputs: vec![WorkflowPort {
                        id: "result".to_string(),
                        artifact_type: "result/heat_plane_quad_2d".to_string(),
                        name: None,
                        required: None,
                        cardinality: None,
                        dataset_value: None,
                    }],
                    outputs: vec![WorkflowPort {
                        id: "summary".to_string(),
                        artifact_type: "report/summary".to_string(),
                        name: None,
                        required: None,
                        cardinality: None,
                        dataset_value: None,
                    }],
                },
                WorkflowNode {
                    id: "export_csv".to_string(),
                    kind: WorkflowNodeKind::Export,
                    operator_id: Some("export.summary_csv".to_string()),
                    name: Some("Export summary CSV".to_string()),
                    description: None,
                    config: Some(serde_json::json!({
                        "fields": ["max_temperature", "max_heat_flux"]
                    })),
                    cache_policy: None,
                    inputs: vec![WorkflowPort {
                        id: "summary".to_string(),
                        artifact_type: "report/summary".to_string(),
                        name: None,
                        required: None,
                        cardinality: None,
                        dataset_value: None,
                    }],
                    outputs: vec![WorkflowPort {
                        id: "csv".to_string(),
                        artifact_type: "export/csv".to_string(),
                        name: None,
                        required: None,
                        cardinality: None,
                        dataset_value: None,
                    }],
                },
                WorkflowNode {
                    id: "csv_output".to_string(),
                    kind: WorkflowNodeKind::Output,
                    operator_id: None,
                    name: Some("CSV output".to_string()),
                    description: None,
                    config: None,
                    cache_policy: None,
                    inputs: vec![WorkflowPort {
                        id: "csv".to_string(),
                        artifact_type: "export/csv".to_string(),
                        name: None,
                        required: None,
                        cardinality: None,
                        dataset_value: None,
                    }],
                    outputs: vec![],
                },
            ],
            edges: vec![
                WorkflowEdge {
                    id: "edge-heat-input".to_string(),
                    from: WorkflowNodePortRef {
                        node: "heat_model".to_string(),
                        port: "model".to_string(),
                    },
                    to: WorkflowNodePortRef {
                        node: "solve_heat".to_string(),
                        port: "model".to_string(),
                    },
                    artifact_type: "study_model/heat_plane_quad_2d".to_string(),
                    dataset_value: None,
                },
                WorkflowEdge {
                    id: "edge-heat-result".to_string(),
                    from: WorkflowNodePortRef {
                        node: "solve_heat".to_string(),
                        port: "result".to_string(),
                    },
                    to: WorkflowNodePortRef {
                        node: "extract_summary".to_string(),
                        port: "result".to_string(),
                    },
                    artifact_type: "result/heat_plane_quad_2d".to_string(),
                    dataset_value: None,
                },
                WorkflowEdge {
                    id: "edge-summary".to_string(),
                    from: WorkflowNodePortRef {
                        node: "extract_summary".to_string(),
                        port: "summary".to_string(),
                    },
                    to: WorkflowNodePortRef {
                        node: "export_csv".to_string(),
                        port: "summary".to_string(),
                    },
                    artifact_type: "report/summary".to_string(),
                    dataset_value: None,
                },
                WorkflowEdge {
                    id: "edge-csv".to_string(),
                    from: WorkflowNodePortRef {
                        node: "export_csv".to_string(),
                        port: "csv".to_string(),
                    },
                    to: WorkflowNodePortRef {
                        node: "csv_output".to_string(),
                        port: "csv".to_string(),
                    },
                    artifact_type: "export/csv".to_string(),
                    dataset_value: None,
                },
            ],
        };

        let run = run_workflow_graph(WorkflowGraphRunRequest {
            graph,
            input_artifacts: BTreeMap::from([(
                "heat_model".to_string(),
                serde_json::json!({
                    "nodes": [
                        { "id": "h0", "x": 0, "y": 0, "fix_temperature": true, "temperature": 100, "heat_load": 0 },
                        { "id": "h1", "x": 1, "y": 0, "fix_temperature": false, "temperature": 0, "heat_load": 0 },
                        { "id": "h2", "x": 1, "y": 1, "fix_temperature": true, "temperature": 20, "heat_load": 0 },
                        { "id": "h3", "x": 0, "y": 1, "fix_temperature": true, "temperature": 20, "heat_load": 0 }
                    ],
                    "elements": [
                        { "id": "hq0", "node_i": 0, "node_j": 1, "node_k": 2, "node_l": 3, "thickness": 0.02, "conductivity": 45 }
                    ]
                }),
            )]),
        })
        .expect("solve -> extract -> export -> output graph should run");

        let exported = run
            .artifacts
            .get("csv_output.csv")
            .cloned()
            .expect("csv export artifact should exist");
        assert_eq!(run.completed_nodes.len(), 5);
        assert_eq!(exported["format"], serde_json::json!("csv"));
        let content = exported["content"]
            .as_str()
            .expect("csv content should be a string");
        assert!(content.contains("key,value"));
        assert!(content.contains("max_temperature,100"));
        assert!(content.contains("max_heat_flux"));
    }

    #[test]
    fn rejects_workflow_graph_with_mismatched_dataset_contract() {
        let graph = WorkflowGraph {
            schema_version: "kyuubiki.workflow-graph/v1".to_string(),
            id: "workflow.invalid-dataset-contract".to_string(),
            name: "Invalid dataset contract".to_string(),
            version: "1.0.0".to_string(),
            description: Some("Graph with mismatched artifact and dataset semantic type".to_string()),
            dataset_contract: Some(WorkflowDatasetContract {
                id: "dataset.invalid/v1".to_string(),
                version: "1.0.0".to_string(),
                values: vec![WorkflowDatasetValueInfo {
                    id: "bad_summary".to_string(),
                    data_class: "result".to_string(),
                    element_type: "json_object".to_string(),
                    shape: WorkflowDatasetShape::default(),
                    semantic_type: Some("result/thermal_plane_quad_2d".to_string()),
                    unit: None,
                    encoding: None,
                    schema_ref: None,
                }],
                metadata: BTreeMap::new(),
            }),
            entry_nodes: vec!["in".to_string()],
            output_nodes: vec!["out".to_string()],
            defaults: WorkflowDefaults::default(),
            nodes: vec![
                WorkflowNode {
                    id: "in".to_string(),
                    kind: WorkflowNodeKind::Input,
                    operator_id: None,
                    name: None,
                    description: None,
                    config: None,
                    cache_policy: None,
                    inputs: vec![],
                    outputs: vec![WorkflowPort {
                        id: "value".to_string(),
                        artifact_type: "report/summary".to_string(),
                        name: None,
                        required: None,
                        cardinality: None,
                        dataset_value: Some("bad_summary".to_string()),
                    }],
                },
                WorkflowNode {
                    id: "out".to_string(),
                    kind: WorkflowNodeKind::Output,
                    operator_id: None,
                    name: None,
                    description: None,
                    config: None,
                    cache_policy: None,
                    inputs: vec![WorkflowPort {
                        id: "value".to_string(),
                        artifact_type: "report/summary".to_string(),
                        name: None,
                        required: None,
                        cardinality: None,
                        dataset_value: Some("bad_summary".to_string()),
                    }],
                    outputs: vec![],
                },
            ],
            edges: vec![WorkflowEdge {
                id: "e0".to_string(),
                from: WorkflowNodePortRef {
                    node: "in".to_string(),
                    port: "value".to_string(),
                },
                to: WorkflowNodePortRef {
                    node: "out".to_string(),
                    port: "value".to_string(),
                },
                artifact_type: "report/summary".to_string(),
                dataset_value: Some("bad_summary".to_string()),
            }],
        };

        let error = run_workflow_graph(WorkflowGraphRunRequest {
            graph,
            input_artifacts: BTreeMap::from([(
                "in".to_string(),
                serde_json::json!({ "max_temperature": 100.0 }),
            )]),
        })
        .expect_err("dataset contract mismatch should be rejected");

        assert!(error.contains("semantic_type"));
    }
}
