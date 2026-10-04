use super::super::{Order, search_unit_shape, tests::shuffled};
use super::*;
use crate::solver_control::{SolverControl, with_solver_observer};

#[path = "modal_roundoff_grid_heterogeneous_fixture.rs"]
mod fixture;
use fixture::{Fixture, Profile};

#[path = "modal_roundoff_grid_normalized_tests.rs"]
mod normalized_tests;

#[path = "modal_roundoff_grid_reassembly_tests.rs"]
mod reassembly_tests;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Boundary {
    SpectrumBlocked,
    SeedValid,
    Corrected,
    ResidualRejected,
}

fn expected(profile: Profile, segments: usize, scale: f64) -> Boundary {
    match (profile, segments) {
        (_, 64) | (Profile::UnequalLengths, 96) => Boundary::SeedValid,
        (Profile::Graded, 96) if scale == 1e-10 => Boundary::SeedValid,
        (Profile::Graded | Profile::Layered, 96) => Boundary::Corrected,
        (Profile::Graded | Profile::Layered, 128) => Boundary::SpectrumBlocked,
        (Profile::UnequalLengths, 128) if scale == 1e-10 => Boundary::Corrected,
        (Profile::UnequalLengths, 128) => Boundary::ResidualRejected,
        _ => panic!("unregistered heterogeneous fixture"),
    }
}

fn expected_calls(boundary: Boundary, segments: usize, canonical: bool) -> usize {
    if segments == 128 {
        if canonical { 10 } else { 11 }
    } else if boundary == Boundary::SeedValid {
        3
    } else {
        4
    }
}

#[test]
fn triangular_grid_canonical_heterogeneous_readback_and_numbering_boundaries() {
    let mut counts = [0; 4];
    for profile in [Profile::Graded, Profile::Layered, Profile::UnequalLengths] {
        for segments in [64, 96, 128] {
            for scale in [1.0, 1e14, 1e-10] {
                let boundary = expected(profile, segments, scale);
                counts[boundary as usize] += 1;
                let fixture = match Fixture::prepare(profile, segments, scale) {
                    Ok(fixture) => {
                        assert_ne!(boundary, Boundary::SpectrumBlocked);
                        fixture
                    }
                    Err(error) => {
                        assert_eq!(boundary, Boundary::SpectrumBlocked, "{error}");
                        assert!(error.starts_with("spectrum preparation:"), "{error}");
                        let request = Fixture::request(profile, segments, scale);
                        let public =
                            crate::modal_frame_2d::solve_modal_frame_2d(&request).unwrap_err();
                        assert!(
                            public.contains("normalized modal roundoff recovery failed"),
                            "{public}"
                        );
                        assert_eq!(error, format!("spectrum preparation: {public}"));
                        println!(
                            "heterogeneous blocked profile={} segments={segments} scale={scale:e} error={error} public={public}",
                            profile.name()
                        );
                        continue;
                    }
                };
                let size = fixture.seed.len();
                let initial = fixture.checked(&fixture.seed).unwrap().0;
                assert_eq!(initial <= 1e-8, boundary == Boundary::SeedValid);
                let mut raw_calls = 0;
                let raw = search_unit_shape(
                    &fixture.directions,
                    &fixture.seed,
                    &fixture.system.mass,
                    1e-8,
                    |v| {
                        raw_calls += 1;
                        fixture.checked(v)
                    },
                );
                assert_eq!(raw.is_ok(), boundary != Boundary::ResidualRejected);
                assert_eq!(raw_calls, expected_calls(boundary, segments, false));
                if let Ok(result) = &raw {
                    assert_eq!(raw_calls, result.usage.certificates);
                    fixture.check_readback(&result.shape);
                } else {
                    assert!(
                        raw.as_ref()
                            .unwrap_err()
                            .contains("exhausted its bounded policies")
                    );
                }
                println!(
                    "heterogeneous raw profile={} segments={segments} scale={scale:e} initial={initial:e} accepted={} calls={raw_calls}",
                    profile.name(),
                    raw.is_ok()
                );
                let mut reference: Option<Vec<f64>> = None;
                let mut accepted = 0;
                for (layout, permutation) in [
                    ("identity", (0..size).collect::<Vec<_>>()),
                    ("reversed", (0..size).rev().collect()),
                    ("shuffle-7", shuffled(size, 7)),
                    ("shuffle-113", shuffled(size, 113)),
                ] {
                    let matrix: Vec<Vec<_>> = permutation
                        .iter()
                        .map(|&i| {
                            permutation
                                .iter()
                                .map(|&j| fixture.directions[i][j])
                                .collect()
                        })
                        .collect();
                    let mass: Vec<_> = permutation
                        .iter()
                        .map(|&i| fixture.system.mass[i])
                        .collect();
                    for sign in [1.0, -1.0] {
                        let seed: Vec<_> = permutation
                            .iter()
                            .map(|&i| sign * fixture.seed[i])
                            .collect();
                        let restore = |v: &[f64]| {
                            let mut original = vec![0.0; size];
                            for (&i, &a) in permutation.iter().zip(v) {
                                original[i] = a;
                            }
                            original
                        };
                        let mut calls = 0;
                        let result =
                            search_canonical_unit_shape(&matrix, &seed, &mass, 1e-8, |v| {
                                calls += 1;
                                let (relative, residual) = fixture.checked(&restore(v))?;
                                Ok((relative, permutation.iter().map(|&i| residual[i]).collect()))
                            });
                        assert!(calls <= super::super::MAX_TOTAL_CERTIFICATES);
                        assert_eq!(
                            result.is_ok(),
                            boundary != Boundary::ResidualRejected,
                            "{profile:?}/{segments}/{scale:e}/{layout}/{sign}"
                        );
                        assert_eq!(calls, expected_calls(boundary, segments, true));
                        match result {
                            Ok(result) => {
                                accepted += 1;
                                assert_eq!(calls, result.usage.certificates);
                                assert_eq!(
                                    result.usage.attempts,
                                    if segments == 128 { 3 } else { 1 }
                                );
                                assert_eq!(
                                    result.order,
                                    if segments == 128 {
                                        Order::GridNorm
                                    } else {
                                        Order::Reverse
                                    }
                                );
                                let plan = Plan::canonical(size).unwrap();
                                assert_eq!(
                                    result.usage.component_visits,
                                    plan.preflight_visits + result.usage.attempts * plan.fit_visits
                                );
                                assert_eq!(
                                    result.shape[result.anchor].to_bits(),
                                    seed[result.anchor].to_bits()
                                );
                                let original = restore(&result.shape);
                                let independent = fixture.check_readback(&original);
                                let positive: Vec<_> = original.iter().map(|v| sign * v).collect();
                                if let Some(reference) = &reference {
                                    assert!(
                                        positive
                                            .iter()
                                            .zip(reference)
                                            .all(|(a, b)| a.to_bits() == b.to_bits())
                                    );
                                } else {
                                    reference = Some(positive);
                                }
                                println!(
                                    "heterogeneous canonical profile={} segments={segments} scale={scale:e} layout={layout} sign={sign} accepted=true calls={calls} order={:?} relative={:e} independent={independent:e}",
                                    profile.name(),
                                    result.order,
                                    fixture.checked(&original).unwrap().0
                                );
                            }
                            Err(error) => {
                                assert!(
                                    error
                                        .contains("attempts=3, certificates=10, norm_rejections=0"),
                                    "{error}"
                                );
                                println!(
                                    "heterogeneous canonical profile={} segments={segments} scale={scale:e} layout={layout} sign={sign} accepted=false calls={calls} error={error}",
                                    profile.name()
                                );
                            }
                        }
                    }
                }
                assert_eq!(
                    accepted,
                    if boundary == Boundary::ResidualRejected {
                        0
                    } else {
                        8
                    }
                );
                println!(
                    "heterogeneous total profile={} segments={segments} scale={scale:e} accepted={accepted} rejected={}",
                    profile.name(),
                    8 - accepted
                );
            }
        }
    }
    assert_eq!(counts, [6, 13, 6, 2]);
    println!(
        "heterogeneous fixture boundaries blocked=6 retained=13 corrected=6 rejected=2; canonical executions=168 accepted=152 rejected=16"
    );
}

#[test]
fn triangular_grid_canonical_heterogeneous_public_failure_preserves_fresh_replay() {
    let request = Fixture::request(Profile::Graded, 64, 1.0);
    let baseline = crate::modal_frame_2d::solve_modal_frame_2d(&request).unwrap();
    Fixture::check_public_readback(&baseline);
    let failure = Fixture::request(Profile::Graded, 128, 1.0);
    let error = crate::modal_frame_2d::solve_modal_frame_2d(&failure).unwrap_err();
    assert!(
        error.contains("normalized modal roundoff recovery failed"),
        "{error}"
    );
    let replay = crate::modal_frame_2d::solve_modal_frame_2d(&request).unwrap();
    Fixture::check_public_readback(&replay);
    assert_eq!(baseline, replay);
    assert!(
        baseline.modes[0]
            .shape
            .iter()
            .zip(&replay.modes[0].shape)
            .all(|(a, b)| a.to_bits() == b.to_bits())
    );
}

#[test]
fn triangular_grid_canonical_heterogeneous_late_cancellation_returns_no_shape_and_replays() {
    let fixture = Fixture::prepare(Profile::Layered, 96, 1.0).unwrap();
    assert!(fixture.checked(&fixture.seed).unwrap().0 > 1e-8);
    let control = SolverControl::default();
    let cancel = control.clone();
    let mut calls = 0;
    let error = with_solver_observer(
        &control,
        move |progress| {
            if progress.stage == SolverStage::ModalRoundoffValidate && progress.completed_steps == 5
            {
                cancel.request_cancel();
            }
        },
        || {
            search_canonical_unit_shape(
                &fixture.directions,
                &fixture.seed,
                &fixture.system.mass,
                1e-8,
                |v| {
                    calls += 1;
                    fixture.checked(v)
                },
            )
        },
    )
    .unwrap_err();
    assert!(error.contains("cancel"), "{error}");
    assert_eq!(calls, 4);
    let replay = search_canonical_unit_shape(
        &fixture.directions,
        &fixture.seed,
        &fixture.system.mass,
        1e-8,
        |v| fixture.checked(v),
    )
    .unwrap();
    assert_eq!(replay.usage.certificates, 4);
    fixture.check_readback(&replay.shape);
}
