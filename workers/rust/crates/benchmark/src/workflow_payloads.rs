use crate::models::{BenchmarkCase, BenchmarkWorkload};
use serde::Serialize;
use serde_json::Value;

pub(crate) fn workflow_payload_for_case(case: &BenchmarkCase) -> (&'static str, Value) {
    match &case.workload {
        BenchmarkWorkload::AxialBar(request) => payload("solve.bar_1d", request),
        BenchmarkWorkload::ThermalBar1d(request) => payload("solve.thermal_bar_1d", request),
        BenchmarkWorkload::AcousticBar1d(request) => payload("solve.acoustic_bar_1d", request),
        BenchmarkWorkload::HeatBar1d(request) => payload("solve.heat_bar_1d", request),
        BenchmarkWorkload::ElectrostaticBar1d(request) => {
            payload("solve.electrostatic_bar_1d", request)
        }
        BenchmarkWorkload::MagnetostaticBar1d(request) => {
            payload("solve.magnetostatic_bar_1d", request)
        }
        BenchmarkWorkload::AdvectionDiffusionBar1d(request) => {
            payload("solve.advection_diffusion_bar_1d", request)
        }
        BenchmarkWorkload::Torsion1d(request) => payload("solve.torsion_1d", request),
        BenchmarkWorkload::Spring1d(request) => payload("solve.spring_1d", request),
        BenchmarkWorkload::Spring2d(request) => payload("solve.spring_2d", request),
        BenchmarkWorkload::Spring3d(request) => payload("solve.spring_3d", request),
        BenchmarkWorkload::NonlinearSpring1d(request) => {
            payload("solve.nonlinear_spring_1d", request)
        }
        BenchmarkWorkload::ContactGap1d(request) => payload("solve.contact_gap_1d", request),
        BenchmarkWorkload::Beam1d(request) => payload("solve.beam_1d", request),
        BenchmarkWorkload::ThermalBeam1d(request) => payload("solve.thermal_beam_1d", request),
        BenchmarkWorkload::Frame2d(request) => payload("solve.frame_2d", request),
        BenchmarkWorkload::Frame3d(request) => payload("solve.frame_3d", request),
        BenchmarkWorkload::ThermalFrame2d(request) => payload("solve.thermal_frame_2d", request),
        BenchmarkWorkload::ThermalFrame3d(request) => payload("solve.thermal_frame_3d", request),
        BenchmarkWorkload::ModalFrame2d(request) => payload("solve.modal_frame_2d", request),
        BenchmarkWorkload::BucklingBeam1d(request) => payload("solve.buckling_beam_1d", request),
        BenchmarkWorkload::BucklingFrame2d(request) => payload("solve.buckling_frame_2d", request),
        BenchmarkWorkload::Frame2dPDelta(request) => payload("solve.frame_2d_p_delta", request),
        BenchmarkWorkload::ModalFrame3d(request) => payload("solve.modal_frame_3d", request),
        BenchmarkWorkload::SolidTetra3d(request) => payload("solve.solid_tetra_3d", request),
        BenchmarkWorkload::Truss2d(request) => payload("solve.truss_2d", request),
        BenchmarkWorkload::Truss3d(request) => payload("solve.truss_3d", request),
        BenchmarkWorkload::ThermalTruss2d(request) => payload("solve.thermal_truss_2d", request),
        BenchmarkWorkload::ThermalTruss3d(request) => payload("solve.thermal_truss_3d", request),
        BenchmarkWorkload::PlaneTriangle2d(request) => payload("solve.plane_triangle_2d", request),
        BenchmarkWorkload::PlaneQuad2d(request) => payload("solve.plane_quad_2d", request),
        BenchmarkWorkload::ThermalPlaneTriangle2d(request) => {
            payload("solve.thermal_plane_triangle_2d", request)
        }
        BenchmarkWorkload::ThermalPlaneQuad2d(request) => {
            payload("solve.thermal_plane_quad_2d", request)
        }
        BenchmarkWorkload::HeatPlaneTriangle2d(request) => {
            payload("solve.heat_plane_triangle_2d", request)
        }
        BenchmarkWorkload::HeatPlaneQuad2d(request) => payload("solve.heat_plane_quad_2d", request),
        BenchmarkWorkload::ElectrostaticPlaneTriangle2d(request) => {
            payload("solve.electrostatic_plane_triangle_2d", request)
        }
        BenchmarkWorkload::ElectrostaticPlaneQuad2d(request) => {
            payload("solve.electrostatic_plane_quad_2d", request)
        }
        BenchmarkWorkload::MagnetostaticPlaneTriangle2d(request) => {
            payload("solve.magnetostatic_plane_triangle_2d", request)
        }
        BenchmarkWorkload::MagnetostaticPlaneQuad2d(request) => {
            payload("solve.magnetostatic_plane_quad_2d", request)
        }
        BenchmarkWorkload::StokesFlowPlaneTriangle2d(request) => {
            payload("solve.stokes_flow_triangle_2d", request)
        }
        BenchmarkWorkload::StokesFlowPlaneQuad2d(request) => {
            payload("solve.stokes_flow_quad_2d", request)
        }
        BenchmarkWorkload::HeadlessActionManifest | BenchmarkWorkload::DirectFemManifest => {
            panic!("headless manifest benchmarks are not workflow solve payloads")
        }
    }
}

fn payload<T>(operator_id: &'static str, request: &T) -> (&'static str, Value)
where
    T: Serialize,
{
    (
        operator_id,
        serde_json::to_value(request).expect("benchmark request should encode as workflow payload"),
    )
}
