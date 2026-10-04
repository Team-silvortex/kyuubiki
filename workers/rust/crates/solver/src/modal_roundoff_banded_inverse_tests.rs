use super::{Prepared, Profile, Wide, search_canonical_direction, vector_norm};
use crate::solver_control::{SolverControl, SolverStage, with_solver_observer};
use std::{cell::Cell, rc::Rc};

#[path = "modal_roundoff_banded_inverse_reference.rs"]
mod reference;

#[path = "modal_roundoff_banded_inverse_control_tests.rs"]
mod controls;

fn matrix(fixture: &Prepared) -> Vec<Vec<Wide>> {
    let mut matrix = fixture.directions.clone();
    for (i, row) in matrix.iter_mut().enumerate() {
        row[i] = row[i].add(Wide::from(fixture.value));
    }
    matrix
}

fn continuous_residual(fixture: &Prepared, vector: &[Wide]) -> f64 {
    let size = vector.len();
    let mut residual = Vec::with_capacity(size);
    for (i, row) in fixture.directions.iter().enumerate() {
        let value = (i.saturating_sub(3)..(i + 4).min(size))
            .fold(Wide::default(), |sum, j| sum.add(row[j].mul(vector[j])));
        residual.push(value.rounded());
    }
    vector_norm(residual.into_iter()).unwrap()
        / (fixture.value * vector_norm(vector.iter().map(|v| v.rounded())).unwrap())
}

fn rounded(direction: &[Wide]) -> Vec<f64> {
    direction.iter().map(|v| v.rounded()).collect()
}

fn bits(values: &[f64]) -> Vec<u64> {
    values.iter().map(|v| v.to_bits()).collect()
}

struct Pipeline {
    internal: Vec<f64>,
    physical: Vec<f64>,
    residual: f64,
    readback: f64,
    checks: [usize; 2],
}

fn pipeline(fixture: &Prepared, seed: &[f64]) -> Pipeline {
    let mut calls = 0;
    let accepted = search_canonical_direction(&fixture.directions, seed, 1e-8, |v| {
        calls += 1;
        fixture.checked(v)
    })
    .unwrap();
    assert!(calls <= 19 && accepted.usage.attempts <= 3);
    assert_eq!(calls, accepted.usage.certificates);
    assert_eq!(
        accepted.shape[accepted.anchor].to_bits(),
        seed[accepted.anchor].to_bits()
    );
    let residual = fixture.checked(&accepted.shape).unwrap().0;
    assert!(residual <= 1e-8);
    let (published, readback) = fixture.publish_grid(&accepted.shape).unwrap();
    assert!(published.usage.certificates <= 19 && published.usage.attempts <= 3);
    assert!(readback <= 1e-8);
    Pipeline {
        internal: accepted.shape,
        physical: published.shape,
        residual,
        readback,
        checks: [calls, published.usage.certificates],
    }
}

#[test]
fn modal_banded_inverse_heterogeneous_frozen_root_and_two_stage_readback() {
    let mut counts = [0; 3];
    for profile in [Profile::Graded, Profile::Layered] {
        for scale in [1.0, 1e14, 1e-10] {
            let fixture = Prepared::new(profile, scale);
            let seed_bits = bits(&fixture.seed);
            let root = fixture.value.to_bits();
            let error = crate::solve_modal_frame_2d(&fixture.input).unwrap_err();
            assert!(error.starts_with("normalized modal roundoff recovery failed:"));
            let (lower, upper) =
                super::reference::bracket(&fixture.directions, fixture.value).unwrap();
            assert!((upper - lower) / fixture.value < 1e-14);
            assert!((fixture.value / ((lower + upper) * 0.5) - 1.0).abs() < 1e-12);
            let factor = reference::Factor::prepare(&matrix(&fixture)).unwrap();
            let initial = fixture.checked(&fixture.seed).unwrap().0;
            assert!(initial > 1e-8);
            let directions = factor
                .directions(&fixture.seed, reference::MAX_STEPS)
                .unwrap();
            assert_eq!(directions.len(), 4);
            for (step, direction) in directions.iter().enumerate() {
                let continuous = continuous_residual(&fixture, direction);
                let residual = fixture.checked(&rounded(direction)).unwrap().0;
                assert!(continuous.is_finite() && continuous < 2e-10);
                assert!(residual > initial && residual > 1e-8);
                if step == 3 {
                    assert!(continuous < 1e-14);
                }
                counts[0] += 1;
                println!(
                    "banded inverse profile={} scale={scale:e} step={} continuous={continuous:e} rounded={residual:e}",
                    profile.name(),
                    step + 1
                );
            }
            let direct =
                search_canonical_direction(&fixture.directions, &fixture.seed, 1e-8, |v| {
                    fixture.checked(v)
                });
            assert_eq!(
                direct.is_ok(),
                !(matches!(profile, Profile::Graded) && scale == 1e14)
            );
            counts[1] += usize::from(direct.is_ok());
            let baseline = pipeline(&fixture, &rounded(directions.last().unwrap()));
            counts[2] += 1;
            assert_eq!(bits(&fixture.seed), seed_bits);
            assert_eq!(fixture.value.to_bits(), root);
            println!(
                "banded inverse pipeline profile={} scale={scale:e} internal={:e} physical_json={:e} checks={:?}",
                profile.name(),
                baseline.residual,
                baseline.readback,
                baseline.checks
            );
        }
    }
    assert_eq!(counts, [24, 5, 6]);
    println!(
        "banded inverse fixtures=6 nearest_rounding_rejections=24 direct_grid_successes=5 two_stage_successes=6"
    );
}

#[test]
fn modal_banded_inverse_sign_and_binary_scale_preserve_checked_candidate_bits() {
    let mut count = 0;
    for profile in [Profile::Graded, Profile::Layered] {
        for scale in [1.0, 1e14, 1e-10] {
            let fixture = Prepared::new(profile, scale);
            let factor = reference::Factor::prepare(&matrix(&fixture)).unwrap();
            let original = bits(&fixture.seed);
            let baseline_directions = factor.directions(&fixture.seed, 4).unwrap();
            let baseline = pipeline(&fixture, &rounded(baseline_directions.last().unwrap()));
            for amplitude in [-2.0, -1.0, 0.5, 2.0] {
                let seed: Vec<_> = fixture.seed.iter().map(|v| amplitude * v).collect();
                let directions = factor.directions(&seed, 4).unwrap();
                for (direction, reference) in directions.iter().zip(&baseline_directions) {
                    assert_eq!(
                        bits(
                            &direction
                                .iter()
                                .map(|v| v.rounded() / amplitude)
                                .collect::<Vec<_>>()
                        ),
                        bits(&rounded(reference))
                    );
                }
                let candidate = pipeline(&fixture, &rounded(directions.last().unwrap()));
                assert_eq!(
                    bits(
                        &candidate
                            .internal
                            .iter()
                            .map(|v| v / amplitude)
                            .collect::<Vec<_>>()
                    ),
                    bits(&baseline.internal)
                );
                let sign = 1.0_f64.copysign(amplitude);
                assert_eq!(
                    bits(
                        &candidate
                            .physical
                            .iter()
                            .map(|v| sign * v)
                            .collect::<Vec<_>>()
                    ),
                    bits(&baseline.physical)
                );
                assert_eq!(candidate.checks, baseline.checks);
                assert_eq!(candidate.residual.to_bits(), baseline.residual.to_bits());
                assert_eq!(candidate.readback.to_bits(), baseline.readback.to_bits());
                count += 1;
            }
            assert_eq!(bits(&fixture.seed), original);
        }
    }
    assert_eq!(count, 24);
    println!("banded inverse amplitude_sign_controls=24 two_stage_successes=24");
}

#[test]
fn modal_banded_inverse_real_final_cancellation_fault_and_fresh_replay() {
    let fixture = Prepared::new(Profile::Graded, 1e14);
    let original = bits(&fixture.seed);
    let factor = reference::Factor::prepare(&matrix(&fixture)).unwrap();
    let directions = factor.directions(&fixture.seed, 4).unwrap();
    let seed = rounded(directions.last().unwrap());
    let baseline = pipeline(&fixture, &seed);
    let observed = Rc::new(Cell::new(false));
    let saw = observed.clone();
    let control = SolverControl::default();
    let cancel = control.clone();
    let error = with_solver_observer(
        &control,
        move |p| {
            if p.stage == SolverStage::ModalIteration && p.completed_steps == 4 {
                saw.set(true);
                cancel.request_cancel();
            }
        },
        || factor.directions(&fixture.seed, 4),
    )
    .err()
    .unwrap();
    assert!(observed.get() && error.contains("cancel"));
    let replay = factor.directions(&fixture.seed, 4).unwrap();
    assert_eq!(bits(&rounded(replay.last().unwrap())), bits(&seed));
    let observed = Rc::new(Cell::new(false));
    let saw = observed.clone();
    let control = SolverControl::default();
    let cancel = control.clone();
    let error = with_solver_observer(
        &control,
        move |p| {
            if p.stage == SolverStage::ModalRoundoffValidate && p.completed_steps == 5 {
                saw.set(true);
                cancel.request_cancel();
            }
        },
        || fixture.publish_grid(&baseline.internal),
    )
    .err()
    .unwrap();
    assert!(observed.get() && error.contains("cancel"));
    let mut calls = 0;
    let error = search_canonical_direction(&fixture.directions, &seed, 1e-8, |v| {
        calls += 1;
        if calls == baseline.checks[0] {
            return Err("injected final banded-candidate certificate fault".into());
        }
        fixture.checked(v)
    })
    .unwrap_err();
    assert!(error.contains("injected final"));
    assert_eq!(calls, baseline.checks[0]);
    let replay = pipeline(&fixture, &seed);
    assert_eq!(bits(&replay.internal), bits(&baseline.internal));
    assert_eq!(bits(&replay.physical), bits(&baseline.physical));
    assert_eq!(replay.readback.to_bits(), baseline.readback.to_bits());
    assert_eq!(bits(&fixture.seed), original);
}
