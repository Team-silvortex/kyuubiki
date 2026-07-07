use std::time::Instant;

use kyuubiki_protocol::{
    SolvePlaneQuad2dResult, SolvePlaneTriangle2dRequest, SolvePlaneTriangle2dResult,
};

use crate::linear_algebra::{
    SparseMatrix, reduce_sparse_system, solve_spd_system_profile_with_options,
};
use crate::linear_solver_profile::SpdSolveOptions;

#[derive(Debug, Clone)]
pub struct PlaneProfileStage {
    pub label: &'static str,
    pub rss_kib: u64,
    pub elapsed_ms: f64,
}

#[derive(Debug, Clone)]
pub struct PlaneTriangleProfile {
    pub result: SolvePlaneTriangle2dResult,
    pub solver_iterations: usize,
    pub solver_matrix_non_zero_count: usize,
    pub solver_residual_norm: f64,
    pub stages: Vec<PlaneProfileStage>,
}

#[derive(Debug, Clone)]
pub struct PlaneQuadProfile {
    pub result: SolvePlaneQuad2dResult,
    pub solver_iterations: usize,
    pub solver_matrix_non_zero_count: usize,
    pub solver_residual_norm: f64,
    pub stages: Vec<PlaneProfileStage>,
}

#[derive(Debug, Clone)]
pub(crate) struct PlaneDisplacementProfileStage {
    pub(crate) label: &'static str,
    pub(crate) rss_kib: u64,
    pub(crate) elapsed_ms: f64,
}

pub(crate) fn push_plane_profile_stage(
    stages: &mut Vec<PlaneProfileStage>,
    enabled: bool,
    label: &'static str,
    started: Instant,
) {
    if !enabled {
        return;
    }

    stages.push(PlaneProfileStage {
        label,
        rss_kib: current_rss_kib(),
        elapsed_ms: started.elapsed().as_secs_f64() * 1000.0,
    });
}

#[derive(Debug, Clone)]
pub(crate) struct PlaneDisplacementProfile {
    pub(crate) displacements: Vec<f64>,
    pub(crate) solver_iterations: usize,
    pub(crate) solver_matrix_non_zero_count: usize,
    pub(crate) solver_residual_norm: f64,
    pub(crate) stages: Vec<PlaneDisplacementProfileStage>,
}

pub(crate) fn profile_plane_displacements_with_options(
    request: &SolvePlaneTriangle2dRequest,
    global_stiffness: &SparseMatrix,
    force_vector: &[f64],
    solve_options: SpdSolveOptions,
) -> Result<PlaneDisplacementProfile, String> {
    let constrained = constrained_plane_dofs(request);
    let mut stages = Vec::new();

    let started = Instant::now();
    let (reduced_stiffness, reduced_force, free) =
        reduce_sparse_system(global_stiffness, force_vector, &constrained);
    push_stage(&mut stages, "reduce_system", started);

    let started = Instant::now();
    let solve_profile =
        solve_spd_system_profile_with_options(&reduced_stiffness, &reduced_force, solve_options)?;
    push_stage(&mut stages, "solve_spd_system", started);
    for stage in solve_profile.stages.iter() {
        stages.push(PlaneDisplacementProfileStage {
            label: stage.label,
            rss_kib: current_rss_kib(),
            elapsed_ms: stage.elapsed_ms,
        });
    }

    let started = Instant::now();
    let mut displacements = vec![0.0; request.nodes.len() * 2];
    for (index, &dof) in free.iter().enumerate() {
        displacements[dof] = solve_profile.solution[index];
    }
    push_stage(&mut stages, "scatter_solution", started);

    Ok(PlaneDisplacementProfile {
        displacements,
        solver_iterations: solve_profile.iterations,
        solver_matrix_non_zero_count: solve_profile.matrix_non_zero_count,
        solver_residual_norm: solve_profile.residual_norm,
        stages,
    })
}

fn constrained_plane_dofs(request: &SolvePlaneTriangle2dRequest) -> Vec<usize> {
    request
        .nodes
        .iter()
        .enumerate()
        .flat_map(|(index, node)| {
            [
                node.fix_x.then_some(index * 2),
                node.fix_y.then_some(index * 2 + 1),
            ]
        })
        .flatten()
        .collect()
}

fn push_stage(
    stages: &mut Vec<PlaneDisplacementProfileStage>,
    label: &'static str,
    started: Instant,
) {
    stages.push(PlaneDisplacementProfileStage {
        label,
        rss_kib: current_rss_kib(),
        elapsed_ms: started.elapsed().as_secs_f64() * 1000.0,
    });
}

pub(crate) fn current_rss_kib() -> u64 {
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
