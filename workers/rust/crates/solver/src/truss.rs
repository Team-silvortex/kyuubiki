use std::time::{Duration, Instant};

use crate::linear_algebra::{
    SparseMatrix, add_at, reduce_sparse_system, solve_spd_system,
    solve_spd_system_profile_with_options,
};
use crate::linear_solver_profile::SpdSolveOptions;
use crate::truss_summary::{
    max_truss_2d_displacement, max_truss_3d_displacement, max_truss_3d_strain_energy_density,
    max_truss_3d_stress, max_truss_strain_energy_density, max_truss_stress,
    total_truss_2d_strain_energy, total_truss_3d_strain_energy, validate_small_displacement_truss,
    validate_small_displacement_truss_3d,
};
use kyuubiki_protocol::{
    SolveTruss2dRequest, SolveTruss2dResult, SolveTruss3dRequest, SolveTruss3dResult,
    Truss3dElementResult, Truss3dNodeResult, TrussElementResult, TrussNodeResult,
};

pub fn solve_truss_2d(request: &SolveTruss2dRequest) -> Result<SolveTruss2dResult, String> {
    solve_truss_2d_internal(request, false, SpdSolveOptions::default())
        .map(|profile| profile.result)
}

#[derive(Debug, Clone)]
pub struct Truss2dProfileStage {
    pub label: &'static str,
    pub rss_kib: u64,
    pub elapsed_ms: f64,
}

#[derive(Debug, Clone)]
pub struct Truss2dProfile {
    pub result: SolveTruss2dResult,
    pub stages: Vec<Truss2dProfileStage>,
    pub solver_iterations: usize,
    pub solver_matrix_non_zero_count: usize,
    pub solver_residual_norm: f64,
}

pub fn profile_truss_2d(request: &SolveTruss2dRequest) -> Result<Truss2dProfile, String> {
    profile_truss_2d_with_options(request, SpdSolveOptions::default())
}

pub fn profile_truss_2d_with_options(
    request: &SolveTruss2dRequest,
    options: SpdSolveOptions,
) -> Result<Truss2dProfile, String> {
    solve_truss_2d_internal(request, true, options)
}

fn solve_truss_2d_internal(
    request: &SolveTruss2dRequest,
    collect_stages: bool,
    solve_options: SpdSolveOptions,
) -> Result<Truss2dProfile, String> {
    validate_truss_request(request)?;

    let dof_count = request.nodes.len() * 2;
    let mut global_stiffness = SparseMatrix::new(dof_count);
    let mut force_vector = vec![0.0; dof_count];
    let mut stages = Vec::new();
    let mut stage_started = Instant::now();

    for (index, node) in request.nodes.iter().enumerate() {
        force_vector[index * 2] = node.load_x;
        force_vector[index * 2 + 1] = node.load_y;
    }

    for element in &request.elements {
        let node_i = &request.nodes[element.node_i];
        let node_j = &request.nodes[element.node_j];
        let dx = node_j.x - node_i.x;
        let dy = node_j.y - node_i.y;
        let length = (dx * dx + dy * dy).sqrt();
        let c = dx / length;
        let s = dy / length;
        let k = element.youngs_modulus * element.area / length;

        let local = [
            [c * c, c * s, -c * c, -c * s],
            [c * s, s * s, -c * s, -s * s],
            [-c * c, -c * s, c * c, c * s],
            [-c * s, -s * s, c * s, s * s],
        ];

        let map = [
            element.node_i * 2,
            element.node_i * 2 + 1,
            element.node_j * 2,
            element.node_j * 2 + 1,
        ];

        for row in 0..4 {
            for column in 0..4 {
                add_at(
                    &mut global_stiffness,
                    map[row],
                    map[column],
                    k * local[row][column],
                );
            }
        }
    }
    push_truss_2d_stage(
        &mut stages,
        collect_stages,
        "assemble_global",
        stage_started.elapsed(),
    );

    stage_started = Instant::now();
    let constrained = request
        .nodes
        .iter()
        .enumerate()
        .flat_map(|(index, node)| {
            let mut dofs = Vec::new();
            if node.fix_x {
                dofs.push(index * 2);
            }
            if node.fix_y {
                dofs.push(index * 2 + 1);
            }
            dofs
        })
        .collect::<Vec<_>>();

    let (reduced_stiffness, reduced_force, free) =
        reduce_sparse_system(&global_stiffness, &force_vector, &constrained);
    push_truss_2d_stage(
        &mut stages,
        collect_stages,
        "reduce_system",
        stage_started.elapsed(),
    );

    stage_started = Instant::now();
    let solve_profile =
        solve_spd_system_profile_with_options(&reduced_stiffness, &reduced_force, solve_options)?;
    let solver_iterations = solve_profile.iterations;
    let solver_matrix_non_zero_count = solve_profile.matrix_non_zero_count;
    let solver_residual_norm = solve_profile.residual_norm;
    if collect_stages {
        for stage in solve_profile.stages.iter() {
            stages.push(Truss2dProfileStage {
                label: stage.label,
                rss_kib: current_rss_kib(),
                elapsed_ms: stage.elapsed_ms,
            });
        }
    }
    let reduced_displacements = solve_profile.solution;
    push_truss_2d_stage(
        &mut stages,
        collect_stages,
        "solve_system",
        stage_started.elapsed(),
    );

    stage_started = Instant::now();
    let mut displacements = vec![0.0; dof_count];
    for (index, &dof) in free.iter().enumerate() {
        displacements[dof] = reduced_displacements[index];
    }

    let nodes = request
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| TrussNodeResult {
            index,
            id: node.id.clone(),
            x: node.x,
            y: node.y,
            ux: displacements[index * 2],
            uy: displacements[index * 2 + 1],
        })
        .collect::<Vec<_>>();

    let elements = request
        .elements
        .iter()
        .enumerate()
        .map(|(index, element)| {
            let node_i = &request.nodes[element.node_i];
            let node_j = &request.nodes[element.node_j];
            let dx = node_j.x - node_i.x;
            let dy = node_j.y - node_i.y;
            let length = (dx * dx + dy * dy).sqrt();
            let c = dx / length;
            let s = dy / length;

            let ux_i = displacements[element.node_i * 2];
            let uy_i = displacements[element.node_i * 2 + 1];
            let ux_j = displacements[element.node_j * 2];
            let uy_j = displacements[element.node_j * 2 + 1];
            let axial_extension = (ux_j - ux_i) * c + (uy_j - uy_i) * s;
            let strain = axial_extension / length;
            let stress = element.youngs_modulus * strain;
            let strain_energy_density = 0.5 * stress * strain;

            TrussElementResult {
                index,
                id: element.id.clone(),
                node_i: element.node_i,
                node_j: element.node_j,
                length,
                strain,
                stress,
                axial_force: stress * element.area,
                strain_energy_density,
            }
        })
        .collect::<Vec<_>>();

    let max_displacement = max_truss_2d_displacement(&nodes);
    let max_stress = max_truss_stress(&elements);
    let total_strain_energy = total_truss_2d_strain_energy(request, &elements);
    let max_strain_energy_density = max_truss_strain_energy_density(&elements);

    validate_small_displacement_truss(request, max_displacement)?;
    push_truss_2d_stage(
        &mut stages,
        collect_stages,
        "assemble_result",
        stage_started.elapsed(),
    );

    Ok(Truss2dProfile {
        result: SolveTruss2dResult {
            input: request.clone(),
            nodes,
            elements,
            max_displacement,
            max_stress,
            total_strain_energy,
            max_strain_energy_density,
        },
        stages,
        solver_iterations,
        solver_matrix_non_zero_count,
        solver_residual_norm,
    })
}

fn push_truss_2d_stage(
    stages: &mut Vec<Truss2dProfileStage>,
    enabled: bool,
    label: &'static str,
    elapsed: Duration,
) {
    if !enabled {
        return;
    }

    stages.push(Truss2dProfileStage {
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

pub fn solve_truss_3d(request: &SolveTruss3dRequest) -> Result<SolveTruss3dResult, String> {
    validate_truss_3d_request(request)?;

    let dof_count = request.nodes.len() * 3;
    let mut global_stiffness = SparseMatrix::new(dof_count);
    let mut force_vector = vec![0.0; dof_count];

    for (index, node) in request.nodes.iter().enumerate() {
        force_vector[index * 3] = node.load_x;
        force_vector[index * 3 + 1] = node.load_y;
        force_vector[index * 3 + 2] = node.load_z;
    }

    for element in &request.elements {
        let node_i = &request.nodes[element.node_i];
        let node_j = &request.nodes[element.node_j];
        let dx = node_j.x - node_i.x;
        let dy = node_j.y - node_i.y;
        let dz = node_j.z - node_i.z;
        let length = (dx * dx + dy * dy + dz * dz).sqrt();
        let l = dx / length;
        let m = dy / length;
        let n = dz / length;
        let k = element.youngs_modulus * element.area / length;

        let local = [
            [l * l, l * m, l * n, -l * l, -l * m, -l * n],
            [l * m, m * m, m * n, -l * m, -m * m, -m * n],
            [l * n, m * n, n * n, -l * n, -m * n, -n * n],
            [-l * l, -l * m, -l * n, l * l, l * m, l * n],
            [-l * m, -m * m, -m * n, l * m, m * m, m * n],
            [-l * n, -m * n, -n * n, l * n, m * n, n * n],
        ];

        let map = [
            element.node_i * 3,
            element.node_i * 3 + 1,
            element.node_i * 3 + 2,
            element.node_j * 3,
            element.node_j * 3 + 1,
            element.node_j * 3 + 2,
        ];

        for row in 0..6 {
            for column in 0..6 {
                add_at(
                    &mut global_stiffness,
                    map[row],
                    map[column],
                    k * local[row][column],
                );
            }
        }
    }

    let constrained = request
        .nodes
        .iter()
        .enumerate()
        .flat_map(|(index, node)| {
            let mut dofs = Vec::new();
            if node.fix_x {
                dofs.push(index * 3);
            }
            if node.fix_y {
                dofs.push(index * 3 + 1);
            }
            if node.fix_z {
                dofs.push(index * 3 + 2);
            }
            dofs
        })
        .collect::<Vec<_>>();

    let (reduced_stiffness, reduced_force, free) =
        reduce_sparse_system(&global_stiffness, &force_vector, &constrained);
    let reduced_displacements = solve_spd_system(&reduced_stiffness, &reduced_force)?;

    let mut displacements = vec![0.0; dof_count];
    for (index, &dof) in free.iter().enumerate() {
        displacements[dof] = reduced_displacements[index];
    }

    let nodes = request
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| Truss3dNodeResult {
            index,
            id: node.id.clone(),
            x: node.x,
            y: node.y,
            z: node.z,
            ux: displacements[index * 3],
            uy: displacements[index * 3 + 1],
            uz: displacements[index * 3 + 2],
        })
        .collect::<Vec<_>>();

    let elements = request
        .elements
        .iter()
        .enumerate()
        .map(|(index, element)| {
            let node_i = &request.nodes[element.node_i];
            let node_j = &request.nodes[element.node_j];
            let dx = node_j.x - node_i.x;
            let dy = node_j.y - node_i.y;
            let dz = node_j.z - node_i.z;
            let length = (dx * dx + dy * dy + dz * dz).sqrt();
            let l = dx / length;
            let m = dy / length;
            let n = dz / length;

            let ux_i = displacements[element.node_i * 3];
            let uy_i = displacements[element.node_i * 3 + 1];
            let uz_i = displacements[element.node_i * 3 + 2];
            let ux_j = displacements[element.node_j * 3];
            let uy_j = displacements[element.node_j * 3 + 1];
            let uz_j = displacements[element.node_j * 3 + 2];
            let axial_extension = (ux_j - ux_i) * l + (uy_j - uy_i) * m + (uz_j - uz_i) * n;
            let strain = axial_extension / length;
            let stress = element.youngs_modulus * strain;
            let strain_energy_density = 0.5 * stress * strain;

            Truss3dElementResult {
                index,
                id: element.id.clone(),
                node_i: element.node_i,
                node_j: element.node_j,
                length,
                strain,
                stress,
                axial_force: stress * element.area,
                strain_energy_density,
            }
        })
        .collect::<Vec<_>>();

    let max_displacement = max_truss_3d_displacement(&nodes);
    let max_stress = max_truss_3d_stress(&elements);
    let total_strain_energy = total_truss_3d_strain_energy(request, &elements);
    let max_strain_energy_density = max_truss_3d_strain_energy_density(&elements);

    validate_small_displacement_truss_3d(request, max_displacement)?;

    Ok(SolveTruss3dResult {
        input: request.clone(),
        nodes,
        elements,
        max_displacement,
        max_stress,
        total_strain_energy,
        max_strain_energy_density,
    })
}

fn validate_truss_request(request: &SolveTruss2dRequest) -> Result<(), String> {
    if request.nodes.len() < 2 {
        return Err("truss must define at least two nodes".to_string());
    }

    if request.elements.is_empty() {
        return Err("truss must define at least one element".to_string());
    }

    if !request.nodes.iter().any(|node| node.fix_x || node.fix_y) {
        return Err("truss must include at least one support".to_string());
    }

    for element in &request.elements {
        if element.node_i >= request.nodes.len() || element.node_j >= request.nodes.len() {
            return Err("truss element references an out-of-range node".to_string());
        }

        if !(element.area.is_finite() && element.area > 0.0) {
            return Err("truss element area must be positive".to_string());
        }

        if !(element.youngs_modulus.is_finite() && element.youngs_modulus > 0.0) {
            return Err("truss element youngs_modulus must be positive".to_string());
        }
    }

    Ok(())
}

fn validate_truss_3d_request(request: &SolveTruss3dRequest) -> Result<(), String> {
    if request.nodes.len() < 3 {
        return Err("3d truss must define at least three nodes".to_string());
    }

    if request.elements.is_empty() {
        return Err("3d truss must define at least one element".to_string());
    }

    let constrained_dofs = request.nodes.iter().fold(0, |sum, node| {
        sum + usize::from(node.fix_x) + usize::from(node.fix_y) + usize::from(node.fix_z)
    });
    if constrained_dofs < 6 {
        return Err("3d truss must restrain at least six degrees of freedom".to_string());
    }

    for element in &request.elements {
        if element.node_i >= request.nodes.len() || element.node_j >= request.nodes.len() {
            return Err("3d truss element references an out-of-range node".to_string());
        }
        if !(element.area.is_finite() && element.area > 0.0) {
            return Err("3d truss element area must be positive".to_string());
        }
        if !(element.youngs_modulus.is_finite() && element.youngs_modulus > 0.0) {
            return Err("3d truss element youngs_modulus must be positive".to_string());
        }
        let node_i = &request.nodes[element.node_i];
        let node_j = &request.nodes[element.node_j];
        let length = ((node_j.x - node_i.x).powi(2)
            + (node_j.y - node_i.y).powi(2)
            + (node_j.z - node_i.z).powi(2))
        .sqrt();
        if length <= 1.0e-12 {
            return Err("3d truss element length must be positive".to_string());
        }
    }

    Ok(())
}
