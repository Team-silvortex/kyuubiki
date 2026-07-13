use crate::heat_plane_2d_element::{
    plane_triangle_scalar_gradient, precompute_heat_plane_quad_element,
    precompute_heat_plane_triangle_element,
};
use crate::heat_plane_2d_validation::{
    validate_heat_plane_quad_request, validate_heat_plane_triangle_request,
};
use crate::linear_algebra::{
    SparseMatrix, add_at, reduce_sparse_system_with_prescribed, solve_spd_system,
    solve_spd_system_profile_with_options,
};
use crate::linear_solver_profile::SpdSolveOptions;
use kyuubiki_protocol::{
    HeatPlaneNodeResult, HeatPlaneQuadElementResult, HeatPlaneTriangleElementResult,
    SolveHeatPlaneQuad2dRequest, SolveHeatPlaneQuad2dResult, SolveHeatPlaneTriangle2dRequest,
    SolveHeatPlaneTriangle2dResult,
};
use std::time::{Duration, Instant};

pub fn solve_heat_plane_triangle_2d(
    request: &SolveHeatPlaneTriangle2dRequest,
) -> Result<SolveHeatPlaneTriangle2dResult, String> {
    validate_heat_plane_triangle_request(request)?;

    let dof_count = request.nodes.len();
    let mut global_stiffness = SparseMatrix::new(dof_count);
    let mut heat_vector = vec![0.0; dof_count];
    let computed_elements = request
        .elements
        .iter()
        .map(|element| precompute_heat_plane_triangle_element(request, element))
        .collect::<Result<Vec<_>, String>>()?;

    for (index, node) in request.nodes.iter().enumerate() {
        heat_vector[index] = node.heat_load;
    }

    for (element, computed) in request.elements.iter().zip(computed_elements.iter()) {
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
    }

    let prescribed = request
        .nodes
        .iter()
        .enumerate()
        .filter_map(|(index, node)| node.fix_temperature.then_some((index, node.temperature)))
        .collect::<Vec<_>>();

    let (reduced_stiffness, reduced_heat, free) =
        reduce_sparse_system_with_prescribed(&global_stiffness, &heat_vector, &prescribed);
    let reduced_temperatures = solve_spd_system(&reduced_stiffness, &reduced_heat)?;

    let mut temperatures = vec![0.0; dof_count];
    for &(index, value) in &prescribed {
        temperatures[index] = value;
    }
    for (index, &dof) in free.iter().enumerate() {
        temperatures[dof] = reduced_temperatures[index];
    }

    let nodes = request
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| HeatPlaneNodeResult {
            index,
            id: node.id.clone(),
            x: node.x,
            y: node.y,
            temperature: temperatures[index],
            heat_load: node.heat_load,
        })
        .collect::<Vec<_>>();

    let elements = request
        .elements
        .iter()
        .zip(computed_elements.iter())
        .enumerate()
        .map(|(index, (element, computed))| {
            let element_temperatures = [
                temperatures[element.node_i],
                temperatures[element.node_j],
                temperatures[element.node_k],
            ];
            let gradient = plane_triangle_scalar_gradient(
                &computed.gradient_x,
                &computed.gradient_y,
                &element_temperatures,
            );
            let heat_flux_x = -element.conductivity * gradient[0];
            let heat_flux_y = -element.conductivity * gradient[1];
            let heat_flux_magnitude =
                (heat_flux_x * heat_flux_x + heat_flux_y * heat_flux_y).sqrt();

            HeatPlaneTriangleElementResult {
                index,
                id: element.id.clone(),
                node_i: element.node_i,
                node_j: element.node_j,
                node_k: element.node_k,
                area: computed.area,
                average_temperature: element_temperatures.iter().sum::<f64>() / 3.0,
                temperature_gradient_x: gradient[0],
                temperature_gradient_y: gradient[1],
                heat_flux_x,
                heat_flux_y,
                heat_flux_magnitude,
                heat_flow_rate: heat_flux_magnitude * computed.area * element.thickness,
            }
        })
        .collect::<Vec<_>>();

    let max_temperature = nodes
        .iter()
        .map(|node| node.temperature.abs())
        .fold(0.0_f64, f64::max);
    let max_heat_flux = elements
        .iter()
        .map(|element| element.heat_flux_magnitude.abs())
        .fold(0.0_f64, f64::max);
    let total_abs_heat_flow_rate = elements
        .iter()
        .map(|element| element.heat_flow_rate.abs())
        .sum();

    Ok(SolveHeatPlaneTriangle2dResult {
        input: request.clone(),
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
    solve_heat_plane_quad_2d_internal(request, false, SpdSolveOptions::default())
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
    solve_heat_plane_quad_2d_internal(request, true, solve_options)
}

fn solve_heat_plane_quad_2d_internal(
    request: &SolveHeatPlaneQuad2dRequest,
    collect_memory_stages: bool,
    solve_options: SpdSolveOptions,
) -> Result<HeatPlaneQuadProfile, String> {
    validate_heat_plane_quad_request(request)?;

    let dof_count = request.nodes.len();
    let mut global_stiffness = SparseMatrix::new(dof_count);
    let mut heat_vector = vec![0.0; dof_count];
    let mut memory_stages = Vec::new();
    let mut stage_started = Instant::now();
    let computed_elements = request
        .elements
        .iter()
        .map(|element| precompute_heat_plane_quad_element(request, element))
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

    for (element, computed) in request.elements.iter().zip(computed_elements.iter()) {
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
    }

    let prescribed = request
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
    let (reduced_stiffness, reduced_heat, free) =
        reduce_sparse_system_with_prescribed(&global_stiffness, &heat_vector, &prescribed);
    push_heat_plane_quad_memory_stage(
        &mut memory_stages,
        collect_memory_stages,
        "reduce_system",
        stage_started.elapsed(),
    );
    stage_started = Instant::now();
    let solve_profile =
        solve_spd_system_profile_with_options(&reduced_stiffness, &reduced_heat, solve_options)?;
    let solver_iterations = solve_profile.iterations;
    let solver_matrix_non_zero_count = solve_profile.matrix_non_zero_count;
    let solver_residual_norm = solve_profile.residual_norm;
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

    let mut temperatures = vec![0.0; dof_count];
    for &(index, value) in &prescribed {
        temperatures[index] = value;
    }
    for (index, &dof) in free.iter().enumerate() {
        temperatures[dof] = reduced_temperatures[index];
    }

    let nodes = request
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| HeatPlaneNodeResult {
            index,
            id: node.id.clone(),
            x: node.x,
            y: node.y,
            temperature: temperatures[index],
            heat_load: node.heat_load,
        })
        .collect::<Vec<_>>();

    let elements = request
        .elements
        .iter()
        .zip(computed_elements.iter())
        .enumerate()
        .map(|(index, (element, computed))| {
            let first_temperatures = [
                temperatures[element.node_i],
                temperatures[element.node_j],
                temperatures[element.node_k],
            ];
            let second_temperatures = [
                temperatures[element.node_i],
                temperatures[element.node_k],
                temperatures[element.node_l],
            ];
            let first_gradient = plane_triangle_scalar_gradient(
                &computed.first.gradient_x,
                &computed.first.gradient_y,
                &first_temperatures,
            );
            let second_gradient = plane_triangle_scalar_gradient(
                &computed.second.gradient_x,
                &computed.second.gradient_y,
                &second_temperatures,
            );
            let total_area = computed.first.area + computed.second.area;
            let weighted = |left: f64, right: f64| -> f64 {
                ((left * computed.first.area) + (right * computed.second.area)) / total_area
            };
            let heat_flux_x =
                -element.conductivity * weighted(first_gradient[0], second_gradient[0]);
            let heat_flux_y =
                -element.conductivity * weighted(first_gradient[1], second_gradient[1]);
            let heat_flux_magnitude =
                (heat_flux_x * heat_flux_x + heat_flux_y * heat_flux_y).sqrt();

            HeatPlaneQuadElementResult {
                index,
                id: element.id.clone(),
                node_i: element.node_i,
                node_j: element.node_j,
                node_k: element.node_k,
                node_l: element.node_l,
                area: total_area,
                average_temperature: (temperatures[element.node_i]
                    + temperatures[element.node_j]
                    + temperatures[element.node_k]
                    + temperatures[element.node_l])
                    / 4.0,
                temperature_gradient_x: weighted(first_gradient[0], second_gradient[0]),
                temperature_gradient_y: weighted(first_gradient[1], second_gradient[1]),
                heat_flux_x,
                heat_flux_y,
                heat_flux_magnitude,
                heat_flow_rate: heat_flux_magnitude * total_area * element.thickness,
            }
        })
        .collect::<Vec<_>>();

    let max_temperature = nodes
        .iter()
        .map(|node| node.temperature.abs())
        .fold(0.0_f64, f64::max);
    let max_heat_flux = elements
        .iter()
        .map(|element| element.heat_flux_magnitude.abs())
        .fold(0.0_f64, f64::max);
    let total_abs_heat_flow_rate = elements
        .iter()
        .map(|element| element.heat_flow_rate.abs())
        .sum();

    push_heat_plane_quad_memory_stage(
        &mut memory_stages,
        collect_memory_stages,
        "assemble",
        stage_started.elapsed(),
    );

    Ok(HeatPlaneQuadProfile {
        result: SolveHeatPlaneQuad2dResult {
            input: request.clone(),
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
