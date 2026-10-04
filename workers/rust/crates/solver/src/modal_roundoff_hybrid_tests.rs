use super::*;
use crate::linear_algebra::{SparseMatrix, add_at};
use crate::modal_frame_assembly::element_masses;
use crate::modal_frame_spectrum::{checked_mode_shape, frame_eigenpairs};
use crate::modal_sparse::reduce_sparse_modal_system;

#[path = "modal_roundoff_hybrid_control_tests.rs"]
mod control;

#[path = "../tests/support/modal_roundoff_reference.rs"]
mod reference;

pub(super) fn check_published(segments: usize, step: f64, value: f64, candidate: &[f64]) {
    use kyuubiki_protocol::{
        ModalFrame2dModeResult, SolveModalFrame2dRequest, SolveModalFrame2dResult,
    };
    use serde_json::json;
    let nodes: Vec<_> = (0..=segments)
        .map(|i| {
            json!({"id":format!("node-{i}"), "x":i as f64 * step, "y":0.0,
        "fix_x":true, "fix_y":i==0, "fix_rz":i==0, "load_x":0.0, "load_y":0.0, "moment_z":0.0})
        })
        .collect();
    let elements: Vec<_> = (0..segments).map(|i| json!({"id":format!("beam-{i}"), "node_i":i, "node_j":i+1,
        "area":1.0, "youngs_modulus":step.powi(3), "moment_of_inertia":1.0, "section_modulus":1.0, "density":step.recip()})).collect();
    let input: SolveModalFrame2dRequest =
        serde_json::from_value(json!({"nodes":nodes, "elements":elements, "mode_count":1}))
            .unwrap();
    let free_dofs: Vec<_> = (1..=segments)
        .flat_map(|i| [3 * i + 1, 3 * i + 2])
        .collect();
    let mut shape = vec![0.0; 3 * (segments + 1)];
    for (&dof, &v) in free_dofs.iter().zip(candidate) {
        shape[dof] = v;
    }
    let norm = crate::modal_math::checked_shape_norm(&shape).unwrap();
    assert!((norm - 1.0).abs() < 1e-10);
    let frequency = value.sqrt() / std::f64::consts::TAU;
    let result = SolveModalFrame2dResult {
        input,
        modes: vec![ModalFrame2dModeResult {
            index: 0,
            eigenvalue_rad_s_squared: value,
            natural_frequency_rad_s: value.sqrt(),
            natural_frequency_hz: frequency,
            period_s: frequency.recip(),
            participation_norm: norm,
            shape,
        }],
        free_dofs,
        total_mass: segments as f64,
        min_frequency_hz: frequency,
        max_frequency_hz: frequency,
    };
    let restored: SolveModalFrame2dResult =
        serde_json::from_slice(&serde_json::to_vec(&result).unwrap()).unwrap();
    assert_eq!(restored, result);
    assert!(
        restored.modes[0]
            .shape
            .iter()
            .zip(&result.modes[0].shape)
            .all(|(a, b)| a.to_bits() == b.to_bits())
    );
    let independent = reference::reassembled_residual(&restored);
    assert!(
        independent <= 1e-8,
        "independent physical residual={independent:e}"
    );
}

fn scaled_bending(segments: usize, step: f64) -> ReducedSparseModalSystem {
    scaled_bending_parts(segments, step).0
}

pub(super) fn scaled_bending_parts(
    segments: usize,
    step: f64,
) -> (ReducedSparseModalSystem, Vec<Vec<f64>>) {
    let size = 2 * (segments + 1);
    let mut matrix = SparseMatrix::new(size);
    let mut mass = vec![0.0; size];
    for e in 0..segments {
        let length = ((e + 1) as f64 * step - e as f64 * step).abs();
        let local = crate::frame_2d_math::frame_local_stiffness(1.0, step.powi(3), 1.0, length);
        let map = [1, 2, 4, 5];
        for (i, &row) in map.iter().enumerate() {
            for (j, &col) in map.iter().enumerate() {
                add_at(&mut matrix, 2 * e + i, 2 * e + j, local[row][col]);
            }
        }
        let [_, translation, rotation] = element_masses("beam", step.recip(), 1.0, length).unwrap();
        for node in [e, e + 1] {
            mass[2 * node] += translation;
            mass[2 * node + 1] += rotation;
        }
    }
    let mut physical = vec![vec![0.0; size - 2]; size - 2];
    for row in 2..size {
        for &(column, value) in matrix.row_entries(row) {
            if column >= 2 {
                physical[row - 2][column - 2] = value;
            }
        }
    }
    (
        reduce_sparse_modal_system(&matrix, &mass, &[0, 1]).unwrap(),
        physical,
    )
}

#[test]
fn hybrid_qr_proposal_recovers_nearly_dependent_columns_without_normal_equations() {
    let columns = vec![vec![1.0, 1.0, 1.0], vec![1.0, 1.0 + 1e-9, 1.0 - 1e-9]];
    assert!(FineFactor::gram(&columns).is_err());
    let factor = qr::QrFit::factor(&columns).unwrap();
    let rhs: Vec<_> = (0..3)
        .map(|i| columns[0][i] + 2.0 * columns[1][i])
        .collect();
    let solution = factor.solve(&rhs).unwrap();
    assert!((solution[0] - 1.0).abs() < 1e-6 && (solution[1] - 2.0).abs() < 1e-6);
    for (i, &target) in rhs.iter().enumerate() {
        assert!((solution[0] * columns[0][i] + solution[1] * columns[1][i] - target).abs() < 1e-14);
    }
}

#[test]
fn hybrid_qr_physical_proposals_compare_extreme_coordinate_roundoff() {
    for step in [1.0, 1e14, 1e-10] {
        let system = scaled_bending(128, step);
        let spectrum = frame_eigenpairs(&system, Some(1)).unwrap();
        let (value, vector) = &spectrum.pairs[0];
        let (shape, _) =
            checked_mode_shape(vector, &system.mass, &(0..256).collect::<Vec<_>>(), 256).unwrap();
        let shape = system
            .operator
            .roundoff_comparison_seed(*value, &shape, &system.mass)
            .unwrap();
        let mut matrix = system.operator.dense_fallback_matrix().unwrap();
        for (i, row) in matrix.iter_mut().enumerate() {
            for (a, &m) in row.iter_mut().zip(&system.mass) {
                *a /= m.sqrt().recip();
            }
            row[i] -= value * system.mass[i] * system.mass[i].sqrt().recip();
        }
        let gram = BlockFit::prepare_automatic(matrix.clone(), &shape).unwrap();
        let fit = BlockFit::prepare_qr_automatic(matrix.clone(), &shape).unwrap();
        let anchor = shape
            .iter()
            .zip(&system.mass)
            .enumerate()
            .max_by(|(_, (a, ma)), (_, (b, mb))| {
                (a.abs() * ma.sqrt()).total_cmp(&(b.abs() * mb.sqrt()))
            })
            .unwrap()
            .0;
        let full = BlockFit::prepare_qr_partition(
            matrix.clone(),
            (0..shape.len()).filter(|&i| i != anchor).collect(),
        )
        .unwrap();
        let baseline = gram.correct(&shape, 1e-8, |v| {
            let applied = system.operator.apply_physical_compensated(v)?;
            system
                .operator
                .physical_residual(*value, v, &system.mass, &applied)
        });
        let outcome = fit.correct(&shape, 1e-8, |v| {
            let applied = system.operator.apply_physical_compensated(v)?;
            system
                .operator
                .physical_residual(*value, v, &system.mass, &applied)
        });
        let hybrid = gram.correct_hybrid(&fit, &shape, 1e-8, |v| {
            let applied = system.operator.apply_physical_compensated(v)?;
            system
                .operator
                .physical_residual(*value, v, &system.mass, &applied)
        });
        let global = gram.correct_hybrid(&full, &shape, 1e-8, |v| {
            let applied = system.operator.apply_physical_compensated(v)?;
            system
                .operator
                .physical_residual(*value, v, &system.mass, &applied)
        });
        let micro = grid::correct(&matrix, &shape, 1e-8, |v| {
            let applied = system.operator.apply_physical_compensated(v)?;
            system
                .operator
                .physical_residual(*value, v, &system.mass, &applied)
        });
        let mut best = (f64::INFINITY, 1.0);
        let (mut upper, mut lower) = (1.0_f64, 1.0_f64);
        for _ in 0..16 {
            upper = upper.next_up();
            lower = lower.next_down();
            for scale in [upper, lower] {
                let candidate: Vec<_> = shape.iter().map(|v| v * scale).collect();
                let applied = system
                    .operator
                    .apply_physical_compensated(&candidate)
                    .unwrap();
                let relative = system
                    .operator
                    .physical_residual(*value, &candidate, &system.mass, &applied)
                    .unwrap()
                    .0;
                if relative < best.0 {
                    best = (relative, scale);
                }
            }
        }
        println!(
            "amplitude step={step:e}: relative={:e} scale={:.17e}",
            best.0, best.1
        );
        assert!(
            best.0 > 1e-8,
            "amplitude probes do not qualify this fixture"
        );
        for (name, result) in [
            ("Gram", &baseline),
            ("QR", &outcome),
            ("Gram/QR", &hybrid),
            ("Gram/global QR", &global),
            ("grid", &micro),
        ] {
            match result {
                Ok(candidate) => {
                    let applied = system
                        .operator
                        .apply_physical_compensated(candidate)
                        .unwrap();
                    let relative = system
                        .operator
                        .physical_residual(*value, candidate, &system.mass, &applied)
                        .unwrap()
                        .0;
                    println!("{name} physical step={step:e}: relative={relative:e}");
                    assert_ne!(step, 1e-10, "tiny-coordinate reference remains unresolved");
                    check_published(128, step, *value, candidate);
                }
                Err(error) => {
                    assert!(error.contains("unchanged residual gate"), "{error}");
                    assert!(
                        step == 1e-10 || name == "grid",
                        "{name} must preserve qualified scale fixtures"
                    );
                    println!("{name} physical step={step:e}: {error}");
                }
            }
        }
        assert_eq!(baseline.is_ok(), step != 1e-10);
        assert_eq!(outcome.is_ok(), step != 1e-10);
        assert_eq!(hybrid.is_ok(), step != 1e-10);
        assert_eq!(global.is_ok(), step != 1e-10);
        assert!(micro.is_err());
    }
}
