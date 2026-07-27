use crate::cohesive_interface_2d::CohesiveInterface2dState;
use crate::cohesive_interface_mesh_2d::{ValidatedModel, assemble};
use crate::cohesive_interface_mesh_2d_control::{ControlStep, restricted_norm, vector_norm};
use crate::linear_dense::solve_linear_system;

pub(crate) struct LoadStepOutcome {
    pub(crate) displacements: Vec<f64>,
    pub(crate) states: Vec<CohesiveInterface2dState>,
    pub(crate) iterations: usize,
    pub(crate) residual_norm: f64,
    pub(crate) converged: bool,
    pub(crate) failure_reason: Option<String>,
}

pub(crate) fn solve_load_step(
    model: &ValidatedModel<'_>,
    step: usize,
    control: &ControlStep,
    committed_displacements: &[f64],
    committed_states: &[CohesiveInterface2dState],
) -> LoadStepOutcome {
    let mut trial_displacements = committed_displacements.to_vec();
    for &dof in &model.fixed_dofs {
        trial_displacements[dof] = control.prescribed_displacements[dof];
    }
    let load_scale = vector_norm(&model.external_loads).max(1.0);
    let mut last_norm = f64::INFINITY;

    for iteration in 1..=model.max_iterations {
        let assembly = assemble(model, step, &trial_displacements, committed_states);
        let residual = model
            .external_loads
            .iter()
            .zip(&assembly.internal_forces)
            .map(|(external, internal)| control.load_factor * external - internal)
            .collect::<Vec<_>>();
        last_norm = restricted_norm(&residual, &model.free_dofs);
        if last_norm <= model.tolerance * load_scale {
            let states = assembly
                .evaluations
                .into_iter()
                .map(|evaluation| evaluation.state)
                .collect();
            return LoadStepOutcome {
                displacements: trial_displacements,
                states,
                iterations: iteration,
                residual_norm: last_norm,
                converged: true,
                failure_reason: None,
            };
        }

        let reduced_matrix = model
            .free_dofs
            .iter()
            .map(|&row| {
                model
                    .free_dofs
                    .iter()
                    .map(|&column| assembly.tangent[row][column])
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let reduced_residual = model
            .free_dofs
            .iter()
            .map(|&dof| residual[dof])
            .collect::<Vec<_>>();
        let increment = match solve_linear_system(reduced_matrix, reduced_residual) {
            Ok(increment) => increment,
            Err(error) => {
                return failed_step(
                    committed_displacements,
                    committed_states,
                    iteration,
                    last_norm,
                    format!(
                        "load step {} tangent solve failed: {error}; check constraints and connectivity",
                        step + 1
                    ),
                );
            }
        };
        for (&dof, delta) in model.free_dofs.iter().zip(increment) {
            trial_displacements[dof] += delta;
        }
        if trial_displacements.iter().any(|value| !value.is_finite()) {
            return failed_step(
                committed_displacements,
                committed_states,
                iteration,
                last_norm,
                format!("load step {} produced non-finite displacement", step + 1),
            );
        }
    }

    failed_step(
        committed_displacements,
        committed_states,
        model.max_iterations,
        last_norm,
        format!(
            "load step {} did not converge within {} iterations",
            step + 1,
            model.max_iterations
        ),
    )
}

fn failed_step(
    displacements: &[f64],
    states: &[CohesiveInterface2dState],
    iterations: usize,
    residual_norm: f64,
    reason: String,
) -> LoadStepOutcome {
    LoadStepOutcome {
        displacements: displacements.to_vec(),
        states: states.to_vec(),
        iterations,
        residual_norm,
        converged: false,
        failure_reason: Some(reason),
    }
}
