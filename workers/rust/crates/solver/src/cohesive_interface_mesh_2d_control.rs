use std::collections::HashSet;

use kyuubiki_protocol::SolveCohesiveInterfaceMesh2dRequest;

const DEFAULT_LOAD_STEPS: usize = 10;
const MAX_LOAD_STEPS: usize = 4096;

pub(crate) struct ControlStep {
    pub(crate) load_factor: f64,
    pub(crate) prescribed_displacements: Vec<f64>,
}

pub(crate) fn build_controls(
    request: &SolveCohesiveInterfaceMesh2dRequest,
    dof_count: usize,
    free_dofs: &[usize],
    target_displacements: &[f64],
) -> Result<Vec<ControlStep>, String> {
    if let Some(history) = &request.control_history {
        if request.load_steps.is_some() {
            return Err("control_history and load_steps are mutually exclusive".to_string());
        }
        if target_displacements.iter().any(|value| *value != 0.0) {
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
                if input.prescribed_displacements.len() * 2 != dof_count
                    || input
                        .prescribed_displacements
                        .iter()
                        .flatten()
                        .any(|value| !value.is_finite())
                {
                    return Err(format!(
                        "control_history step {step} displacement vector must match finite node data"
                    ));
                }
                let values = input
                    .prescribed_displacements
                    .iter()
                    .flatten()
                    .copied()
                    .collect::<Vec<_>>();
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
                prescribed_displacements: target_displacements
                    .iter()
                    .map(|value| factor * value)
                    .collect(),
            }
        })
        .collect())
}

pub(crate) fn restricted_norm(values: &[f64], dofs: &[usize]) -> f64 {
    dofs.iter()
        .map(|&dof| values[dof] * values[dof])
        .sum::<f64>()
        .sqrt()
}

pub(crate) fn vector_norm(values: &[f64]) -> f64 {
    values.iter().map(|value| value * value).sum::<f64>().sqrt()
}
