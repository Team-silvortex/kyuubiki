mod acoustic_quality;
mod bridge;
mod catalog;
mod cfd_diagnostics;
mod chunking;
mod coupled_workflows;
mod dynamic_quality;
mod electrostatic_diagnostics;
mod electrostatic_quality;
mod heat_bridge;
mod magnetostatic_bridge;
mod magnetostatic_diagnostics;
mod magnetostatic_quality;
mod modal_quality;
mod operator_sdk_bridges;
mod operator_sdk_host;
mod operator_sdk_material_envelope;
mod operator_sdk_material_margins;
mod operator_sdk_material_pareto;
mod operator_sdk_parameter_sweep;
mod operator_sdk_quality_objective;
mod operator_sdk_runtime;
mod operator_sdk_workflow_extensions;
mod structural_quality;
mod thermal_diagnostics;
mod thermal_quality;
mod thermo_diagnostics;
mod transport_diagnostics;
mod transport_quality;
mod workflow;
mod workflow_bundle_exports;
mod workflow_bundle_focus;
mod workflow_bundle_transforms;
mod workflow_contract;
mod workflow_coupled_readiness;
mod workflow_executor;
mod workflow_focus_chain;
mod workflow_guard_transforms;
mod workflow_parameter_sweep;
mod workflow_parameter_sweep_results;
mod workflow_quality_lineage;
mod workflow_quality_objective;
mod workflow_quality_sweep_plan;
mod workflow_quality_sweep_request;
mod workflow_quality_terms;
mod workflow_reporting;
mod workflow_security;
mod workflow_solve_executor;
mod workflow_summary_transforms;
mod workflow_summary_validation;

#[cfg(test)]
mod tests;

pub use catalog::{built_in_operator_descriptors, describe_built_in_operator};
pub use chunking::chunk_result;
pub use coupled_workflows::{
    run_electrostatic_to_heat_to_thermo_plane_quad_2d_workflow,
    run_electrostatic_to_heat_to_thermo_plane_triangle_2d_workflow,
    run_heat_to_thermo_plane_quad_2d_workflow, run_heat_to_thermo_plane_triangle_2d_workflow,
};
pub use heat_bridge::{
    bridge_heat_result_to_thermal_plane_quad_model,
    bridge_heat_result_to_thermal_plane_triangle_model,
};
pub use kyuubiki_operator_sdk::{OperatorSdkReadinessIssue, OperatorSdkReadinessSeverity};
pub use operator_sdk_host::{
    DeferredDynamicLoadActivator, DynamicLibraryOperatorActivator, DynamicOperatorHostSession,
    ExternalOperatorHostConfig, ExternalOperatorHostError, ExternalOperatorLoadReport,
    ExternalOperatorPackageReadinessSummary, ExternalOperatorPreflightReport,
    ExternalOperatorRejectedPackage, ExternalOperatorTrustPolicy,
    built_in_registry_with_external_packages, load_external_operator_packages_with_deferred_host,
    load_external_operator_packages_with_dynamic_host, preflight_external_operator_packages,
};
pub use operator_sdk_runtime::{BuiltInOperatorRegistryKind, built_in_operator_registry};
pub use workflow::run_workflow_graph;
pub use workflow_executor::{
    is_supported_workflow_operator, run_solve_operator, solve_operator_runtime_manifest,
    supported_workflow_operator_ids,
};

use kyuubiki_protocol::{
    AnalysisResult, SolveAcousticBar1dRequest, SolveAdvectionDiffusionBar1dRequest,
    SolveBarRequest, SolveBeam1dRequest, SolveContactGap1dRequest, SolveElectrostaticBar1dRequest,
    SolveElectrostaticPlaneQuad2dRequest, SolveElectrostaticPlaneTriangle2dRequest,
    SolveFrame2dRequest, SolveFrame3dRequest, SolveHarmonicSpring1dRequest, SolveHeatBar1dRequest,
    SolveHeatPlaneQuad2dRequest, SolveHeatPlaneTriangle2dRequest, SolveMagnetostaticBar1dRequest,
    SolveMagnetostaticPlaneQuad2dRequest, SolveMagnetostaticPlaneTriangle2dRequest,
    SolveModalFrame2dRequest, SolveModalFrame3dRequest, SolveNonlinearSpring1dRequest,
    SolvePlaneQuad2dRequest, SolvePlaneTriangle2dRequest, SolveSolidTetra3dRequest,
    SolveSpring1dRequest, SolveSpring2dRequest, SolveSpring3dRequest,
    SolveStokesFlowPlaneQuad2dRequest, SolveStokesFlowPlaneTriangle2dRequest,
    SolveThermalBar1dRequest, SolveThermalBeam1dRequest, SolveThermalFrame2dRequest,
    SolveThermalFrame3dRequest, SolveThermalPlaneQuad2dRequest, SolveThermalPlaneTriangle2dRequest,
    SolveThermalTruss2dRequest, SolveThermalTruss3dRequest, SolveTorsion1dRequest,
    SolveTransientHeatBar1dRequest, SolveTransientSpring1dRequest, SolveTruss2dRequest,
    SolveTruss3dRequest,
};
use kyuubiki_solver::{
    solve_acoustic_bar_1d, solve_advection_diffusion_bar_1d, solve_bar_1d, solve_beam_1d,
    solve_contact_gap_1d, solve_electrostatic_bar_1d, solve_electrostatic_plane_quad_2d,
    solve_electrostatic_plane_triangle_2d, solve_frame_2d, solve_frame_3d,
    solve_harmonic_spring_1d, solve_heat_bar_1d, solve_heat_plane_quad_2d,
    solve_heat_plane_triangle_2d, solve_magnetostatic_bar_1d, solve_magnetostatic_plane_quad_2d,
    solve_magnetostatic_plane_triangle_2d, solve_modal_frame_2d, solve_modal_frame_3d,
    solve_nonlinear_spring_1d, solve_plane_quad_2d, solve_plane_triangle_2d, solve_solid_tetra_3d,
    solve_spring_1d, solve_spring_2d, solve_spring_3d, solve_stokes_flow_plane_quad_2d,
    solve_stokes_flow_plane_triangle_2d, solve_thermal_bar_1d, solve_thermal_beam_1d,
    solve_thermal_frame_2d, solve_thermal_frame_3d, solve_thermal_plane_quad_2d,
    solve_thermal_plane_triangle_2d, solve_thermal_truss_2d, solve_thermal_truss_3d,
    solve_torsion_1d, solve_transient_heat_bar_1d, solve_transient_spring_1d, solve_truss_2d,
    solve_truss_3d,
};

#[derive(Debug, Clone, PartialEq)]
pub enum EngineSolveRequest {
    Bar1d(SolveBarRequest),
    AcousticBar1d(SolveAcousticBar1dRequest),
    ThermalBar1d(SolveThermalBar1dRequest),
    HeatBar1d(SolveHeatBar1dRequest),
    TransientHeatBar1d(SolveTransientHeatBar1dRequest),
    ElectrostaticBar1d(SolveElectrostaticBar1dRequest),
    MagnetostaticBar1d(SolveMagnetostaticBar1dRequest),
    AdvectionDiffusionBar1d(SolveAdvectionDiffusionBar1dRequest),
    MagnetostaticPlaneTriangle2d(SolveMagnetostaticPlaneTriangle2dRequest),
    MagnetostaticPlaneQuad2d(SolveMagnetostaticPlaneQuad2dRequest),
    ElectrostaticPlaneTriangle2d(SolveElectrostaticPlaneTriangle2dRequest),
    ElectrostaticPlaneQuad2d(SolveElectrostaticPlaneQuad2dRequest),
    HeatPlaneTriangle2d(SolveHeatPlaneTriangle2dRequest),
    HeatPlaneQuad2d(SolveHeatPlaneQuad2dRequest),
    StokesFlowPlaneTriangle2d(SolveStokesFlowPlaneTriangle2dRequest),
    StokesFlowPlaneQuad2d(SolveStokesFlowPlaneQuad2dRequest),
    ThermalTruss2d(SolveThermalTruss2dRequest),
    ThermalTruss3d(SolveThermalTruss3dRequest),
    Spring1d(SolveSpring1dRequest),
    TransientSpring1d(SolveTransientSpring1dRequest),
    HarmonicSpring1d(SolveHarmonicSpring1dRequest),
    NonlinearSpring1d(SolveNonlinearSpring1dRequest),
    ContactGap1d(SolveContactGap1dRequest),
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
    SolidTetra3d(SolveSolidTetra3dRequest),
    ModalFrame2d(SolveModalFrame2dRequest),
    ModalFrame3d(SolveModalFrame3dRequest),
    PlaneTriangle2d(SolvePlaneTriangle2dRequest),
    ThermalPlaneTriangle2d(SolveThermalPlaneTriangle2dRequest),
    PlaneQuad2d(SolvePlaneQuad2dRequest),
    ThermalPlaneQuad2d(SolveThermalPlaneQuad2dRequest),
    Frame2d(SolveFrame2dRequest),
}

pub fn solve(request: EngineSolveRequest) -> Result<AnalysisResult, String> {
    match request {
        EngineSolveRequest::Bar1d(request) => solve_bar_1d(&request).map(AnalysisResult::Bar1d),
        EngineSolveRequest::AcousticBar1d(request) => {
            solve_acoustic_bar_1d(&request).map(AnalysisResult::AcousticBar1d)
        }
        EngineSolveRequest::ThermalBar1d(request) => {
            solve_thermal_bar_1d(&request).map(AnalysisResult::ThermalBar1d)
        }
        EngineSolveRequest::HeatBar1d(request) => {
            solve_heat_bar_1d(&request).map(AnalysisResult::HeatBar1d)
        }
        EngineSolveRequest::TransientHeatBar1d(request) => {
            solve_transient_heat_bar_1d(&request).map(AnalysisResult::TransientHeatBar1d)
        }
        EngineSolveRequest::ElectrostaticBar1d(request) => {
            solve_electrostatic_bar_1d(&request).map(AnalysisResult::ElectrostaticBar1d)
        }
        EngineSolveRequest::MagnetostaticBar1d(request) => {
            solve_magnetostatic_bar_1d(&request).map(AnalysisResult::MagnetostaticBar1d)
        }
        EngineSolveRequest::AdvectionDiffusionBar1d(request) => {
            solve_advection_diffusion_bar_1d(&request).map(AnalysisResult::AdvectionDiffusionBar1d)
        }
        EngineSolveRequest::MagnetostaticPlaneTriangle2d(request) => {
            solve_magnetostatic_plane_triangle_2d(&request)
                .map(AnalysisResult::MagnetostaticPlaneTriangle2d)
        }
        EngineSolveRequest::MagnetostaticPlaneQuad2d(request) => {
            solve_magnetostatic_plane_quad_2d(&request)
                .map(AnalysisResult::MagnetostaticPlaneQuad2d)
        }
        EngineSolveRequest::ElectrostaticPlaneTriangle2d(request) => {
            solve_electrostatic_plane_triangle_2d(&request)
                .map(AnalysisResult::ElectrostaticPlaneTriangle2d)
        }
        EngineSolveRequest::ElectrostaticPlaneQuad2d(request) => {
            solve_electrostatic_plane_quad_2d(&request)
                .map(AnalysisResult::ElectrostaticPlaneQuad2d)
        }
        EngineSolveRequest::HeatPlaneTriangle2d(request) => {
            solve_heat_plane_triangle_2d(&request).map(AnalysisResult::HeatPlaneTriangle2d)
        }
        EngineSolveRequest::HeatPlaneQuad2d(request) => {
            solve_heat_plane_quad_2d(&request).map(AnalysisResult::HeatPlaneQuad2d)
        }
        EngineSolveRequest::StokesFlowPlaneTriangle2d(request) => {
            solve_stokes_flow_plane_triangle_2d(&request)
                .map(AnalysisResult::StokesFlowPlaneTriangle2d)
        }
        EngineSolveRequest::StokesFlowPlaneQuad2d(request) => {
            solve_stokes_flow_plane_quad_2d(&request).map(AnalysisResult::StokesFlowPlaneQuad2d)
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
        EngineSolveRequest::TransientSpring1d(request) => {
            solve_transient_spring_1d(&request).map(AnalysisResult::TransientSpring1d)
        }
        EngineSolveRequest::HarmonicSpring1d(request) => {
            solve_harmonic_spring_1d(&request).map(AnalysisResult::HarmonicSpring1d)
        }
        EngineSolveRequest::NonlinearSpring1d(request) => {
            solve_nonlinear_spring_1d(&request).map(AnalysisResult::NonlinearSpring1d)
        }
        EngineSolveRequest::ContactGap1d(request) => {
            solve_contact_gap_1d(&request).map(AnalysisResult::ContactGap1d)
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
        EngineSolveRequest::SolidTetra3d(request) => {
            solve_solid_tetra_3d(&request).map(AnalysisResult::SolidTetra3d)
        }
        EngineSolveRequest::ModalFrame2d(request) => {
            solve_modal_frame_2d(&request).map(AnalysisResult::ModalFrame2d)
        }
        EngineSolveRequest::ModalFrame3d(request) => {
            solve_modal_frame_3d(&request).map(AnalysisResult::ModalFrame3d)
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
