use super::*;

fn repeated_bending(segments: usize, order: &[usize]) -> ReducedSparseModalSystem {
    let width = 2 * segments;
    assert_eq!(order.len(), 2 * width);
    let mut inverse = vec![usize::MAX; order.len()];
    for (new, &old) in order.iter().enumerate() {
        assert!(old < order.len() && inverse[old] == usize::MAX);
        inverse[old] = new;
    }
    let mut matrix = SparseMatrix::new(order.len());
    let element = [
        [12.0, 6.0, -12.0, 6.0],
        [6.0, 4.0, -6.0, 2.0],
        [-12.0, -6.0, 12.0, -6.0],
        [6.0, 2.0, -6.0, 4.0],
    ];
    for block in 0..2 {
        for segment in 0..segments {
            for (row, values) in element.iter().enumerate() {
                for (column, &value) in values.iter().enumerate() {
                    let (row, column) = (2 * segment + row, 2 * segment + column);
                    if row >= 2 && column >= 2 {
                        add_at(
                            &mut matrix,
                            inverse[block * width + row - 2],
                            inverse[block * width + column - 2],
                            value,
                        );
                    }
                }
            }
        }
    }
    let mass: Vec<_> = order
        .iter()
        .map(|i| {
            let i = i % width;
            (if i % 2 == 0 { 1.0 } else { 1.0 / 12.0 }) * if i >= width - 2 { 0.5 } else { 1.0 }
        })
        .collect();
    reduce_sparse_modal_system(&matrix, &mass, &[]).unwrap()
}

fn rotated(reference: &[Wide], order: &[usize], coefficients: [f64; 2]) -> Vec<f64> {
    let width = reference.len();
    order
        .iter()
        .map(|i| reference[i % width].rounded() * coefficients[i / width])
        .collect()
}

pub(super) fn wide_overlap(left: &[f64], right: &[f64]) -> f64 {
    let dot = |left: &[f64], right: &[f64]| {
        left.iter()
            .zip(right)
            .fold(Wide::default(), |sum, (&a, &b)| {
                sum.add(Wide::from(a).mul(Wide::from(b)))
            })
    };
    dot(left, right).rounded().abs()
        / dot(left, left).rounded().sqrt()
        / dot(right, right).rounded().sqrt()
}

pub(super) fn subspace_distance(vectors: &[Vec<f64>], reference: &[Wide], order: &[usize]) -> f64 {
    assert_eq!(vectors.len(), 2);
    let width = reference.len();
    let unit = |vector: Vec<Wide>| {
        let norm = vector
            .iter()
            .fold(Wide::default(), |sum, &v| sum.add(v.mul(v)))
            .rounded()
            .sqrt();
        vector
            .into_iter()
            .map(|v| v.div(Wide::from(norm)))
            .collect::<Vec<_>>()
    };
    let actual: Vec<_> = vectors
        .iter()
        .map(|v| unit(wide_vector(&original_order(v, order))))
        .collect();
    let reference = unit(reference.to_vec());
    let mut sum = Wide::default();
    for i in 0..2 * width {
        for j in 0..2 * width {
            let actual = actual
                .iter()
                .fold(Wide::default(), |sum, v| sum.add(v[i].mul(v[j])));
            let expected = if i / width == j / width {
                reference[i % width].mul(reference[j % width])
            } else {
                Wide::default()
            };
            let delta = actual.sub(expected);
            sum = sum.add(delta.mul(delta));
        }
    }
    sum.rounded().sqrt()
}

fn check_blocks(oracle: &BendingReference, value: f64, vector: &[f64], order: &[usize]) {
    let original = original_order(vector, order);
    for block in original.chunks_exact(original.len() / 2) {
        if block.iter().any(|v| *v != 0.0) {
            let independent = oracle.residual(Wide::from(value), &wide_vector(block));
            assert!(
                independent <= 1e-8,
                "independent block residual={independent:e}"
            );
        }
    }
}

#[test]
fn subspace_joint_admission_keeps_rotated_repeated_modes_under_reassembly() {
    for segments in [32, 64] {
        let oracle = BendingReference::new(segments);
        let (value, reference) = oracle.first_mode();
        let value = value.rounded();
        for (name, order) in orders(4 * segments) {
            let system = repeated_bending(segments, &order);
            for rotation in [[1.0, 0.0], [0.6, 0.8], [0.8, -0.6]] {
                let first = rotated(&reference, &order, rotation);
                let first = automatic_fit(&system, value, &first)
                    .correct(&first, 1e-8, |v| normalized_check(&system, value, v))
                    .unwrap();
                let previous = vec![first.clone()];
                let unchanged = previous.clone();
                let guard = BasisGuard::prepare(order.len(), &previous, 1e-10).unwrap();
                let second = rotated(&reference, &order, [-rotation[1], rotation[0]]);
                for (exponent, sign) in [(-80, 1.0), (0, 1.0), (0, -1.0), (80, 1.0)] {
                    let vector: Vec<_> = second
                        .iter()
                        .zip(&first)
                        .map(|(&v, &prior)| (v + 0.25 * prior) * sign * 2.0_f64.powi(exponent))
                        .collect();
                    let original = vector.clone();
                    let candidate = guard
                        .correct(&automatic_fit(&system, value, &vector), &vector, 1e-8, |v| {
                            normalized_check(&system, value, v)
                        })
                        .unwrap_or_else(|error| {
                            panic!("subspace segments={segments} {name} rotation={rotation:?} power={exponent}: {error}")
                        });
                    let residual = normalized_check(&system, value, &candidate).unwrap().0;
                    let overlap = wide_overlap(&first, &candidate);
                    let distance =
                        subspace_distance(&[first.clone(), candidate.clone()], &reference, &order);
                    println!(
                        "subspace segments={segments} {name} rotation={rotation:?} power={exponent} sign={sign} residual={residual:e} overlap={overlap:e} distance={distance:e}"
                    );
                    assert!(residual <= 1e-8 && overlap <= 1e-10 && distance < 1e-10);
                    check_blocks(&oracle, value, &first, &order);
                    check_blocks(&oracle, value, &candidate, &order);
                    assert_eq!(vector, original);
                    assert_eq!(previous, unchanged);
                }
            }
        }
    }
}

#[test]
fn subspace_joint_admission_rejects_residual_qualified_duplicate_modes() {
    let order = orders(256).pop().unwrap().1;
    let system = repeated_bending(64, &order);
    let oracle = BendingReference::new(64);
    let (value, reference) = oracle.first_mode();
    let value = value.rounded();
    let vector = rotated(&reference, &order, [0.6, 0.8]);
    let vector = automatic_fit(&system, value, &vector)
        .correct(&vector, 1e-8, |v| normalized_check(&system, value, v))
        .unwrap();
    assert!(normalized_check(&system, value, &vector).unwrap().0 <= 1e-8);
    assert!(subspace_distance(&[vector.clone(), vector.clone()], &reference, &order) > 1.0);
    let previous = vec![vector.clone()];
    let guard = BasisGuard::prepare(vector.len(), &previous, 1e-10).unwrap();
    let fit = automatic_fit(&system, value, &vector);
    for scale in [1.0, -1.0, 2.0_f64.powi(80), 2.0_f64.powi(-80)] {
        let seed: Vec<_> = vector.iter().map(|v| v * scale).collect();
        let original = seed.clone();
        let mut calls = 0;
        let error = guard
            .correct(&fit, &seed, 1e-8, |v| {
                calls += 1;
                normalized_check(&system, value, v)
            })
            .unwrap_err();
        assert!(error.contains("lost an independent direction"), "{error}");
        assert_eq!(calls, 0, "independence precedes the residual fast path");
        assert_eq!(seed, original);
        assert_eq!(previous, vec![vector.clone()]);
    }
}

#[test]
fn subspace_joint_admission_corrects_perturbed_repeated_modes_with_fresh_residuals() {
    let oracle = BendingReference::new(64);
    let (value, reference) = oracle.first_mode();
    let value = value.rounded();
    for (name, order) in orders(256) {
        let system = repeated_bending(64, &order);
        let first = rotated(&reference, &order, [0.6, 0.8]);
        let first = automatic_fit(&system, value, &first)
            .correct(&first, 1e-8, |v| normalized_check(&system, value, v))
            .unwrap();
        let guard = BasisGuard::prepare(256, &[first.clone()], 1e-10).unwrap();
        let second = rotated(&reference, &order, [-0.8, 0.6]);
        let scale = stable_l2_norm(second.iter().copied());
        let coordinate = order.iter().position(|&i| i == 127).unwrap();
        for noise in [1e-13, 1e-11] {
            let mut vector: Vec<_> = second
                .iter()
                .zip(&first)
                .map(|(&b, &a)| b + 0.25 * a)
                .collect();
            vector[coordinate] += noise * scale;
            let original = vector.clone();
            let projected = guard.project_seed(&vector).unwrap();
            let before = normalized_check(&system, value, &projected).unwrap().0;
            assert!(
                before > 1e-8,
                "must exercise correction, not the residual fast path"
            );
            let mut calls = 0;
            let candidate = guard
                .correct(
                    &automatic_fit(&system, value, &vector),
                    &vector,
                    1e-8,
                    |v| {
                        calls += 1;
                        normalized_check(&system, value, v)
                    },
                )
                .unwrap_or_else(|error| panic!("perturbed {name} noise={noise:e}: {error}"));
            let after = normalized_check(&system, value, &candidate).unwrap().0;
            let overlap = wide_overlap(&first, &candidate);
            let distance =
                subspace_distance(&[first.clone(), candidate.clone()], &reference, &order);
            println!(
                "subspace perturbed {name} noise={noise:e} before={before:e} after={after:e} overlap={overlap:e} distance={distance:e} calls={calls}"
            );
            assert!(after <= 1e-8 && overlap <= 1e-10 && distance < 1e-10);
            assert!(calls > 2 && calls <= 81);
            check_blocks(&oracle, value, &candidate, &order);
            assert_eq!(vector, original);
        }
    }
}

#[test]
fn subspace_joint_admission_validates_bases_and_preserves_accepted_vectors() {
    for (size, previous, tolerance) in [
        (1, vec![], 1e-10),
        (257, vec![], 1e-10),
        (4, vec![vec![1.0]], 1e-10),
        (4, vec![vec![0.0; 4]], 1e-10),
        (4, vec![vec![f64::NAN; 4]], 1e-10),
        (4, vec![vec![f64::INFINITY; 4]], 1e-10),
        (4, vec![vec![1.0; 4]; 2], 1e-10),
        (4, vec![vec![1.0; 4]; 4], 1e-10),
        (4, vec![], 0.0),
        (4, vec![], f64::NAN),
        (4, vec![], f64::INFINITY),
        (4, vec![], 1e-3),
    ] {
        assert!(BasisGuard::prepare(size, &previous, tolerance).is_err());
    }
    let previous = vec![vec![1.0, 0.0, 0.0, 0.0], vec![0.0, 1.0, 0.0, 0.0]];
    let guard = BasisGuard::prepare(4, &previous, 1e-10).unwrap();
    let vector = vec![0.0, -0.0, 0.6, 0.8];
    assert_eq!(
        guard
            .project_seed(&vector)
            .unwrap()
            .iter()
            .map(|v| v.to_bits())
            .collect::<Vec<_>>(),
        vector.iter().map(|v| v.to_bits()).collect::<Vec<_>>()
    );
    assert!(guard.check(&[0.0; 4]).is_err());
    assert!(guard.check(&[1.0; 3]).is_err());
    assert!(guard.check(&[f64::NAN; 4]).is_err());
    assert!(
        guard
            .check(&[1.0, 0.0, 0.0, 0.0])
            .unwrap_err()
            .contains("orthogonality gate")
    );
}

#[test]
fn subspace_joint_admission_rechecks_residual_and_overlap_after_projection_and_correction() {
    let matrix = vec![
        vec![0.0, 0.0, 0.0, 0.0],
        vec![0.0, 1.0, 0.0, 0.0],
        vec![0.0, 0.0, 2.0, 0.0],
        vec![0.0, 0.0, 0.0, 3.0],
    ];
    let vector = vec![1.0, 0.0, 0.0, 0.0];
    let previous = vec![vec![0.25, 1.0, 0.0, 0.0]];
    let guard = BasisGuard::prepare(4, &previous, 1e-10).unwrap();
    let fit = BlockFit::prepare_automatic(matrix, &vector).unwrap();
    let check = |v: &[f64]| {
        let applied: Vec<_> = v
            .iter()
            .enumerate()
            .map(|(i, v)| (i + 1) as f64 * v)
            .collect();
        Ok((
            relative_residual(1.0, v, &applied)?,
            residual_vector(1.0, v, &applied)?,
        ))
    };
    assert_eq!(check(&vector).unwrap().0, 0.0);
    let projected = guard.project_seed(&vector).unwrap();
    assert!(check(&projected).unwrap().0 > 1e-8);
    let individually_valid = fit.correct(&projected, 1e-8, check).unwrap();
    assert!(check(&individually_valid).unwrap().0 <= 1e-8);
    assert!(guard.check(&individually_valid).is_err());
    let error = guard.correct(&fit, &vector, 1e-8, check).unwrap_err();
    assert!(error.contains("orthogonality gate"), "{error}");
    assert_eq!(vector, vec![1.0, 0.0, 0.0, 0.0]);
    assert_eq!(previous, vec![vec![0.25, 1.0, 0.0, 0.0]]);
}

#[test]
fn subspace_joint_admission_reuses_one_factor_after_bounded_failure() {
    let order = orders(256).pop().unwrap().1;
    let system = repeated_bending(64, &order);
    let oracle = BendingReference::new(64);
    let (value, reference) = oracle.first_mode();
    let value = value.rounded();
    let first = rotated(&reference, &order, [0.6, 0.8]);
    let previous = vec![first.clone()];
    let guard = BasisGuard::prepare(256, &previous, 1e-10).unwrap();
    let second = rotated(&reference, &order, [-0.8, 0.6]);
    let vector: Vec<_> = first
        .iter()
        .zip(second)
        .map(|(&a, b)| 0.25 * a + b)
        .collect();
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
            let fit = automatic_fit(&system, value, &vector);
            let mut calls = 0;
            let error = guard
                .correct(&fit, &vector, 1e-30, |v| {
                    calls += 1;
                    normalized_check(&system, value, v)
                })
                .unwrap_err();
            assert!(
                error.contains("did not reach its unchanged residual gate"),
                "{error}"
            );
            assert!(calls <= 81, "one bounded fit plus one final check: {calls}");
            let candidate = guard
                .correct(&fit, &vector, 1e-8, |v| normalized_check(&system, value, v))
                .unwrap();
            assert!(wide_overlap(&first, &candidate) <= 1e-10);
            check_blocks(&oracle, value, &candidate, &order);
            println!("subspace bounded checked_calls={calls}");
            Ok::<(), String>(())
        },
    )
    .unwrap();
    assert_eq!(factors.get(), 1);
    assert_eq!(vector, original);
    assert_eq!(previous, vec![first]);
}

#[test]
fn subspace_joint_admission_cancels_during_projection_and_before_final_return() {
    let order = orders(128).pop().unwrap().1;
    let system = repeated_bending(32, &order);
    let (value, reference) = BendingReference::new(32).first_mode();
    let value = value.rounded();
    let first = rotated(&reference, &order, [0.6, 0.8]);
    let previous = vec![first.clone()];
    let guard = BasisGuard::prepare(128, &previous, 1e-10).unwrap();
    let second = rotated(&reference, &order, [-0.8, 0.6]);
    let vector: Vec<_> = first
        .iter()
        .zip(second)
        .map(|(&a, b)| 0.25 * a + b)
        .collect();
    let fit = automatic_fit(&system, value, &vector);
    let original = vector.clone();
    for phase in 0..3 {
        let control = SolverControl::default();
        let cancel = control.clone();
        let updates = Rc::new(Cell::new(0));
        let observed = updates.clone();
        let error = with_solver_observer(
            &control,
            move |point| {
                if point.stage == SolverStage::ModalVectorUpdate && point.completed_steps == 0 {
                    observed.set(observed.get() + 1);
                }
                let inside_update = point.stage == SolverStage::ModalVectorUpdate
                    && point.completed_steps == 64
                    && observed.get() == if phase == 0 { 1 } else { 3 };
                let final_return =
                    point.stage == SolverStage::ModalValidation && point.completed_steps == 2;
                if (phase < 2 && inside_update) || (phase == 2 && final_return) {
                    cancel.request_cancel();
                }
            },
            || guard.correct(&fit, &vector, 1e-8, |v| normalized_check(&system, value, v)),
        )
        .unwrap_err();
        assert!(error.contains("cancel"), "{error}");
        let last = control.last_checkpoint().unwrap();
        assert_eq!(
            last.stage,
            if phase == 2 {
                SolverStage::ModalValidation
            } else {
                SolverStage::ModalVectorUpdate
            }
        );
        assert_eq!(last.completed_steps, if phase == 2 { 2 } else { 64 });
        if phase < 2 {
            assert_eq!(updates.get(), if phase == 0 { 1 } else { 3 });
        }
        assert_eq!(vector, original);
        assert_eq!(previous, vec![first.clone()]);
        let replay = guard
            .correct(&fit, &vector, 1e-8, |v| normalized_check(&system, value, v))
            .unwrap();
        assert!(wide_overlap(&first, &replay) <= 1e-10);
    }
    for step in [0, 1] {
        let control = SolverControl::default();
        let cancel = control.clone();
        let error = with_solver_observer(
            &control,
            move |point| {
                if point.stage == SolverStage::ModalValidation && point.completed_steps == step {
                    cancel.request_cancel();
                }
            },
            || BasisGuard::prepare(128, &previous, 1e-10).map(|_| ()),
        )
        .unwrap_err();
        assert!(error.contains("cancel"), "{error}");
        assert_eq!(previous, vec![first.clone()]);
        assert!(BasisGuard::prepare(128, &previous, 1e-10).is_ok());
    }
}

#[test]
fn subspace_joint_admission_validates_final_callback_certificates() {
    let vector = vec![0.0, 1.0, 0.0, 0.0];
    let fit = BlockFit::prepare_automatic(
        vec![
            vec![1.0, 0.0, 0.0, 0.0],
            vec![0.0; 4],
            vec![0.0, 0.0, 2.0, 0.0],
            vec![0.0, 0.0, 0.0, 3.0],
        ],
        &vector,
    )
    .unwrap();
    let previous = vec![vec![1.0, 0.0, 0.0, 0.0]];
    let guard = BasisGuard::prepare(4, &previous, 1e-10).unwrap();
    for final_certificate in [
        (1.0, vec![0.0; 4]),
        (f64::NAN, vec![0.0; 4]),
        (-1.0, vec![0.0; 4]),
        (0.0, vec![0.0; 3]),
        (0.0, vec![f64::INFINITY; 4]),
    ] {
        let mut calls = 0;
        let error = guard
            .correct(&fit, &vector, 1e-8, |_| {
                calls += 1;
                if calls == 1 {
                    Ok((0.0, vec![0.0; 4]))
                } else {
                    Ok(final_certificate.clone())
                }
            })
            .unwrap_err();
        assert!(error.contains("final residual certificate"), "{error}");
        assert_eq!(calls, 2);
    }
    for tolerance in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(
            guard
                .correct(&fit, &vector, tolerance, |_| panic!(
                    "invalid gate cannot measure"
                ))
                .is_err()
        );
    }
    for seed in [
        vec![0.0; 4],
        vec![1.0; 3],
        vec![f64::NAN; 4],
        vec![f64::INFINITY; 4],
    ] {
        assert!(
            guard
                .correct(&fit, &seed, 1e-8, |_| panic!("invalid seed cannot measure"))
                .is_err()
        );
    }
}

#[test]
fn subspace_joint_admission_projects_multiple_scaled_basis_directions_without_rank_loss() {
    let previous = vec![
        vec![-2.0_f64.powi(-80), 0.0, 0.0, 0.0],
        vec![0.0, 2.0_f64.powi(80), 0.0, 0.0],
    ];
    let original = previous.clone();
    let guard = BasisGuard::prepare(4, &previous, 1e-10).unwrap();
    let candidate = guard.project_seed(&[0.25, -0.5, 0.6, 0.8]).unwrap();
    assert_eq!(candidate[0], 0.0);
    assert_eq!(candidate[1], 0.0);
    assert!(candidate[2].abs() > 0.0 && candidate[3].abs() > 0.0);
    for prior in &previous {
        assert!(wide_overlap(&candidate, prior) <= 1e-10);
    }
    assert!(
        guard
            .project_seed(&[0.25, -0.5, 1e-4, 0.0])
            .unwrap_err()
            .contains("lost an independent direction")
    );
    assert_eq!(previous, original);
    for scalar in [f64::from_bits(1), f64::MAX] {
        assert!(BasisGuard::prepare(4, &[vec![scalar, 0.0, 0.0, 0.0]], 1e-10).is_ok());
    }
    let loss = vec![f64::MAX, f64::from_bits(1), 0.0, 0.0];
    let error = BasisGuard::prepare(4, &[loss.clone()], 1e-10)
        .err()
        .unwrap();
    assert!(
        error.contains("cannot discard a nonzero component"),
        "{error}"
    );
    assert_eq!(loss[1].to_bits(), 1);
}

#[test]
fn subspace_joint_admission_retains_distinct_roots_and_close_diagonal_modes() {
    for gap in [1e-2, 1e-5, 1e-8, 1e-10] {
        let roots = [1.0, 1.0 + gap, 3.0, 4.0];
        for (_, order) in orders(4) {
            let previous = vec![
                order
                    .iter()
                    .map(|&i| if i == 0 { 1.0 } else { 0.0 })
                    .collect(),
            ];
            let vector: Vec<_> = order.iter().map(|&i| [0.25, 1.0, 1e-4, 0.0][i]).collect();
            let matrix: Vec<Vec<_>> = order
                .iter()
                .enumerate()
                .map(|(row, &i)| {
                    (0..4)
                        .map(|column| {
                            if row == column {
                                roots[i] - roots[1]
                            } else {
                                0.0
                            }
                        })
                        .collect()
                })
                .collect();
            let fit = BlockFit::prepare_automatic(matrix, &vector).unwrap();
            let guard = BasisGuard::prepare(4, &previous, 1e-10).unwrap();
            let candidate = guard
                .correct(&fit, &vector, 1e-8, |v| {
                    let applied: Vec<_> =
                        order.iter().zip(v).map(|(&i, &v)| roots[i] * v).collect();
                    Ok((
                        relative_residual(roots[1], v, &applied)?,
                        residual_vector(roots[1], v, &applied)?,
                    ))
                })
                .unwrap();
            let restored = original_order(&candidate, &order);
            assert!(restored[1].abs() > 0.0);
            assert_eq!(restored[0], 0.0);
            assert!(restored[2].abs() <= 1e-8 * restored[1].abs());
            assert!(wide_overlap(&candidate, &previous[0]) <= 1e-10);
        }
    }
}

pub(super) fn physical_mass_overlap(left: &[f64], right: &[f64], mass: &[f64]) -> f64 {
    let dot = |left: &[f64], right: &[f64]| {
        left.iter()
            .zip(right)
            .zip(mass)
            .fold(Wide::default(), |sum, ((&a, &b), &m)| {
                sum.add(Wide::from(a).mul(Wide::from(b)).mul(Wide::from(m)))
            })
    };
    dot(left, right).rounded().abs()
        / dot(left, left).rounded().sqrt()
        / dot(right, right).rounded().sqrt()
}

#[test]
fn subspace_joint_admission_checks_physical_mass_orthogonality_after_json_readback() {
    let segments = 64;
    let width = 2 * segments;
    let physical_width = 3 * (segments + 1);
    let oracle = BendingReference::new(segments);
    let (value, reference) = oracle.first_mode();
    let value = value.rounded();
    let original_mass = bending(segments).mass;
    for (name, order) in orders(2 * width) {
        let mut system = repeated_bending(segments, &order);
        system.free_dofs = order
            .iter()
            .map(|&i| (i / width) * physical_width + 3 * (i % width / 2 + 1) + 1 + i % 2)
            .collect();
        let mut mass = vec![1.0; 2 * physical_width];
        for (&dof, &m) in system.free_dofs.iter().zip(&system.mass) {
            mass[dof] = m;
        }
        for rotation in [[1.0, 0.0], [0.6, 0.8]] {
            let first = rotated(&reference, &order, rotation);
            let first = automatic_fit(&system, value, &first)
                .correct(&first, 1e-8, |v| normalized_check(&system, value, v))
                .unwrap();
            let second = rotated(&reference, &order, [-rotation[1], rotation[0]]);
            let guard = BasisGuard::prepare(order.len(), &[first.clone()], 1e-10).unwrap();
            let second = guard
                .correct(
                    &automatic_fit(&system, value, &second),
                    &second,
                    1e-8,
                    |v| normalized_check(&system, value, v),
                )
                .unwrap();
            let modes: Vec<_> = [first, second]
                .iter()
                .enumerate()
                .map(|(index, vector)| {
                    let (mut shape, _) =
                        checked_mode_shape(vector, &mass, &system.free_dofs, mass.len()).unwrap();
                    let mut reduced: Vec<_> = system.free_dofs.iter().map(|&i| shape[i]).collect();
                    system
                        .operator
                        .polish_published_shape(value, &mut reduced, &system.mass, 1e-8)
                        .unwrap();
                    for (&dof, &v) in system.free_dofs.iter().zip(&reduced) {
                        shape[dof] = v;
                    }
                    let norm = checked_shape_norm(&shape).unwrap();
                    assert!((norm - 1.0).abs() < 1e-10);
                    let frequency = value.sqrt();
                    ModalFrame2dModeResult {
                        index,
                        eigenvalue_rad_s_squared: value,
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
                assert_eq!(mode.eigenvalue_rad_s_squared.to_bits(), value.to_bits());
                assert!(
                    mode.shape
                        .iter()
                        .zip(&modes[index].shape)
                        .all(|(a, b)| a.to_bits() == b.to_bits())
                );
                assert!(
                    mode.shape
                        .chunks_exact(physical_width)
                        .all(|block| block[..3].iter().all(|v| *v == 0.0))
                );
                assert!(mode.shape.chunks_exact(3).all(|node| node[0] == 0.0));
                assert!((mode.natural_frequency_hz * mode.period_s - 1.0).abs() < 1e-12);
                let reduced: Vec<_> = system.free_dofs.iter().map(|&i| mode.shape[i]).collect();
                let actual = physical_check(&system, value, &reduced).unwrap().0;
                assert!(actual <= 1e-8, "physical {name} mode={index}: {actual:e}");
                for block in original_order(&reduced, &order).chunks_exact(width) {
                    if block.iter().any(|v| *v != 0.0) {
                        let independent = oracle.physical_residual(
                            Wide::from(value),
                            &wide_vector(block),
                            &original_mass,
                        );
                        assert!(
                            independent <= 1e-8,
                            "physical independent {name} mode={index}: {independent:e}"
                        );
                    }
                }
                whitened.push(
                    reduced
                        .iter()
                        .zip(&system.mass)
                        .map(|(&v, &m)| v * m.sqrt())
                        .collect::<Vec<_>>(),
                );
            }
            let overlap = physical_mass_overlap(&restored[0].shape, &restored[1].shape, &mass);
            let distance = subspace_distance(&whitened, &reference, &order);
            println!(
                "subspace physical {name} rotation={rotation:?} overlap={overlap:e} distance={distance:e}"
            );
            assert!(overlap <= 1e-10 && distance < 1e-10);
            assert!(physical_mass_overlap(&restored[0].shape, &restored[0].shape, &mass) > 0.99);
        }
    }
}
