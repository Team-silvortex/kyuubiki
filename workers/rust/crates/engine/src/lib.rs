use kyuubiki_protocol::{
    AnalysisResult, ResultChunkKind, ResultChunkRequest, ResultChunkResponse, SolveBarRequest,
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
    use super::{EngineSolveRequest, chunk_result, solve};
    use kyuubiki_protocol::{
        AnalysisResult, ResultChunkKind, ResultChunkRequest, SolveBarRequest, SolveTruss2dRequest,
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
}
