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
    SolveTorsion1dRequest, SolveTruss2dRequest, SolveTruss3dRequest,
};
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
        run_heat_to_thermo_plane_quad_2d_workflow, solve,
    };
    use kyuubiki_protocol::{
        AnalysisResult, HeatToThermoPlaneQuad2dWorkflowRequest, HeatPlaneNodeInput,
        HeatPlaneQuadElementInput, OperatorKind, ResultChunkKind, ResultChunkRequest,
        SolveBarRequest, SolveHeatPlaneQuad2dRequest, SolveThermalPlaneQuad2dRequest,
        SolveTruss2dRequest, ThermalPlaneNodeInput, ThermalPlaneQuadElementInput,
        TrussElementInput, TrussNodeInput,
    };

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
}
