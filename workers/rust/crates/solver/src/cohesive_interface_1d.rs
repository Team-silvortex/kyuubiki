use kyuubiki_protocol::{
    CohesiveInterface1dRegime, CohesiveInterface1dStepResult, SolveCohesiveInterface1dRequest,
    SolveCohesiveInterface1dResult,
};

const MAX_HISTORY_STEPS: usize = 4096;
const HISTORY_TOLERANCE: f64 = 1.0e-12;

pub fn solve_cohesive_interface_1d(
    request: &SolveCohesiveInterface1dRequest,
) -> Result<SolveCohesiveInterface1dResult, String> {
    validate_request(request)?;
    let onset_separation = request.peak_traction / request.initial_stiffness;
    if !onset_separation.is_finite() || onset_separation <= 0.0 {
        return Err("peak_traction / initial_stiffness must be finite and positive".to_string());
    }
    if request.failure_separation <= onset_separation {
        return Err("failure_separation must exceed peak_traction / initial_stiffness".to_string());
    }

    let mut max_opening = 0.0_f64;
    let mut steps = Vec::with_capacity(request.separation_history.len());
    for (step, &separation) in request.separation_history.iter().enumerate() {
        let previous_max_opening = max_opening;
        max_opening = max_opening.max(separation.max(0.0));
        steps.push(evaluate_step(
            request,
            step,
            separation,
            previous_max_opening,
            max_opening,
            onset_separation,
        ));
    }

    let max_traction = steps
        .iter()
        .map(|step| step.traction.max(0.0))
        .fold(0.0_f64, f64::max);
    let max_damage = steps.iter().map(|step| step.damage).fold(0.0_f64, f64::max);

    Ok(SolveCohesiveInterface1dResult {
        input: request.clone(),
        onset_separation,
        fracture_energy: 0.5 * request.peak_traction * request.failure_separation,
        steps,
        max_traction,
        max_damage,
        fully_failed: max_damage >= 1.0,
    })
}

fn evaluate_step(
    request: &SolveCohesiveInterface1dRequest,
    step: usize,
    separation: f64,
    previous_max_opening: f64,
    max_opening: f64,
    onset_separation: f64,
) -> CohesiveInterface1dStepResult {
    if separation < 0.0 {
        return step_result(
            step,
            separation,
            request.compression_stiffness * separation,
            request.compression_stiffness,
            damage(max_opening, onset_separation, request.failure_separation),
            max_opening,
            CohesiveInterface1dRegime::Compression,
        );
    }

    let damage = damage(max_opening, onset_separation, request.failure_separation);
    let historical_unloading = separation + HISTORY_TOLERANCE < previous_max_opening;
    let (traction, tangent_stiffness, regime) = if damage >= 1.0 {
        (0.0, 0.0, CohesiveInterface1dRegime::Failed)
    } else if historical_unloading {
        (
            (1.0 - damage) * request.initial_stiffness * separation,
            (1.0 - damage) * request.initial_stiffness,
            CohesiveInterface1dRegime::UnloadingReloading,
        )
    } else if max_opening <= onset_separation {
        (
            request.initial_stiffness * separation,
            request.initial_stiffness,
            CohesiveInterface1dRegime::ElasticOpening,
        )
    } else {
        (
            (1.0 - damage) * request.initial_stiffness * separation,
            -request.peak_traction / (request.failure_separation - onset_separation),
            CohesiveInterface1dRegime::Softening,
        )
    };

    step_result(
        step,
        separation,
        traction,
        tangent_stiffness,
        damage,
        max_opening,
        regime,
    )
}

fn damage(max_opening: f64, onset_separation: f64, failure_separation: f64) -> f64 {
    if max_opening <= onset_separation {
        0.0
    } else if max_opening >= failure_separation {
        1.0
    } else {
        failure_separation * (max_opening - onset_separation)
            / (max_opening * (failure_separation - onset_separation))
    }
}

#[allow(clippy::too_many_arguments)]
fn step_result(
    step: usize,
    separation: f64,
    traction: f64,
    tangent_stiffness: f64,
    damage: f64,
    max_opening: f64,
    regime: CohesiveInterface1dRegime,
) -> CohesiveInterface1dStepResult {
    CohesiveInterface1dStepResult {
        step,
        separation,
        traction,
        tangent_stiffness,
        damage,
        max_opening,
        regime,
    }
}

fn validate_request(request: &SolveCohesiveInterface1dRequest) -> Result<(), String> {
    if request.id.trim().is_empty() {
        return Err("cohesive interface id must not be empty".to_string());
    }
    if request.id.len() > 256 {
        return Err("cohesive interface id must contain at most 256 bytes".to_string());
    }
    for (name, value) in [
        ("initial_stiffness", request.initial_stiffness),
        ("compression_stiffness", request.compression_stiffness),
        ("peak_traction", request.peak_traction),
        ("failure_separation", request.failure_separation),
    ] {
        if !value.is_finite() || value <= 0.0 {
            return Err(format!("{name} must be finite and positive"));
        }
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
