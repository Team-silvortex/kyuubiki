use super::super::ThermalFrame3dConstraintSystem;
use super::{projected_row, validate_direction};
use crate::linear_algebra::{PrescribedSystemValidator, SparseMatrix, add_at, solve_spd_system};
use crate::solver_control::{SolverControl, SolverStage, with_solver_observer};
use kyuubiki_protocol::SolveThermalFrame3dRequest;
use serde_json::json;
use std::cell::Cell;

fn request(rotation: bool, directions: &[[f64; 3]], count: usize) -> SolveThermalFrame3dRequest {
    let constraints = directions.iter().enumerate().map(|(index, direction)| {
        json!({"id":format!("guide-{index}"), "node":1, "direction":direction})
    }).collect::<Vec<_>>();
    serde_json::from_value(json!({
        "nodes":(0..count).map(|index| {
            let fixed = index != 1;
            json!({"id":format!("n{index}"), "x":index, "y":0.0, "z":0.0,
                "fix_x":fixed || rotation, "fix_y":fixed || rotation, "fix_z":fixed || rotation,
                "fix_rx":fixed || !rotation, "fix_ry":fixed || !rotation, "fix_rz":fixed || !rotation,
                "load_x":0.0, "load_y":0.0, "load_z":0.0,
                "moment_x":0.0, "moment_y":0.0, "moment_z":0.0, "temperature_delta":0.0})
        }).collect::<Vec<_>>(),
        "elements":[], "directional_springs":[], "directional_rotational_springs":[],
        "directional_constraints":if rotation { vec![] } else { constraints.clone() },
        "directional_rotational_constraints":if rotation { constraints } else { vec![] }
    })).unwrap()
}

fn diagonal(size: usize, scale: f64) -> SparseMatrix {
    let mut matrix = SparseMatrix::new(size);
    for index in 0..size {
        add_at(
            &mut matrix,
            index,
            index,
            scale * 2.0_f64.powi((index % 3 + 1) as i32),
        );
    }
    matrix
}

#[test]
fn projected_original_equations_check_translation_and_rotation_without_forbidding_reactions() {
    for rotation in [false, true] {
        let offset = if rotation { 9 } else { 6 };
        let gate = ThermalFrame3dConstraintSystem::build(&request(rotation, &[[1.0, 1.0, 0.0]], 2))
            .unwrap();
        for power in [-600, 0, 600] {
            let scale = 2.0_f64.powi(power);
            let matrix = diagonal(12, scale);
            for sign in [-1.0, 1.0] {
                let mut solution = vec![0.0; 12];
                solution[offset] = sign;
                solution[offset + 1] = -sign;
                let mut force = vec![0.0; 12];
                force[offset] = 5.0 * sign * scale;
                force[offset + 1] = -sign * scale;
                force[0] = 17.0 * scale;
                gate.validate_original(&matrix, &force, &solution).unwrap();

                let mut wrong = solution.clone();
                wrong[offset] *= 2.0;
                wrong[offset + 1] *= 2.0;
                assert!(
                    gate.validate_original(&matrix, &force, &wrong)
                        .unwrap_err()
                        .contains("free equilibrium")
                );
                wrong[offset + 1] = 0.0;
                assert!(
                    gate.validate_original(&matrix, &force, &wrong)
                        .unwrap_err()
                        .contains("constraint value changed")
                );
                wrong = solution.clone();
                wrong[0] = f64::from_bits(1);
                assert!(
                    gate.validate_original(&matrix, &force, &wrong)
                        .unwrap_err()
                        .contains("fixed value changed")
                );
                force[offset] += sign * scale;
                assert!(gate.validate_original(&matrix, &force, &solution).is_err());
            }
        }
    }
}

#[test]
fn projected_original_equations_accept_two_nonorthogonal_supports_and_signed_directions() {
    for sign in [-1.0, 1.0] {
        let gate = ThermalFrame3dConstraintSystem::build(&request(
            false,
            &[[sign, sign, 0.0], [0.0, -7.0 * sign, -7.0 * sign]],
            2,
        ))
        .unwrap();
        let matrix = diagonal(12, 1.0);
        let mut solution = vec![0.0; 12];
        solution[6..9].copy_from_slice(&[1.0, -1.0, 1.0]);
        let mut force = vec![0.0; 12];
        force[6..9].copy_from_slice(&[5.0, -3.0, 6.0]);
        gate.validate_original(&matrix, &force, &solution).unwrap();
        solution[8] = 2.0;
        assert!(gate.validate_original(&matrix, &force, &solution).is_err());
    }
}

#[test]
fn projected_original_equations_preserve_small_nonzero_support_components_through_solve() {
    for sign in [-1.0, 1.0] {
        let epsilon = sign * 2.0_f64.powi(-60);
        let gate =
            ThermalFrame3dConstraintSystem::build(&request(false, &[[1.0, epsilon, 0.0]], 2))
                .unwrap();
        let matrix = diagonal(12, 1.0);
        let mut force = vec![0.0; 12];
        force[6] = -2.0 * epsilon;
        force[7] = 4.0;
        let (reduced, rhs) = gate.project(&matrix, &force).unwrap();
        let solution = gate
            .restore(&solve_spd_system(&reduced, &rhs).unwrap())
            .unwrap();
        assert_eq!(solution[6], -epsilon);
        assert_eq!(solution[7], 1.0);
        gate.validate_original(&matrix, &force, &solution).unwrap();
        let mut clipped = solution;
        clipped[6] = 0.0;
        assert!(gate.validate_original(&matrix, &force, &clipped).is_err());
    }
}

#[test]
fn projected_original_equations_match_coordinate_gate_and_all_fixed_reactions() {
    for fixed in [false, true] {
        let directions = if fixed {
            vec![[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]
        } else {
            vec![]
        };
        let gate = ThermalFrame3dConstraintSystem::build(&request(false, &directions, 2)).unwrap();
        let matrix = diagonal(12, 1.0);
        let support = (0..12)
            .filter(|index| fixed || !(6..9).contains(index))
            .collect::<Vec<_>>();
        let reference = PrescribedSystemValidator::new_homogeneous(&matrix, &support).unwrap();
        for (force, solution) in [
            (vec![0.0; 12], vec![0.0; 12]),
            (vec![1.0; 12], vec![0.0; 12]),
            (vec![0.0; 12], vec![1.0; 12]),
        ] {
            assert_eq!(
                gate.validate_original(&matrix, &force, &solution).is_ok(),
                reference.validate(&force, &solution).is_ok()
            );
        }
    }
}

#[test]
fn projected_original_equations_validate_complete_inputs_before_skipping_reaction_rows() {
    let gate = ThermalFrame3dConstraintSystem::build(&request(
        false,
        &[[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        2,
    ))
    .unwrap();
    let mut matrix = diagonal(12, 1.0);
    let mut force = vec![0.0; 12];
    let mut solution = force.clone();
    assert!(
        gate.validate_original(&matrix, &force[..11], &solution)
            .is_err()
    );
    assert!(
        gate.validate_original(&matrix, &force, &solution[..11])
            .is_err()
    );
    assert!(
        gate.validate_original(&SparseMatrix::new(11), &force, &solution)
            .is_err()
    );
    force[0] = f64::NAN;
    assert!(gate.validate_original(&matrix, &force, &solution).is_err());
    force[0] = 0.0;
    solution[0] = f64::INFINITY;
    assert!(gate.validate_original(&matrix, &force, &solution).is_err());
    solution[0] = 0.0;
    add_at(&mut matrix, 0, 12, 1.0);
    assert!(gate.validate_original(&matrix, &force, &solution).is_err());
    let mut matrix = diagonal(12, 1.0);
    add_at(&mut matrix, 0, 0, f64::NAN);
    assert!(gate.validate_original(&matrix, &force, &solution).is_err());
}

#[test]
fn projected_original_equations_merge_coefficients_before_measuring_equilibrium() {
    let gate =
        ThermalFrame3dConstraintSystem::build(&request(false, &[[1.0, 1.0, 0.0]], 2)).unwrap();
    let mut matrix = SparseMatrix::new(12);
    for row in [6, 7] {
        for column in [6, 7] {
            add_at(&mut matrix, row, column, 2.0_f64.powi(600));
        }
    }
    add_at(&mut matrix, 6, 8, 1.0);
    let mut solution = vec![0.0; 12];
    solution[6..9].copy_from_slice(&[1.0, -1.0, 0.0]);
    gate.validate_original(&matrix, &[0.0; 12], &solution)
        .unwrap();
    // A large exactly cancelled reaction coefficient must not hide the weak z equation.
    solution[8] = 1.0;
    assert!(
        gate.validate_original(&matrix, &[0.0; 12], &solution)
            .is_err()
    );
}

#[test]
fn projected_original_equations_fail_closed_when_projection_normalization_loses_range() {
    let mut matrix = diagonal(12, 2.0_f64.powi(900));
    add_at(&mut matrix, 6, 0, f64::from_bits(1));
    let direction = [2.0_f64.sqrt().recip(), -2.0_f64.sqrt().recip(), 0.0];
    assert!(
        projected_row(&matrix, &[0.0; 12], 6, direction, &mut vec![])
            .unwrap_err()
            .contains("numerical range")
    );
    let mut matrix = diagonal(12, 1.0);
    add_at(&mut matrix, 6, 0, f64::MIN_POSITIVE);
    assert!(
        projected_row(&matrix, &[0.0; 12], 6, direction, &mut vec![])
            .unwrap_err()
            .contains("numerical range")
    );
}

#[test]
fn projected_original_equations_keep_direction_identity_scaled_and_axis_identity_exact() {
    let direction = [2.0_f64.sqrt().recip(), 2.0_f64.sqrt().recip(), 0.0];
    for value in [f64::from_bits(1), 1.0, 1.0e308] {
        validate_direction(direction, [value, -value, 0.0]).unwrap();
        validate_direction(direction, [value, -value, f64::MAX]).unwrap();
        assert!(validate_direction(direction, [value, 0.0, 0.0]).is_err());
        assert!(validate_direction([1.0, 0.0, 0.0], [value, 0.0, 0.0]).is_err());
    }
}

#[test]
fn projected_original_equations_cancel_wide_rows_and_completion_then_replay() {
    let count = 342;
    let size = count * 6;
    let gate =
        ThermalFrame3dConstraintSystem::build(&request(false, &[[1.0, 1.0, 0.0]], count)).unwrap();
    let mut matrix = diagonal(size, 1.0);
    for column in 12..size {
        add_at(&mut matrix, 6, column, 1.0);
        add_at(&mut matrix, 7, column, 1.0);
    }
    let mut force = vec![0.0; size];
    force[6..8].copy_from_slice(&[5.0, -1.0]);
    let mut solution = vec![0.0; size];
    solution[6..8].copy_from_slice(&[1.0, -1.0]);
    for (stage, completed, occurrence) in [
        (SolverStage::ConstraintValidate, 0, 1),
        (SolverStage::ConstraintValidateRow, 1024, 1),
        (SolverStage::ConstraintValidateRow, 1024, 5),
        (SolverStage::ConstraintValidate, (3 * size) as u64, 1),
    ] {
        let control = SolverControl::default();
        let cancel = control.clone();
        let seen = Cell::new(0);
        let result = with_solver_observer(
            &control,
            move |point| {
                if point.stage == stage && point.completed_steps == completed {
                    seen.set(seen.get() + 1);
                    if seen.get() == occurrence {
                        cancel.request_cancel();
                    }
                }
            },
            || gate.validate_original(&matrix, &force, &solution),
        );
        assert!(result.unwrap_err().contains("cancelled"));
        assert!(control.was_interrupted());
        gate.validate_original(&matrix, &force, &solution).unwrap();
    }
}
