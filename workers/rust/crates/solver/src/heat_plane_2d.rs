use crate::heat_plane_2d_element::{
    precompute_heat_plane_quad_element, precompute_heat_plane_triangle_element,
};
use crate::heat_plane_2d_validation::{
    validate_heat_plane_quad_request, validate_heat_plane_triangle_request,
};
use crate::heat_plane_balance::{recover_with_heat_balance, validate_heat_contact_balance};
use crate::heat_plane_contact::{
    assemble_heat_contacts, prepare_heat_contacts, recover_heat_contacts,
};
use crate::heat_plane_results::{recover_nodes, recover_quad, recover_triangle, total_heat_flow};
use crate::linear_algebra::{
    SparseMatrix, add_at, reduce_sparse_system_with_prescribed,
    solve_spd_system_profile_with_options, sparse_residual_norm,
};
use crate::linear_solver_profile::SpdSolveOptions;
use crate::scalar_plane_kernel::shift_prescribed_reference;
use crate::solver_control::{SolverStage, checkpoint, checkpoint_chunk};
use crate::solver_postprocess::{
    collect_results, max_results, restore_solution, try_collect_results,
};
use kyuubiki_protocol::{
    SolveHeatPlaneQuad2dRequest, SolveHeatPlaneQuad2dResult, SolveHeatPlaneTriangle2dRequest,
    SolveHeatPlaneTriangle2dResult,
};
use std::{
    borrow::Cow,
    time::{Duration, Instant},
};

pub fn solve_heat_plane_triangle_2d(
    request: &SolveHeatPlaneTriangle2dRequest,
) -> Result<SolveHeatPlaneTriangle2dResult, String> {
    solve_heat_plane_triangle_2d_internal(Cow::Borrowed(request), SpdSolveOptions::default())
}

pub fn solve_heat_plane_triangle_2d_owned(
    request: SolveHeatPlaneTriangle2dRequest,
) -> Result<SolveHeatPlaneTriangle2dResult, String> {
    solve_heat_plane_triangle_2d_internal(Cow::Owned(request), SpdSolveOptions::default())
}

pub fn solve_heat_plane_triangle_2d_with_options(
    request: &SolveHeatPlaneTriangle2dRequest,
    options: SpdSolveOptions,
) -> Result<SolveHeatPlaneTriangle2dResult, String> {
    solve_heat_plane_triangle_2d_internal(Cow::Borrowed(request), options)
}

fn solve_heat_plane_triangle_2d_internal(
    request: Cow<'_, SolveHeatPlaneTriangle2dRequest>,
    options: SpdSolveOptions,
) -> Result<SolveHeatPlaneTriangle2dResult, String> {
    validate_heat_plane_triangle_request(request.as_ref())?;
    let contacts = prepare_heat_contacts(
        &request.nodes,
        &request.contact_interfaces,
        request.elements.iter().map(|element| {
            (
                [element.node_i, element.node_j, element.node_k],
                element.thickness,
            )
        }),
    )?;
    checkpoint(SolverStage::ElementPrecompute, 0)?;

    let dof_count = request.nodes.len();
    let mut global_stiffness = SparseMatrix::new(dof_count);
    let mut heat_vector = vec![0.0; dof_count];
    let computed_elements = request
        .elements
        .iter()
        .enumerate()
        .map(|(index, element)| {
            let computed = precompute_heat_plane_triangle_element(request.as_ref(), element)?;
            checkpoint_chunk(
                SolverStage::ElementPrecompute,
                index + 1,
                request.elements.len(),
            )?;
            Ok(computed)
        })
        .collect::<Result<Vec<_>, String>>()?;

    for (index, node) in request.nodes.iter().enumerate() {
        heat_vector[index] = node.heat_load;
    }

    checkpoint(SolverStage::ElementAssembly, 0)?;
    for (index, (element, computed)) in request
        .elements
        .iter()
        .zip(computed_elements.iter())
        .enumerate()
    {
        let map = [element.node_i, element.node_j, element.node_k];
        for row in 0..3 {
            for column in 0..3 {
                add_at(
                    &mut global_stiffness,
                    map[row],
                    map[column],
                    computed.stiffness[row][column],
                );
            }
        }
        checkpoint_chunk(
            SolverStage::ElementAssembly,
            index + 1,
            request.elements.len(),
        )?;
    }

    assemble_heat_contacts(&contacts, &mut global_stiffness)?;
    let mut prescribed = request
        .nodes
        .iter()
        .enumerate()
        .filter_map(|(index, node)| node.fix_temperature.then_some((index, node.temperature)))
        .collect::<Vec<_>>();

    let reference = shift_prescribed_reference(&mut prescribed)?;
    let (reduced_stiffness, reduced_heat, free) =
        reduce_sparse_system_with_prescribed(&global_stiffness, &heat_vector, &prescribed)?;
    let reduced_temperatures =
        solve_spd_system_profile_with_options(&reduced_stiffness, &reduced_heat, options.clone())?
            .solution;

    let mut temperatures = restore_solution(dof_count, &prescribed, &free, &reduced_temperatures)?;

    let recovery = recover_with_heat_balance(
        &mut temperatures,
        &free,
        &reduced_stiffness,
        &reduced_heat,
        options,
        |temperatures| {
            let contact_interfaces =
                recover_heat_contacts(&request.contact_interfaces, &contacts, temperatures)?;
            validate_heat_contact_balance(
                &request.nodes,
                temperatures,
                &contact_interfaces,
                request
                    .elements
                    .iter()
                    .zip(&computed_elements)
                    .map(|(element, computed)| {
                        (
                            [element.node_i, element.node_j, element.node_k],
                            &computed.stiffness,
                        )
                    }),
                request.elements.len(),
            )?;
            Ok(contact_interfaces)
        },
    )?;

    let nodes = recover_nodes(&request.nodes, &temperatures, reference)?;
    let elements = try_collect_results(
        SolverStage::ResultElements,
        request
            .elements
            .iter()
            .zip(computed_elements.iter())
            .enumerate()
            .map(|(index, (element, computed))| {
                recover_triangle(index, element, computed, &temperatures, reference)
            }),
    )?;

    let max_temperature = max_results(SolverStage::ResultNodeSummary, &nodes, |node| {
        node.temperature.abs()
    })?;
    let max_heat_flux = max_results(SolverStage::ResultElementSummary, &elements, |element| {
        element.heat_flux_magnitude.abs()
    })?;
    let total_abs_heat_flow_rate =
        total_heat_flow(elements.iter().map(|element| element.heat_flow_rate))?;

    Ok(SolveHeatPlaneTriangle2dResult {
        contact_interfaces: recovery.contacts,
        input: request.into_owned(),
        nodes,
        elements,
        max_temperature,
        max_heat_flux,
        total_abs_heat_flow_rate,
    })
}

pub fn solve_heat_plane_quad_2d(
    request: &SolveHeatPlaneQuad2dRequest,
) -> Result<SolveHeatPlaneQuad2dResult, String> {
    solve_heat_plane_quad_2d_internal(Cow::Borrowed(request), false, SpdSolveOptions::default())
        .map(|profile| profile.result)
}

pub fn solve_heat_plane_quad_2d_owned(
    request: SolveHeatPlaneQuad2dRequest,
) -> Result<SolveHeatPlaneQuad2dResult, String> {
    solve_heat_plane_quad_2d_internal(Cow::Owned(request), false, SpdSolveOptions::default())
        .map(|profile| profile.result)
}

#[derive(Debug, Clone)]
pub struct HeatPlaneQuadMemoryStage {
    pub label: &'static str,
    pub rss_kib: u64,
    pub elapsed_ms: f64,
}

#[derive(Debug, Clone)]
pub struct HeatPlaneQuadProfile {
    pub result: SolveHeatPlaneQuad2dResult,
    pub memory_stages: Vec<HeatPlaneQuadMemoryStage>,
    pub solver_iterations: usize,
    pub solver_matrix_non_zero_count: usize,
    pub solver_residual_norm: f64,
}

pub fn profile_heat_plane_quad_2d(
    request: &SolveHeatPlaneQuad2dRequest,
) -> Result<HeatPlaneQuadProfile, String> {
    profile_heat_plane_quad_2d_with_options(request, SpdSolveOptions::default())
}

pub fn profile_heat_plane_quad_2d_with_options(
    request: &SolveHeatPlaneQuad2dRequest,
    solve_options: SpdSolveOptions,
) -> Result<HeatPlaneQuadProfile, String> {
    solve_heat_plane_quad_2d_internal(Cow::Borrowed(request), true, solve_options)
}

fn solve_heat_plane_quad_2d_internal(
    request: Cow<'_, SolveHeatPlaneQuad2dRequest>,
    collect_memory_stages: bool,
    solve_options: SpdSolveOptions,
) -> Result<HeatPlaneQuadProfile, String> {
    validate_heat_plane_quad_request(request.as_ref())?;
    let contacts = prepare_heat_contacts(
        &request.nodes,
        &request.contact_interfaces,
        request.elements.iter().map(|element| {
            (
                [
                    element.node_i,
                    element.node_j,
                    element.node_k,
                    element.node_l,
                ],
                element.thickness,
            )
        }),
    )?;
    checkpoint(SolverStage::ElementPrecompute, 0)?;

    let dof_count = request.nodes.len();
    let mut global_stiffness = SparseMatrix::new(dof_count);
    let mut heat_vector = vec![0.0; dof_count];
    let mut memory_stages = Vec::new();
    let mut stage_started = Instant::now();
    let computed_elements = request
        .elements
        .iter()
        .enumerate()
        .map(|(index, element)| {
            let computed = precompute_heat_plane_quad_element(request.as_ref(), element)?;
            checkpoint_chunk(
                SolverStage::ElementPrecompute,
                index + 1,
                request.elements.len(),
            )?;
            Ok(computed)
        })
        .collect::<Result<Vec<_>, String>>()?;
    push_heat_plane_quad_memory_stage(
        &mut memory_stages,
        collect_memory_stages,
        "precompute",
        stage_started.elapsed(),
    );
    stage_started = Instant::now();

    for (index, node) in request.nodes.iter().enumerate() {
        heat_vector[index] = node.heat_load;
    }

    checkpoint(SolverStage::ElementAssembly, 0)?;
    for (index, (element, computed)) in request
        .elements
        .iter()
        .zip(computed_elements.iter())
        .enumerate()
    {
        let triangles = [
            (
                [element.node_i, element.node_j, element.node_k],
                &computed.first,
            ),
            (
                [element.node_i, element.node_k, element.node_l],
                &computed.second,
            ),
        ];

        for (nodes, triangle) in triangles {
            let map = [nodes[0], nodes[1], nodes[2]];
            for row in 0..3 {
                for column in 0..3 {
                    add_at(
                        &mut global_stiffness,
                        map[row],
                        map[column],
                        triangle.stiffness[row][column],
                    );
                }
            }
        }
        checkpoint_chunk(
            SolverStage::ElementAssembly,
            index + 1,
            request.elements.len(),
        )?;
    }

    assemble_heat_contacts(&contacts, &mut global_stiffness)?;
    let mut prescribed = request
        .nodes
        .iter()
        .enumerate()
        .filter_map(|(index, node)| node.fix_temperature.then_some((index, node.temperature)))
        .collect::<Vec<_>>();

    push_heat_plane_quad_memory_stage(
        &mut memory_stages,
        collect_memory_stages,
        "assemble_global",
        stage_started.elapsed(),
    );
    stage_started = Instant::now();
    let reference = shift_prescribed_reference(&mut prescribed)?;
    let (reduced_stiffness, reduced_heat, free) =
        reduce_sparse_system_with_prescribed(&global_stiffness, &heat_vector, &prescribed)?;
    push_heat_plane_quad_memory_stage(
        &mut memory_stages,
        collect_memory_stages,
        "reduce_system",
        stage_started.elapsed(),
    );
    stage_started = Instant::now();
    let solve_profile = solve_spd_system_profile_with_options(
        &reduced_stiffness,
        &reduced_heat,
        solve_options.clone(),
    )?;
    let mut solver_iterations = solve_profile.iterations;
    let solver_matrix_non_zero_count = solve_profile.matrix_non_zero_count;
    let mut solver_residual_norm = solve_profile.residual_norm;
    let reduced_temperatures = solve_profile.solution;
    push_heat_plane_quad_memory_stage(
        &mut memory_stages,
        collect_memory_stages,
        "solve_system",
        stage_started.elapsed(),
    );
    if collect_memory_stages {
        memory_stages.extend(solve_profile.stages.into_iter().map(|stage| {
            HeatPlaneQuadMemoryStage {
                label: stage.label,
                rss_kib: current_rss_kib(),
                elapsed_ms: stage.elapsed_ms,
            }
        }));
    }
    stage_started = Instant::now();

    let mut temperatures = restore_solution(dof_count, &prescribed, &free, &reduced_temperatures)?;

    let balance_started = Instant::now();
    let recovery =
        recover_with_heat_balance(
            &mut temperatures,
            &free,
            &reduced_stiffness,
            &reduced_heat,
            solve_options,
            |temperatures| {
                let contact_interfaces =
                    recover_heat_contacts(&request.contact_interfaces, &contacts, temperatures)?;
                validate_heat_contact_balance(
                    &request.nodes,
                    temperatures,
                    &contact_interfaces,
                    request.elements.iter().zip(&computed_elements).flat_map(
                        |(element, computed)| {
                            [
                                (
                                    [element.node_i, element.node_j, element.node_k],
                                    &computed.first.stiffness,
                                ),
                                (
                                    [element.node_i, element.node_k, element.node_l],
                                    &computed.second.stiffness,
                                ),
                            ]
                        },
                    ),
                    request.elements.len().saturating_mul(2),
                )?;
                Ok(contact_interfaces)
            },
        )?;
    solver_iterations += recovery.refinement_iterations;
    if recovery.refinement_passes > 0 {
        let refined = collect_results(
            SolverStage::ResidualValidate,
            free.iter().map(|&index| temperatures[index]),
        )?;
        solver_residual_norm = sparse_residual_norm(&reduced_stiffness, &reduced_heat, &refined)?;
    }
    if !contacts.is_empty() {
        push_heat_plane_quad_memory_stage(
            &mut memory_stages,
            collect_memory_stages,
            if recovery.refinement_passes > 0 {
                "contact_balance_refinement"
            } else {
                "contact_balance"
            },
            balance_started.elapsed(),
        );
    }

    let nodes = recover_nodes(&request.nodes, &temperatures, reference)?;
    let elements = try_collect_results(
        SolverStage::ResultElements,
        request
            .elements
            .iter()
            .zip(computed_elements.iter())
            .enumerate()
            .map(|(index, (element, computed))| {
                recover_quad(index, element, computed, &temperatures, reference)
            }),
    )?;

    let max_temperature = max_results(SolverStage::ResultNodeSummary, &nodes, |node| {
        node.temperature.abs()
    })?;
    let max_heat_flux = max_results(SolverStage::ResultElementSummary, &elements, |element| {
        element.heat_flux_magnitude.abs()
    })?;
    let total_abs_heat_flow_rate =
        total_heat_flow(elements.iter().map(|element| element.heat_flow_rate))?;

    push_heat_plane_quad_memory_stage(
        &mut memory_stages,
        collect_memory_stages,
        "assemble",
        stage_started.elapsed(),
    );

    Ok(HeatPlaneQuadProfile {
        result: SolveHeatPlaneQuad2dResult {
            contact_interfaces: recovery.contacts,
            input: request.into_owned(),
            nodes,
            elements,
            max_temperature,
            max_heat_flux,
            total_abs_heat_flow_rate,
        },
        memory_stages,
        solver_iterations,
        solver_matrix_non_zero_count,
        solver_residual_norm,
    })
}

fn push_heat_plane_quad_memory_stage(
    stages: &mut Vec<HeatPlaneQuadMemoryStage>,
    enabled: bool,
    label: &'static str,
    elapsed: Duration,
) {
    if !enabled {
        return;
    }

    stages.push(HeatPlaneQuadMemoryStage {
        label,
        rss_kib: current_rss_kib(),
        elapsed_ms: elapsed.as_secs_f64() * 1000.0,
    });
}

fn current_rss_kib() -> u64 {
    #[cfg(target_os = "linux")]
    {
        if let Ok(statm) = std::fs::read_to_string("/proc/self/statm") {
            if let Some(resident_pages) = statm.split_whitespace().nth(1) {
                if let Ok(resident_pages) = resident_pages.parse::<u64>() {
                    let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
                    if page_size > 0 {
                        return resident_pages * page_size as u64 / 1024;
                    }
                }
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        let mut usage = std::mem::MaybeUninit::<libc::rusage>::uninit();
        let status = unsafe { libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) };
        if status == 0 {
            let usage = unsafe { usage.assume_init() };
            return (usage.ru_maxrss as u64) / 1024;
        }
    }

    0
}
