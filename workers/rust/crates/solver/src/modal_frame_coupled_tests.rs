use super::*;
use crate::linear_algebra::{SparseMatrix, add_at};
use crate::modal_sparse::reduce_sparse_modal_system;

#[path = "modal_frame_automatic_tests.rs"]
mod automatic;

fn reordered_bending(order: &[usize]) -> ReducedSparseModalSystem {
    let size = order.len();
    let mut inverse = vec![usize::MAX; size];
    for (new, &old) in order.iter().enumerate() {
        assert!(old < size && inverse[old] == usize::MAX);
        inverse[old] = new;
    }
    let mut matrix = SparseMatrix::new(size);
    let element = [
        [12.0, 6.0, -12.0, 6.0],
        [6.0, 4.0, -6.0, 2.0],
        [-12.0, -6.0, 12.0, -6.0],
        [6.0, 2.0, -6.0, 4.0],
    ];
    for segment in 0..size / 2 {
        for (row, values) in element.iter().enumerate() {
            for (column, &value) in values.iter().enumerate() {
                let (row, column) = (2 * segment + row, 2 * segment + column);
                if row >= 2 && column >= 2 {
                    add_at(&mut matrix, inverse[row - 2], inverse[column - 2], value);
                }
            }
        }
    }
    let mass: Vec<_> = order
        .iter()
        .map(|&i| {
            (if i % 2 == 0 { 1.0 } else { 1.0 / 12.0 }) * if i >= size - 2 { 0.5 } else { 1.0 }
        })
        .collect();
    reduce_sparse_modal_system(&matrix, &mass, &[]).unwrap()
}

fn orders(size: usize) -> Vec<(&'static str, Vec<usize>)> {
    vec![
        ("original", (0..size).collect()),
        ("reverse", (0..size).rev().collect()),
        (
            "grouped",
            (0..size).step_by(2).chain((1..size).step_by(2)).collect(),
        ),
        ("mixed", (0..size).map(|i| (73 * i + 19) % size).collect()),
    ]
}

fn original_order(vector: &[f64], order: &[usize]) -> Vec<f64> {
    let mut original = vec![0.0; order.len()];
    for (&old, &v) in order.iter().zip(vector) {
        original[old] = v;
    }
    original
}

fn coupled_fit(system: &ReducedSparseModalSystem, value: f64, order: &[usize]) -> BlockFit {
    let mut matrix = system.operator.dense_fallback_matrix().unwrap();
    for (i, row) in matrix.iter_mut().enumerate() {
        row[i] -= value;
    }
    BlockFit::prepare_coupled(matrix, fine_indices(order)).unwrap()
}

fn fine_indices(order: &[usize]) -> Vec<usize> {
    order
        .iter()
        .enumerate()
        .filter_map(|(new, old)| (old % 2 == 1).then_some(new))
        .collect()
}

#[test]
fn coupled_candidates_validate_reassembled_permutations_of_the_long_bending_mode() {
    let original = bending(128);
    let oracle = BendingReference::new(128);
    let seed = jacobi_eigenpairs(original.operator.dense_fallback_matrix().unwrap()).unwrap();
    let mut pairs = seed[..1].to_vec();
    refine_with_budget(&original, &mut pairs, 1e-8, 4).unwrap_err();
    let (value, seed) = &pairs[0];
    let (reference_value, reference) = oracle.first_mode();
    assert!((*value / reference_value.rounded() - 1.0).abs() < 1e-14);
    for (name, order) in orders(256) {
        let system = reordered_bending(&order);
        let fit = coupled_fit(&system, *value, &order);
        for (exponent, sign) in [(-80, 1.0), (0, 1.0), (0, -1.0), (80, 1.0)] {
            let scale = sign * 2.0_f64.powi(exponent);
            let vector: Vec<_> = order.iter().map(|&i| seed[i] * scale).collect();
            let unchanged = vector.clone();
            let before = normalized_check(&system, *value, &vector).unwrap().0;
            let candidate = fit
                .correct(&vector, 1e-8, |v| normalized_check(&system, *value, v))
                .unwrap_or_else(|error| panic!("{name} scale={scale:e}: {error}"));
            let actual = normalized_check(&system, *value, &candidate).unwrap().0;
            let wide = wide_vector(&original_order(&candidate, &order));
            let independent = oracle.residual(Wide::from(*value), &wide);
            println!(
                "coupled {name} exponent={exponent} sign={sign} before={before:e} actual={actual:e} independent={independent:e}"
            );
            assert!(before > 1e-8 && actual <= 1e-8 && independent <= 1e-8);
            assert!((actual / independent - 1.0).abs() < 1e-6);
            assert!(oracle.direction_error(&wide, &reference) < 1e-10);
            assert_eq!(vector, unchanged);
        }
    }
}

#[test]
fn coupled_reference_samples_keep_true_residuals_after_coordinate_reassembly() {
    for segments in [64, 80, 100, 128] {
        let oracle = BendingReference::new(segments);
        let (value, reference) = oracle.first_mode();
        for (name, order) in orders(2 * segments) {
            let system = reordered_bending(&order);
            let vector: Vec<_> = order.iter().map(|&i| reference[i].rounded()).collect();
            let initial = normalized_check(&system, value.rounded(), &vector)
                .unwrap()
                .0;
            let candidate = coupled_fit(&system, value.rounded(), &order)
                .correct(&vector, 1e-8, |v| {
                    normalized_check(&system, value.rounded(), v)
                })
                .unwrap_or_else(|error| panic!("segments={segments} {name}: {error}"));
            let actual = normalized_check(&system, value.rounded(), &candidate)
                .unwrap()
                .0;
            let independent = oracle.residual(
                Wide::from(value.rounded()),
                &wide_vector(&original_order(&candidate, &order)),
            );
            println!(
                "coupled sample segments={segments} {name} actual={actual:e} wide={independent:e}"
            );
            assert!(actual <= 1e-8 && independent <= 1e-8);
            assert!((actual / independent - 1.0).abs() < 1e-6);
            assert!(
                oracle.direction_error(
                    &wide_vector(&original_order(&candidate, &order)),
                    &reference
                ) < 1e-12
            );
            if initial <= 1e-8 {
                assert_eq!(candidate, vector);
            }
        }
    }
}

#[test]
fn coupled_physical_candidates_survive_reassembled_coordinates_and_json_readback() {
    let original = bending(128);
    let oracle = BendingReference::new(128);
    let seed = jacobi_eigenpairs(original.operator.dense_fallback_matrix().unwrap()).unwrap();
    let mut pairs = seed[..1].to_vec();
    refine_with_budget(&original, &mut pairs, 1e-8, 4).unwrap_err();
    let (value, seed) = &pairs[0];
    for (name, order) in orders(256) {
        let mut system = reordered_bending(&order);
        let vector: Vec<_> = order.iter().map(|&i| seed[i]).collect();
        let candidate = coupled_fit(&system, *value, &order)
            .correct(&vector, 1e-8, |v| normalized_check(&system, *value, v))
            .unwrap();
        // Preserve original physical node identities while rebuilding reduced algebra.
        system.free_dofs = order.iter().map(|i| 3 * (i / 2 + 1) + 1 + i % 2).collect();
        let size = 3 * 129;
        let mut mass = vec![1.0; size];
        for (&dof, &m) in system.free_dofs.iter().zip(&system.mass) {
            mass[dof] = m;
        }
        let (mut shape, _) =
            checked_mode_shape(&candidate, &mass, &system.free_dofs, size).unwrap();
        let reduced: Vec<_> = system.free_dofs.iter().map(|&i| shape[i]).collect();
        let mut matrix = system.operator.dense_fallback_matrix().unwrap();
        for (i, row) in matrix.iter_mut().enumerate() {
            for (a, &m) in row.iter_mut().zip(&system.mass) {
                *a /= m.sqrt().recip();
            }
            row[i] -= value * system.mass[i] * system.mass[i].sqrt().recip();
        }
        let fit = BlockFit::prepare_coupled(matrix, fine_indices(&order)).unwrap();
        let mut candidate = fit
            .correct(&reduced, 1e-8, |v| physical_check(&system, *value, v))
            .unwrap_or_else(|error| panic!("physical {name}: {error}"));
        system
            .operator
            .polish_published_shape(*value, &mut candidate, &system.mass, 1e-8)
            .unwrap();
        for (&dof, &v) in system.free_dofs.iter().zip(&candidate) {
            shape[dof] = v;
        }
        let norm = checked_shape_norm(&shape).unwrap();
        assert!((norm - 1.0).abs() < 1e-10);
        let frequency = value.sqrt();
        let mode = ModalFrame2dModeResult {
            index: 0,
            eigenvalue_rad_s_squared: *value,
            natural_frequency_rad_s: frequency,
            natural_frequency_hz: frequency / std::f64::consts::TAU,
            period_s: std::f64::consts::TAU / frequency,
            participation_norm: norm,
            shape,
        };
        let restored: ModalFrame2dModeResult =
            serde_json::from_slice(&serde_json::to_vec(&mode).unwrap()).unwrap();
        assert_eq!(restored, mode);
        assert!(
            mode.shape
                .iter()
                .zip(&restored.shape)
                .all(|(a, b)| a.to_bits() == b.to_bits())
        );
        assert!(restored.shape[..3].iter().all(|v| *v == 0.0));
        assert!(restored.shape.chunks_exact(3).all(|node| node[0] == 0.0));
        assert!((restored.natural_frequency_hz * restored.period_s - 1.0).abs() < 1e-12);
        let reduced: Vec<_> = system
            .free_dofs
            .iter()
            .map(|&i| restored.shape[i])
            .collect();
        let actual = physical_check(&system, *value, &reduced).unwrap().0;
        let independent = oracle.physical_residual(
            Wide::from(*value),
            &wide_vector(&original_order(&reduced, &order)),
            &original.mass,
        );
        println!("coupled physical {name} readback={actual:e} independent={independent:e}");
        assert!(actual <= 1e-8 && independent <= 1e-8);
        assert!((actual / independent - 1.0).abs() < 1e-6);
    }
}

#[test]
fn coupled_search_reuses_one_factor_and_replays_after_private_budget_failure() {
    let order = orders(256).pop().unwrap().1;
    let system = reordered_bending(&order);
    let oracle = BendingReference::new(128);
    let (value, reference) = oracle.first_mode();
    let vector: Vec<_> = order.iter().map(|&i| reference[i].rounded()).collect();
    let original = vector.clone();
    let factors = Rc::new(Cell::new(0));
    let observed = factors.clone();
    with_solver_observer(
        &SolverControl::default(),
        move |point| {
            if point.stage == SolverStage::DenseFactor && point.completed_steps == 0 {
                observed.set(observed.get() + 1);
            }
        },
        || {
            let fit = coupled_fit(&system, value.rounded(), &order);
            let mut calls = 0;
            let error = fit
                .correct(&vector, 1e-30, |v| {
                    calls += 1;
                    normalized_check(&system, value.rounded(), v)
                })
                .unwrap_err();
            assert!(error.contains("did not reach its unchanged residual gate"));
            assert!(
                calls <= 80,
                "the four-cycle search must remain bounded: {calls}"
            );
            assert_eq!(vector, original);
            println!("coupled bounded checked_calls={calls}");
            let candidate = fit
                .correct(&vector, 1e-8, |v| {
                    normalized_check(&system, value.rounded(), v)
                })
                .unwrap();
            assert!(
                oracle.residual(
                    Wide::from(value.rounded()),
                    &wide_vector(&original_order(&candidate, &order))
                ) <= 1e-8
            );
            assert_eq!(vector, original);
            Ok::<(), String>(())
        },
    )
    .unwrap();
    assert_eq!(factors.get(), 1);
}
