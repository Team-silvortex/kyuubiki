use kyuubiki_protocol::{Job, JobStatus, ProgressEvent};

mod bar_1d;
mod beam_1d;
mod electrostatic_plane_2d;
mod electrostatic_plane_2d_validation;
mod frame_2d;
mod frame_2d_math;
mod frame_3d;
mod frame_3d_math;
mod heat_plane_2d;
mod heat_plane_2d_validation;
mod linear_algebra;
mod magnetostatic_bar_1d;
mod magnetostatic_plane_2d;
mod modal_frame_2d;
mod modal_frame_3d;
mod modal_math;
mod nonlinear_spring_1d;
mod plane_2d;
mod plane_2d_math;
mod spring;
mod stokes_flow_plane_2d;
mod thermal_frame_3d;
mod thermal_plane_2d;
mod thermal_plane_2d_validation;
mod thermal_truss;
mod torsion_1d;
mod truss;

pub use bar_1d::{
    solve_bar_1d, solve_electrostatic_bar_1d, solve_heat_bar_1d, solve_thermal_bar_1d,
};
pub use beam_1d::{solve_beam_1d, solve_thermal_beam_1d};
pub use electrostatic_plane_2d::{
    solve_electrostatic_plane_quad_2d, solve_electrostatic_plane_triangle_2d,
};
pub use frame_2d::{solve_frame_2d, solve_thermal_frame_2d};
pub use frame_3d::solve_frame_3d;
pub use heat_plane_2d::{
    HeatPlaneQuadMemoryStage, HeatPlaneQuadProfile, profile_heat_plane_quad_2d,
    solve_heat_plane_quad_2d, solve_heat_plane_triangle_2d,
};
pub use magnetostatic_bar_1d::solve_magnetostatic_bar_1d;
pub use magnetostatic_plane_2d::{
    solve_magnetostatic_plane_quad_2d, solve_magnetostatic_plane_triangle_2d,
};
pub use modal_frame_2d::solve_modal_frame_2d;
pub use modal_frame_3d::solve_modal_frame_3d;
pub use nonlinear_spring_1d::{solve_contact_gap_1d, solve_nonlinear_spring_1d};
pub use plane_2d::{solve_plane_quad_2d, solve_plane_triangle_2d};
pub use spring::{solve_spring_1d, solve_spring_2d, solve_spring_3d};
pub use stokes_flow_plane_2d::solve_stokes_flow_plane_quad_2d;
pub use thermal_frame_3d::solve_thermal_frame_3d;
pub use thermal_plane_2d::{solve_thermal_plane_quad_2d, solve_thermal_plane_triangle_2d};
pub use thermal_truss::{solve_thermal_truss_2d, solve_thermal_truss_3d};
pub use torsion_1d::solve_torsion_1d;
pub use truss::{solve_truss_2d, solve_truss_3d};

pub struct MockSolver {
    step_count: u64,
}

impl MockSolver {
    pub fn new(step_count: u64) -> Self {
        Self {
            step_count: step_count.max(1),
        }
    }

    pub fn solve(&self, job: &Job) -> Vec<ProgressEvent> {
        let mut events = Vec::with_capacity((self.step_count + 1) as usize);

        for step in 1..=self.step_count {
            let progress = step as f32 / self.step_count as f32;
            let mut event = ProgressEvent::new(job.job_id.clone(), JobStatus::Solving, progress);
            event.iteration = Some(step);
            event.residual = Some(1.0 / (step as f64 + 1.0));
            event.peak_memory = Some(512 + step * 32);
            event.message = Some(format!("mock solve step {step}/{}", self.step_count));
            events.push(event);
        }

        events.push(ProgressEvent::new(
            job.job_id.clone(),
            JobStatus::Completed,
            1.0,
        ));

        events
    }
}

#[cfg(test)]
mod tests;
