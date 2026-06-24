use kyuubiki_protocol::{AnalysisResult, ResultChunkKind, ResultChunkRequest, ResultChunkResponse};
use serde_json::Value;

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
        (AnalysisResult::HeatBar1d(result), ResultChunkKind::Nodes) => encode_slice(&result.nodes)?,
        (AnalysisResult::HeatBar1d(result), ResultChunkKind::Elements) => {
            encode_slice(&result.elements)?
        }
        (AnalysisResult::ElectrostaticBar1d(result), ResultChunkKind::Nodes) => {
            encode_slice(&result.nodes)?
        }
        (AnalysisResult::ElectrostaticBar1d(result), ResultChunkKind::Elements) => {
            encode_slice(&result.elements)?
        }
        (AnalysisResult::MagnetostaticBar1d(result), ResultChunkKind::Nodes) => {
            encode_slice(&result.nodes)?
        }
        (AnalysisResult::MagnetostaticBar1d(result), ResultChunkKind::Elements) => {
            encode_slice(&result.elements)?
        }
        (AnalysisResult::ElectrostaticPlaneTriangle2d(result), ResultChunkKind::Nodes) => {
            encode_slice(&result.nodes)?
        }
        (AnalysisResult::ElectrostaticPlaneTriangle2d(result), ResultChunkKind::Elements) => {
            encode_slice(&result.elements)?
        }
        (AnalysisResult::ElectrostaticPlaneQuad2d(result), ResultChunkKind::Nodes) => {
            encode_slice(&result.nodes)?
        }
        (AnalysisResult::ElectrostaticPlaneQuad2d(result), ResultChunkKind::Elements) => {
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
