use super::{independent_components, jacobi_eigenpairs};
use crate::linear_algebra::stable_l2_norm;
use crate::solver_control::{SolverControl, SolverStage, with_solver_observer};

fn close(actual: f64, expected: f64) {
    assert!(actual.is_finite());
    assert!(
        (actual / expected - 1.0).abs() < 1.0e-12,
        "{actual:e} != {expected:e}"
    );
}

#[test]
fn independent_dense_blocks_keep_their_own_representable_scales() {
    let mut matrix = vec![vec![0.0; 6]; 6];
    for (index, scale) in [1.0e-200, 1.0, 1.0e200].into_iter().enumerate() {
        let (a, b) = (index, index + 3);
        matrix[a][a] = 2.0 * scale;
        matrix[b][b] = 2.0 * scale;
        matrix[a][b] = scale;
        matrix[b][a] = scale;
    }
    let pairs = jacobi_eigenpairs(matrix.clone()).unwrap();
    assert_eq!(pairs.len(), 6);
    for ((value, vector), expected) in pairs
        .iter()
        .zip([1.0e-200, 3.0e-200, 1.0, 3.0, 1.0e200, 3.0e200])
    {
        close(*value, expected);
        close(stable_l2_norm(vector.iter().copied()), 1.0);
        let residual =
            stable_l2_norm(matrix.iter().zip(vector).map(|(row, v)| {
                row.iter().zip(vector).map(|(a, x)| a * x).sum::<f64>() - value * v
            }));
        assert!(residual / value.abs() < 1.0e-12, "residual={residual:e}");
    }
    for (i, (_, left)) in pairs.iter().enumerate() {
        for (_, right) in pairs.iter().skip(i + 1) {
            let dot: f64 = left.iter().zip(right).map(|(a, b)| a * b).sum();
            assert!(dot.abs() < 1.0e-12);
        }
    }
}

#[test]
fn stiff_unrelated_diagonal_cannot_hide_a_soft_asymmetric_block() {
    let error = jacobi_eigenpairs(vec![
        vec![2.0e-200, 1.0e-200, 0.0],
        vec![0.5e-200, 2.0e-200, 0.0],
        vec![0.0, 0.0, 1.0e200],
    ])
    .unwrap_err();
    assert!(error.contains("not symmetric"), "{error}");
}

#[test]
fn independent_zero_and_negative_modes_are_not_relabelled_by_a_stiff_block() {
    let pairs = jacobi_eigenpairs(vec![
        vec![-1.0e-200, 0.0, 0.0],
        vec![0.0, 0.0, 0.0],
        vec![0.0, 0.0, 1.0e200],
    ])
    .unwrap();
    close(pairs[0].0, -1.0e-200);
    assert_eq!(pairs[1].0, 0.0);
    close(pairs[2].0, 1.0e200);
}

#[test]
fn nonzero_coupling_is_not_silently_cut_to_create_independent_blocks() {
    let tiny = f64::from_bits(1);
    let error = jacobi_eigenpairs(vec![vec![1.0, tiny], vec![tiny, 1.0e300]]).unwrap_err();
    assert!(
        error.contains("scaling") && error.contains("representable"),
        "{error}"
    );
}

#[test]
fn exact_component_detection_keeps_tiny_bridges_and_ignores_signed_zero() {
    let tiny = f64::from_bits(1);
    let matrix = vec![
        vec![1.0, tiny, 0.0, -0.0],
        vec![tiny, 1.0, 0.25, 0.0],
        vec![0.0, 0.25, 1.0, 0.0],
        vec![-0.0, 0.0, 0.0, 1.0],
    ];
    assert_eq!(
        independent_components(&matrix).unwrap(),
        vec![vec![0, 1, 2], vec![3]]
    );
}

#[test]
fn component_discovery_cancels_at_a_chunk_boundary_and_replays() {
    let mut matrix = vec![vec![0.0; 128]; 128];
    for (i, row) in matrix.iter_mut().enumerate() {
        row[i] = 1.0;
    }
    let control = SolverControl::default();
    let cancel = control.clone();
    let error = with_solver_observer(
        &control,
        move |point| {
            if point.stage == SolverStage::ModalSweep && point.completed_steps == 64 {
                cancel.request_cancel();
            }
        },
        || {
            let result = independent_components(&matrix);
            assert!(
                result.is_err(),
                "discovery must observe cancellation before scope exit"
            );
            result
        },
    )
    .unwrap_err();
    assert!(error.contains("cancel"), "{error}");
    assert_eq!(control.last_checkpoint().unwrap().completed_steps, 64);
    let pairs = jacobi_eigenpairs(matrix).unwrap();
    assert_eq!(pairs.len(), 128);
    assert!(pairs.iter().all(|(value, _)| *value == 1.0));
}
