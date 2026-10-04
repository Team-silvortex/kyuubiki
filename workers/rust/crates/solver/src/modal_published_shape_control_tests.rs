use super::*;
use std::cell::RefCell;

fn interrupt_validation(stage: SolverStage, pass: usize, after_repair: bool) {
    let op = coupled(1.0, 130);
    let original = vec![1.0; 130];
    let mut shape = original.clone();
    shape[0] += 1e-5;
    let seed = bits(&shape);
    let repairing = Rc::new(Cell::new(false));
    let passes = Rc::new(Cell::new(0));
    let seen = Rc::new(Cell::new(false));
    let (active, count, hit) = (repairing.clone(), passes.clone(), seen.clone());
    let control = SolverControl::default();
    let cancel = control.clone();
    let error = with_solver_observer(
        &control,
        move |point| {
            if point.stage == SolverStage::ModalValidation {
                active.set(true);
                count.set(0);
            }
            if point.stage != stage || (after_repair && !active.get()) {
                return;
            }
            if point.completed_steps == 0 {
                count.set(count.get() + 1);
            }
            if count.get() == pass && point.completed_steps == 64 {
                hit.set(true);
                cancel.request_cancel();
            }
        },
        || {
            let result = op.polish_published_shape(1.0, &mut shape, &[1.0; 130], 1e-8);
            assert!(
                result.is_err(),
                "validation must cancel before accepting a shape"
            );
            result
        },
    )
    .unwrap_err();
    assert!(seen.get(), "missing {stage:?} pass {pass}");
    assert!(error.contains("cancel"), "{error}");
    assert_eq!(
        bits(&shape),
        seed,
        "no candidate may be committed on cancellation"
    );
    assert_eq!(control.last_checkpoint().unwrap().stage, stage);
    assert_eq!(control.last_checkpoint().unwrap().completed_steps, 64);
    op.polish_published_shape(1.0, &mut shape, &[1.0; 130], 1e-8)
        .unwrap();
    assert!((shape[0] - shape[1]).abs() < 1e-14);
    let applied = op.apply_physical_compensated(&shape).unwrap();
    assert!(
        op.physical_residual(1.0, &shape, &[1.0; 130], &applied)
            .unwrap()
            .0
            <= 1e-8
    );
}

#[test]
fn published_shape_scan_and_all_norm_passes_cancel_before_acceptance() {
    interrupt_validation(SolverStage::ModalShapeScan, 1, false);
    for pass in 1..=3 {
        interrupt_validation(SolverStage::ModalShapeNorm, pass, false);
    }
}

#[test]
fn published_shape_repaired_candidate_is_not_committed_when_final_validation_cancels() {
    interrupt_validation(SolverStage::ModalShapeScan, 1, true);
    for pass in 1..=3 {
        interrupt_validation(SolverStage::ModalShapeNorm, pass, true);
    }
}

#[test]
fn published_shape_validation_trace_bounds_every_scan_and_norm_pass() {
    let op = operator(&[2.0; 129], &[1.0; 129]);
    let trace = Rc::new(RefCell::new(Vec::new()));
    let observed = trace.clone();
    let result = with_solver_observer(
        &SolverControl::default(),
        move |point| {
            if matches!(
                point.stage,
                SolverStage::ModalShapeScan | SolverStage::ModalShapeNorm
            ) {
                observed
                    .borrow_mut()
                    .push((point.stage, point.completed_steps));
            }
        },
        || op.physical_residual(2.0, &[1.0; 129], &[1.0; 129], &[2.0; 129]),
    )
    .unwrap();
    assert_eq!(result.0, 0.0);
    let mut expected = Vec::new();
    for stage in [
        SolverStage::ModalShapeScan,
        SolverStage::ModalShapeNorm,
        SolverStage::ModalShapeNorm,
        SolverStage::ModalShapeNorm,
    ] {
        expected.extend([0, 64, 128, 129].map(|step| (stage, step)));
    }
    assert_eq!(*trace.borrow(), expected);
}

#[test]
fn published_shape_validation_preserves_residual_bits_across_scales_and_chunk_edges() {
    for size in [1, 63, 64, 65, 127, 128, 129, 4097] {
        for scale in [1.0, 1e-200, 1e200] {
            let stiffness: Vec<_> = (0..size)
                .map(|i| (2.0 + (i % 5) as f64 / 7.0) * scale)
                .collect();
            let mass = vec![scale; size];
            let op = operator(&stiffness, &mass);
            let shape: Vec<_> = (0..size)
                .map(|i| (1.0 + (i % 7) as f64 / 32.0) * if i % 2 == 0 { 1.0 } else { -1.0 })
                .collect();
            let applied = op.apply_physical_compensated(&shape).unwrap();
            let target: Vec<_> = shape
                .iter()
                .enumerate()
                .map(|(i, v)| {
                    positive_product([1.0, mass[i], op.inverse_mass_sqrt[i], v.abs()]).copysign(*v)
                })
                .collect();
            let residual: Vec<_> = applied.iter().zip(&target).map(|(a, b)| a - b).collect();
            let reference = crate::linear_algebra::stable_l2_norm(residual.iter().copied())
                / crate::linear_algebra::stable_l2_norm(applied.iter().copied()).max(
                    crate::linear_algebra::stable_l2_norm(target.iter().copied()),
                );
            let actual = op.physical_residual(1.0, &shape, &mass, &applied).unwrap();
            assert_eq!(
                actual.0.to_bits(),
                reference.to_bits(),
                "size={size}, scale={scale:e}"
            );
            assert_eq!(bits(&actual.1), bits(&residual));
        }
    }
}

#[test]
fn published_shape_scan_cancels_before_reaching_an_invalid_tail() {
    let op = operator(&[2.0; 129], &[1.0; 129]);
    for invalid in [f64::NAN, f64::INFINITY, 0.0, -1.0] {
        let mut mass = vec![1.0; 129];
        mass[128] = invalid;
        let control = SolverControl::default();
        let cancel = control.clone();
        let error = with_solver_observer(
            &control,
            move |point| {
                if point.stage == SolverStage::ModalShapeScan && point.completed_steps == 64 {
                    cancel.request_cancel();
                }
            },
            || op.physical_residual(2.0, &[1.0; 129], &mass, &[2.0; 129]),
        )
        .unwrap_err();
        assert!(
            error.contains("cancel"),
            "validation reached the invalid tail: {error}"
        );
        assert_eq!(control.last_checkpoint().unwrap().completed_steps, 64);
        assert!(
            op.physical_residual(2.0, &[1.0; 129], &mass, &[2.0; 129])
                .is_err()
        );
        assert_eq!(
            op.physical_residual(2.0, &[1.0; 129], &[1.0; 129], &[2.0; 129])
                .unwrap()
                .0,
            0.0
        );
    }
}
