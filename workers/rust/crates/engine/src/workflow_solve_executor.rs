use crate::{EngineSolveRequest, solve};
use kyuubiki_protocol::AnalysisResult;
use serde_json::Value;

pub const SUPPORTED_SOLVE_OPERATORS: &[&str] = &[
    "solve.bar_1d",
    "solve.acoustic_bar_1d",
    "solve.thermal_bar_1d",
    "solve.heat_bar_1d",
    "solve.transient_heat_bar_1d",
    "solve.electrostatic_bar_1d",
    "solve.magnetostatic_bar_1d",
    "solve.advection_diffusion_bar_1d",
    "solve.magnetostatic_plane_triangle_2d",
    "solve.magnetostatic_plane_quad_2d",
    "solve.electrostatic_plane_triangle_2d",
    "solve.electrostatic_plane_quad_2d",
    "solve.heat_plane_triangle_2d",
    "solve.heat_plane_quad_2d",
    "solve.stokes_flow_triangle_2d",
    "solve.stokes_flow_quad_2d",
    "solve.thermal_truss_2d",
    "solve.frame_3d",
    "solve.solid_tetra_3d",
    "solve.plane_triangle_2d",
    "solve.thermal_plane_triangle_2d",
    "solve.plane_quad_2d",
    "solve.thermal_frame_3d",
    "solve.thermal_plane_quad_2d",
    "solve.thermal_truss_3d",
    "solve.torsion_1d",
    "solve.spring_1d",
    "solve.transient_spring_1d",
    "solve.harmonic_spring_1d",
    "solve.nonlinear_spring_1d",
    "solve.contact_gap_1d",
    "solve.spring_2d",
    "solve.spring_3d",
    "solve.truss_2d",
    "solve.truss_3d",
    "solve.frame_2d",
    "solve.modal_frame_2d",
    "solve.buckling_beam_1d",
    "solve.buckling_frame_2d",
    "solve.modal_frame_3d",
    "solve.beam_1d",
    "solve.thermal_beam_1d",
    "solve.thermal_frame_2d",
];

const SOLVER_PROVENANCE_SCHEMA_VERSION: &str = "kyuubiki.engine-solver-provenance/v1";
const SOLVE_OPERATOR_RUNTIME_MANIFEST_SCHEMA_VERSION: &str =
    "kyuubiki.engine-solve-operator-runtime-manifest/v1";

pub fn solve_operator_runtime_manifest() -> Value {
    let operators = SUPPORTED_SOLVE_OPERATORS
        .iter()
        .map(|operator_id| {
            serde_json::json!({
                "operator_id": operator_id,
                "result_type": result_type_for_operator(operator_id),
                "provenance_schema": SOLVER_PROVENANCE_SCHEMA_VERSION,
            })
        })
        .collect::<Vec<_>>();

    serde_json::json!({
        "schema_version": SOLVE_OPERATOR_RUNTIME_MANIFEST_SCHEMA_VERSION,
        "runtime_owner": "runtime_engine_solver",
        "runtime_api": "workflow_solve_executor",
        "operator_count": operators.len(),
        "operators": operators,
        "provenance_schema": SOLVER_PROVENANCE_SCHEMA_VERSION,
        "execution_contract": {
            "input_encoding": "json",
            "output_encoding": "json",
            "result_provenance_field": "_solver_provenance"
        }
    })
}

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
        "solve.acoustic_bar_1d" => encode_solve_result(
            solve(EngineSolveRequest::AcousticBar1d(decode(payload)?))?,
            |result| match result {
                AnalysisResult::AcousticBar1d(result) => Some(result),
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
        "solve.transient_heat_bar_1d" => encode_solve_result(
            solve(EngineSolveRequest::TransientHeatBar1d(decode(payload)?))?,
            |result| match result {
                AnalysisResult::TransientHeatBar1d(result) => Some(result),
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
        "solve.advection_diffusion_bar_1d" => encode_solve_result(
            solve(EngineSolveRequest::AdvectionDiffusionBar1d(decode(
                payload,
            )?))?,
            |result| match result {
                AnalysisResult::AdvectionDiffusionBar1d(result) => Some(result),
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
        "solve.stokes_flow_triangle_2d" => encode_solve_result(
            solve(EngineSolveRequest::StokesFlowPlaneTriangle2d(decode(
                payload,
            )?))?,
            |result| match result {
                AnalysisResult::StokesFlowPlaneTriangle2d(result) => Some(result),
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
        "solve.solid_tetra_3d" => encode_solve_result(
            solve(EngineSolveRequest::SolidTetra3d(decode(payload)?))?,
            |result| match result {
                AnalysisResult::SolidTetra3d(result) => Some(result),
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
        "solve.transient_spring_1d" => encode_solve_result(
            solve(EngineSolveRequest::TransientSpring1d(decode(payload)?))?,
            |result| match result {
                AnalysisResult::TransientSpring1d(result) => Some(result),
                _ => None,
            },
            operator_id,
        ),
        "solve.harmonic_spring_1d" => encode_solve_result(
            solve(EngineSolveRequest::HarmonicSpring1d(decode(payload)?))?,
            |result| match result {
                AnalysisResult::HarmonicSpring1d(result) => Some(result),
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
        "solve.buckling_beam_1d" => encode_solve_result(
            solve(EngineSolveRequest::BucklingBeam1d(decode(payload)?))?,
            |result| match result {
                AnalysisResult::BucklingBeam1d(result) => Some(result),
                _ => None,
            },
            operator_id,
        ),
        "solve.buckling_frame_2d" => encode_solve_result(
            solve(EngineSolveRequest::BucklingFrame2d(decode(payload)?))?,
            |result| match result {
                AnalysisResult::BucklingFrame2d(result) => Some(result),
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
    let mut encoded = serde_json::to_value(selected).map_err(|err| err.to_string())?;
    attach_solver_provenance(&mut encoded, operator_id);
    Ok(encoded)
}

fn attach_solver_provenance(result: &mut Value, operator_id: &str) {
    let Some(object) = result.as_object_mut() else {
        return;
    };
    object.insert(
        "_solver_provenance".to_string(),
        serde_json::json!({
            "schema_version": SOLVER_PROVENANCE_SCHEMA_VERSION,
            "provenance_owner": "runtime_engine_solver",
            "operator_id": operator_id,
            "result_type": result_type_for_operator(operator_id),
            "engine": "kyuubiki-engine",
            "execution_path": "workflow_solve_executor",
            "persistence_hint": {
                "artifact_type": result_type_for_operator(operator_id),
                "recommended_retention_scope": "run",
                "stable_key_fields": ["operator_id", "result_type"]
            },
            "lineage": {
                "solver_dispatch_verified": true,
                "result_serialized": true
            }
        }),
    );
}

fn result_type_for_operator(operator_id: &str) -> String {
    operator_id
        .strip_prefix("solve.")
        .map(|suffix| format!("result/{}", suffix.replace('.', "_")))
        .unwrap_or_else(|| "result/unknown".to_string())
}
