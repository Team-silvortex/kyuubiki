use super::{TridiagonalSystem, validate};
use crate::solver_control::{SolverControl, SolverStage, with_solver_observer};

#[test]
fn original_chain_constraints_check_interior_equations_not_only_segment_boundaries() {
    let system = TridiagonalSystem {
        diagonal: &[2.0; 5],
        lower: &[0.0; 4],
        upper: &[0.0; 4],
        rhs: &[0.0; 5],
    };
    let fixed = [true, false, false, false, true];
    let prescribed = [(0, 0.0), (4, 0.0)];
    assert!(
        validate(&system, &fixed, &prescribed, &[0.0, 0.0, 1.0, 0.0, 0.0])
            .unwrap_err()
            .contains("row 2")
    );
    validate(&system, &fixed, &prescribed, &[0.0; 5]).unwrap();
}

#[test]
fn original_chain_constraints_check_exact_fixed_values_and_allow_reactions() {
    let system = TridiagonalSystem {
        diagonal: &[1.0; 2],
        lower: &[0.0],
        upper: &[0.0],
        rhs: &[100.0; 2],
    };
    let q = f64::from_bits(1);
    assert!(
        validate(&system, &[true; 2], &[(0, q), (1, 7.0)], &[0.0, 7.0])
            .unwrap_err()
            .contains("prescribed value changed")
    );
    validate(&system, &[true; 2], &[(0, q), (1, 7.0)], &[q, 7.0]).unwrap();
}

#[test]
fn original_chain_constraints_exclude_fixed_zero_roundoff_and_validate_dimensions() {
    let system = TridiagonalSystem {
        diagonal: &[1.0; 2],
        lower: &[0.0],
        upper: &[1.0e308],
        rhs: &[0.0; 2],
    };
    assert!(
        validate(&system, &[false, true], &[(1, 0.0)], &[1.0e-16, 0.0])
            .unwrap_err()
            .contains("row 0")
    );
    assert!(validate(&system, &[true], &[], &[0.0; 2]).is_err());
    assert!(validate(&system, &[true; 2], &[], &[0.0]).is_err());
    assert!(validate(&system, &[true; 2], &[(2, 0.0)], &[0.0; 2]).is_err());
    assert!(validate(&system, &[true; 2], &[], &[f64::NAN, 0.0]).is_err());
}

#[test]
fn original_chain_constraints_cancel_full_scan_and_completion_then_replay() {
    let diagonal = vec![1.0; 130];
    let off_diagonal = vec![0.0; 129];
    let rhs = vec![0.0; 130];
    let system = TridiagonalSystem {
        diagonal: &diagonal,
        lower: &off_diagonal,
        upper: &off_diagonal,
        rhs: &rhs,
    };
    for step in [64, 192, 260] {
        let control = SolverControl::default();
        let cancel = control.clone();
        let result = with_solver_observer(
            &control,
            move |point| {
                if point.stage == SolverStage::ConstraintValidate && point.completed_steps == step {
                    cancel.request_cancel();
                }
            },
            || validate(&system, &[false; 130], &[], &rhs),
        );
        assert!(result.unwrap_err().contains("cancelled"));
        assert!(control.was_interrupted());
        validate(&system, &[false; 130], &[], &rhs).unwrap();
    }
}
