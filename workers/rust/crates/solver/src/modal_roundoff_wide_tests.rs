use super::{
    hybrid_tests::{check_published, scaled_bending_parts},
    wide::{WideProposal, unit_shape},
    *,
};

#[test]
fn scaled_wide_proposals_measure_rounding_normalization_and_physical_correction() {
    for step in [1.0_f64, 1e14, 1e-10] {
        let (system, physical_matrix) = scaled_bending_parts(128, step);
        let scales: Vec<_> = (0..256)
            .map(|i| if i % 2 == 0 { 1.0 } else { step.recip() })
            .collect();
        let proposal = WideProposal::prepare(&physical_matrix, &system.mass, &scales).unwrap();
        let (value, physical) = proposal.first_mode().unwrap();
        let internal = proposal.physical_residual(value, &physical).unwrap();
        assert!(internal < 1e-12, "wide intermediate residual={internal:e}");
        let spectrum = crate::modal_frame_spectrum::frame_eigenpairs(&system, Some(1)).unwrap();
        assert!((value / spectrum.pairs[0].0 - 1.0).abs() < 1e-14);
        println!(
            "scaled wide step={step:e}: wide={internal:e} root_drift={:e} production_root={:.17e}",
            (value / spectrum.pairs[0].0 - 1.0).abs(),
            spectrum.pairs[0].0
        );
        let raw: Vec<_> = physical.iter().map(|v| v.rounded()).collect();
        let norm = crate::modal_math::checked_shape_norm(&raw).unwrap();
        let rounded_unit: Vec<_> = raw.iter().map(|v| v / norm).collect();
        let wide_unit = unit_shape(&physical).unwrap();
        let normalized: Vec<_> = physical
            .iter()
            .zip(&system.mass)
            .map(|(&v, &m)| v.mul(wide::Wide::from(m.sqrt())).rounded())
            .collect();
        let (expanded, _) = crate::modal_frame_spectrum::checked_mode_shape(
            &normalized,
            &system.mass,
            &(0..256).collect::<Vec<_>>(),
            256,
        )
        .unwrap();
        let normalized_applied = system.operator.apply_compensated(&normalized).unwrap();
        let normalized_relative =
            relative_residual(value, &normalized, &normalized_applied).unwrap();
        let chain = super::normalized(&system, value, &normalized, 1e-8).and_then(|vector| {
            let (shape, _) = crate::modal_frame_spectrum::checked_mode_shape(
                &vector,
                &system.mass,
                &(0..256).collect::<Vec<_>>(),
                256,
            )?;
            let mut shape = shape;
            system.operator.polish_published_shape_with_recovery(
                value,
                &mut shape,
                &system.mass,
                1e-8,
                true,
            )?;
            Ok(shape)
        });
        println!(
            "scaled wide step={step:e} normalized initial={normalized_relative:e} full chain={:?}",
            chain.as_ref().map(|v| {
                let applied = system.operator.apply_physical_compensated(v).unwrap();
                system
                    .operator
                    .physical_residual(value, v, &system.mass, &applied)
                    .unwrap()
                    .0
            })
        );
        assert_eq!(chain.is_ok(), step != 1e-10, "{chain:?}");
        match &chain {
            Ok(candidate) => check_published(128, step, value, candidate),
            Err(error) => {
                assert!(
                    error.contains("physical modal roundoff recovery failed"),
                    "{error}"
                );
                assert!(error.contains("unchanged residual gate"), "{error}");
            }
        }
        let mut matrix = system.operator.dense_fallback_matrix().unwrap();
        for (i, row) in matrix.iter_mut().enumerate() {
            for (a, &m) in row.iter_mut().zip(&system.mass) {
                *a /= m.sqrt().recip();
            }
            row[i] -= value * system.mass[i] * system.mass[i].sqrt().recip();
        }
        for (name, shape) in [
            ("raw rounded", raw),
            ("rounded then unit", rounded_unit),
            ("wide then unit", wide_unit),
            ("normalized then expanded", expanded),
        ] {
            let applied = system.operator.apply_physical_compensated(&shape).unwrap();
            let relative = system
                .operator
                .physical_residual(value, &shape, &system.mass, &applied)
                .unwrap()
                .0;
            let fit = BlockFit::prepare_automatic(matrix.clone(), &shape).unwrap();
            let corrected = fit.correct(&shape, 1e-8, |v| {
                let applied = system.operator.apply_physical_compensated(v)?;
                system
                    .operator
                    .physical_residual(value, v, &system.mass, &applied)
            });
            println!(
                "scaled wide step={step:e} {name}: root={value:.17e} initial={relative:e} correction={:?}",
                corrected.as_ref().map(|v| {
                    let applied = system.operator.apply_physical_compensated(v).unwrap();
                    system
                        .operator
                        .physical_residual(value, v, &system.mass, &applied)
                        .unwrap()
                        .0
                })
            );
            assert!(
                relative > 1e-8,
                "wide intermediates do not certify rounded output"
            );
            let expected = if name == "raw rounded" {
                step == 1e14
            } else {
                step != 1e-10
            };
            assert_eq!(
                corrected.is_ok(),
                expected,
                "step={step:e} {name}: {corrected:?}"
            );
            match corrected {
                Ok(candidate) if name != "raw rounded" => {
                    check_published(128, step, value, &candidate)
                }
                Err(error) => assert!(error.contains("unchanged residual gate"), "{error}"),
                Ok(_) => {}
            }
        }
    }
}

#[test]
fn scaled_wide_known_diagonal_modes_and_unit_shapes_have_independent_analytic_checks() {
    let matrix = vec![vec![1.0, 0.0], vec![0.0, 4.0]];
    let mass = [1.0, 1.0];
    let proposal = WideProposal::prepare(&matrix, &mass, &mass).unwrap();
    let (root, vector) = proposal.first_mode().unwrap();
    assert_eq!(root, 1.0);
    let rounded: Vec<_> = vector.iter().map(|v| v.rounded()).collect();
    assert_eq!(rounded[0], 1.0);
    assert_eq!(rounded[1], 2.0_f64.powi(-64));
    let residual = (rounded[0] - root * rounded[0]).hypot(4.0 * rounded[1] - root * rounded[1]);
    assert!(residual < 1e-18);
    assert!(proposal.physical_residual(f64::MAX, &vector).is_err());
    assert!(proposal.physical_residual(1.0, &[]).is_err());
    assert!(
        proposal
            .physical_residual(1.0, &[wide::Wide::from(0.0); 2])
            .is_err()
    );
    let physical = [wide::Wide::from(3.0), wide::Wide::from(4.0)];
    assert_eq!(unit_shape(&physical).unwrap(), [0.6, 0.8]);
    for exponent in [-200, 200] {
        let factor = wide::Wide::from(2.0_f64.powi(exponent));
        let scaled: Vec<_> = physical.iter().map(|&v| v.mul(factor)).collect();
        assert_eq!(unit_shape(&scaled).unwrap(), [0.6, 0.8]);
    }
}

#[test]
fn scaled_wide_invalid_range_rank_band_and_shape_inputs_fail_without_partial_results() {
    use crate::solver_control::{SolverControl, SolverStage, with_solver_observer};
    use std::{cell::Cell, rc::Rc};
    let counts = Rc::new(Cell::new(0));
    let observed = counts.clone();
    with_solver_observer(
        &SolverControl::default(),
        move |p| {
            if p.stage == SolverStage::DenseFactor {
                observed.set(observed.get() + 1);
            }
        },
        || {
            assert!(
                WideProposal::prepare(&vec![vec![1.0; 257]; 257], &[1.0; 257], &[1.0; 257])
                    .is_err()
            );
            Ok::<(), String>(())
        },
    )
    .unwrap();
    assert_eq!(counts.get(), 0);
    for matrix in [
        vec![],
        vec![vec![1.0]],
        vec![vec![1.0, 0.0], vec![0.0]],
        vec![vec![1.0, 2.0], vec![0.0, 1.0]],
        vec![vec![1.0; 2]; 2],
        vec![vec![1.0, 0.0], vec![0.0, -1.0]],
        vec![vec![f64::NAN; 2]; 2],
        vec![vec![1e101, 0.0], vec![0.0, 1.0]],
    ] {
        assert!(WideProposal::prepare(&matrix, &[1.0; 2], &[1.0; 2]).is_err());
    }
    let mut outside_band = vec![vec![0.0; 5]; 5];
    for (i, row) in outside_band.iter_mut().enumerate() {
        row[i] = 1.0;
    }
    outside_band[0][4] = 1e-30;
    outside_band[4][0] = 1e-30;
    assert!(WideProposal::prepare(&outside_band, &[1.0; 5], &[1.0; 5]).is_err());
    let identity = vec![vec![1.0, 0.0], vec![0.0, 1.0]];
    for mass in [[0.0, 1.0], [f64::INFINITY, 1.0], [1e-30, 1.0]] {
        assert!(WideProposal::prepare(&identity, &mass, &[1.0; 2]).is_err());
    }
    for scales in [[0.0, 1.0], [f64::NAN, 1.0], [1e21, 1.0]] {
        assert!(WideProposal::prepare(&identity, &[1.0; 2], &scales).is_err());
    }
    assert!(unit_shape(&[]).is_err());
    assert!(unit_shape(&[wide::Wide::from(0.0); 2]).is_err());
    assert!(unit_shape(&[wide::Wide::from(1.0); 257]).is_err());
    assert!(unit_shape(&[wide::Wide::from(1e100), wide::Wide::from(1e-300)]).is_err());
    let mut expansive = vec![vec![0.0; 64]; 64];
    for (i, row) in expansive.iter_mut().enumerate() {
        row[i] = if i == 0 { 1.0 } else { 1e10 + 1.0 };
        if i > 0 {
            row[i - 1] = 1e5;
        }
        if i < 63 {
            row[i + 1] = 1e5;
        }
    }
    let saved = expansive.clone();
    let bounded = WideProposal::prepare(&expansive, &[1.0; 64], &[1.0; 64]).unwrap();
    assert!(bounded.first_mode().unwrap_err().contains("range"));
    assert!(bounded.first_mode().unwrap_err().contains("range"));
    assert_eq!(expansive, saved);
}

#[test]
fn scaled_wide_factor_budget_and_phase_cancellation_preserve_borrowed_models_and_replay() {
    use crate::solver_control::{SolverControl, SolverStage, with_solver_observer};
    use std::{cell::Cell, rc::Rc};
    let (system, matrix) = scaled_bending_parts(8, 1.0);
    let original = matrix.clone();
    let scales = vec![1.0; 16];
    let factors = Rc::new(Cell::new(0));
    let iterations = Rc::new(Cell::new(0));
    let substitutions = Rc::new(Cell::new(0));
    let (f, i, s) = (factors.clone(), iterations.clone(), substitutions.clone());
    let baseline = with_solver_observer(
        &SolverControl::default(),
        move |p| {
            if p.stage == SolverStage::DenseFactor && p.completed_steps == 0 {
                f.set(f.get() + 1);
            }
            if p.stage == SolverStage::ModalIteration {
                i.set(i.get() + 1);
            }
            if p.stage == SolverStage::DenseSubstitution && p.completed_steps == 0 {
                s.set(s.get() + 1);
            }
        },
        || WideProposal::prepare(&matrix, &system.mass, &scales)?.first_mode(),
    )
    .unwrap();
    assert_eq!(
        (factors.get(), iterations.get(), substitutions.get()),
        (1, 32, 32)
    );
    let factor = WideProposal::prepare(&matrix, &system.mass, &scales).unwrap();
    for (stage, step) in [
        (SolverStage::ModalRoundoffPrepare, 0),
        (SolverStage::DenseFactor, 4),
        (SolverStage::ModalIteration, 3),
        (SolverStage::DenseSubstitution, 16),
        (SolverStage::ModalRoundoffValidate, 0),
    ] {
        let control = SolverControl::default();
        let cancel = control.clone();
        let error = with_solver_observer(
            &control,
            move |p| {
                if p.stage == stage && p.completed_steps == step {
                    cancel.request_cancel();
                }
            },
            || {
                if matches!(
                    stage,
                    SolverStage::ModalRoundoffPrepare | SolverStage::DenseFactor
                ) {
                    WideProposal::prepare(&matrix, &system.mass, &scales)?.first_mode()
                } else {
                    factor.first_mode()
                }
            },
        )
        .unwrap_err();
        assert!(error.contains("cancel"), "{error}");
        assert_eq!(control.last_checkpoint().unwrap().stage, stage);
        let replay = factor.first_mode().unwrap();
        assert_eq!(replay.0.to_bits(), baseline.0.to_bits());
        assert_eq!(
            unit_shape(&replay.1).unwrap(),
            unit_shape(&baseline.1).unwrap()
        );
        assert_eq!(matrix, original);
    }
    let shape = vec![wide::Wide::from(1.0); 128];
    for (stage, step) in [
        (SolverStage::ModalShapeScan, 64),
        (SolverStage::ModalVectorUpdate, 64),
        (SolverStage::ModalRoundoffValidate, 1),
    ] {
        let control = SolverControl::default();
        let cancel = control.clone();
        let error = with_solver_observer(
            &control,
            move |p| {
                if p.stage == stage && p.completed_steps == step {
                    cancel.request_cancel();
                }
            },
            || unit_shape(&shape),
        )
        .unwrap_err();
        assert!(error.contains("cancel"));
        assert_eq!(
            unit_shape(&shape).unwrap(),
            unit_shape(&vec![wide::Wide::from(1.0); 128]).unwrap()
        );
    }
}
