use super::*;
use crate::solver_control::{SolverControl, SolverStage, with_solver_observer};
use std::{cell::Cell, rc::Rc};

#[test]
fn modal_material_inward_chart_real_physical_fault_sign_cancel_and_replay() {
    for young in [1.0, 16.0] {
        let mut input = Fixture::request(Profile::Layered, 128, 1e-10);
        for e in &mut input.elements {
            e.youngs_modulus *= young;
            e.density *= 4.0;
        }
        let layout = Rebuilt::prepare(&input).unwrap();
        let fixture = Prepared::from_request(layout.canonical.clone());
        let original = bits(&fixture.seed);
        let root = fixture.value.to_bits();
        let (_, internal) = internal(&fixture);
        let seed = physical_seed(&fixture, &internal.shape);
        let retained = bits(&seed);
        let matrix =
            physical_directions(&fixture.physical, &fixture.system.mass, fixture.value).unwrap();
        let checked = |v: &[f64]| {
            let applied = fixture.system.operator.apply_physical_compensated(v)?;
            fixture.system.operator.physical_residual(
                fixture.value,
                v,
                &fixture.system.mass,
                &applied,
            )
        };
        let baseline = search_canonical_unit_shape_inward(
            &matrix,
            &seed,
            &fixture.system.mass,
            1e-8,
            internal.order,
            checked,
        )
        .unwrap();
        for sign in [-1.0, 1.0] {
            let signed: Vec<_> = seed.iter().map(|v| sign * v).collect();
            let actual = search_canonical_unit_shape_inward(
                &matrix,
                &signed,
                &fixture.system.mass,
                1e-8,
                internal.order,
                checked,
            )
            .unwrap();
            assert_eq!(
                bits(&actual.shape.iter().map(|v| sign * v).collect::<Vec<_>>()),
                bits(&baseline.shape)
            );
            assert_eq!(actual.usage.certificates, baseline.usage.certificates);
            let output = Fixture::canonical_recovered_result(
                &fixture.input,
                fixture.total_mass,
                fixture.value,
                &actual.shape,
            );
            Fixture::check_public_readback(&layout.restore(output).unwrap());
        }
        let mut calls = 0;
        let error = super::super::super::single_policy_tests::one_factor(|| {
            search_canonical_unit_shape_inward(
                &matrix,
                &seed,
                &fixture.system.mass,
                1e-8,
                internal.order,
                |v| {
                    calls += 1;
                    let (_, residual) = checked(v)?;
                    Ok((1.0, residual))
                },
            )
        })
        .unwrap_err();
        assert!(
            error.contains("attempts=1") && error.contains("exhausted its bounded policies"),
            "{error}"
        );
        assert!(calls <= 6);
        calls = 0;
        let error = super::super::super::single_policy_tests::one_factor(|| {
            search_canonical_unit_shape_inward(
                &matrix,
                &seed,
                &fixture.system.mass,
                1e-8,
                internal.order,
                |v| {
                    calls += 1;
                    if calls == baseline.usage.certificates {
                        return Err(
                            "unchanged residual gate: injected inward final operator fault".into(),
                        );
                    }
                    checked(v)
                },
            )
        })
        .unwrap_err();
        assert_eq!(
            error,
            "unchanged residual gate: injected inward final operator fault"
        );
        assert_eq!(calls, baseline.usage.certificates);
        let baseline = run_inward(&input).unwrap();
        let changed = renumber(&input, 113);
        let frozen = changed.clone();
        let control = SolverControl::default();
        let cancel = control.clone();
        let saw = Rc::new(Cell::new(false));
        let observed = saw.clone();
        let error = with_solver_observer(
            &control,
            move |p| {
                if p.stage == SolverStage::ResultNodes && p.completed_steps == 129 {
                    observed.set(true);
                    cancel.request_cancel();
                }
            },
            || run_inward(&changed).map(|_| ()),
        )
        .unwrap_err();
        assert!(saw.get() && error.contains("cancel"), "{error}");
        let replay = run_inward(&changed).unwrap();
        compare_inward(&baseline, &replay);
        assert_eq!(changed, frozen);
        assert_eq!(bits(&seed), retained);
        assert_eq!(bits(&fixture.seed), original);
        assert_eq!(fixture.value.to_bits(), root);
    }
    println!(
        "material inward real_sign_readbacks=4 physical_rejections=2 final_faults=2 last_restore_cancellations=2 fresh_exact_replays=2 physical_factors=1"
    );
}
