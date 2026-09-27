//! Frozen pre-fix dense CST precompute for paired benchmarks on positive orientation only.

use super::{PlaneTriangleComputed, plane_stress_d_matrix, signed_triangle_area_from_coordinates};

pub(super) fn compute(
    coordinates: [[f64; 2]; 3],
    thickness: f64,
    modulus: f64,
    poisson: f64,
) -> Result<PlaneTriangleComputed, String> {
    let [i, j, k] = coordinates;
    let area = signed_triangle_area_from_coordinates(coordinates).abs();
    if area <= 1e-12 {
        return Err("plane element area must be positive".into());
    }
    let b1 = j[1] - k[1];
    let b2 = k[1] - i[1];
    let b3 = i[1] - j[1];
    let c1 = k[0] - j[0];
    let c2 = i[0] - k[0];
    let c3 = j[0] - i[0];
    let factor = 1.0 / (2.0 * area);
    let b_matrix = [
        [b1 * factor, 0.0, b2 * factor, 0.0, b3 * factor, 0.0],
        [0.0, c1 * factor, 0.0, c2 * factor, 0.0, c3 * factor],
        [
            c1 * factor,
            b1 * factor,
            c2 * factor,
            b2 * factor,
            c3 * factor,
            b3 * factor,
        ],
    ];
    let d_matrix = plane_stress_d_matrix(modulus, poisson);
    let mut bt = [[0.0; 3]; 6];
    for (row, input_row) in b_matrix.iter().enumerate() {
        for (column, value) in input_row.iter().enumerate() {
            bt[column][row] = *value;
        }
    }
    let mut bt_d = [[0.0; 3]; 6];
    for row in 0..6 {
        for column in 0..3 {
            bt_d[row][column] = (0..3)
                .map(|index| bt[row][index] * d_matrix[index][column])
                .sum();
        }
    }
    let mut stiffness = [[0.0; 6]; 6];
    for row in 0..6 {
        for column in 0..6 {
            stiffness[row][column] = (0..3)
                .map(|index| bt_d[row][index] * b_matrix[index][column])
                .sum();
        }
    }
    let scale = thickness * area;
    for row in &mut stiffness {
        for value in row {
            *value *= scale;
        }
    }
    Ok(PlaneTriangleComputed {
        stiffness,
        area,
        b_matrix,
        d_matrix,
    })
}
