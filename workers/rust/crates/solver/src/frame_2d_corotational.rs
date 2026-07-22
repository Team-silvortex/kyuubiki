use crate::frame_2d_math::frame_dof_map;
use crate::frame_2d_stability::Frame2dStabilitySystem;
use crate::linear_algebra::{SparseMatrix, add_at, reduce_sparse_system, sparse_to_dense};
use crate::linear_banded::SymmetricBandCholesky;
use crate::linear_dense::solve_linear_system;
use kyuubiki_protocol::{Frame2dElementInput, Frame2dPDeltaStepResult, SolveFrame2dPDeltaRequest};

const MAX_ITERATIONS: usize = 32;
const RESIDUAL_TOLERANCE: f64 = 1.0e-8;
const MAX_DENSE_FALLBACK_DOFS: usize = 1_024;

pub(crate) fn solve_corotational_steps(
    request: &SolveFrame2dPDeltaRequest,
    system: &Frame2dStabilitySystem,
    initial_imperfection: &[f64],
    maximum_load_factor: f64,
    critical_factor: f64,
    load_steps: usize,
) -> Result<Vec<Frame2dPDeltaStepResult>, String> {
    let frame = &request.buckling.frame;
    let initial_positions = frame
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| {
            (
                node.x + initial_imperfection[index * 3],
                node.y + initial_imperfection[index * 3 + 1],
            )
        })
        .collect::<Vec<_>>();
    let mut displacement = vec![0.0; frame.nodes.len() * 3];
    let mut steps = Vec::with_capacity(load_steps);

    for step in 1..=load_steps {
        let load_factor = maximum_load_factor * step as f64 / load_steps as f64;
        let mut converged = false;
        let mut residual_norm = f64::INFINITY;
        let mut iterations = 0;

        for iteration in 1..=MAX_ITERATIONS {
            iterations = iteration;
            let (tangent, internal) =
                assemble_tangent_and_internal(&initial_positions, &frame.elements, &displacement)?;
            let residual = residual(&system.reference_force, &internal, load_factor);
            let (reduced_tangent, reduced_residual, free) =
                reduce_sparse_system(&tangent, &residual, &system.constrained_dofs);
            residual_norm =
                normalized_residual(&reduced_residual, &system.reference_force, load_factor);
            if residual_norm <= RESIDUAL_TOLERANCE {
                converged = true;
                break;
            }

            let delta = solve_tangent(&reduced_tangent, &reduced_residual)?;
            if !apply_backtracked_increment(
                &initial_positions,
                &frame.elements,
                &system.reference_force,
                &free,
                &delta,
                load_factor,
                residual_norm,
                &mut displacement,
            )? {
                break;
            }
        }

        steps.push(Frame2dPDeltaStepResult {
            step,
            load_factor,
            critical_factor_ratio: load_factor / critical_factor,
            iterations,
            converged,
            residual_norm,
            imperfection_amplification: imperfection_amplification(
                initial_imperfection,
                &displacement,
            ),
            max_incremental_displacement: max_translation(&displacement),
            displacements: displacement.clone(),
        });
        if !converged {
            break;
        }
    }
    Ok(steps)
}

fn assemble_tangent_and_internal(
    positions: &[(f64, f64)],
    elements: &[Frame2dElementInput],
    displacement: &[f64],
) -> Result<(SparseMatrix, Vec<f64>), String> {
    let mut tangent = SparseMatrix::new(displacement.len());
    let mut internal = vec![0.0; displacement.len()];
    for element in elements {
        let map = frame_dof_map(element.node_i, element.node_j);
        let element_displacement = gather(displacement, &map);
        let element_internal = element_internal_force(positions, element, &element_displacement)?;
        let element_tangent = analytic_tangent(positions, element, &element_displacement)?;
        for row in 0..6 {
            internal[map[row]] += element_internal[row];
            for column in 0..6 {
                add_at(
                    &mut tangent,
                    map[row],
                    map[column],
                    element_tangent[row][column],
                );
            }
        }
    }
    Ok((tangent, internal))
}

fn assemble_internal(
    positions: &[(f64, f64)],
    elements: &[Frame2dElementInput],
    displacement: &[f64],
) -> Result<Vec<f64>, String> {
    let mut internal = vec![0.0; displacement.len()];
    for element in elements {
        let map = frame_dof_map(element.node_i, element.node_j);
        let force = element_internal_force(positions, element, &gather(displacement, &map))?;
        for row in 0..6 {
            internal[map[row]] += force[row];
        }
    }
    Ok(internal)
}

fn element_internal_force(
    positions: &[(f64, f64)],
    element: &Frame2dElementInput,
    displacement: &[f64; 6],
) -> Result<[f64; 6], String> {
    let (xi, yi) = positions[element.node_i];
    let (xj, yj) = positions[element.node_j];
    let dx0 = xj - xi;
    let dy0 = yj - yi;
    let length0 = dx0.hypot(dy0);
    let dx = dx0 + displacement[3] - displacement[0];
    let dy = dy0 + displacement[4] - displacement[1];
    let length = dx.hypot(dy);
    if !(length0.is_finite() && length.is_finite() && length0 > 1.0e-12 && length > 1.0e-12) {
        return Err("corotational frame element collapsed or has invalid geometry".into());
    }
    let angle_change = normalize_angle(dy.atan2(dx) - dy0.atan2(dx0));
    let phi_i = displacement[2] - angle_change;
    let phi_j = displacement[5] - angle_change;
    let axial_force = element.youngs_modulus * element.area * (length - length0) / length0;
    let bending = element.youngs_modulus * element.moment_of_inertia / length0;
    let moment_i = bending * (4.0 * phi_i + 2.0 * phi_j);
    let moment_j = bending * (2.0 * phi_i + 4.0 * phi_j);
    let c = dx / length;
    let s = dy / length;
    let shear = (moment_i + moment_j) / length;
    Ok([
        -axial_force * c - shear * s,
        -axial_force * s + shear * c,
        moment_i,
        axial_force * c + shear * s,
        axial_force * s - shear * c,
        moment_j,
    ])
}

fn analytic_tangent(
    positions: &[(f64, f64)],
    element: &Frame2dElementInput,
    displacement: &[f64; 6],
) -> Result<[[f64; 6]; 6], String> {
    let (xi, yi) = positions[element.node_i];
    let (xj, yj) = positions[element.node_j];
    let dx0 = xj - xi;
    let dy0 = yj - yi;
    let length0 = dx0.hypot(dy0);
    let dx = dx0 + displacement[3] - displacement[0];
    let dy = dy0 + displacement[4] - displacement[1];
    let length = dx.hypot(dy);
    if !(length0.is_finite() && length.is_finite() && length0 > 1.0e-12 && length > 1.0e-12) {
        return Err("corotational frame element collapsed or has invalid geometry".into());
    }

    let c = dx / length;
    let s = dy / length;
    let angle_change = normalize_angle(dy.atan2(dx) - dy0.atan2(dx0));
    let phi_i = displacement[2] - angle_change;
    let phi_j = displacement[5] - angle_change;
    let axial_stiffness = element.youngs_modulus * element.area / length0;
    let bending = element.youngs_modulus * element.moment_of_inertia / length0;
    let axial_force = axial_stiffness * (length - length0);
    let moment_i = bending * (4.0 * phi_i + 2.0 * phi_j);
    let moment_j = bending * (2.0 * phi_i + 4.0 * phi_j);

    let axial_gradient = [-c, -s, 0.0, c, s, 0.0];
    let angle_gradient = [s / length, -c / length, 0.0, -s / length, c / length, 0.0];
    let mut rotation_i_gradient = angle_gradient.map(|value| -value);
    let mut rotation_j_gradient = rotation_i_gradient;
    rotation_i_gradient[2] += 1.0;
    rotation_j_gradient[5] += 1.0;

    let mut tangent = [[0.0; 6]; 6];
    add_outer(
        &mut tangent,
        &axial_gradient,
        &axial_gradient,
        axial_stiffness,
    );
    add_outer(
        &mut tangent,
        &rotation_i_gradient,
        &rotation_i_gradient,
        4.0 * bending,
    );
    add_outer(
        &mut tangent,
        &rotation_i_gradient,
        &rotation_j_gradient,
        2.0 * bending,
    );
    add_outer(
        &mut tangent,
        &rotation_j_gradient,
        &rotation_i_gradient,
        2.0 * bending,
    );
    add_outer(
        &mut tangent,
        &rotation_j_gradient,
        &rotation_j_gradient,
        4.0 * bending,
    );

    let length_hessian = [
        [s * s / length, -s * c / length],
        [-s * c / length, c * c / length],
    ];
    let angle_hessian = [
        [
            2.0 * s * c / length.powi(2),
            (s * s - c * c) / length.powi(2),
        ],
        [
            (s * s - c * c) / length.powi(2),
            -2.0 * s * c / length.powi(2),
        ],
    ];
    let translation_dofs = [(0, 0, -1.0), (1, 1, -1.0), (3, 0, 1.0), (4, 1, 1.0)];
    for &(row, row_axis, row_sign) in &translation_dofs {
        for &(column, column_axis, column_sign) in &translation_dofs {
            tangent[row][column] += row_sign
                * column_sign
                * (axial_force * length_hessian[row_axis][column_axis]
                    - (moment_i + moment_j) * angle_hessian[row_axis][column_axis]);
        }
    }
    Ok(tangent)
}

fn add_outer(matrix: &mut [[f64; 6]; 6], left: &[f64; 6], right: &[f64; 6], scale: f64) {
    for row in 0..6 {
        for column in 0..6 {
            matrix[row][column] += scale * left[row] * right[column];
        }
    }
}

#[cfg(test)]
fn numerical_tangent(
    positions: &[(f64, f64)],
    element: &Frame2dElementInput,
    displacement: &[f64; 6],
) -> Result<[[f64; 6]; 6], String> {
    let (xi, yi) = positions[element.node_i];
    let (xj, yj) = positions[element.node_j];
    let length = (xj - xi).hypot(yj - yi).max(1.0);
    let mut tangent = [[0.0; 6]; 6];
    for column in 0..6 {
        let epsilon = if column == 2 || column == 5 {
            1.0e-7
        } else {
            length * 1.0e-7
        };
        let mut plus = *displacement;
        let mut minus = *displacement;
        plus[column] += epsilon;
        minus[column] -= epsilon;
        let force_plus = element_internal_force(positions, element, &plus)?;
        let force_minus = element_internal_force(positions, element, &minus)?;
        for row in 0..6 {
            tangent[row][column] = (force_plus[row] - force_minus[row]) / (2.0 * epsilon);
        }
    }
    Ok(tangent)
}

fn apply_backtracked_increment(
    positions: &[(f64, f64)],
    elements: &[Frame2dElementInput],
    external: &[f64],
    free: &[usize],
    delta: &[f64],
    load_factor: f64,
    current_norm: f64,
    displacement: &mut Vec<f64>,
) -> Result<bool, String> {
    let mut scale = 1.0;
    for _ in 0..10 {
        let mut trial = displacement.clone();
        for (index, &dof) in free.iter().enumerate() {
            trial[dof] += scale * delta[index];
        }
        let internal = assemble_internal(positions, elements, &trial)?;
        let trial_residual = residual(external, &internal, load_factor);
        let reduced = free
            .iter()
            .map(|&dof| trial_residual[dof])
            .collect::<Vec<_>>();
        if normalized_residual(&reduced, external, load_factor) < current_norm {
            *displacement = trial;
            return Ok(true);
        }
        scale *= 0.5;
    }
    Ok(false)
}

fn solve_tangent(matrix: &SparseMatrix, rhs: &[f64]) -> Result<Vec<f64>, String> {
    if let Ok(Some(factor)) = SymmetricBandCholesky::try_factor(matrix, 8_000_000) {
        return factor.solve(rhs);
    }
    if rhs.len() <= MAX_DENSE_FALLBACK_DOFS {
        return solve_linear_system(sparse_to_dense(matrix), rhs.to_vec());
    }
    Err("corotational frame tangent is not positive definite at this load step".into())
}

fn residual(external: &[f64], internal: &[f64], load_factor: f64) -> Vec<f64> {
    external
        .iter()
        .zip(internal)
        .map(|(external, internal)| load_factor * external - internal)
        .collect()
}

fn normalized_residual(residual: &[f64], external: &[f64], load_factor: f64) -> f64 {
    let numerator = residual.iter().map(|value| value.abs()).fold(0.0, f64::max);
    let denominator = external
        .iter()
        .map(|value| (load_factor * value).abs())
        .fold(1.0, f64::max);
    numerator / denominator
}

fn gather(values: &[f64], map: &[usize; 6]) -> [f64; 6] {
    [
        values[map[0]],
        values[map[1]],
        values[map[2]],
        values[map[3]],
        values[map[4]],
        values[map[5]],
    ]
}

fn normalize_angle(angle: f64) -> f64 {
    angle.sin().atan2(angle.cos())
}

fn imperfection_amplification(initial: &[f64], displacement: &[f64]) -> f64 {
    let mut numerator = 0.0;
    let mut denominator = 0.0;
    for node in 0..initial.len() / 3 {
        for offset in 0..2 {
            numerator += initial[node * 3 + offset] * displacement[node * 3 + offset];
            denominator += initial[node * 3 + offset].powi(2);
        }
    }
    1.0 + numerator / denominator.max(f64::MIN_POSITIVE)
}

fn max_translation(displacements: &[f64]) -> f64 {
    (0..displacements.len() / 3)
        .map(|node| (displacements[node * 3].powi(2) + displacements[node * 3 + 1].powi(2)).sqrt())
        .fold(0.0_f64, f64::max)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rigid_rotation_produces_no_internal_force() {
        let angle: f64 = 0.47;
        let length: f64 = 2.3;
        let positions = [(0.0, 0.0), (length, 0.0)];
        let element = Frame2dElementInput {
            id: "rigid-motion".into(),
            node_i: 0,
            node_j: 1,
            area: 0.01,
            youngs_modulus: 210.0e9,
            moment_of_inertia: 1.0e-5,
            section_modulus: 1.0e-4,
        };
        let displacement = [
            0.0,
            0.0,
            angle,
            length * angle.cos() - length,
            length * angle.sin(),
            angle,
        ];

        let internal = element_internal_force(&positions, &element, &displacement).unwrap();
        assert!(internal.iter().all(|value| value.abs() < 1.0e-5));
    }

    #[test]
    fn analytic_tangent_matches_central_difference() {
        let positions = [(0.2, -0.1), (2.1, 1.2)];
        let element = Frame2dElementInput {
            id: "tangent-check".into(),
            node_i: 0,
            node_j: 1,
            area: 0.01,
            youngs_modulus: 210.0e9,
            moment_of_inertia: 1.0e-5,
            section_modulus: 1.0e-4,
        };
        let displacement = [0.03, -0.01, 0.04, -0.02, 0.05, -0.03];
        let analytic = analytic_tangent(&positions, &element, &displacement).unwrap();
        let numerical = numerical_tangent(&positions, &element, &displacement).unwrap();

        for row in 0..6 {
            for column in 0..6 {
                let scale = numerical[row][column].abs().max(1.0);
                let relative = (analytic[row][column] - numerical[row][column]).abs() / scale;
                assert!(
                    relative < 2.0e-7,
                    "tangent[{row}][{column}] analytic={}, numerical={}, relative={relative}",
                    analytic[row][column],
                    numerical[row][column]
                );
            }
        }
    }
}
