use crate::frame_2d_math::frame_dof_map;
use crate::linear_algebra::{SparseMatrix, add_at};
use kyuubiki_protocol::Frame2dElementInput;

pub(crate) fn assemble_tangent_and_internal(
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

pub(crate) fn assemble_internal(
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
    let delta_x = displacement[3] - displacement[0];
    let delta_y = displacement[4] - displacement[1];
    let dx = dx0 + delta_x;
    let dy = dy0 + delta_y;
    let length = dx.hypot(dy);
    validate_geometry(length0, length)?;
    let angle_change = relative_angle(dx0, dy0, dx, dy);
    let phi_i = displacement[2] - angle_change;
    let phi_j = displacement[5] - angle_change;
    let extension = stable_length_change(dx0, dy0, delta_x, delta_y, length0, length);
    let axial_force = element.youngs_modulus * element.area * extension / length0;
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
    let delta_x = displacement[3] - displacement[0];
    let delta_y = displacement[4] - displacement[1];
    let dx = dx0 + delta_x;
    let dy = dy0 + delta_y;
    let length = dx.hypot(dy);
    validate_geometry(length0, length)?;

    let c = dx / length;
    let s = dy / length;
    let angle_change = relative_angle(dx0, dy0, dx, dy);
    let phi_i = displacement[2] - angle_change;
    let phi_j = displacement[5] - angle_change;
    let axial_stiffness = element.youngs_modulus * element.area / length0;
    let bending = element.youngs_modulus * element.moment_of_inertia / length0;
    let axial_force =
        axial_stiffness * stable_length_change(dx0, dy0, delta_x, delta_y, length0, length);
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

fn validate_geometry(length0: f64, length: f64) -> Result<(), String> {
    if length0.is_finite() && length.is_finite() && length0 > 1.0e-12 && length > 1.0e-12 {
        Ok(())
    } else {
        Err("corotational frame element collapsed or has invalid geometry".into())
    }
}

fn add_outer(matrix: &mut [[f64; 6]; 6], left: &[f64; 6], right: &[f64; 6], scale: f64) {
    for row in 0..6 {
        for column in 0..6 {
            matrix[row][column] += scale * left[row] * right[column];
        }
    }
}

fn stable_length_change(
    dx0: f64,
    dy0: f64,
    delta_x: f64,
    delta_y: f64,
    length0: f64,
    length: f64,
) -> f64 {
    (2.0 * dx0 * delta_x + delta_x * delta_x + 2.0 * dy0 * delta_y + delta_y * delta_y)
        / (length + length0)
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

fn relative_angle(dx0: f64, dy0: f64, dx: f64, dy: f64) -> f64 {
    (dx0 * dy - dy0 * dx).atan2(dx0 * dx + dy0 * dy)
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

    #[test]
    fn stable_geometry_measures_tiny_short_element_changes() {
        let length0: f64 = 0.004;
        let tiny_extension: f64 = -1.0e-19;
        let rounded_length = (length0 + tiny_extension).hypot(0.0);
        let stable =
            stable_length_change(length0, 0.0, tiny_extension, 0.0, length0, rounded_length);
        assert_eq!(rounded_length - length0, 0.0);
        assert!((stable - tiny_extension).abs() < 1.0e-30);

        let angle = 1.0e-12_f64;
        let dx = length0 * angle.cos();
        let dy = length0 * angle.sin();
        let measured = relative_angle(length0, 0.0, dx, dy);
        assert!((measured - angle).abs() < 1.0e-24);
    }
}
