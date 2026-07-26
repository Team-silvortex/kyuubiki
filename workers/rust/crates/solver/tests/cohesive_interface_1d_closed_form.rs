use kyuubiki_protocol::{CohesiveInterface1dRegime, SolveCohesiveInterface1dRequest};
use kyuubiki_solver::solve_cohesive_interface_1d;

const K: f64 = 1_000.0;
const PEAK: f64 = 10.0;
const FAILURE: f64 = 0.05;
const ONSET: f64 = PEAK / K;

#[test]
fn matches_bilinear_traction_separation_envelope() {
    let midpoint = 0.5 * (ONSET + FAILURE);
    let result = solve_cohesive_interface_1d(&request(vec![0.0, ONSET, midpoint, FAILURE]))
        .expect("cohesive envelope should solve");

    assert_close(result.onset_separation, ONSET);
    assert_close(result.fracture_energy, 0.5 * PEAK * FAILURE);
    assert_close(result.steps[1].traction, PEAK);
    assert_close(result.steps[2].traction, 0.5 * PEAK);
    assert_close(result.steps[2].tangent_stiffness, -PEAK / (FAILURE - ONSET));
    assert_eq!(result.steps[2].regime, CohesiveInterface1dRegime::Softening);
    assert_eq!(result.steps[3].regime, CohesiveInterface1dRegime::Failed);
    assert_close(result.steps[3].traction, 0.0);
    assert!(result.fully_failed);
}

#[test]
fn unloading_and_reloading_freeze_history_damage() {
    let peak_opening = 0.03;
    let result =
        solve_cohesive_interface_1d(&request(vec![peak_opening, 0.015, 0.025, peak_opening]))
            .expect("cyclic cohesive history should solve");

    let peak_damage = result.steps[0].damage;
    for step in &result.steps[1..3] {
        assert_close(step.damage, peak_damage);
        assert_close(step.max_opening, peak_opening);
        assert_eq!(step.regime, CohesiveInterface1dRegime::UnloadingReloading);
        assert_close(step.traction, step.tangent_stiffness * step.separation);
    }
    assert_close(result.steps[3].damage, peak_damage);
    assert_eq!(result.steps[3].regime, CohesiveInterface1dRegime::Softening);
}

#[test]
fn compression_remains_active_after_complete_opening_failure() {
    let result = solve_cohesive_interface_1d(&request(vec![FAILURE, -0.002]))
        .expect("closed failed interface should carry compression");

    assert_eq!(result.steps[0].regime, CohesiveInterface1dRegime::Failed);
    assert_eq!(
        result.steps[1].regime,
        CohesiveInterface1dRegime::Compression
    );
    assert_close(result.steps[1].traction, -4.0);
    assert_close(result.steps[1].tangent_stiffness, 2_000.0);
    assert_close(result.steps[1].damage, 1.0);
}

#[test]
fn rejects_invalid_contracts_and_non_finite_history() {
    let mut invalid_failure = request(vec![0.0]);
    invalid_failure.failure_separation = ONSET;
    assert!(solve_cohesive_interface_1d(&invalid_failure).is_err());

    let invalid_history = request(vec![f64::NAN]);
    assert!(solve_cohesive_interface_1d(&invalid_history).is_err());

    let empty_history = request(vec![]);
    assert!(solve_cohesive_interface_1d(&empty_history).is_err());

    let mut oversized_id = request(vec![0.0]);
    oversized_id.id = "x".repeat(257);
    assert!(solve_cohesive_interface_1d(&oversized_id).is_err());

    let mut unrepresentable_onset = request(vec![0.0]);
    unrepresentable_onset.initial_stiffness = f64::MAX;
    unrepresentable_onset.peak_traction = f64::MIN_POSITIVE;
    assert!(solve_cohesive_interface_1d(&unrepresentable_onset).is_err());
}

fn request(separation_history: Vec<f64>) -> SolveCohesiveInterface1dRequest {
    SolveCohesiveInterface1dRequest {
        id: "interface-0".to_string(),
        initial_stiffness: K,
        compression_stiffness: 2_000.0,
        peak_traction: PEAK,
        failure_separation: FAILURE,
        separation_history,
    }
}

fn assert_close(actual: f64, expected: f64) {
    let tolerance = 1.0e-10 * expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= tolerance,
        "expected {expected}, got {actual}"
    );
}
