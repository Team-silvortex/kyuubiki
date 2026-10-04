use super::*;

#[test]
fn irregular_wide_spectrum_checks_known_roots_and_rejects_unfinished_work() {
    let matrix: Vec<Vec<_>> = [[2.0, -1.0, 0.0], [-1.0, 2.0, -1.0], [0.0, -1.0, 2.0]]
        .iter()
        .map(|row| row.iter().map(|&v| Wide::from(v)).collect())
        .collect();
    let root = wide_sqrt(Wide::from(2.0));
    let expected = [
        Wide::from(2.0).sub(root),
        Wide::from(2.0),
        Wide::from(2.0).add(root),
    ];
    for ((value, vector), reference) in spectrum(&matrix, 40).unwrap().iter().zip(expected) {
        assert!(value.sub(reference).rounded().abs() < 1e-27);
        check_wide_pair(&matrix, *value, vector);
    }
    assert!(spectrum(&matrix, 0).unwrap_err().contains("exhausted"));
    assert!(spectrum(&matrix, 41).unwrap_err().contains("budget"));
    let mut invalid = matrix.clone();
    invalid[0][1] = Wide::from(0.0);
    assert!(spectrum(&invalid, 40).unwrap_err().contains("symmetric"));
    assert!(
        inertia(&invalid, Wide::from(1.0))
            .unwrap_err()
            .contains("symmetric")
    );
    assert!(spectrum(&[], 40).unwrap_err().contains("bounded"));
    let diagonal = vec![
        vec![Wide::from(1.0), Wide::default()],
        vec![Wide::default(), Wide::from(2.0)],
    ];
    assert_eq!(spectrum(&diagonal, 0).unwrap().len(), 2);
    assert!(
        inertia(&diagonal, Wide::from(1.0))
            .unwrap_err()
            .contains("pivot")
    );
    invalid = diagonal;
    invalid[1][1] = Wide::from(-1.0);
    assert!(spectrum(&invalid, 0).unwrap_err().contains("positive"));
}

#[test]
fn irregular_reference_checks_reassembled_entries_products_and_four_root_inertia_brackets() {
    for segments in [8, 16] {
        for spring in [1e-4, 0.1, 1.0] {
            let oracle = IrregularReference::new(segments, spring);
            let normalized = oracle.normalized();
            let pairs = oracle.pairs();
            assert_eq!(pairs.len(), oracle.mass.len());
            let maximum = pairs
                .iter()
                .map(|(value, vector)| check_wide_pair(&normalized, *value, vector))
                .fold(0.0_f64, f64::max);
            for (index, (value, _)) in pairs.iter().take(4).enumerate() {
                let lo = value.mul(Wide::from(1.0 - 1e-9));
                let hi = value.mul(Wide::from(1.0 + 1e-9));
                assert_eq!(inertia(&normalized, lo).unwrap(), index);
                assert_eq!(inertia(&normalized, hi).unwrap(), index + 1);
            }
            for (name, order) in orders(oracle.mass.len()) {
                let system = oracle.system(&order);
                let matrix = system.operator.dense_fallback_matrix().unwrap();
                for (i, row) in matrix.iter().enumerate() {
                    for (j, &a) in row.iter().enumerate() {
                        assert_eq!(a, normalized[order[i]][order[j]].rounded());
                    }
                }
                for &(a, b, _) in &oracle.links {
                    assert!(a / (2 * segments) != b / (2 * segments));
                    assert!(normalized[a][b].rounded() < 0.0);
                }
                let vector: Vec<_> = order.iter().map(|&i| (i as f64 + 0.25).cos()).collect();
                let applied = system.operator.apply_compensated(&vector).unwrap();
                let expected =
                    apply_wide(&normalized, &wide_vector(&original_order(&vector, &order)));
                for (&a, &old) in applied.iter().zip(&order) {
                    assert!((a - expected[old].rounded()).abs() < 1e-13, "{name}");
                }
            }
            println!(
                "irregular oracle segments={segments} spring={spring:e} wide_residual={maximum:e} roots={:?}",
                pairs[..4].iter().map(|p| p.0.rounded()).collect::<Vec<_>>()
            );
        }
    }
}

#[test]
fn irregular_candidates_admit_four_production_seed_modes_under_nonseparable_reassembly() {
    for segments in [8, 16] {
        for spring in [1e-4, 0.1, 1.0] {
            let oracle = IrregularReference::new(segments, spring);
            let references = oracle.pairs();
            for (name, order) in orders(oracle.mass.len()) {
                let system = oracle.system(&order);
                let pairs = production_candidates(&system);
                let accepted = admit_pairs(&system, &pairs);
                check_cluster(
                    &oracle,
                    &system,
                    &order,
                    &accepted,
                    &references,
                    name,
                    "production",
                );
            }
        }
    }
}

#[test]
fn irregular_candidates_correct_four_perturbed_modes_with_one_prepared_inverse() {
    let segments = 16;
    for spring in [1e-4, 0.1] {
        let oracle = IrregularReference::new(segments, spring);
        let references = oracle.pairs();
        for (name, order) in orders(oracle.mass.len()) {
            let system = oracle.system(&order);
            let inverse = system.prepare_normalized_inverse().unwrap();
            let mut accepted: Vec<(f64, Vec<f64>)> = Vec::new();
            for (index, (value, reference)) in references.iter().take(4).enumerate() {
                let value = value.rounded();
                let previous: Vec<_> = accepted.iter().map(|p| p.1.clone()).collect();
                let guard = BasisGuard::prepare(order.len(), &previous, 1e-10).unwrap();
                let mut seed: Vec<_> = order.iter().map(|&i| reference[i].rounded()).collect();
                for prior in &previous {
                    for (v, &p) in seed.iter_mut().zip(prior) {
                        *v += 0.125 * p;
                    }
                }
                seed[order.iter().position(|&i| i == 31).unwrap()] += 1e-10;
                let unchanged = seed.clone();
                let projected = guard.project_seed(&seed).unwrap();
                let before = normalized_check(&system, value, &projected).unwrap().0;
                assert!(before > 1e-8, "mode={index} {name}: {before:e}");
                let proposal = inverse_proposal(&system, &inverse, &guard, value, &seed).unwrap();
                let proposed = normalized_check(&system, value, &proposal).unwrap().0;
                let mut calls = 0;
                let candidate = guard
                    .correct(
                        &automatic_fit(&system, value, &proposal),
                        &proposal,
                        1e-8,
                        |v| {
                            calls += 1;
                            normalized_check(&system, value, v)
                        },
                    )
                    .unwrap_or_else(|e| panic!("spring={spring:e} {name} mode={index}: {e}"));
                let after = normalized_check(&system, value, &candidate).unwrap().0;
                println!(
                    "irregular perturb spring={spring:e} {name} mode={index} before={before:e} proposed={proposed:e} after={after:e} fit_calls={calls}"
                );
                assert!(proposed < before && (2..=81).contains(&calls));
                assert_eq!(seed, unchanged);
                assert_eq!(
                    previous,
                    accepted.iter().map(|p| p.1.clone()).collect::<Vec<_>>()
                );
                accepted.push((value, candidate));
            }
            check_cluster(
                &oracle,
                &system,
                &order,
                &accepted,
                &references,
                name,
                "perturbed",
            );
        }
    }
}

#[test]
fn irregular_candidates_recheck_four_physical_modes_after_json_readback() {
    let segments = 16;
    let width = 2 * segments;
    let physical_width = 3 * (segments + 1);
    for spring in [1e-4, 1.0] {
        let oracle = IrregularReference::new(segments, spring);
        let references = oracle.pairs();
        for (name, order) in orders(oracle.mass.len()) {
            let mut system = oracle.system(&order);
            system.free_dofs = order
                .iter()
                .map(|&i| i / width * physical_width + 3 * (i % width / 2 + 1) + 1 + i % 2)
                .collect();
            let mut mass = vec![1.0; 3 * physical_width];
            for (&dof, &m) in system.free_dofs.iter().zip(&system.mass) {
                mass[dof] = m;
            }
            let accepted = admit_pairs(&system, &production_candidates(&system));
            let modes: Vec<_> = accepted
                .iter()
                .enumerate()
                .map(|(index, (value, vector))| {
                    let (shape, norm) =
                        checked_mode_shape(vector, &mass, &system.free_dofs, mass.len()).unwrap();
                    let frequency = value.sqrt();
                    ModalFrame2dModeResult {
                        index,
                        eigenvalue_rad_s_squared: *value,
                        natural_frequency_rad_s: frequency,
                        natural_frequency_hz: frequency / std::f64::consts::TAU,
                        period_s: std::f64::consts::TAU / frequency,
                        participation_norm: norm,
                        shape,
                    }
                })
                .collect();
            let restored: Vec<ModalFrame2dModeResult> =
                serde_json::from_slice(&serde_json::to_vec(&modes).unwrap()).unwrap();
            assert_eq!(restored, modes);
            let mut whitened = Vec::new();
            for (index, mode) in restored.iter().enumerate() {
                assert_eq!(mode.index, index);
                assert_eq!(
                    mode.eigenvalue_rad_s_squared.to_bits(),
                    accepted[index].0.to_bits()
                );
                assert!(
                    (mode.eigenvalue_rad_s_squared / references[index].0.rounded() - 1.0).abs()
                        < 1e-8
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
                assert!(mode.shape.chunks_exact(3).all(|n| n[0] == 0.0));
                assert!(
                    mode.shape
                        .chunks_exact(physical_width)
                        .all(|b| b[..3].iter().all(|v| *v == 0.0))
                );
                let reduced: Vec<_> = system.free_dofs.iter().map(|&i| mode.shape[i]).collect();
                let actual = physical_check(&system, accepted[index].0, &reduced)
                    .unwrap()
                    .0;
                let independent =
                    oracle.residual(accepted[index].0, &original_order(&reduced, &order), true);
                assert!(actual <= 1e-8 && independent <= 1e-8);
                assert!((actual - independent).abs() < 1e-11);
                println!(
                    "irregular physical spring={spring:e} {name} mode={index} actual={actual:e} independent={independent:e}"
                );
                whitened.push(original_order(
                    &reduced
                        .iter()
                        .zip(&system.mass)
                        .map(|(&v, &m)| v * m.sqrt())
                        .collect::<Vec<_>>(),
                    &order,
                ));
            }
            for i in 0..4 {
                for j in 0..i {
                    assert!(
                        physical_mass_overlap(&restored[i].shape, &restored[j].shape, &mass)
                            <= 1e-10
                    );
                }
            }
            let reference: Vec<_> = references.iter().take(4).map(|p| p.1.clone()).collect();
            let distance = projector_distance(&whitened, &reference);
            println!("irregular physical projector spring={spring:e} {name} distance={distance:e}");
            assert!(distance < 1e-10);
        }
    }
}

#[test]
fn irregular_four_mode_admission_rejects_duplicates_and_misassigned_roots() {
    let oracle = IrregularReference::new(16, 0.1);
    let order = orders(96).pop().unwrap().1;
    let system = oracle.system(&order);
    let accepted = admit_pairs(&system, &production_candidates(&system));
    let previous: Vec<_> = accepted.iter().map(|p| p.1.clone()).collect();
    let unchanged = previous.clone();
    let guard = BasisGuard::prepare(96, &previous, 1e-10).unwrap();
    for (value, vector) in &accepted {
        assert!(normalized_check(&system, *value, vector).unwrap().0 <= 1e-8);
        let mut calls = 0;
        let error = guard
            .correct(&automatic_fit(&system, *value, vector), vector, 1e-8, |v| {
                calls += 1;
                normalized_check(&system, *value, v)
            })
            .unwrap_err();
        assert!(error.contains("lost an independent direction"), "{error}");
        assert_eq!(calls, 0);
    }
    let guard = BasisGuard::prepare(96, &previous[..3], 1e-10).unwrap();
    let (wrong, seed) = (accepted[0].0, &accepted[3].1);
    let original = seed.clone();
    let before = normalized_check(&system, wrong, seed).unwrap().0;
    assert!(before > 0.9);
    let mut calls = 0;
    let error = guard
        .correct(&automatic_fit(&system, wrong, seed), seed, 1e-8, |v| {
            calls += 1;
            normalized_check(&system, wrong, v)
        })
        .unwrap_err();
    assert!(calls <= 81);
    println!("irregular wrong root before={before:e} calls={calls} error={error}");
    assert_eq!(seed, &original);
    assert_eq!(previous, unchanged);
}

#[test]
fn irregular_fourth_candidate_cancels_against_three_prior_directions_and_replays() {
    let oracle = IrregularReference::new(16, 0.1);
    let order = orders(96).pop().unwrap().1;
    let system = oracle.system(&order);
    let references = oracle.pairs();
    let accepted = admit_pairs(&system, &production_candidates(&system));
    let previous: Vec<_> = accepted[..3].iter().map(|p| p.1.clone()).collect();
    let original_basis = previous.clone();
    let guard = BasisGuard::prepare(96, &previous, 1e-10).unwrap();
    let value = references[3].0.rounded();
    let mut seed: Vec<_> = order
        .iter()
        .map(|&i| references[3].1[i].rounded())
        .collect();
    for prior in &previous {
        for (v, &p) in seed.iter_mut().zip(prior) {
            *v += 0.125 * p;
        }
    }
    seed[order.iter().position(|&i| i == 31).unwrap()] += 1e-10;
    let original = seed.clone();
    let inverse = system.prepare_normalized_inverse().unwrap();
    let proposal = inverse_proposal(&system, &inverse, &guard, value, &seed).unwrap();
    for (stage, step, update_pass) in [
        (SolverStage::ModalVectorUpdate, 64, 5),
        (SolverStage::DenseSubstitution, 64, 0),
        (SolverStage::ModalValidation, 4, 0),
    ] {
        let control = SolverControl::default();
        let cancel = control.clone();
        let updates = Cell::new(0);
        let error = with_solver_observer(
            &control,
            move |point| {
                if point.stage == SolverStage::ModalVectorUpdate && point.completed_steps == 0 {
                    updates.set(updates.get() + 1);
                }
                if point.stage == stage
                    && point.completed_steps == step
                    && (update_pass == 0 || updates.get() == update_pass)
                {
                    cancel.request_cancel();
                }
            },
            || inverse_proposal(&system, &inverse, &guard, value, &seed),
        )
        .unwrap_err();
        assert!(error.contains("cancel"), "{error}");
        let last = control.last_checkpoint().unwrap();
        assert_eq!((last.stage, last.completed_steps), (stage, step));
        let replay = with_solver_observer(
            &SolverControl::default(),
            |_| {},
            || inverse_proposal(&system, &inverse, &guard, value, &seed),
        )
        .unwrap();
        assert_eq!(proposal, replay);
        assert!(
            proposal
                .iter()
                .zip(&replay)
                .all(|(a, b)| a.to_bits() == b.to_bits())
        );
        assert_eq!(seed, original);
        assert_eq!(previous, original_basis);
    }
    let result = guard
        .correct(
            &automatic_fit(&system, value, &proposal),
            &proposal,
            1e-8,
            |v| normalized_check(&system, value, v),
        )
        .unwrap();
    assert!(oracle.residual(value, &original_order(&result, &order), false) <= 1e-8);
    for prior in &previous {
        assert!(wide_overlap(&result, prior) <= 1e-10);
    }
}

fn production_candidates(system: &ReducedSparseModalSystem) -> Vec<(f64, Vec<f64>)> {
    jacobi_eigenpairs(system.operator.dense_fallback_matrix().unwrap())
        .unwrap()
        .into_iter()
        .take(4)
        .collect()
}

fn admit_pairs(
    system: &ReducedSparseModalSystem,
    pairs: &[(f64, Vec<f64>)],
) -> Vec<(f64, Vec<f64>)> {
    let mut accepted: Vec<(f64, Vec<f64>)> = Vec::new();
    for (value, vector) in pairs {
        let previous: Vec<_> = accepted.iter().map(|p| p.1.clone()).collect();
        let guard = BasisGuard::prepare(vector.len(), &previous, 1e-10).unwrap();
        let candidate = guard
            .correct(&automatic_fit(system, *value, vector), vector, 1e-8, |v| {
                normalized_check(system, *value, v)
            })
            .unwrap();
        accepted.push((*value, candidate));
    }
    accepted
}

fn check_wide_pair(matrix: &[Vec<Wide>], value: Wide, vector: &[Wide]) -> f64 {
    let applied = apply_wide(matrix, vector);
    let relative = wide_norm(
        applied
            .iter()
            .zip(vector)
            .map(|(&a, &v)| a.sub(value.mul(v))),
    ) / wide_norm(applied.iter().copied());
    assert!(relative < 1e-22, "wide reference residual={relative:e}");
    relative
}

fn check_cluster(
    oracle: &IrregularReference,
    system: &ReducedSparseModalSystem,
    order: &[usize],
    accepted: &[(f64, Vec<f64>)],
    references: &[(Wide, Vec<Wide>)],
    name: &str,
    kind: &str,
) {
    let mut original = Vec::new();
    for (index, (value, vector)) in accepted.iter().enumerate() {
        assert!((value / references[index].0.rounded() - 1.0).abs() < 1e-8);
        let actual = normalized_check(system, *value, vector).unwrap().0;
        let reordered = original_order(vector, order);
        let independent = oracle.residual(*value, &reordered, false);
        println!(
            "irregular internal kind={kind} segments={} {name} mode={index} actual={actual:e} independent={independent:e}",
            oracle.segments
        );
        assert!(actual <= 1e-8 && independent <= 1e-8);
        assert!((actual - independent).abs() < 1e-11);
        for (_, prior) in &accepted[..index] {
            assert!(wide_overlap(vector, prior) <= 1e-10);
        }
        original.push(reordered);
    }
    let reference: Vec<_> = references.iter().take(4).map(|p| p.1.clone()).collect();
    let distance = projector_distance(&original, &reference);
    println!(
        "irregular projector kind={kind} segments={} {name} distance={distance:e}",
        oracle.segments
    );
    assert!(distance < 1e-10, "{distance:e}");
}
