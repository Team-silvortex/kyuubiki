use super::*;

#[path = "modal_frame_subspace_reference.rs"]
mod subspace;

fn automatic_fit(system: &ReducedSparseModalSystem, value: f64, vector: &[f64]) -> BlockFit {
    let mut matrix = system.operator.dense_fallback_matrix().unwrap();
    for (i, row) in matrix.iter_mut().enumerate() {
        row[i] -= value;
    }
    BlockFit::prepare_automatic(matrix, vector).unwrap()
}

#[test]
fn automatic_partitions_check_reassembled_first_mode_candidates_without_coordinate_labels() {
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
        for (exponent, sign) in [(-80, 1.0), (0, 1.0), (0, -1.0), (80, 1.0)] {
            let scale = sign * 2.0_f64.powi(exponent);
            let vector: Vec<_> = order.iter().map(|&i| seed[i] * scale).collect();
            let unchanged = vector.clone();
            let candidate = automatic_fit(&system, *value, &vector)
                .correct(&vector, 1e-8, |v| normalized_check(&system, *value, v))
                .unwrap_or_else(|error| {
                    panic!("automatic {name} exponent={exponent} sign={sign}: {error}")
                });
            let actual = normalized_check(&system, *value, &candidate).unwrap().0;
            let wide = wide_vector(&original_order(&candidate, &order));
            let independent = oracle.residual(Wide::from(*value), &wide);
            println!(
                "automatic {name} exponent={exponent} sign={sign} actual={actual:e} independent={independent:e}"
            );
            assert!(actual <= 1e-8 && independent <= 1e-8);
            assert!((actual / independent - 1.0).abs() < 1e-6);
            assert!(oracle.direction_error(&wide, &reference) < 1e-10);
            assert_eq!(vector, unchanged);
        }
    }
}

#[test]
fn automatic_partitions_check_reference_samples_and_preserve_accepted_directions() {
    for segments in [64, 80, 100, 128] {
        let oracle = BendingReference::new(segments);
        let (value, reference) = oracle.first_mode();
        for (name, order) in orders(2 * segments) {
            let system = reordered_bending(&order);
            let vector: Vec<_> = order.iter().map(|&i| reference[i].rounded()).collect();
            let original = vector.clone();
            let initial = normalized_check(&system, value.rounded(), &vector)
                .unwrap()
                .0;
            let candidate = automatic_fit(&system, value.rounded(), &vector)
                .correct(&vector, 1e-8, |v| {
                    normalized_check(&system, value.rounded(), v)
                })
                .unwrap_or_else(|error| {
                    panic!("automatic sample segments={segments} {name}: {error}")
                });
            let actual = normalized_check(&system, value.rounded(), &candidate)
                .unwrap()
                .0;
            let wide = wide_vector(&original_order(&candidate, &order));
            let independent = oracle.residual(Wide::from(value.rounded()), &wide);
            println!(
                "automatic sample segments={segments} {name} actual={actual:e} independent={independent:e}"
            );
            assert!(actual <= 1e-8 && independent <= 1e-8);
            assert!((actual / independent - 1.0).abs() < 1e-6);
            assert!(oracle.direction_error(&wide, &reference) < 1e-12);
            assert_eq!(vector, original);
            if initial <= 1e-8 {
                assert_eq!(candidate, vector);
            }
        }
    }
}

#[test]
fn automatic_physical_partitions_validate_shapes_after_json_readback() {
    let original = bending(128);
    let oracle = BendingReference::new(128);
    let seed = jacobi_eigenpairs(original.operator.dense_fallback_matrix().unwrap()).unwrap();
    let mut pairs = seed[..1].to_vec();
    refine_with_budget(&original, &mut pairs, 1e-8, 4).unwrap_err();
    let (value, seed) = &pairs[0];
    for (name, order) in orders(256) {
        let mut system = reordered_bending(&order);
        let vector: Vec<_> = order.iter().map(|&i| seed[i]).collect();
        let candidate = automatic_fit(&system, *value, &vector)
            .correct(&vector, 1e-8, |v| normalized_check(&system, *value, v))
            .unwrap();
        system.free_dofs = order.iter().map(|i| 3 * (i / 2 + 1) + 1 + i % 2).collect();
        let mut mass = vec![1.0; 3 * 129];
        for (&dof, &m) in system.free_dofs.iter().zip(&system.mass) {
            mass[dof] = m;
        }
        let (mut shape, _) =
            checked_mode_shape(&candidate, &mass, &system.free_dofs, mass.len()).unwrap();
        let reduced: Vec<_> = system.free_dofs.iter().map(|&i| shape[i]).collect();
        let mut matrix = system.operator.dense_fallback_matrix().unwrap();
        for (i, row) in matrix.iter_mut().enumerate() {
            for (a, &m) in row.iter_mut().zip(&system.mass) {
                *a /= m.sqrt().recip();
            }
            row[i] -= value * system.mass[i] * system.mass[i].sqrt().recip();
        }
        let fit = BlockFit::prepare_automatic(matrix, &reduced).unwrap();
        let mut candidate = fit
            .correct(&reduced, 1e-8, |v| physical_check(&system, *value, v))
            .unwrap_or_else(|error| panic!("automatic physical {name}: {error}"));
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
        assert_eq!(mode, restored);
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
        println!("automatic physical {name} actual={actual:e} independent={independent:e}");
        assert!(actual <= 1e-8 && independent <= 1e-8);
        assert!((actual / independent - 1.0).abs() < 1e-6);
    }
}

#[test]
fn automatic_partition_fit_reuses_its_factor_after_budget_failure_without_mutating_seed() {
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
            let fit = automatic_fit(&system, value.rounded(), &vector);
            let mut calls = 0;
            let error = fit
                .correct(&vector, 1e-30, |v| {
                    calls += 1;
                    normalized_check(&system, value.rounded(), v)
                })
                .unwrap_err();
            assert!(error.contains("did not reach its unchanged residual gate"));
            assert!(calls <= 80, "bounded private residual callbacks: {calls}");
            assert_eq!(vector, original);
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
            println!("automatic bounded checked_calls={calls}");
            Ok::<(), String>(())
        },
    )
    .unwrap();
    assert_eq!(factors.get(), 1);
}
