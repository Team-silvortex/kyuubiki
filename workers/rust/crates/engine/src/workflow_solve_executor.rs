use crate::{EngineSolveRequest, solve};
use kyuubiki_protocol::AnalysisResult;
use serde_json::Value;

pub const SUPPORTED_SOLVE_OPERATORS: &[&str] = &[
    "solve.bar_1d",
    "solve.thermal_bar_1d",
    "solve.heat_bar_1d",
    "solve.electrostatic_bar_1d",
    "solve.magnetostatic_bar_1d",
    "solve.magnetostatic_plane_triangle_2d",
    "solve.magnetostatic_plane_quad_2d",
    "solve.electrostatic_plane_triangle_2d",
    "solve.electrostatic_plane_quad_2d",
    "solve.heat_plane_triangle_2d",
    "solve.heat_plane_quad_2d",
    "solve.stokes_flow_quad_2d",
    "solve.thermal_truss_2d",
    "solve.frame_3d",
    "solve.plane_triangle_2d",
    "solve.thermal_plane_triangle_2d",
    "solve.plane_quad_2d",
    "solve.thermal_frame_3d",
    "solve.thermal_plane_quad_2d",
    "solve.thermal_truss_3d",
    "solve.torsion_1d",
    "solve.spring_1d",
    "solve.nonlinear_spring_1d",
    "solve.contact_gap_1d",
    "solve.spring_2d",
    "solve.spring_3d",
    "solve.truss_2d",
    "solve.truss_3d",
    "solve.frame_2d",
    "solve.modal_frame_2d",
    "solve.modal_frame_3d",
    "solve.beam_1d",
    "solve.thermal_beam_1d",
    "solve.thermal_frame_2d",
];

pub fn run_solve_operator(operator_id: &str, payload: Value) -> Result<Value, String> {
    match operator_id {
        "solve.bar_1d" => encode_solve_result(
            solve(EngineSolveRequest::Bar1d(decode(payload)?))?,
            |result| match result {
                AnalysisResult::Bar1d(result) => Some(result),
                _ => None,
            },
            operator_id,
        ),
        "solve.thermal_bar_1d" => encode_solve_result(
            solve(EngineSolveRequest::ThermalBar1d(decode(payload)?))?,
            |result| match result {
                AnalysisResult::ThermalBar1d(result) => Some(result),
                _ => None,
            },
            operator_id,
        ),
        "solve.heat_bar_1d" => encode_solve_result(
            solve(EngineSolveRequest::HeatBar1d(decode(payload)?))?,
            |result| match result {
                AnalysisResult::HeatBar1d(result) => Some(result),
                _ => None,
            },
            operator_id,
        ),
        "solve.electrostatic_bar_1d" => encode_solve_result(
            solve(EngineSolveRequest::ElectrostaticBar1d(decode(payload)?))?,
            |result| match result {
                AnalysisResult::ElectrostaticBar1d(result) => Some(result),
                _ => None,
            },
            operator_id,
        ),
        "solve.magnetostatic_bar_1d" => encode_solve_result(
            solve(EngineSolveRequest::MagnetostaticBar1d(decode(payload)?))?,
            |result| match result {
                AnalysisResult::MagnetostaticBar1d(result) => Some(result),
                _ => None,
            },
            operator_id,
        ),
        "solve.magnetostatic_plane_triangle_2d" => encode_solve_result(
            solve(EngineSolveRequest::MagnetostaticPlaneTriangle2d(decode(
                payload,
            )?))?,
            |result| match result {
                AnalysisResult::MagnetostaticPlaneTriangle2d(result) => Some(result),
                _ => None,
            },
            operator_id,
        ),
        "solve.magnetostatic_plane_quad_2d" => encode_solve_result(
            solve(EngineSolveRequest::MagnetostaticPlaneQuad2d(decode(
                payload,
            )?))?,
            |result| match result {
                AnalysisResult::MagnetostaticPlaneQuad2d(result) => Some(result),
                _ => None,
            },
            operator_id,
        ),
        "solve.electrostatic_plane_triangle_2d" => encode_solve_result(
            solve(EngineSolveRequest::ElectrostaticPlaneTriangle2d(decode(
                payload,
            )?))?,
            |result| match result {
                AnalysisResult::ElectrostaticPlaneTriangle2d(result) => Some(result),
                _ => None,
            },
            operator_id,
        ),
        "solve.electrostatic_plane_quad_2d" => encode_solve_result(
            solve(EngineSolveRequest::ElectrostaticPlaneQuad2d(decode(
                payload,
            )?))?,
            |result| match result {
                AnalysisResult::ElectrostaticPlaneQuad2d(result) => Some(result),
                _ => None,
            },
            operator_id,
        ),
        "solve.heat_plane_triangle_2d" => encode_solve_result(
            solve(EngineSolveRequest::HeatPlaneTriangle2d(decode(payload)?))?,
            |result| match result {
                AnalysisResult::HeatPlaneTriangle2d(result) => Some(result),
                _ => None,
            },
            operator_id,
        ),
        "solve.heat_plane_quad_2d" => encode_solve_result(
            solve(EngineSolveRequest::HeatPlaneQuad2d(decode(payload)?))?,
            |result| match result {
                AnalysisResult::HeatPlaneQuad2d(result) => Some(result),
                _ => None,
            },
            operator_id,
        ),
        "solve.stokes_flow_quad_2d" => encode_solve_result(
            solve(EngineSolveRequest::StokesFlowPlaneQuad2d(decode(payload)?))?,
            |result| match result {
                AnalysisResult::StokesFlowPlaneQuad2d(result) => Some(result),
                _ => None,
            },
            operator_id,
        ),
        "solve.thermal_truss_2d" => encode_solve_result(
            solve(EngineSolveRequest::ThermalTruss2d(decode(payload)?))?,
            |result| match result {
                AnalysisResult::ThermalTruss2d(result) => Some(result),
                _ => None,
            },
            operator_id,
        ),
        "solve.frame_3d" => encode_solve_result(
            solve(EngineSolveRequest::Frame3d(decode(payload)?))?,
            |result| match result {
                AnalysisResult::Frame3d(result) => Some(result),
                _ => None,
            },
            operator_id,
        ),
        "solve.thermal_frame_3d" => encode_solve_result(
            solve(EngineSolveRequest::ThermalFrame3d(decode(payload)?))?,
            |result| match result {
                AnalysisResult::ThermalFrame3d(result) => Some(result),
                _ => None,
            },
            operator_id,
        ),
        "solve.plane_triangle_2d" => encode_solve_result(
            solve(EngineSolveRequest::PlaneTriangle2d(decode(payload)?))?,
            |result| match result {
                AnalysisResult::PlaneTriangle2d(result) => Some(result),
                _ => None,
            },
            operator_id,
        ),
        "solve.thermal_plane_triangle_2d" => encode_solve_result(
            solve(EngineSolveRequest::ThermalPlaneTriangle2d(decode(payload)?))?,
            |result| match result {
                AnalysisResult::ThermalPlaneTriangle2d(result) => Some(result),
                _ => None,
            },
            operator_id,
        ),
        "solve.plane_quad_2d" => encode_solve_result(
            solve(EngineSolveRequest::PlaneQuad2d(decode(payload)?))?,
            |result| match result {
                AnalysisResult::PlaneQuad2d(result) => Some(result),
                _ => None,
            },
            operator_id,
        ),
        "solve.thermal_plane_quad_2d" => encode_solve_result(
            solve(EngineSolveRequest::ThermalPlaneQuad2d(decode(payload)?))?,
            |result| match result {
                AnalysisResult::ThermalPlaneQuad2d(result) => Some(result),
                _ => None,
            },
            operator_id,
        ),
        "solve.thermal_truss_3d" => encode_solve_result(
            solve(EngineSolveRequest::ThermalTruss3d(decode(payload)?))?,
            |result| match result {
                AnalysisResult::ThermalTruss3d(result) => Some(result),
                _ => None,
            },
            operator_id,
        ),
        "solve.torsion_1d" => encode_solve_result(
            solve(EngineSolveRequest::Torsion1d(decode(payload)?))?,
            |result| match result {
                AnalysisResult::Torsion1d(result) => Some(result),
                _ => None,
            },
            operator_id,
        ),
        "solve.spring_1d" => encode_solve_result(
            solve(EngineSolveRequest::Spring1d(decode(payload)?))?,
            |result| match result {
                AnalysisResult::Spring1d(result) => Some(result),
                _ => None,
            },
            operator_id,
        ),
        "solve.nonlinear_spring_1d" => encode_solve_result(
            solve(EngineSolveRequest::NonlinearSpring1d(decode(payload)?))?,
            |result| match result {
                AnalysisResult::NonlinearSpring1d(result) => Some(result),
                _ => None,
            },
            operator_id,
        ),
        "solve.contact_gap_1d" => encode_solve_result(
            solve(EngineSolveRequest::ContactGap1d(decode(payload)?))?,
            |result| match result {
                AnalysisResult::ContactGap1d(result) => Some(result),
                _ => None,
            },
            operator_id,
        ),
        "solve.spring_2d" => encode_solve_result(
            solve(EngineSolveRequest::Spring2d(decode(payload)?))?,
            |result| match result {
                AnalysisResult::Spring2d(result) => Some(result),
                _ => None,
            },
            operator_id,
        ),
        "solve.spring_3d" => encode_solve_result(
            solve(EngineSolveRequest::Spring3d(decode(payload)?))?,
            |result| match result {
                AnalysisResult::Spring3d(result) => Some(result),
                _ => None,
            },
            operator_id,
        ),
        "solve.truss_2d" => encode_solve_result(
            solve(EngineSolveRequest::Truss2d(decode(payload)?))?,
            |result| match result {
                AnalysisResult::Truss2d(result) => Some(result),
                _ => None,
            },
            operator_id,
        ),
        "solve.truss_3d" => encode_solve_result(
            solve(EngineSolveRequest::Truss3d(decode(payload)?))?,
            |result| match result {
                AnalysisResult::Truss3d(result) => Some(result),
                _ => None,
            },
            operator_id,
        ),
        "solve.frame_2d" => encode_solve_result(
            solve(EngineSolveRequest::Frame2d(decode(payload)?))?,
            |result| match result {
                AnalysisResult::Frame2d(result) => Some(result),
                _ => None,
            },
            operator_id,
        ),
        "solve.modal_frame_2d" => encode_solve_result(
            solve(EngineSolveRequest::ModalFrame2d(decode(payload)?))?,
            |result| match result {
                AnalysisResult::ModalFrame2d(result) => Some(result),
                _ => None,
            },
            operator_id,
        ),
        "solve.modal_frame_3d" => encode_solve_result(
            solve(EngineSolveRequest::ModalFrame3d(decode(payload)?))?,
            |result| match result {
                AnalysisResult::ModalFrame3d(result) => Some(result),
                _ => None,
            },
            operator_id,
        ),
        "solve.beam_1d" => encode_solve_result(
            solve(EngineSolveRequest::Beam1d(decode(payload)?))?,
            |result| match result {
                AnalysisResult::Beam1d(result) => Some(result),
                _ => None,
            },
            operator_id,
        ),
        "solve.thermal_beam_1d" => encode_solve_result(
            solve(EngineSolveRequest::ThermalBeam1d(decode(payload)?))?,
            |result| match result {
                AnalysisResult::ThermalBeam1d(result) => Some(result),
                _ => None,
            },
            operator_id,
        ),
        "solve.thermal_frame_2d" => encode_solve_result(
            solve(EngineSolveRequest::ThermalFrame2d(decode(payload)?))?,
            |result| match result {
                AnalysisResult::ThermalFrame2d(result) => Some(result),
                _ => None,
            },
            operator_id,
        ),
        _ => Err(format!(
            "unsupported solve operator in first executor: {operator_id}"
        )),
    }
}

fn decode<T>(payload: Value) -> Result<T, String>
where
    T: serde::de::DeserializeOwned,
{
    serde_json::from_value(payload).map_err(|err| err.to_string())
}

fn encode_solve_result<T>(
    result: AnalysisResult,
    selector: impl FnOnce(AnalysisResult) -> Option<T>,
    operator_id: &str,
) -> Result<Value, String>
where
    T: serde::Serialize,
{
    let selected = selector(result)
        .ok_or_else(|| format!("{operator_id} returned an unexpected result variant"))?;
    serde_json::to_value(selected).map_err(|err| err.to_string())
}
