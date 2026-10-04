use super::*;

#[test]
fn connected_candidates_keep_heterogeneous_split_clusters_under_reassembly() {
    for segments in [16, 32] {
        let (base, _) = BendingReference::unit_inertia(segments).first_mode();
        for relative_spring in [1e-8, 1e-4, 1.0] {
            for detune in [0.0, 1e-4, 0.25] {
                let stiffness = 4.0 * (1.0 + detune);
                let spring = base.rounded() * relative_spring;
                let oracle = ConnectedReference::new(segments, stiffness, spring);
                let pairs = oracle.pairs();
                assert!(pairs[1].0 > pairs[0].0);
                for (name, order) in orders(4 * segments) {
                    let system = connected_system(segments, stiffness, spring, &order);
                    let mut accepted = Vec::new();
                    for (index, (value, vector)) in pairs.iter().enumerate() {
                        let mut vector: Vec<_> = order.iter().map(|&i| vector[i]).collect();
                        if index == 1 {
                            for (v, &first) in vector.iter_mut().zip(&accepted[0]) {
                                *v += 0.25 * first;
                            }
                        }
                        let original = vector.clone();
                        let guard = BasisGuard::prepare(order.len(), &accepted, 1e-10).unwrap();
                        let fit = automatic_fit(&system, *value, &vector);
                        let candidate = guard.correct(&fit, &vector, 1e-8, |v| normalized_check(&system, *value, v))
                            .unwrap_or_else(|error| panic!("connected segments={segments} spring={relative_spring:e} detune={detune:e} {name} mode={index}: {error}"));
                        let actual = normalized_check(&system, *value, &candidate).unwrap().0;
                        let independent =
                            oracle.residual(*value, &original_order(&candidate, &order), false);
                        println!(
                            "connected residual segments={segments} spring={relative_spring:e} detune={detune:e} {name} mode={index} actual={actual:e} independent={independent:e}"
                        );
                        assert!(
                            actual <= 1e-8 && independent <= 1e-8,
                            "segments={segments} spring={relative_spring:e} detune={detune:e} {name} mode={index}: actual={actual:e} independent={independent:e}"
                        );
                        assert!((actual / independent - 1.0).abs() < 1e-5);
                        assert_eq!(vector, original);
                        accepted.push(candidate);
                    }
                    let overlap = wide_overlap(&accepted[0], &accepted[1]);
                    let distance = subspace_distance(&accepted, &oracle.direction, &order);
                    println!(
                        "connected segments={segments} spring={relative_spring:e} detune={detune:e} {name} gap={:e} overlap={overlap:e} distance={distance:e}",
                        pairs[1].0 / pairs[0].0 - 1.0
                    );
                    assert!(overlap <= 1e-10 && distance < 1e-10);
                }
            }
        }
    }
}

#[test]
fn connected_reference_checks_assembled_products_and_resolved_first_cluster_roots() {
    let segments = 4;
    let (base, _) = BendingReference::unit_inertia(segments).first_mode();
    for relative_spring in [1e-8, 1e-4, 1.0] {
        for detune in [0.0, 0.25] {
            let stiffness = 4.0 * (1.0 + detune);
            let spring = base.rounded() * relative_spring;
            let oracle = ConnectedReference::new(segments, stiffness, spring);
            for (_, order) in orders(16) {
                let system = connected_system(segments, stiffness, spring, &order);
                let dense_matrix = system.operator.dense_fallback_matrix().unwrap();
                for (i, row) in dense_matrix.iter().enumerate() {
                    for (j, &a) in row.iter().enumerate() {
                        let (old_i, old_j) = (order[i], order[j]);
                        let d = |i| if i < oracle.width { 1.0 } else { 0.5 };
                        assert_eq!(a, oracle.coefficients[old_i][old_j] * d(old_i) * d(old_j));
                    }
                }
                let vector: Vec<_> = order.iter().map(|&i| (i as f64 + 0.5).sin()).collect();
                let applied = system.operator.apply_compensated(&vector).unwrap();
                let independent = oracle.apply(&original_order(&vector, &order), false);
                for (&a, &old) in applied.iter().zip(&order) {
                    assert!((a - independent[old].rounded()).abs() < 1e-13);
                }
                let dense =
                    jacobi_eigenpairs(system.operator.dense_fallback_matrix().unwrap()).unwrap();
                for ((value, _), (reference, _)) in dense.iter().take(2).zip(oracle.pairs()) {
                    assert!((value / reference - 1.0).abs() < 1e-10);
                }
            }
        }
    }
}

#[test]
fn connected_candidates_correct_perturbed_split_modes_without_losing_the_cluster() {
    let segments = 32;
    let (base, _) = BendingReference::unit_inertia(segments).first_mode();
    for (relative_spring, detune) in [(1e-8, 0.0), (1e-4, 1e-4), (1.0, 0.25)] {
        let stiffness = 4.0 * (1.0 + detune);
        let spring = base.rounded() * relative_spring;
        let oracle = ConnectedReference::new(segments, stiffness, spring);
        let pairs = oracle.pairs();
        for (name, order) in orders(128) {
            let system = connected_system(segments, stiffness, spring, &order);
            let first: Vec<_> = order.iter().map(|&i| pairs[0].1[i]).collect();
            let first = automatic_fit(&system, pairs[0].0, &first)
                .correct(&first, 1e-8, |v| normalized_check(&system, pairs[0].0, v))
                .unwrap();
            let prior = vec![first.clone()];
            let guard = BasisGuard::prepare(128, &prior, 1e-10).unwrap();
            let second: Vec<_> = order.iter().map(|&i| pairs[1].1[i]).collect();
            let mut seed: Vec<_> = second
                .iter()
                .zip(&first)
                .map(|(&b, &a)| b + 0.25 * a)
                .collect();
            seed[order.iter().position(|&i| i == 63).unwrap()] +=
                1e-11 * stable_l2_norm(second.iter().copied());
            let original = seed.clone();
            let value = pairs[1].0;
            let projected = guard.project_seed(&seed).unwrap();
            let before = normalized_check(&system, value, &projected).unwrap().0;
            assert!(before > 1e-8);
            let inverse = system.prepare_normalized_inverse().unwrap();
            let proposal = inverse_proposal(&system, &inverse, &guard, value, &seed).unwrap();
            let proposed = normalized_check(&system, value, &proposal).unwrap().0;
            let mut calls = 0;
            let candidate = guard.correct(&automatic_fit(&system, value, &proposal), &proposal, 1e-8, |v| {
                calls += 1;
                normalized_check(&system, value, v)
            }).unwrap_or_else(|error| panic!("perturbed connected spring={relative_spring:e} detune={detune:e} {name}: {error}"));
            let after = normalized_check(&system, value, &candidate).unwrap().0;
            let independent = oracle.residual(value, &original_order(&candidate, &order), false);
            let overlap = wide_overlap(&first, &candidate);
            let distance = subspace_distance(
                &[first.clone(), candidate.clone()],
                &oracle.direction,
                &order,
            );
            println!(
                "connected perturbed spring={relative_spring:e} detune={detune:e} {name} before={before:e} proposed={proposed:e} after={after:e} independent={independent:e} overlap={overlap:e} distance={distance:e} fit_calls={calls}"
            );
            assert!(after <= 1e-8 && independent <= 1e-8 && overlap <= 1e-10 && distance < 1e-10);
            assert!(proposed < before && (2..=81).contains(&calls));
            assert_eq!(seed, original);
            assert_eq!(prior, vec![first]);
        }
    }
}

#[test]
fn connected_candidates_recheck_physical_mass_subspaces_after_json_readback() {
    let segments = 32;
    let width = 2 * segments;
    let physical_width = 3 * (segments + 1);
    let (base, _) = BendingReference::unit_inertia(segments).first_mode();
    for (relative_spring, detune) in [(1e-8, 0.0), (1e-4, 1e-4), (1.0, 0.25)] {
        let stiffness = 4.0 * (1.0 + detune);
        let spring = base.rounded() * relative_spring;
        let oracle = ConnectedReference::new(segments, stiffness, spring);
        let pairs = oracle.pairs();
        for (name, order) in orders(128) {
            let mut system = connected_system(segments, stiffness, spring, &order);
            system.free_dofs = order
                .iter()
                .map(|&i| i / width * physical_width + 3 * (i % width / 2 + 1) + 1 + i % 2)
                .collect();
            let mut mass = vec![1.0; 2 * physical_width];
            for (&dof, &m) in system.free_dofs.iter().zip(&system.mass) {
                mass[dof] = m;
            }
            let mut accepted = Vec::new();
            let mut modes = Vec::new();
            for (index, (value, vector)) in pairs.iter().enumerate() {
                let vector: Vec<_> = order.iter().map(|&i| vector[i]).collect();
                let guard = BasisGuard::prepare(128, &accepted, 1e-10).unwrap();
                let candidate = guard
                    .correct(
                        &automatic_fit(&system, *value, &vector),
                        &vector,
                        1e-8,
                        |v| normalized_check(&system, *value, v),
                    )
                    .unwrap();
                let (shape, norm) =
                    checked_mode_shape(&candidate, &mass, &system.free_dofs, mass.len()).unwrap();
                let frequency = value.sqrt();
                modes.push(ModalFrame2dModeResult {
                    index,
                    eigenvalue_rad_s_squared: *value,
                    natural_frequency_rad_s: frequency,
                    natural_frequency_hz: frequency / std::f64::consts::TAU,
                    period_s: std::f64::consts::TAU / frequency,
                    participation_norm: norm,
                    shape,
                });
                accepted.push(candidate);
            }
            let restored: Vec<ModalFrame2dModeResult> =
                serde_json::from_slice(&serde_json::to_vec(&modes).unwrap()).unwrap();
            assert_eq!(restored, modes);
            let mut whitened = Vec::new();
            for (index, mode) in restored.iter().enumerate() {
                assert_eq!(mode.index, index);
                assert_eq!(
                    mode.eigenvalue_rad_s_squared.to_bits(),
                    pairs[index].0.to_bits()
                );
                assert!(
                    mode.shape
                        .iter()
                        .zip(&modes[index].shape)
                        .all(|(a, b)| a.to_bits() == b.to_bits())
                );
                assert!((mode.participation_norm - 1.0).abs() < 1e-10);
                assert!((checked_shape_norm(&mode.shape).unwrap() - 1.0).abs() < 1e-10);
                assert!((mode.natural_frequency_hz * mode.period_s - 1.0).abs() < 1e-12);
                assert!(mode.shape.chunks_exact(3).all(|node| node[0] == 0.0));
                assert!(
                    mode.shape
                        .chunks_exact(physical_width)
                        .all(|block| block[..3].iter().all(|v| *v == 0.0))
                );
                let reduced: Vec<_> = system.free_dofs.iter().map(|&i| mode.shape[i]).collect();
                let actual = physical_check(&system, pairs[index].0, &reduced).unwrap().0;
                let independent =
                    oracle.residual(pairs[index].0, &original_order(&reduced, &order), true);
                println!(
                    "connected physical spring={relative_spring:e} detune={detune:e} {name} mode={index} actual={actual:e} independent={independent:e}"
                );
                assert!(actual <= 1e-8 && independent <= 1e-8);
                assert!((actual / independent - 1.0).abs() < 1e-5);
                whitened.push(
                    reduced
                        .iter()
                        .zip(&system.mass)
                        .map(|(&v, &m)| v * m.sqrt())
                        .collect::<Vec<_>>(),
                );
            }
            let overlap = physical_mass_overlap(&restored[0].shape, &restored[1].shape, &mass);
            let distance = subspace_distance(&whitened, &oracle.direction, &order);
            println!(
                "connected physical subspace spring={relative_spring:e} detune={detune:e} {name} overlap={overlap:e} distance={distance:e}"
            );
            assert!(overlap <= 1e-10 && distance < 1e-10);
        }
    }
}

#[test]
fn connected_candidates_distinguish_cluster_membership_from_unique_modes_and_duplicates() {
    let segments = 32;
    let (base, _) = BendingReference::unit_inertia(segments).first_mode();
    let spring = base.rounded() * 1e-10;
    let oracle = ConnectedReference::new(segments, 4.0, spring);
    let pairs = oracle.pairs();
    assert!(pairs[1].0 / pairs[0].0 - 1.0 < 1e-8);
    let order = orders(128).pop().unwrap().1;
    let system = connected_system(segments, 4.0, spring, &order);
    let mut accepted = Vec::new();
    for (index, weights) in [[0.6, 0.8], [-0.8, 0.6]].into_iter().enumerate() {
        let vector: Vec<_> = order
            .iter()
            .map(|&i| weights[0] * pairs[0].1[i] + weights[1] * pairs[1].1[i])
            .collect();
        let guard = BasisGuard::prepare(128, &accepted, 1e-10).unwrap();
        let value = pairs[index].0;
        let candidate = guard
            .correct(
                &automatic_fit(&system, value, &vector),
                &vector,
                1e-8,
                |v| normalized_check(&system, value, v),
            )
            .unwrap();
        let independent = oracle.residual(value, &original_order(&candidate, &order), false);
        assert!(independent <= 1e-8);
        accepted.push(candidate);
    }
    let direction: Vec<_> = order.iter().map(|&i| pairs[0].1[i]).collect();
    let alignment = wide_overlap(&accepted[0], &direction);
    let distance = subspace_distance(&accepted, &oracle.direction, &order);
    println!(
        "connected unresolved gap={:e} alignment={alignment:e} distance={distance:e}",
        pairs[1].0 / pairs[0].0 - 1.0
    );
    assert!(alignment > 0.5 && alignment < 0.7 && distance < 1e-10);
    assert!(
        normalized_check(&system, pairs[1].0, &accepted[0])
            .unwrap()
            .0
            <= 1e-8
    );
    let guard = BasisGuard::prepare(128, &accepted[..1], 1e-10).unwrap();
    let fit = automatic_fit(&system, pairs[1].0, &accepted[0]);
    let mut calls = 0;
    let error = guard
        .correct(&fit, &accepted[0], 1e-8, |v| {
            calls += 1;
            normalized_check(&system, pairs[1].0, v)
        })
        .unwrap_err();
    assert!(error.contains("lost an independent direction"), "{error}");
    assert_eq!(calls, 0);
}

#[test]
fn connected_candidates_reject_coordinate_only_stalls_and_replay_with_one_inverse_proposal() {
    let segments = 32;
    let (base, _) = BendingReference::unit_inertia(segments).first_mode();
    let spring = base.rounded() * 1e-8;
    let oracle = ConnectedReference::new(segments, 4.0, spring);
    let pairs = oracle.pairs();
    let order = orders(128).pop().unwrap().1;
    let system = connected_system(segments, 4.0, spring, &order);
    let first: Vec<_> = order.iter().map(|&i| pairs[0].1[i]).collect();
    let previous = vec![first.clone()];
    let guard = BasisGuard::prepare(128, &previous, 1e-10).unwrap();
    let second: Vec<_> = order.iter().map(|&i| pairs[1].1[i]).collect();
    let mut seed: Vec<_> = second
        .iter()
        .zip(&first)
        .map(|(&b, &a)| b + 0.25 * a)
        .collect();
    seed[order.iter().position(|&i| i == 63).unwrap()] +=
        1e-11 * stable_l2_norm(second.iter().copied());
    let original = seed.clone();
    let value = pairs[1].0;
    let fit = automatic_fit(&system, value, &seed);
    let mut calls = 0;
    let error = guard
        .correct(&fit, &seed, 1e-8, |v| {
            calls += 1;
            normalized_check(&system, value, v)
        })
        .unwrap_err();
    assert!(
        error.contains("did not reach its unchanged residual gate"),
        "{error}"
    );
    assert!(calls <= 81);
    println!("connected coordinate-only failure calls={calls} error={error}");
    let factors = Rc::new(Cell::new([0; 2]));
    let observed = factors.clone();
    let inverse = with_solver_observer(
        &SolverControl::default(),
        move |point| {
            record_inverse_preparation(&observed, point);
        },
        || system.prepare_normalized_inverse(),
    )
    .unwrap();
    let proposal = inverse_proposal(&system, &inverse, &guard, value, &seed).unwrap();
    for (stage, step) in [
        (SolverStage::DenseSubstitution, 64),
        (SolverStage::ModalValidation, 2),
    ] {
        let control = SolverControl::default();
        let cancel = control.clone();
        let observed = factors.clone();
        let error = with_solver_observer(
            &control,
            move |point| {
                record_inverse_preparation(&observed, point);
                if point.stage == stage && point.completed_steps == step {
                    cancel.request_cancel();
                }
            },
            || inverse_proposal(&system, &inverse, &guard, value, &seed),
        )
        .unwrap_err();
        assert!(error.contains("cancel"), "{error}");
        let last = control.last_checkpoint().unwrap();
        assert_eq!(last.stage, stage);
        assert_eq!(last.completed_steps, step);
        assert_eq!(seed, original);
        assert_eq!(previous, vec![first.clone()]);
        let observed = factors.clone();
        let replay = with_solver_observer(
            &SolverControl::default(),
            move |point| {
                record_inverse_preparation(&observed, point);
            },
            || inverse_proposal(&system, &inverse, &guard, value, &seed),
        )
        .unwrap();
        assert_eq!(proposal, replay);
    }
    let error = guard
        .correct(&fit, &proposal, 1e-30, |v| {
            normalized_check(&system, value, v)
        })
        .unwrap_err();
    assert!(
        error.contains("did not reach its unchanged residual gate"),
        "{error}"
    );
    let candidate = guard
        .correct(&fit, &proposal, 1e-8, |v| {
            normalized_check(&system, value, v)
        })
        .unwrap();
    assert!(oracle.residual(value, &original_order(&candidate, &order), false) <= 1e-8);
    assert!(wide_overlap(&first, &candidate) <= 1e-10);
    assert_eq!(factors.get(), [1, 1]);
    assert_eq!(seed, original);
    assert_eq!(previous, vec![first]);
}

fn record_inverse_preparation(
    counts: &Cell<[usize; 2]>,
    point: crate::solver_control::SolverCheckpoint,
) {
    if point.completed_steps == 0 {
        let mut next = counts.get();
        match point.stage {
            SolverStage::LinearPrepare => next[0] += 1,
            SolverStage::DenseFactor => next[1] += 1,
            _ => return,
        }
        counts.set(next);
    }
}
