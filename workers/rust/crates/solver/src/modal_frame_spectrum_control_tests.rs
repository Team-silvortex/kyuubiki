use super::*;
use crate::linear_algebra::{SparseMatrix, add_at};
use crate::modal_sparse::reduce_sparse_modal_system;
use crate::solver_control::{SolverControl, with_solver_observer};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

fn diagonal() -> ReducedSparseModalSystem {
    let mut stiffness = SparseMatrix::new(129);
    for index in 0..129 {
        add_at(&mut stiffness, index, index, (index + 1) as f64);
    }
    reduce_sparse_modal_system(&stiffness, &[1.0; 129], &[]).unwrap()
}

fn check(spectrum: &FrameSpectrum) {
    assert_eq!(spectrum.residual_tolerance, 1e-8);
    assert_eq!(spectrum.pairs.len(), 3);
    for (index, (value, vector)) in spectrum.pairs.iter().enumerate() {
        assert_eq!(*value, (index + 1) as f64);
        for (dof, &component) in vector.iter().enumerate() {
            assert_eq!(
                component.to_bits(),
                if dof == index { 1.0_f64.to_bits() } else { 0 }
            );
        }
    }
}

#[test]
fn spectrum_final_norm_passes_cancel_inside_validation_and_replay() {
    let system = diagonal();
    for pass in 1..=9 {
        let control = SolverControl::default();
        let cancel = control.clone();
        let passes = Rc::new(Cell::new(0));
        let count = passes.clone();
        let error = with_solver_observer(
            &control,
            move |point| {
                if point.stage == SolverStage::ModalSpectrumNorm {
                    if point.completed_steps == 0 {
                        count.set(count.get() + 1);
                    }
                    if count.get() == pass && point.completed_steps == 64 {
                        cancel.request_cancel();
                    }
                }
            },
            || {
                let result = frame_eigenpairs(&system, Some(3));
                assert!(
                    result.is_err(),
                    "final validation must not return a partial spectrum"
                );
                result
            },
        )
        .err()
        .expect("must cancel the selected norm pass");
        assert!(
            error.contains("modal_spectrum_norm after 64 steps"),
            "{error}"
        );
        assert_eq!(passes.get(), pass);
        assert_eq!(control.last_checkpoint().unwrap().completed_steps, 64);
        check(&frame_eigenpairs(&system, Some(3)).unwrap());
    }
}

#[test]
fn spectrum_norm_trace_covers_all_returned_modes_without_changing_their_bits() {
    let system = diagonal();
    let trace = Rc::new(RefCell::new(Vec::new()));
    let observed = trace.clone();
    let spectrum = with_solver_observer(
        &SolverControl::default(),
        move |point| {
            if point.stage == SolverStage::ModalSpectrumNorm {
                observed.borrow_mut().push(point.completed_steps);
            }
        },
        || frame_eigenpairs(&system, Some(3)),
    )
    .unwrap();
    check(&spectrum);
    assert_eq!(*trace.borrow(), [0, 64, 128, 129].repeat(9));
}
