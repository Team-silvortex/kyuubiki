//! Previous precompute path, for ordinary fixtures and paired timing only.
//! Shape functions/Jacobian evaluation and the constitutive law are unchanged
//! helpers; absolute-coordinate evaluation and nine-term accumulation stay here.

use super::{GAUSS_COORDINATE, PlaneQuadComputed, plane_stress_d_matrix, quad_gauss_point};

pub(super) fn compute(
    coordinates: [[f64; 2]; 4],
    thickness: f64,
    youngs_modulus: f64,
    poisson_ratio: f64,
) -> Result<PlaneQuadComputed, String> {
    let d_matrix = plane_stress_d_matrix(youngs_modulus, poisson_ratio);
    let gauss_points = [
        quad_gauss_point(coordinates, -GAUSS_COORDINATE, -GAUSS_COORDINATE)?,
        quad_gauss_point(coordinates, GAUSS_COORDINATE, -GAUSS_COORDINATE)?,
        quad_gauss_point(coordinates, GAUSS_COORDINATE, GAUSS_COORDINATE)?,
        quad_gauss_point(coordinates, -GAUSS_COORDINATE, GAUSS_COORDINATE)?,
    ];
    let mut stiffness = [[0.0; 8]; 8];
    let mut area = 0.0;
    for point in &gauss_points {
        area += point.det_jacobian;
        let b_matrix = &point.b_matrix;
        let d_matrix = &d_matrix;
        let scale = thickness * point.det_jacobian;
        for (row, stiffness_row) in stiffness.iter_mut().enumerate() {
            for (column, stiffness_value) in stiffness_row.iter_mut().enumerate() {
                let value = (0..3)
                    .flat_map(|left| {
                        (0..3).map(move |right| {
                            b_matrix[left][row] * d_matrix[left][right] * b_matrix[right][column]
                        })
                    })
                    .sum::<f64>();
                *stiffness_value += value * scale;
            }
        }
    }
    Ok(PlaneQuadComputed {
        stiffness,
        area,
        gauss_points,
        d_matrix,
    })
}
