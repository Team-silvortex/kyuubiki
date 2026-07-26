use kyuubiki_protocol::{
    CohesiveInterface1dStepResult, SolveCohesiveInterface1dRequest, SolveCohesiveInterface1dResult,
};

use crate::cohesive_law::{CohesiveHistory, CohesiveLaw};

const MAX_HISTORY_STEPS: usize = 4096;

pub fn solve_cohesive_interface_1d(
    request: &SolveCohesiveInterface1dRequest,
) -> Result<SolveCohesiveInterface1dResult, String> {
    validate_request(request)?;
    let law = CohesiveLaw::new(
        request.initial_stiffness,
        request.peak_traction,
        request.failure_separation,
        "cohesive interface",
    )?;
    let mut history = CohesiveHistory::default();
    let steps = request
        .separation_history
        .iter()
        .enumerate()
        .map(|(step, &separation)| {
            let response = law.evaluate(
                &mut history,
                separation,
                Some(request.compression_stiffness),
            );
            CohesiveInterface1dStepResult {
                step,
                separation,
                traction: response.traction,
                tangent_stiffness: response.tangent,
                damage: response.damage,
                max_opening: response.max_separation,
                regime: response.regime,
            }
        })
        .collect::<Vec<_>>();

    let max_traction = steps
        .iter()
        .map(|step| step.traction.max(0.0))
        .fold(0.0_f64, f64::max);
    let max_damage = steps.iter().map(|step| step.damage).fold(0.0_f64, f64::max);

    Ok(SolveCohesiveInterface1dResult {
        input: request.clone(),
        onset_separation: law.onset_separation(),
        fracture_energy: law.fracture_energy(),
        steps,
        max_traction,
        max_damage,
        fully_failed: max_damage >= 1.0,
    })
}

fn validate_request(request: &SolveCohesiveInterface1dRequest) -> Result<(), String> {
    validate_id(&request.id)?;
    if !request.compression_stiffness.is_finite() || request.compression_stiffness <= 0.0 {
        return Err("compression_stiffness must be finite and positive".to_string());
    }
    if request.separation_history.is_empty() {
        return Err("separation_history must not be empty".to_string());
    }
    if request.separation_history.len() > MAX_HISTORY_STEPS {
        return Err(format!(
            "separation_history must contain at most {MAX_HISTORY_STEPS} steps"
        ));
    }
    if request
        .separation_history
        .iter()
        .any(|separation| !separation.is_finite())
    {
        return Err("separation_history values must be finite".to_string());
    }
    Ok(())
}

pub(crate) fn validate_id(id: &str) -> Result<(), String> {
    if id.trim().is_empty() {
        return Err("cohesive interface id must not be empty".to_string());
    }
    if id.len() > 256 {
        return Err("cohesive interface id must contain at most 256 bytes".to_string());
    }
    Ok(())
}
