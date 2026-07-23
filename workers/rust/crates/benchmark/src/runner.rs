use std::time::Instant;

use kyuubiki_engine::{EngineSolveRequest, solve};
use kyuubiki_headless_sdk::{action_capability_manifest, direct_fem_capability_manifest};
use kyuubiki_protocol::AnalysisResult;
use kyuubiki_solver::{
    SpdSolveOptions, profile_heat_plane_quad_2d_with_options, profile_truss_2d_with_options,
    solve_beam_1d_with_options, solve_thermal_beam_1d_with_options,
};

use crate::models::{
    BenchmarkCase, BenchmarkMemoryStage, BenchmarkReport, BenchmarkResult, BenchmarkWorkload,
};
use crate::runner_hotspot::summarize_hotspot;
use crate::runner_metrics::apply_metrics;
use crate::runner_preconditioner::{
    effective_preconditioner, parse_preconditioner, preconditioner_comparisons,
    preconditioner_selection_reason, solver_preconditioners,
};
use crate::runner_progress::{print_case_done, print_case_start};
use crate::runner_shape::workload_shape;
use crate::runner_structural::run_thermal_structural_workload;
use crate::runner_util::{current_peak_rss_kib, percentile, unix_timestamp};

pub(crate) fn build_report(
    selected: &[&BenchmarkCase],
    repeat: usize,
    profile: crate::config::BenchmarkProfile,
    matrix: &str,
    solver_preconditioner: &str,
) -> BenchmarkReport {
    build_report_with_progress(
        selected,
        repeat,
        profile,
        matrix,
        solver_preconditioner,
        false,
    )
}

pub(crate) fn build_report_with_progress(
    selected: &[&BenchmarkCase],
    repeat: usize,
    profile: crate::config::BenchmarkProfile,
    matrix: &str,
    solver_preconditioner: &str,
    progress: bool,
) -> BenchmarkReport {
    let preconditioners = solver_preconditioners(solver_preconditioner);
    let tag_results = preconditioners.len() > 1;
    let cases = selected
        .iter()
        .flat_map(|case| {
            preconditioners.iter().map(move |preconditioner| {
                let effective_preconditioner = effective_preconditioner(case, preconditioner);
                let selection_reason = preconditioner_selection_reason(case, preconditioner);
                if progress {
                    print_case_start(case, effective_preconditioner, selection_reason, repeat);
                }
                let mut result =
                    run_case_with_preconditioner(case, repeat, effective_preconditioner, progress);
                result.solver_preconditioner_reason = Some(selection_reason.to_string());
                if tag_results && result.solver_preconditioner.is_some() {
                    result.id = format!("{}#{}", result.id, effective_preconditioner);
                }
                if progress {
                    print_case_done(&result);
                }
                result
            })
        })
        .collect::<Vec<_>>();

    BenchmarkReport {
        repeat,
        profile,
        matrix: matrix.to_string(),
        generated_at_unix_s: unix_timestamp(),
        preconditioner_comparisons: preconditioner_comparisons(&cases),
        cases,
    }
}

#[cfg(test)]
pub(crate) fn run_case(case: &BenchmarkCase, repeat: usize) -> BenchmarkResult {
    run_case_with_preconditioner(case, repeat, "jacobi", false)
}

pub(crate) fn run_case_with_preconditioner(
    case: &BenchmarkCase,
    repeat: usize,
    solver_preconditioner: &str,
    progress: bool,
) -> BenchmarkResult {
    let mut durations = Vec::with_capacity(repeat);
    let (mut node_count, mut element_count, mut dof_count) = workload_shape(&case.workload);
    let mut max_displacement = 0.0;
    let mut max_stress = 0.0;
    let mut peak_rss_kib = current_peak_rss_kib();
    let mut memory_stages = Vec::new();
    let mut solver_iterations = None;
    let mut solver_matrix_non_zero_count = None;
    let mut solver_residual_norm = None;
    let mut solver_preconditioner_name = None;
    let mut error = None;

    for _ in 0..repeat {
        let started = Instant::now();
        let outcome = if let Some(outcome) =
            run_thermal_structural_workload(&case.workload, solver_preconditioner, progress)
        {
            outcome.map(|metrics| {
                apply_metrics(
                    metrics,
                    &mut node_count,
                    &mut element_count,
                    &mut dof_count,
                    &mut max_displacement,
                    &mut max_stress,
                    &mut memory_stages,
                    &mut solver_iterations,
                    &mut solver_matrix_non_zero_count,
                    &mut solver_residual_norm,
                    &mut solver_preconditioner_name,
                );
            })
        } else {
            match &case.workload {
                BenchmarkWorkload::AxialBar(request) => {
                    solve(EngineSolveRequest::Bar1d(request.clone())).map(|result| {
                        let AnalysisResult::Bar1d(result) = result else {
                            unreachable!("bar solve should return bar result")
                        };
                        node_count = result.nodes.len();
                        element_count = result.elements.len();
                        dof_count = result.nodes.len().saturating_sub(1);
                        max_displacement = result.max_displacement;
                        max_stress = result.max_stress;
                    })
                }
                BenchmarkWorkload::AcousticBar1d(request) => {
                    solve(EngineSolveRequest::AcousticBar1d(request.clone())).map(|result| {
                        let AnalysisResult::AcousticBar1d(result) = result else {
                            unreachable!("acoustic solve should return acoustic result")
                        };
                        node_count = result.nodes.len();
                        element_count = result.elements.len();
                        dof_count = result.nodes.len();
                        max_displacement = result.max_pressure;
                        max_stress = result.max_acoustic_intensity;
                    })
                }
                BenchmarkWorkload::HeatBar1d(request) => {
                    solve(EngineSolveRequest::HeatBar1d(request.clone())).map(|result| {
                        let AnalysisResult::HeatBar1d(result) = result else {
                            unreachable!("heat bar solve should return heat result")
                        };
                        node_count = result.nodes.len();
                        element_count = result.elements.len();
                        dof_count = result.nodes.len();
                        max_displacement = result.max_temperature;
                        max_stress = result.max_heat_flux;
                    })
                }
                BenchmarkWorkload::ElectrostaticBar1d(request) => {
                    solve(EngineSolveRequest::ElectrostaticBar1d(request.clone())).map(|result| {
                        let AnalysisResult::ElectrostaticBar1d(result) = result else {
                            unreachable!(
                                "electrostatic bar solve should return electrostatic result"
                            )
                        };
                        node_count = result.nodes.len();
                        element_count = result.elements.len();
                        dof_count = result.nodes.len();
                        max_displacement = result.max_potential;
                        max_stress = result.max_electric_field;
                    })
                }
                BenchmarkWorkload::MagnetostaticBar1d(request) => {
                    solve(EngineSolveRequest::MagnetostaticBar1d(request.clone())).map(|result| {
                        let AnalysisResult::MagnetostaticBar1d(result) = result else {
                            unreachable!(
                                "magnetostatic bar solve should return magnetostatic result"
                            )
                        };
                        node_count = result.nodes.len();
                        element_count = result.elements.len();
                        dof_count = result.nodes.len();
                        max_displacement = result.max_magnetic_potential;
                        max_stress = result.max_magnetic_field_strength;
                    })
                }
                BenchmarkWorkload::AdvectionDiffusionBar1d(request) => solve(
                    EngineSolveRequest::AdvectionDiffusionBar1d(request.clone()),
                )
                .map(|result| {
                    let AnalysisResult::AdvectionDiffusionBar1d(result) = result else {
                        unreachable!("advection-diffusion solve should return transport result")
                    };
                    node_count = result.nodes.len();
                    element_count = result.elements.len();
                    dof_count = result.nodes.len();
                    max_displacement = result.max_concentration;
                    max_stress = result.max_total_flux;
                }),
                BenchmarkWorkload::Torsion1d(request) => {
                    solve(EngineSolveRequest::Torsion1d(request.clone())).map(|result| {
                        let AnalysisResult::Torsion1d(result) = result else {
                            unreachable!("torsion solve should return torsion result")
                        };
                        node_count = result.nodes.len();
                        element_count = result.elements.len();
                        dof_count = result.nodes.len();
                        max_displacement = result.max_rotation;
                        max_stress = result.max_stress;
                    })
                }
                BenchmarkWorkload::Spring1d(request) => {
                    solve(EngineSolveRequest::Spring1d(request.clone())).map(|result| {
                        let AnalysisResult::Spring1d(result) = result else {
                            unreachable!("spring 1d solve should return spring result")
                        };
                        node_count = result.nodes.len();
                        element_count = result.elements.len();
                        dof_count = result.nodes.len();
                        max_displacement = result.max_displacement;
                        max_stress = result.max_force;
                    })
                }
                BenchmarkWorkload::Spring2d(request) => {
                    solve(EngineSolveRequest::Spring2d(request.clone())).map(|result| {
                        let AnalysisResult::Spring2d(result) = result else {
                            unreachable!("spring 2d solve should return spring result")
                        };
                        node_count = result.nodes.len();
                        element_count = result.elements.len();
                        dof_count = result.nodes.len() * 2;
                        max_displacement = result.max_displacement;
                        max_stress = result.max_force;
                    })
                }
                BenchmarkWorkload::Spring3d(request) => {
                    solve(EngineSolveRequest::Spring3d(request.clone())).map(|result| {
                        let AnalysisResult::Spring3d(result) = result else {
                            unreachable!("spring 3d solve should return spring result")
                        };
                        node_count = result.nodes.len();
                        element_count = result.elements.len();
                        dof_count = result.nodes.len() * 3;
                        max_displacement = result.max_displacement;
                        max_stress = result.max_force;
                    })
                }
                BenchmarkWorkload::NonlinearSpring1d(request) => {
                    solve(EngineSolveRequest::NonlinearSpring1d(request.clone())).map(|result| {
                        let AnalysisResult::NonlinearSpring1d(result) = result else {
                            unreachable!("nonlinear spring solve should return nonlinear result")
                        };
                        node_count = result.nodes.len();
                        element_count = result.elements.len();
                        dof_count = result.nodes.len();
                        max_displacement = result.max_displacement;
                        max_stress = result.max_force;
                    })
                }
                BenchmarkWorkload::ContactGap1d(request) => {
                    solve(EngineSolveRequest::ContactGap1d(request.clone())).map(|result| {
                        let AnalysisResult::ContactGap1d(result) = result else {
                            unreachable!("contact gap solve should return contact result")
                        };
                        node_count = result.nodes.len();
                        element_count = result.elements.len();
                        dof_count = result.nodes.len();
                        max_displacement = result.max_displacement;
                        max_stress = result.max_contact_force;
                    })
                }
                BenchmarkWorkload::Beam1d(request) => {
                    let options = SpdSolveOptions {
                        preconditioner: parse_preconditioner(solver_preconditioner),
                        progress_interval: progress.then_some(256),
                    };
                    solve_beam_1d_with_options(request, options).map(|result| {
                        node_count = result.nodes.len();
                        element_count = result.elements.len();
                        dof_count = result.nodes.len() * 2;
                        max_displacement = result.max_displacement;
                        max_stress = result.max_stress;
                        solver_preconditioner_name = Some(solver_preconditioner.to_string());
                    })
                }
                BenchmarkWorkload::ThermalBeam1d(request) => {
                    let options = SpdSolveOptions {
                        preconditioner: parse_preconditioner(solver_preconditioner),
                        progress_interval: progress.then_some(256),
                    };
                    solve_thermal_beam_1d_with_options(request, options).map(|result| {
                        node_count = result.nodes.len();
                        element_count = result.elements.len();
                        dof_count = result.nodes.len() * 2;
                        max_displacement = result.max_displacement;
                        max_stress = result.max_stress;
                        solver_preconditioner_name = Some(solver_preconditioner.to_string());
                    })
                }
                BenchmarkWorkload::ModalFrame2d(request) => {
                    solve(EngineSolveRequest::ModalFrame2d(request.clone())).map(|result| {
                        let AnalysisResult::ModalFrame2d(result) = result else {
                            unreachable!("modal frame 2d solve should return modal result")
                        };
                        node_count = result.input.nodes.len();
                        element_count = result.input.elements.len();
                        dof_count = result.free_dofs.len();
                        max_displacement = result.min_frequency_hz;
                        max_stress = result.max_frequency_hz;
                    })
                }
                BenchmarkWorkload::BucklingBeam1d(request) => {
                    solve(EngineSolveRequest::BucklingBeam1d(request.clone())).map(|result| {
                        let AnalysisResult::BucklingBeam1d(result) = result else {
                            unreachable!("buckling beam solve should return buckling result")
                        };
                        node_count = result.input.nodes.len();
                        element_count = result.input.elements.len();
                        dof_count = result.free_dofs.len();
                        max_displacement = result.minimum_load_factor;
                        max_stress = result.modes[0].residual_norm;
                    })
                }
                BenchmarkWorkload::BucklingFrame2d(request) => {
                    solve(EngineSolveRequest::BucklingFrame2d(request.clone())).map(|result| {
                        let AnalysisResult::BucklingFrame2d(result) = result else {
                            unreachable!("buckling frame solve should return buckling result")
                        };
                        node_count = result.input.frame.nodes.len();
                        element_count = result.input.frame.elements.len();
                        dof_count = result.free_dofs.len();
                        max_displacement = result.minimum_load_factor;
                        max_stress = result.modes[0].residual_norm;
                    })
                }
                BenchmarkWorkload::Frame2dPDelta(request) => {
                    solve(EngineSolveRequest::Frame2dPDelta(request.clone())).and_then(|result| {
                        let AnalysisResult::Frame2dPDelta(result) = result else {
                            unreachable!("frame p-delta solve should return p-delta result")
                        };
                        if !result.converged {
                            let failed = result.steps.iter().find(|step| !step.converged);
                            return Err(match failed {
                                Some(step) => format!(
                                    "frame p-delta benchmark did not converge at step {}: target={}, achieved={}, residual={}, iterations={}, cutbacks={}, reason={:?}, detail={}",
                                    step.step,
                                    step.load_factor,
                                    step.achieved_load_factor.unwrap_or_default(),
                                    step.residual_norm,
                                    step.iterations,
                                    step.cutbacks,
                                    step.failure_reason,
                                    step.failure_detail.as_deref().unwrap_or("none")
                                ),
                                None => "frame p-delta benchmark ended before all requested steps"
                                    .to_string(),
                            });
                        }
                        node_count = result.input.buckling.frame.nodes.len();
                        element_count = result.input.buckling.frame.elements.len();
                        dof_count = result.final_displacements.len();
                        max_displacement = result.max_imperfection_amplification;
                        max_stress = result
                            .steps
                            .iter()
                            .map(|step| step.residual_norm)
                            .fold(0.0_f64, f64::max);
                        Ok(())
                    })
                }
                BenchmarkWorkload::ModalFrame3d(request) => {
                    solve(EngineSolveRequest::ModalFrame3d(request.clone())).map(|result| {
                        let AnalysisResult::ModalFrame3d(result) = result else {
                            unreachable!("modal frame 3d solve should return modal result")
                        };
                        node_count = result.input.nodes.len();
                        element_count = result.input.elements.len();
                        dof_count = result.free_dofs.len();
                        max_displacement = result.min_frequency_hz;
                        max_stress = result.max_frequency_hz;
                    })
                }
                BenchmarkWorkload::SolidTetra3d(request) => {
                    solve(EngineSolveRequest::SolidTetra3d(request.clone())).map(|result| {
                        let AnalysisResult::SolidTetra3d(result) = result else {
                            unreachable!("solid tetra solve should return solid tetra result")
                        };
                        node_count = result.nodes.len();
                        element_count = result.elements.len();
                        dof_count = result.nodes.len() * 3;
                        max_displacement = result.max_displacement;
                        max_stress = result.max_von_mises_stress;
                    })
                }
                BenchmarkWorkload::Truss2d(request) => {
                    let options = SpdSolveOptions {
                        preconditioner: parse_preconditioner(solver_preconditioner),
                        progress_interval: progress.then_some(256),
                    };
                    profile_truss_2d_with_options(request, options).map(|profile| {
                        let result = profile.result;
                        node_count = result.nodes.len();
                        element_count = result.elements.len();
                        dof_count = result.nodes.len() * 2;
                        max_displacement = result.max_displacement;
                        max_stress = result.max_stress;
                        solver_iterations = Some(profile.solver_iterations);
                        solver_matrix_non_zero_count = Some(profile.solver_matrix_non_zero_count);
                        solver_residual_norm = Some(profile.solver_residual_norm);
                        solver_preconditioner_name = Some(solver_preconditioner.to_string());
                        memory_stages = profile
                            .stages
                            .into_iter()
                            .map(|stage| BenchmarkMemoryStage {
                                label: stage.label.to_string(),
                                rss_kib: stage.rss_kib,
                                elapsed_ms: Some(stage.elapsed_ms),
                            })
                            .collect();
                    })
                }
                BenchmarkWorkload::HeatPlaneQuad2d(request) => {
                    let options = SpdSolveOptions {
                        preconditioner: parse_preconditioner(solver_preconditioner),
                        progress_interval: progress.then_some(256),
                    };
                    profile_heat_plane_quad_2d_with_options(request, options).map(|profile| {
                        let result = profile.result;
                        node_count = result.nodes.len();
                        element_count = result.elements.len();
                        dof_count = result.nodes.len();
                        max_displacement = result.max_temperature;
                        max_stress = result.max_heat_flux;
                        solver_iterations = Some(profile.solver_iterations);
                        solver_matrix_non_zero_count = Some(profile.solver_matrix_non_zero_count);
                        solver_residual_norm = Some(profile.solver_residual_norm);
                        solver_preconditioner_name = Some(solver_preconditioner.to_string());
                        memory_stages = profile
                            .memory_stages
                            .into_iter()
                            .map(|stage| BenchmarkMemoryStage {
                                label: stage.label.to_string(),
                                rss_kib: stage.rss_kib,
                                elapsed_ms: Some(stage.elapsed_ms),
                            })
                            .collect();
                    })
                }
                BenchmarkWorkload::HeatPlaneTriangle2d(request) => {
                    solve(EngineSolveRequest::HeatPlaneTriangle2d(request.clone())).map(|result| {
                        let AnalysisResult::HeatPlaneTriangle2d(result) = result else {
                            unreachable!("heat triangle solve should return heat result")
                        };
                        node_count = result.nodes.len();
                        element_count = result.elements.len();
                        dof_count = result.nodes.len();
                        max_displacement = result.max_temperature;
                        max_stress = result.max_heat_flux;
                    })
                }
                BenchmarkWorkload::ElectrostaticPlaneTriangle2d(request) => solve(
                    EngineSolveRequest::ElectrostaticPlaneTriangle2d(request.clone()),
                )
                .map(|result| {
                    let AnalysisResult::ElectrostaticPlaneTriangle2d(result) = result else {
                        unreachable!(
                            "electrostatic triangle solve should return electrostatic result"
                        )
                    };
                    node_count = result.nodes.len();
                    element_count = result.elements.len();
                    dof_count = result.nodes.len();
                    max_displacement = result.max_potential;
                    max_stress = result.max_electric_field;
                }),
                BenchmarkWorkload::ElectrostaticPlaneQuad2d(request) => solve(
                    EngineSolveRequest::ElectrostaticPlaneQuad2d(request.clone()),
                )
                .map(|result| {
                    let AnalysisResult::ElectrostaticPlaneQuad2d(result) = result else {
                        unreachable!("electrostatic quad solve should return electrostatic result")
                    };
                    node_count = result.nodes.len();
                    element_count = result.elements.len();
                    dof_count = result.nodes.len();
                    max_displacement = result.max_potential;
                    max_stress = result.max_electric_field;
                }),
                BenchmarkWorkload::MagnetostaticPlaneTriangle2d(request) => solve(
                    EngineSolveRequest::MagnetostaticPlaneTriangle2d(request.clone()),
                )
                .map(|result| {
                    let AnalysisResult::MagnetostaticPlaneTriangle2d(result) = result else {
                        unreachable!(
                            "magnetostatic triangle solve should return magnetostatic result"
                        )
                    };
                    node_count = result.nodes.len();
                    element_count = result.elements.len();
                    dof_count = result.nodes.len();
                    max_displacement = result.max_vector_potential;
                    max_stress = result.max_magnetic_field_strength;
                }),
                BenchmarkWorkload::MagnetostaticPlaneQuad2d(request) => solve(
                    EngineSolveRequest::MagnetostaticPlaneQuad2d(request.clone()),
                )
                .map(|result| {
                    let AnalysisResult::MagnetostaticPlaneQuad2d(result) = result else {
                        unreachable!("magnetostatic quad solve should return magnetostatic result")
                    };
                    node_count = result.nodes.len();
                    element_count = result.elements.len();
                    dof_count = result.nodes.len();
                    max_displacement = result.max_vector_potential;
                    max_stress = result.max_magnetic_field_strength;
                }),
                BenchmarkWorkload::StokesFlowPlaneQuad2d(request) => solve(
                    EngineSolveRequest::StokesFlowPlaneQuad2d(request.clone()),
                )
                .map(|result| {
                    let AnalysisResult::StokesFlowPlaneQuad2d(result) = result else {
                        unreachable!("stokes solve should return stokes result")
                    };
                    node_count = result.nodes.len();
                    element_count = result.elements.len();
                    dof_count = result.nodes.len() * 3;
                    max_displacement = result.max_velocity;
                    max_stress = result.max_pressure;
                }),
                BenchmarkWorkload::StokesFlowPlaneTriangle2d(request) => solve(
                    EngineSolveRequest::StokesFlowPlaneTriangle2d(request.clone()),
                )
                .map(|result| {
                    let AnalysisResult::StokesFlowPlaneTriangle2d(result) = result else {
                        unreachable!("stokes triangle solve should return stokes triangle result")
                    };
                    node_count = result.nodes.len();
                    element_count = result.elements.len();
                    dof_count = result.nodes.len() * 3;
                    max_displacement = result.max_velocity;
                    max_stress = result.max_pressure;
                }),
                BenchmarkWorkload::Truss3d(request) => {
                    solve(EngineSolveRequest::Truss3d(request.clone())).map(|result| {
                        let AnalysisResult::Truss3d(result) = result else {
                            unreachable!("3d truss solve should return 3d truss result")
                        };
                        node_count = result.nodes.len();
                        element_count = result.elements.len();
                        dof_count = result.nodes.len() * 3;
                        max_displacement = result.max_displacement;
                        max_stress = result.max_stress;
                    })
                }
                BenchmarkWorkload::HeadlessActionManifest => {
                    let manifest = action_capability_manifest();
                    let payload = serde_json::to_vec(&manifest)
                        .map_err(|error| format!("manifest serialization failed: {error}"));
                    payload.map(|payload| {
                        node_count = manifest.len();
                        element_count = manifest
                            .iter()
                            .filter(|entry| entry.direct_fem_route.is_some())
                            .count();
                        dof_count = manifest
                            .iter()
                            .map(|entry| {
                                entry.required_payload_keys.len() + entry.output_keys.len()
                            })
                            .sum();
                        max_displacement = element_count as f64;
                        max_stress = payload.len() as f64;
                    })
                }
                BenchmarkWorkload::DirectFemManifest => {
                    let manifest = direct_fem_capability_manifest();
                    let payload = serde_json::to_vec(&manifest).map_err(|error| {
                        format!("direct FEM manifest serialization failed: {error}")
                    });
                    payload.map(|payload| {
                        node_count = manifest.len();
                        element_count = manifest.len();
                        dof_count = manifest
                            .iter()
                            .map(|entry| {
                                entry.required_payload_keys.len() + entry.output_keys.len()
                            })
                            .sum();
                        max_displacement = manifest.len() as f64;
                        max_stress = payload.len() as f64;
                    })
                }
                _ => unreachable!("thermal structural workloads are handled before main dispatch"),
            }
        };

        durations.push(started.elapsed().as_secs_f64() * 1000.0);
        peak_rss_kib = peak_rss_kib.max(current_peak_rss_kib());

        if let Err(message) = outcome {
            error = Some(message);
            break;
        }
    }

    let ok = error.is_none();
    let mut sorted = durations.clone();
    sorted.sort_by(|lhs, rhs| lhs.total_cmp(rhs));
    let min_ms = sorted.iter().copied().fold(f64::INFINITY, f64::min);
    let max_ms = sorted.iter().copied().fold(0.0, f64::max);
    let mean_ms = if durations.is_empty() {
        0.0
    } else {
        durations.iter().copied().sum::<f64>() / durations.len() as f64
    };
    let median_ms = percentile(&sorted, 0.5);
    let hotspot = summarize_hotspot(&memory_stages, median_ms);

    BenchmarkResult {
        id: case.id.clone(),
        family: case.family.to_string(),
        ok,
        error,
        repeat,
        min_ms: if min_ms.is_finite() { min_ms } else { 0.0 },
        median_ms,
        mean_ms,
        p95_ms: percentile(&sorted, 0.95),
        max_ms,
        dof_count,
        node_count,
        element_count,
        peak_rss_kib,
        memory_stages,
        solver_iterations,
        solver_matrix_non_zero_count,
        solver_residual_norm,
        solver_preconditioner: solver_preconditioner_name,
        solver_preconditioner_reason: None,
        hotspot_label: hotspot.as_ref().map(|summary| summary.label.clone()),
        hotspot_elapsed_ms: hotspot.as_ref().map(|summary| summary.elapsed_ms),
        hotspot_share_pct: hotspot.as_ref().map(|summary| summary.share_pct),
        hotspot_hint: hotspot.map(|summary| summary.hint),
        max_displacement,
        max_stress,
    }
}
