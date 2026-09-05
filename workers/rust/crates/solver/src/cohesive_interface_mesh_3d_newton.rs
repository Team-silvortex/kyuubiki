use crate::cohesive_interface_3d::CohesiveInterface3dState;
use crate::cohesive_interface_mesh_3d::{ValidatedModel, assemble};
use crate::cohesive_interface_mesh_3d_control::{ControlStep, restricted_norm};
use crate::linear_algebra::{reduce_sparse_system, stable_l2_norm};
use crate::linear_symmetric_tangent::{DENSE_PIVOTED_FALLBACK, solve_symmetric_tangent};

const MAX_DENSE_FALLBACK_DOFS: usize = 1_536;

pub(crate) struct LoadStepOutcome {
    pub(crate) displacements: Vec<f64>,
    pub(crate) states: Vec<CohesiveInterface3dState>,
    pub(crate) iterations: usize,
    pub(crate) residual_norm: f64,
    pub(crate) converged: bool,
    pub(crate) failure_reason: Option<String>,
    pub(crate) tangent_non_zero_count: usize,
    pub(crate) tangent_fill_ratio: f64,
    pub(crate) linear_solver: String,
}

#[derive(Clone, Copy)]
struct StepLinearDiagnostics {
    tangent_non_zero_count: usize,
    tangent_fill_ratio: f64,
    linear_solver: &'static str,
}

impl StepLinearDiagnostics {
    fn new() -> Self {
        Self {
            tangent_non_zero_count: 0,
            tangent_fill_ratio: 0.0,
            linear_solver: "none",
        }
    }

    fn observe_tangent(&mut self, non_zero_count: usize, size: usize) {
        self.tangent_non_zero_count = self.tangent_non_zero_count.max(non_zero_count);
        let fill_ratio = if size == 0 {
            0.0
        } else {
            non_zero_count as f64 / (size * size) as f64
        };
        self.tangent_fill_ratio = self.tangent_fill_ratio.max(fill_ratio);
    }

    fn observe_solver(&mut self, method: &'static str) {
        if self.linear_solver == "none" || method == DENSE_PIVOTED_FALLBACK {
            self.linear_solver = method;
        }
    }
}

pub(crate) fn solve_load_step(
    model: &ValidatedModel<'_>,
    control: &ControlStep,
    committed_displacements: &[f64],
    committed_states: &[CohesiveInterface3dState],
) -> LoadStepOutcome {
    let mut trial_displacements = committed_displacements.to_vec();
    for &dof in &model.fixed_dofs {
        trial_displacements[dof] = control.prescribed_displacements[dof];
    }
    let mut load_scale = stable_l2_norm(
        model
            .free_dofs
            .iter()
            .map(|&dof| control.load_factor * model.external_loads[dof]),
    )
    .max(1.0);
    let mut last_norm = f64::INFINITY;
    let mut diagnostics = StepLinearDiagnostics::new();

    for iteration in 1..=model.max_iterations {
        let assembly = assemble(model, &trial_displacements, committed_states);
        diagnostics.observe_tangent(assembly.tangent.non_zero_count(), assembly.tangent.size());
        let residual = model
            .external_loads
            .iter()
            .zip(&assembly.internal_forces)
            .map(|(external, internal)| control.load_factor * external - internal)
            .collect::<Vec<_>>();
        last_norm = restricted_norm(&residual, &model.free_dofs);
        if !load_scale.is_finite()
            || !last_norm.is_finite()
            || residual.iter().any(|value| !value.is_finite())
        {
            return failed_step(
                committed_displacements,
                committed_states,
                iteration,
                last_norm,
                "load step produced a non-finite force residual or load norm".to_string(),
                diagnostics,
            );
        }
        // Prescribed motion can drive equilibrium with no external free-DOF load.
        if iteration == 1 {
            load_scale = load_scale.max(last_norm);
        }
        if last_norm / load_scale <= model.tolerance {
            return LoadStepOutcome {
                displacements: trial_displacements,
                states: assembly
                    .evaluations
                    .into_iter()
                    .map(|evaluation| evaluation.state)
                    .collect(),
                iterations: iteration,
                residual_norm: last_norm,
                converged: true,
                failure_reason: None,
                tangent_non_zero_count: diagnostics.tangent_non_zero_count,
                tangent_fill_ratio: diagnostics.tangent_fill_ratio,
                linear_solver: diagnostics.linear_solver.to_string(),
            };
        }

        let (reduced_matrix, reduced_residual, reduced_free_dofs) =
            reduce_sparse_system(&assembly.tangent, &residual, &model.fixed_dofs);
        let solved = match solve_symmetric_tangent(
            &reduced_matrix,
            &reduced_residual,
            MAX_DENSE_FALLBACK_DOFS,
            "cohesive interface mesh 3d",
        ) {
            Ok(solved) => solved,
            Err(error) => {
                return failed_step(
                    committed_displacements,
                    committed_states,
                    iteration,
                    last_norm,
                    format!("{error}; check constraints and connectivity"),
                    diagnostics,
                );
            }
        };
        diagnostics.observe_solver(solved.method);
        for (&dof, delta) in reduced_free_dofs.iter().zip(solved.solution) {
            trial_displacements[dof] += delta;
        }
        if trial_displacements.iter().any(|value| !value.is_finite()) {
            return failed_step(
                committed_displacements,
                committed_states,
                iteration,
                last_norm,
                "load step produced non-finite displacement".to_string(),
                diagnostics,
            );
        }
    }

    failed_step(
        committed_displacements,
        committed_states,
        model.max_iterations,
        last_norm,
        format!(
            "load step did not converge within {} iterations",
            model.max_iterations
        ),
        diagnostics,
    )
}

fn failed_step(
    displacements: &[f64],
    states: &[CohesiveInterface3dState],
    iterations: usize,
    residual_norm: f64,
    reason: String,
    diagnostics: StepLinearDiagnostics,
) -> LoadStepOutcome {
    LoadStepOutcome {
        displacements: displacements.to_vec(),
        states: states.to_vec(),
        iterations,
        // Failed-step reports must remain JSON-serializable; the reason records overflow.
        residual_norm: if residual_norm.is_finite() {
            residual_norm
        } else {
            f64::MAX
        },
        converged: false,
        failure_reason: Some(reason),
        tangent_non_zero_count: diagnostics.tangent_non_zero_count,
        tangent_fill_ratio: diagnostics.tangent_fill_ratio,
        linear_solver: diagnostics.linear_solver.to_string(),
    }
}
