use super::*;
use crate::modal_frame_assembly::positive_product;
use crate::modal_frame_spectrum::checked_mode_shape;
use crate::modal_math::checked_shape_norm;
use kyuubiki_protocol::ModalFrame2dModeResult;

#[path = "modal_frame_lattice_reference.rs"]
mod lattice;
use lattice::BlockFit;

#[path = "modal_frame_coupled_tests.rs"]
mod coupled;

fn normalized_check(
    system: &ReducedSparseModalSystem,
    value: f64,
    vector: &[f64],
) -> Result<(f64, Vec<f64>), String> {
    let applied = system.operator.apply_compensated(vector)?;
    Ok((
        relative_residual(value, vector, &applied)?,
        residual_vector(value, vector, &applied)?,
    ))
}

fn normalized_fit(system: &ReducedSparseModalSystem, value: f64) -> BlockFit {
    let mut matrix = system.operator.dense_fallback_matrix().unwrap();
    for (i, row) in matrix.iter_mut().enumerate() {
        row[i] -= value;
    }
    BlockFit::prepare(matrix, (1..system.mass.len()).step_by(2).collect()).unwrap()
}

fn physical_check(
    system: &ReducedSparseModalSystem,
    value: f64,
    shape: &[f64],
) -> Result<(f64, Vec<f64>), String> {
    let applied = system.operator.apply_physical_compensated(shape)?;
    let target: Vec<_> = shape
        .iter()
        .zip(&system.mass)
        .map(|(&v, &m)| positive_product([value, m, m.sqrt().recip(), v.abs()]).copysign(v))
        .collect();
    let residual: Vec<_> = applied.iter().zip(&target).map(|(&a, &b)| a - b).collect();
    let relative = stable_l2_norm(residual.iter().copied())
        / stable_l2_norm(applied.iter().copied()).max(stable_l2_norm(target.iter().copied()));
    assert!(relative.is_finite());
    Ok((relative, residual))
}

fn physical_fit(system: &ReducedSparseModalSystem, value: f64) -> BlockFit {
    let mut matrix = system.operator.dense_fallback_matrix().unwrap();
    for (i, row) in matrix.iter_mut().enumerate() {
        for (a, &m) in row.iter_mut().zip(&system.mass) {
            *a /= m.sqrt().recip();
        }
        row[i] -= value * system.mass[i] * system.mass[i].sqrt().recip();
    }
    BlockFit::prepare(matrix, (1..system.mass.len()).step_by(2).collect()).unwrap()
}

fn wide_vector(vector: &[f64]) -> Vec<Wide> {
    vector.iter().map(|&v| Wide::from(v)).collect()
}

#[test]
fn joint_f64_candidate_passes_the_first_bending_mode_gate_without_wide_storage() {
    let system = bending(128);
    let oracle = BendingReference::new(128);
    let seed = jacobi_eigenpairs(system.operator.dense_fallback_matrix().unwrap()).unwrap();
    let mut pairs = seed[..1].to_vec();
    refine_with_budget(&system, &mut pairs, 1e-8, 4).unwrap_err();
    let (value, seed) = &pairs[0];
    let before = normalized_check(&system, *value, seed).unwrap().0;
    let fit = normalized_fit(&system, *value);
    for exponent in [-80, 0, 80] {
        let vector: Vec<_> = seed.iter().map(|v| v * 2.0_f64.powi(exponent)).collect();
        let original = vector.clone();
        let candidate = fit
            .correct(&vector, 1e-8, |v| normalized_check(&system, *value, v))
            .unwrap();
        let actual = normalized_check(&system, *value, &candidate).unwrap().0;
        let independent = oracle.residual(Wide::from(*value), &wide_vector(&candidate));
        println!(
            "joint normalized exponent={exponent} before={before:e} actual={actual:e} independent={independent:e}"
        );
        assert!(before > 1e-8 && actual <= 1e-8 && independent <= 1e-8);
        assert!((actual / independent - 1.0).abs() < 1e-6);
        assert_eq!(
            vector, original,
            "candidate construction never overwrites the borrowed seed"
        );
        let (root, reference) = oracle.first_mode();
        assert!((*value / root.rounded() - 1.0).abs() < 1e-14);
        assert!(oracle.direction_error(&wide_vector(&candidate), &reference) < 1e-10);
    }
}

#[test]
fn joint_f64_candidates_resolve_sampled_rounded_wide_reference_modes() {
    for segments in [64, 80, 100, 128] {
        let system = bending(segments);
        let oracle = BendingReference::new(segments);
        let (value, reference) = oracle.first_mode();
        let vector: Vec<_> = reference.iter().map(|v| v.rounded()).collect();
        let initial = normalized_check(&system, value.rounded(), &vector)
            .unwrap()
            .0;
        let fit = normalized_fit(&system, value.rounded());
        let candidate = fit
            .correct(&vector, 1e-8, |v| {
                normalized_check(&system, value.rounded(), v)
            })
            .unwrap();
        let actual = normalized_check(&system, value.rounded(), &candidate)
            .unwrap()
            .0;
        let independent = oracle.residual(Wide::from(value.rounded()), &wide_vector(&candidate));
        println!(
            "joint reference segments={segments} actual={actual:e} independent={independent:e}"
        );
        assert!(actual <= 1e-8 && independent <= 1e-8);
        assert!(oracle.direction_error(&wide_vector(&candidate), &reference) < 1e-12);
        if initial <= 1e-8 {
            assert_eq!(
                candidate, vector,
                "an already accepted direction stays intact"
            );
        }
    }
}

#[test]
fn joint_candidate_keeps_the_physical_shape_gate_after_protocol_json_readback() {
    let segments = 128;
    let mut system = bending(segments);
    let oracle = BendingReference::new(segments);
    let seed = jacobi_eigenpairs(system.operator.dense_fallback_matrix().unwrap()).unwrap();
    let mut pairs = seed[..1].to_vec();
    refine_with_budget(&system, &mut pairs, 1e-8, 4).unwrap_err();
    let (value, vector) = pairs.remove(0);
    let vector = normalized_fit(&system, value)
        .correct(&vector, 1e-8, |v| normalized_check(&system, value, v))
        .unwrap();
    system.free_dofs = (1..=segments)
        .flat_map(|i| [3 * i + 1, 3 * i + 2])
        .collect();
    let size = 3 * (segments + 1);
    let mut mass = vec![1.0; size];
    for (&dof, &m) in system.free_dofs.iter().zip(&system.mass) {
        mass[dof] = m;
    }
    let (mut shape, _) = checked_mode_shape(&vector, &mass, &system.free_dofs, size).unwrap();
    let reduced: Vec<_> = system.free_dofs.iter().map(|&i| shape[i]).collect();
    let before = physical_check(&system, value, &reduced).unwrap().0;
    let mut candidate = physical_fit(&system, value)
        .correct(&reduced, 1e-8, |v| physical_check(&system, value, v))
        .unwrap();
    system
        .operator
        .polish_published_shape(value, &mut candidate, &system.mass, 1e-8)
        .unwrap();
    for (&dof, &v) in system.free_dofs.iter().zip(&candidate) {
        shape[dof] = v;
    }
    let norm = checked_shape_norm(&shape).unwrap();
    assert!((norm - 1.0).abs() < 1e-10);
    let frequency = value.sqrt();
    let mode = ModalFrame2dModeResult {
        index: 0,
        eigenvalue_rad_s_squared: value,
        natural_frequency_rad_s: frequency,
        natural_frequency_hz: frequency / std::f64::consts::TAU,
        period_s: std::f64::consts::TAU / frequency,
        participation_norm: norm,
        shape,
    };
    let encoded = serde_json::to_vec(&mode).unwrap();
    let restored: ModalFrame2dModeResult = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(restored, mode);
    assert!(
        mode.shape
            .iter()
            .zip(&restored.shape)
            .all(|(a, b)| a.to_bits() == b.to_bits())
    );
    assert_eq!(
        mode.eigenvalue_rad_s_squared.to_bits(),
        restored.eigenvalue_rad_s_squared.to_bits()
    );
    let reduced: Vec<_> = system
        .free_dofs
        .iter()
        .map(|&i| restored.shape[i])
        .collect();
    let actual = physical_check(&system, restored.eigenvalue_rad_s_squared, &reduced)
        .unwrap()
        .0;
    let independent =
        oracle.physical_residual(Wide::from(value), &wide_vector(&reduced), &system.mass);
    println!("joint physical before={before:e} readback={actual:e} independent={independent:e}");
    assert!(actual <= 1e-8 && independent <= 1e-8);
    assert!(restored.shape[..3].iter().all(|v| *v == 0.0));
    assert!(restored.shape.chunks_exact(3).all(|node| node[0] == 0.0));
    assert!((restored.natural_frequency_hz * restored.period_s - 1.0).abs() < 1e-12);
}

#[test]
fn joint_fit_budget_failure_keeps_the_borrowed_seed_unchanged() {
    let system = bending(128);
    let oracle = BendingReference::new(128);
    let (value, reference) = oracle.first_mode();
    let vector: Vec<_> = reference.iter().map(|v| v.rounded()).collect();
    let original = vector.clone();
    let fit = normalized_fit(&system, value.rounded());
    let error = fit
        .correct(&vector, 1e-30, |v| {
            normalized_check(&system, value.rounded(), v)
        })
        .unwrap_err();
    assert!(
        error.contains("did not reach its unchanged residual gate"),
        "{error}"
    );
    assert_eq!(vector, original);
    let replay = fit
        .correct(&vector, 1e-8, |v| {
            normalized_check(&system, value.rounded(), v)
        })
        .unwrap();
    assert!(oracle.residual(Wide::from(value.rounded()), &wide_vector(&replay)) <= 1e-8);
}

#[test]
fn joint_fit_rejects_invalid_partitions_dimensions_and_residual_certificates() {
    let identity = vec![vec![1.0, 0.0], vec![0.0, 1.0]];
    for subset in [vec![], vec![0, 0], vec![1, 0], vec![2], vec![0, 1]] {
        assert!(BlockFit::prepare(identity.clone(), subset).is_err());
    }
    assert!(BlockFit::prepare(vec![vec![0.0, 0.0], vec![0.0, 1.0]], vec![0]).is_err());
    assert!(BlockFit::prepare(vec![vec![1.0]], vec![0]).is_err());
    assert!(BlockFit::prepare(vec![vec![0.0; 257]; 257], vec![0]).is_err());
    assert!(BlockFit::prepare(vec![vec![f64::NAN, 0.0], vec![0.0, 1.0]], vec![0]).is_err());
    let fit = BlockFit::prepare(identity, vec![0]).unwrap();
    for tolerance in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(
            fit.correct(&[1.0, 1.0], tolerance, |_| unreachable!())
                .is_err()
        );
    }
    assert!(fit.correct(&[1.0], 1e-8, |_| unreachable!()).is_err());
    assert!(
        fit.correct(&[f64::NAN, 1.0], 1e-8, |_| unreachable!())
            .is_err()
    );
    for relative in [f64::NAN, f64::INFINITY, -1.0] {
        assert!(
            fit.correct(&[1.0, 1.0], 1e-8, |_| Ok((relative, vec![0.0; 2])))
                .is_err()
        );
    }
    assert!(
        fit.correct(&[1.0, 1.0], 1e-8, |_| Ok((0.0, vec![0.0])))
            .is_err()
    );
    assert!(
        fit.correct(&[1.0, 1.0], 1e-8, |_| Ok((0.0, vec![f64::NAN; 2])))
            .is_err()
    );
}

#[test]
fn joint_fit_cancels_inside_substitution_and_before_checked_candidate_acceptance() {
    let system = bending(128);
    let oracle = BendingReference::new(128);
    let (value, reference) = oracle.first_mode();
    let vector: Vec<_> = reference.iter().map(|v| v.rounded()).collect();
    let original = vector.clone();
    let fit = normalized_fit(&system, value.rounded());
    let control = SolverControl::default();
    let cancel = control.clone();
    let error = with_solver_observer(
        &control,
        move |point| {
            if point.stage == SolverStage::DenseSubstitution && point.completed_steps == 64 {
                cancel.request_cancel();
            }
        },
        || {
            fit.correct(&vector, 1e-8, |v| {
                normalized_check(&system, value.rounded(), v)
            })
        },
    )
    .unwrap_err();
    assert!(error.contains("cancel"), "{error}");
    assert_eq!(vector, original);
    let accepted = fit
        .correct(&vector, 1e-8, |v| {
            normalized_check(&system, value.rounded(), v)
        })
        .unwrap();
    let control = SolverControl::default();
    let cancel = control.clone();
    let checked = Rc::new(Cell::new(false));
    let observed = checked.clone();
    let error = with_solver_observer(
        &control,
        |_| {},
        || {
            fit.correct(&accepted, 1e-8, |v| {
                let result = normalized_check(&system, value.rounded(), v)?;
                assert!(result.0 <= 1e-8);
                observed.set(true);
                cancel.request_cancel();
                Ok(result)
            })
        },
    )
    .unwrap_err();
    assert!(checked.get() && error.contains("cancel"));
    let replay = fit
        .correct(&accepted, 1e-8, |v| {
            normalized_check(&system, value.rounded(), v)
        })
        .unwrap();
    assert_eq!(replay, accepted);
}
