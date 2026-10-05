use super::super::super::{Selection, search_canonical_direction_ranked};
use super::{Factor, Prepared, Profile, bits, one_factor, rounded};
use crate::solver_control::{SolverControl, SolverStage, with_solver_observer};
use std::{cell::Cell, rc::Rc};

fn seed(fixture: &Prepared, amplitude: f64) -> Vec<f64> {
    let seed: Vec<_> = fixture.seed.iter().map(|v| amplitude * v).collect();
    rounded(
        &Factor::prepare_shifted(&fixture.directions, fixture.value)
            .unwrap()
            .final_direction(&seed, 4)
            .unwrap(),
    )
}

#[test]
fn modal_banded_ranked_actual_final_fault_cancel_and_replay() {
    let fixture = Prepared::new(Profile::Graded, 1e14);
    let original = bits(&fixture.seed);
    let proposed = seed(&fixture, 1.0);
    let internal = one_factor(|| {
        search_canonical_direction_ranked(&fixture.directions, &proposed, 1e-8, |v| {
            fixture.checked(v)
        })
    })
    .unwrap();
    let physical = one_factor(|| {
        fixture.physical_candidate_ordered(&internal.shape, Selection::RankedThenReverse, || {})
    })
    .unwrap();
    assert_eq!(internal.usage.certificates, 4);
    let mut calls = 0;
    let error = one_factor(|| {
        search_canonical_direction_ranked(&fixture.directions, &proposed, 1e-8, |v| {
            calls += 1;
            if calls == internal.usage.certificates {
                return Err("unchanged residual gate: injected ranked final operator fault".into());
            }
            fixture.checked(v)
        })
    })
    .unwrap_err();
    assert!(error.contains("injected ranked final") && calls == 4);
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
        || fixture.physical_candidate_ordered(&internal.shape, Selection::RankedThenReverse, || {}),
    )
    .unwrap_err();
    assert!(observed.get() && error.contains("cancel"));
    let replay = one_factor(|| {
        fixture.physical_candidate_ordered(&internal.shape, Selection::RankedThenReverse, || {})
    })
    .unwrap();
    assert_eq!(bits(&replay.shape), bits(&physical.shape));
    assert_eq!(replay.usage.certificates, physical.usage.certificates);
    let residual = super::super::super::Fixture::check_recovered_readback(
        &fixture.input,
        fixture.total_mass,
        fixture.value,
        &replay.shape,
    );
    assert!(residual <= 1e-8);
    assert_eq!(bits(&fixture.seed), original);
}

#[test]
fn modal_banded_ranked_sign_binary_amplitude_and_frozen_root() {
    let mut controls = 0;
    for profile in [Profile::Graded, Profile::Layered] {
        for scale in [1.0, 1e14, 1e-10] {
            let fixture = Prepared::new(profile, scale);
            let original = bits(&fixture.seed);
            let root = fixture.value.to_bits();
            let baseline_seed = seed(&fixture, 1.0);
            let baseline =
                search_canonical_direction_ranked(&fixture.directions, &baseline_seed, 1e-8, |v| {
                    fixture.checked(v)
                })
                .unwrap();
            let baseline_shape = fixture
                .physical_candidate_ordered(&baseline.shape, Selection::RankedThenReverse, || {})
                .unwrap();
            for amplitude in [-2.0, 0.5, 2.0] {
                let proposed = seed(&fixture, amplitude);
                let internal =
                    search_canonical_direction_ranked(&fixture.directions, &proposed, 1e-8, |v| {
                        fixture.checked(v)
                    })
                    .unwrap();
                assert_eq!(
                    bits(
                        &internal
                            .shape
                            .iter()
                            .map(|v| v / amplitude)
                            .collect::<Vec<_>>()
                    ),
                    bits(&baseline.shape)
                );
                assert_eq!(internal.usage.attempts, baseline.usage.attempts);
                assert_eq!(internal.usage.certificates, baseline.usage.certificates);
                let physical = fixture
                    .physical_candidate_ordered(
                        &internal.shape,
                        Selection::RankedThenReverse,
                        || {},
                    )
                    .unwrap();
                let sign = 1.0_f64.copysign(amplitude);
                assert_eq!(
                    bits(&physical.shape.iter().map(|v| sign * v).collect::<Vec<_>>()),
                    bits(&baseline_shape.shape)
                );
                assert_eq!(physical.usage.attempts, baseline_shape.usage.attempts);
                assert_eq!(
                    physical.usage.certificates,
                    baseline_shape.usage.certificates
                );
                assert!(
                    super::super::super::Fixture::check_recovered_readback(
                        &fixture.input,
                        fixture.total_mass,
                        fixture.value,
                        &physical.shape
                    ) <= 1e-8
                );
                controls += 1;
            }
            assert_eq!(bits(&fixture.seed), original);
            assert_eq!(fixture.value.to_bits(), root);
        }
    }
    assert_eq!(controls, 18);
    println!("banded ranked amplitude_sign_controls=18 exact_checked_replays=18");
}
