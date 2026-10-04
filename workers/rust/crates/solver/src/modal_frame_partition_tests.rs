use super::*;

fn diagonal() -> Vec<Vec<f64>> {
    vec![
        vec![2.0, 0.0, 0.0, 0.0],
        vec![0.0, 10.0, 0.0, 0.0],
        vec![0.0, 0.0, 20.0, 0.0],
        vec![0.0, 0.0, 0.0, 1.0],
    ]
}

#[test]
fn automatic_partition_selection_is_bounded_and_uses_rank_not_coordinate_labels() {
    let dots = Rc::new(Cell::new(0));
    let observed = dots.clone();
    let fine = with_solver_observer(
        &SolverControl::default(),
        move |point| {
            if point.stage == SolverStage::ModalVectorDot && point.completed_steps == 0 {
                observed.set(observed.get() + 1);
            }
        },
        || select_fine(&diagonal(), &[1.0; 4]),
    )
    .unwrap();
    assert_eq!(fine, vec![1, 2]);
    assert_eq!(
        dots.get(),
        10,
        "two reorthogonalization passes at each of two pivots"
    );
    let mut dependent = vec![vec![1.0; 4]; 4];
    let error = select_fine(&dependent, &[1.0; 4]).unwrap_err();
    assert!(error.contains("lost independent columns"), "{error}");
    dependent[2][2] = 2.0;
    let fine = select_fine(&dependent, &[1.0; 4]).unwrap();
    assert_eq!(fine.len(), 2);
    assert!(fine.windows(2).all(|pair| pair[0] < pair[1]));
}

#[test]
fn automatic_partition_selection_preserves_sampled_scales_signs_and_permutations() {
    let matrix = diagonal();
    let vector = [1.0; 4];
    let expected = select_fine(&matrix, &vector).unwrap();
    let order = [2, 0, 3, 1];
    for matrix_power in [-500, 0, 500] {
        let reordered: Vec<Vec<_>> = order
            .iter()
            .map(|&i| {
                order
                    .iter()
                    .map(|&j| matrix[i][j] * 2.0_f64.powi(matrix_power))
                    .collect()
            })
            .collect();
        for vector_power in [-80, 0, 80] {
            let scaled: Vec<_> = order
                .iter()
                .map(|&i| -vector[i] * 2.0_f64.powi(vector_power))
                .collect();
            let mut actual: Vec<_> = select_fine(&reordered, &scaled)
                .unwrap()
                .into_iter()
                .map(|i| order[i])
                .collect();
            actual.sort_unstable();
            assert_eq!(actual, expected);
        }
    }
}

#[test]
fn automatic_partition_selection_handles_zero_subnormal_and_maximum_component_grids() {
    for value in [0.0, f64::from_bits(1), f64::MIN_POSITIVE, 1.0, f64::MAX] {
        assert!(grid_log(value).is_finite());
    }
    let vector = [f64::MAX, 0.0, -0.0, -f64::from_bits(1)];
    assert_eq!(select_fine(&diagonal(), &vector).unwrap(), vec![1, 2]);
    assert_eq!(vector[2].to_bits(), (-0.0_f64).to_bits());
}

#[test]
fn automatic_partition_selection_rejects_invalid_inputs_and_scaling_loss() {
    let matrix = diagonal();
    for vector in [
        vec![1.0],
        vec![f64::NAN; 4],
        vec![f64::INFINITY; 4],
        vec![0.0; 4],
    ] {
        assert!(select_fine(&matrix, &vector).is_err());
    }
    for matrix in [
        vec![],
        vec![vec![1.0]],
        vec![vec![0.0; 257]; 257],
        vec![vec![0.0; 4]; 4],
        vec![vec![f64::NAN; 4]; 4],
        vec![vec![1.0; 3]; 4],
    ] {
        assert!(select_fine(&matrix, &[1.0; 4]).is_err());
    }
    let mut matrix = diagonal();
    matrix[0][0] = f64::MAX;
    matrix[0][1] = f64::from_bits(1);
    let error = select_fine(&matrix, &[1.0; 4]).unwrap_err();
    assert!(error.contains("cannot discard a nonzero entry"), "{error}");
}

#[test]
fn automatic_partition_selection_cancels_before_returning_a_subset_and_replays() {
    let matrix = diagonal();
    for step in [1, 2] {
        let control = SolverControl::default();
        let cancel = control.clone();
        let error = with_solver_observer(
            &control,
            move |point| {
                if point.stage == SolverStage::ModalValidation && point.completed_steps == step {
                    cancel.request_cancel();
                }
            },
            || select_fine(&matrix, &[1.0; 4]),
        )
        .unwrap_err();
        assert!(error.contains("cancel"), "{error}");
        assert_eq!(matrix, diagonal());
        assert_eq!(select_fine(&matrix, &[1.0; 4]).unwrap(), vec![1, 2]);
    }
}
