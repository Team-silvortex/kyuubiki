use super::*;

fn separated_columns() -> Vec<Vec<f64>> {
    (0..12)
        .map(|i| {
            let mut column = vec![0.0; 15];
            column[i % 3] = 1.0;
            column[3 + i] = (i as f64 + 1.0) / 100.0;
            column
        })
        .collect()
}

#[test]
fn projected_coupling_graph_ignores_index_adjacency_and_bounds_pair_work() {
    let columns = separated_columns();
    let pairs = coupled_pairs(&columns).unwrap();
    assert_eq!(pairs.len(), 18);
    assert!(pairs.len() <= 4 * columns.len());
    assert!(pairs.windows(2).all(|pair| pair[0] < pair[1]));
    assert!(pairs.iter().all(|&(a, b)| a < b && a % 3 == b % 3));
    assert!(pairs.iter().all(|&(a, b)| b - a > 1));
    assert!(coupled_pairs(&vec![vec![0.0; 3]; 12]).unwrap().is_empty());
}

#[test]
fn projected_coupling_graph_is_stable_under_binary_scales_signs_and_permutations() {
    let columns = separated_columns();
    let expected = coupled_pairs(&columns).unwrap();
    let order: Vec<_> = (0..12).map(|i| (5 * i + 7) % 12).collect();
    for exponent in [-900, 0, 900] {
        let permuted: Vec<_> = order
            .iter()
            .enumerate()
            .map(|(new, &old)| {
                columns[old]
                    .iter()
                    .rev()
                    .map(|v| v * 2.0_f64.powi(exponent) * if new % 2 == 0 { 1.0 } else { -1.0 })
                    .collect()
            })
            .collect();
        let mut actual: Vec<_> = coupled_pairs(&permuted)
            .unwrap()
            .iter()
            .map(|&(a, b)| (order[a].min(order[b]), order[a].max(order[b])))
            .collect();
        actual.sort_unstable();
        assert_eq!(actual, expected);
    }
}

#[test]
fn projected_coupling_graph_rejects_invalid_columns_without_accepting_proposals() {
    assert!(coupled_pairs(&[vec![1.0, 0.0], vec![1.0]]).is_err());
    assert!(coupled_pairs(&[vec![f64::NAN]]).is_err());
    assert!(coupled_pairs(&[vec![f64::INFINITY]]).is_err());
    assert!(coupled_pairs(&[vec![f64::MAX; 2]]).is_err());
    assert!(coupled_pairs(&[]).unwrap().is_empty());
    assert!(coupled_pairs(&[vec![1.0]]).unwrap().is_empty());
}

#[test]
fn coupled_preparation_cancels_during_graph_scan_and_replays_fresh() {
    let mut matrix = vec![vec![0.0; 12]; 12];
    for (i, row) in matrix.iter_mut().enumerate() {
        row[i] = 1.0;
    }
    let original = matrix.clone();
    let control = SolverControl::default();
    let cancel = control.clone();
    let error = with_solver_observer(
        &control,
        move |point| {
            if point.stage == SolverStage::ModalVectorScan && point.completed_steps == 3 {
                cancel.request_cancel();
            }
        },
        || BlockFit::prepare_coupled(matrix.clone(), vec![0]),
    )
    .err()
    .unwrap();
    assert!(error.contains("cancel"), "{error}");
    assert_eq!(matrix, original);
    let fit = BlockFit::prepare_coupled(matrix, vec![0]).unwrap();
    let vector = vec![1.0; 12];
    let candidate = fit
        .correct(&vector, 1e-8, |_| Ok((0.0, vec![0.0; 12])))
        .unwrap();
    assert_eq!(candidate, vector);
}
