use std::collections::HashSet;

use kyuubiki_protocol::SolveCohesiveInterfaceMesh3dRequest;

use crate::linear_algebra::stable_l2_norm;

const DEFAULT_LOAD_STEPS: usize = 10;
const MAX_LOAD_STEPS: usize = 4096;

pub(crate) struct ControlStep {
    pub(crate) load_factor: f64,
    pub(crate) prescribed_displacements: Vec<f64>,
}

pub(crate) fn build_controls(
    request: &SolveCohesiveInterfaceMesh3dRequest,
    free_dofs: &[usize],
    targets: &[f64],
) -> Result<Vec<ControlStep>, String> {
    if let Some(history) = &request.control_history {
        if request.load_steps.is_some() {
            return Err("control_history and load_steps are mutually exclusive".to_string());
        }
        if targets.iter().any(|value| *value != 0.0) {
            return Err(
                "control_history and node prescribed_displacement targets are mutually exclusive"
                    .to_string(),
            );
        }
        if history.is_empty() || history.len() > MAX_LOAD_STEPS {
            return Err(format!(
                "control_history must contain 1..={MAX_LOAD_STEPS} steps"
            ));
        }
        let free = free_dofs.iter().copied().collect::<HashSet<_>>();
        return history
            .iter()
            .enumerate()
            .map(|(step, input)| {
                if !input.load_factor.is_finite() {
                    return Err(format!("control_history step {step} load_factor is not finite"));
                }
                if input.prescribed_displacements.len() != request.nodes.len()
                    || input
                        .prescribed_displacements
                        .iter()
                        .flatten()
                        .any(|value| !value.is_finite())
                {
                    return Err(format!(
                        "control_history step {step} displacement vectors must match finite node data"
                    ));
                }
                let mut values = vec![0.0; request.nodes.len() * 3];
                for (node, displacement) in input.prescribed_displacements.iter().enumerate() {
                    for axis in 0..3 {
                        values[node * 3 + axis] = displacement[axis];
                    }
                }
                if free.iter().any(|&dof| values[dof] != 0.0) {
                    return Err(format!(
                        "control_history step {step} prescribes a free dof"
                    ));
                }
                Ok(ControlStep {
                    load_factor: input.load_factor,
                    prescribed_displacements: values,
                })
            })
            .collect();
    }

    let load_steps = request.load_steps.unwrap_or(DEFAULT_LOAD_STEPS);
    if load_steps == 0 || load_steps > MAX_LOAD_STEPS {
        return Err(format!("load_steps must be in 1..={MAX_LOAD_STEPS}"));
    }
    Ok((0..load_steps)
        .map(|step| {
            let factor = (step + 1) as f64 / load_steps as f64;
            ControlStep {
                load_factor: factor,
                prescribed_displacements: targets.iter().map(|value| factor * value).collect(),
            }
        })
        .collect())
}

pub(crate) fn restricted_norm(values: &[f64], dofs: &[usize]) -> f64 {
    stable_l2_norm(dofs.iter().map(|&dof| values[dof]))
}

pub(crate) fn vector_norm(values: &[f64]) -> f64 {
    stable_l2_norm(values.iter().copied())
}
