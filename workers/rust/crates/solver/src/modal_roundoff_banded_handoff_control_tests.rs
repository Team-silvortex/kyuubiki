use super::super::{
    Accepted, Selection, checked_mode_shape, physical_directions,
    search_canonical_direction_ranked, search_canonical_unit_shape_single,
};
use super::{
    Prepared, Profile, Wide, bits, reference::Factor, rounded, single_policy_tests::one_factor,
};
use crate::solver_control::{SolverControl, SolverStage, with_solver_observer};
use std::{cell::Cell, rc::Rc};

type Checked<'a> = dyn FnMut(&[f64]) -> Result<(f64, Vec<f64>), String> + 'a;

fn internal(fixture: &Prepared, amplitude: f64) -> Accepted {
    let seed: Vec<_> = fixture.seed.iter().map(|v| amplitude * v).collect();
    let proposed = rounded(
        &Factor::prepare_shifted(&fixture.directions, fixture.value)
            .unwrap()
            .final_direction(&seed, 4)
            .unwrap(),
    );
    search_canonical_direction_ranked(&fixture.directions, &proposed, 1e-8, |v| fixture.checked(v))
        .unwrap()
}

fn physical_input(fixture: &Prepared, internal: &Accepted) -> (Vec<Vec<Wide>>, Vec<f64>) {
    let size = internal.shape.len();
    let (shape, _) = checked_mode_shape(
        &internal.shape,
        &fixture.system.mass,
        &(0..size).collect::<Vec<_>>(),
        size,
    )
    .unwrap();
    let seed = fixture
        .system
        .operator
        .roundoff_comparison_seed(fixture.value, &shape, &fixture.system.mass)
        .unwrap();
    let matrix =
        physical_directions(&fixture.physical, &fixture.system.mass, fixture.value).unwrap();
    (matrix, seed)
}

#[test]
fn modal_banded_handoff_physical_rejection_final_fault_cancel_and_replay() {
    for scale in [1e14, 1e-10] {
        let fixture = Prepared::new(Profile::Layered, scale);
        let original = bits(&fixture.seed);
        let root = fixture.value.to_bits();
        let internal = internal(&fixture, 1.0);
        let (matrix, seed) = physical_input(&fixture, &internal);
        let retained = bits(&seed);
        let check = |v: &[f64]| {
            let applied = fixture.system.operator.apply_physical_compensated(v)?;
            fixture.system.operator.physical_residual(
                fixture.value,
                v,
                &fixture.system.mass,
                &applied,
            )
        };
        let attempt = |checked: &mut Checked<'_>| {
            search_canonical_unit_shape_single(
                &matrix,
                &seed,
                &fixture.system.mass,
                1e-8,
                internal.order,
                checked,
            )
        };
        let physical = one_factor(|| attempt(&mut |v| check(v))).unwrap();
        assert_eq!(physical.order, internal.order);
        assert_eq!(physical.usage.certificates, 4);
        // A good internal certificate must not admit a failing physical certificate.
        let mut calls = 0;
        let error = one_factor(|| {
            attempt(&mut |v| {
                calls += 1;
                let (_, residual) = check(v)?;
                Ok((1.0, residual))
            })
        })
        .unwrap_err();
        assert!(
            error.contains("exhausted its bounded policies")
                && error.contains("attempts=1")
                && error.contains("norm_rejections=0"),
            "{error}"
        );
        assert_eq!(calls, 3);
        calls = 0;
        let error = one_factor(|| {
            attempt(&mut |v| {
                calls += 1;
                if calls == physical.usage.certificates {
                    return Err(
                        "unchanged residual gate: injected handoff final physical fault".into(),
                    );
                }
                check(v)
            })
        })
        .unwrap_err();
        assert!(error.contains("injected handoff final physical fault"));
        assert_eq!(calls, 4);

        let control = SolverControl::default();
        let cancel = control.clone();
        let saw = Rc::new(Cell::new(false));
        let observed = saw.clone();
        let factors = Rc::new(Cell::new(0));
        let counted = factors.clone();
        let error = with_solver_observer(
            &control,
            move |p| {
                if p.stage == SolverStage::DenseFactor && p.completed_steps == 0 {
                    counted.set(counted.get() + 1);
                }
                if p.stage == SolverStage::ModalRoundoffValidate && p.completed_steps == 5 {
                    observed.set(true);
                    cancel.request_cancel();
                }
            },
            || attempt(&mut |v| check(v)),
        )
        .unwrap_err();
        assert!(saw.get() && error.contains("cancel"));
        assert_eq!(factors.get(), 1);
        let replay = one_factor(|| attempt(&mut |v| check(v))).unwrap();
        assert_eq!(bits(&replay.shape), bits(&physical.shape));
        assert_eq!(replay.usage.certificates, physical.usage.certificates);
        assert!(
            super::super::Fixture::check_recovered_readback(
                &fixture.input,
                fixture.total_mass,
                fixture.value,
                &replay.shape,
            ) <= 1e-8
        );
        assert_eq!(bits(&seed), retained);
        assert_eq!(bits(&fixture.seed), original);
        assert_eq!(fixture.value.to_bits(), root);
    }
    println!(
        "banded handoff physical_rejections=2 final_faults=2 restoration_cancellations=2 exact_replays=2 no_fallback_factors=1"
    );
}

#[test]
fn modal_banded_handoff_sign_binary_amplitude_and_frozen_root() {
    let mut controls = 0;
    for profile in [Profile::Graded, Profile::Layered] {
        for scale in [1.0, 1e14, 1e-10] {
            let fixture = Prepared::new(profile, scale);
            let original = bits(&fixture.seed);
            let root = fixture.value.to_bits();
            let baseline = internal(&fixture, 1.0);
            let physical = one_factor(|| {
                fixture.physical_candidate_selected(&baseline.shape, Some(baseline.order), || {})
            })
            .unwrap();
            for amplitude in [-2.0, 0.5, 2.0] {
                let proposed = internal(&fixture, amplitude);
                assert_eq!(proposed.order, baseline.order);
                assert_eq!(proposed.usage.attempts, baseline.usage.attempts);
                assert_eq!(proposed.usage.certificates, baseline.usage.certificates);
                assert_eq!(
                    bits(
                        &proposed
                            .shape
                            .iter()
                            .map(|v| v / amplitude)
                            .collect::<Vec<_>>()
                    ),
                    bits(&baseline.shape)
                );
                let actual = one_factor(|| {
                    fixture.physical_candidate_ordered(
                        &proposed.shape,
                        Selection::Single(proposed.order),
                        || {},
                    )
                })
                .unwrap();
                assert_eq!(actual.usage.certificates, physical.usage.certificates);
                let sign = 1.0_f64.copysign(amplitude);
                assert_eq!(
                    bits(&actual.shape.iter().map(|v| sign * v).collect::<Vec<_>>()),
                    bits(&physical.shape)
                );
                assert!(
                    super::super::Fixture::check_recovered_readback(
                        &fixture.input,
                        fixture.total_mass,
                        fixture.value,
                        &actual.shape,
                    ) <= 1e-8
                );
                controls += 1;
            }
            assert_eq!(bits(&fixture.seed), original);
            assert_eq!(fixture.value.to_bits(), root);
        }
    }
    assert_eq!(controls, 18);
    println!(
        "banded handoff amplitude_sign_controls=18 exact_checked_replays=18 physical_factors=1"
    );
}
