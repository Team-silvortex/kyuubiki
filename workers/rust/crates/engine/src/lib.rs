use kyuubiki_protocol::{
    AnalysisResult, ResultChunkKind, ResultChunkRequest, ResultChunkResponse, SolveBarRequest,
    SolvePlaneTriangle2dRequest, SolveTruss2dRequest, SolveTruss3dRequest,
};
use kyuubiki_solver::{solve_bar_1d, solve_plane_triangle_2d, solve_truss_2d, solve_truss_3d};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
pub enum EngineSolveRequest {
    Bar1d(SolveBarRequest),
    Truss2d(SolveTruss2dRequest),
    Truss3d(SolveTruss3dRequest),
    PlaneTriangle2d(SolvePlaneTriangle2dRequest),
}

pub fn solve(request: EngineSolveRequest) -> Result<AnalysisResult, String> {
    match request {
        EngineSolveRequest::Bar1d(request) => solve_bar_1d(&request).map(AnalysisResult::Bar1d),
        EngineSolveRequest::Truss2d(request) => {
            solve_truss_2d(&request).map(AnalysisResult::Truss2d)
        }
        EngineSolveRequest::Truss3d(request) => {
            solve_truss_3d(&request).map(AnalysisResult::Truss3d)
        }
        EngineSolveRequest::PlaneTriangle2d(request) => {
            solve_plane_triangle_2d(&request).map(AnalysisResult::PlaneTriangle2d)
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
        (AnalysisResult::Truss2d(result), ResultChunkKind::Nodes) => encode_slice(&result.nodes)?,
        (AnalysisResult::Truss2d(result), ResultChunkKind::Elements) => {
            encode_slice(&result.elements)?
        }
        (AnalysisResult::Truss3d(result), ResultChunkKind::Nodes) => encode_slice(&result.nodes)?,
        (AnalysisResult::Truss3d(result), ResultChunkKind::Elements) => {
            encode_slice(&result.elements)?
        }
        (AnalysisResult::PlaneTriangle2d(result), ResultChunkKind::Nodes) => {
            encode_slice(&result.nodes)?
        }
        (AnalysisResult::PlaneTriangle2d(result), ResultChunkKind::Elements) => {
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
