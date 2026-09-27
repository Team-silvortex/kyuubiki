//! Frozen pre-optimization cofactor implementation for tests and paired benchmarks.
//! Valid only for the ordinary, well-scaled fixtures where the old kernel works.

use super::{
    SolidTetra3dElementKernel, elasticity_matrix, multiply_6x6_6x12, multiply_12x6_6x12,
    validate_properties,
};
use kyuubiki_protocol::SolidTetra3dElementInput;

pub(super) fn kernel(
    points: [[f64; 3]; 4],
    element: &SolidTetra3dElementInput,
) -> Result<SolidTetra3dElementKernel, String> {
    validate_properties(element)?;
    if points.iter().flatten().any(|value| !value.is_finite()) {
        return Err(format!(
            "solid tetra element {} coordinates must be finite",
            element.id
        ));
    }
    let (volume, mean_ratio_quality, b) = geometry(points, &element.id)?;
    let d = elasticity_matrix(element.youngs_modulus, element.poisson_ratio);
    let db = multiply_6x6_6x12(&d, &b);
    let stiffness = multiply_12x6_6x12(&b, &db, volume);
    Ok(SolidTetra3dElementKernel {
        volume,
        mean_ratio_quality,
        b,
        d,
        stiffness,
    })
}

pub(super) fn geometry(
    points: [[f64; 3]; 4],
    id: &str,
) -> Result<(f64, f64, [[f64; 12]; 6]), String> {
    let matrix = points.map(|point| [1.0, point[0], point[1], point[2]]);
    let determinant = det4(&matrix);
    let volume = determinant.abs() / 6.0;
    if volume == 0.0 {
        return Err(format!("solid tetra element {id} has zero volume"));
    }
    let edge_squared_sum = [(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)]
        .into_iter()
        .map(|(a, b)| {
            (0..3)
                .map(|axis| (points[a][axis] - points[b][axis]).powi(2))
                .sum::<f64>()
        })
        .sum::<f64>();
    let mean_ratio_quality = 12.0 * (3.0 * volume).powf(2.0 / 3.0) / edge_squared_sum;
    if !mean_ratio_quality.is_finite() || mean_ratio_quality <= 1.0e-12 {
        return Err(format!(
            "solid tetra element {id} is near-degenerate (mean_ratio_quality={mean_ratio_quality:.6e})"
        ));
    }
    let inverse = invert4(matrix)?;
    let mut b = [[0.0; 12]; 6];
    for node in 0..4 {
        let bx = inverse[1][node];
        let by = inverse[2][node];
        let bz = inverse[3][node];
        let offset = node * 3;
        b[0][offset] = bx;
        b[1][offset + 1] = by;
        b[2][offset + 2] = bz;
        b[3][offset] = by;
        b[3][offset + 1] = bx;
        b[4][offset + 1] = bz;
        b[4][offset + 2] = by;
        b[5][offset] = bz;
        b[5][offset + 2] = bx;
    }
    Ok((volume, mean_ratio_quality, b))
}

fn det4(matrix: &[[f64; 4]; 4]) -> f64 {
    (0..4)
        .map(|column| {
            let sign = if column % 2 == 0 { 1.0 } else { -1.0 };
            sign * matrix[0][column] * det3(minor3(matrix, 0, column))
        })
        .sum()
}

fn det3(matrix: [[f64; 3]; 3]) -> f64 {
    matrix[0][0] * (matrix[1][1] * matrix[2][2] - matrix[1][2] * matrix[2][1])
        - matrix[0][1] * (matrix[1][0] * matrix[2][2] - matrix[1][2] * matrix[2][0])
        + matrix[0][2] * (matrix[1][0] * matrix[2][1] - matrix[1][1] * matrix[2][0])
}

fn minor3(matrix: &[[f64; 4]; 4], skip_row: usize, skip_column: usize) -> [[f64; 3]; 3] {
    let mut result = [[0.0; 3]; 3];
    let mut output_row = 0;
    for (row, values) in matrix.iter().enumerate() {
        if row == skip_row {
            continue;
        }
        let mut output_column = 0;
        for (column, value) in values.iter().enumerate() {
            if column != skip_column {
                result[output_row][output_column] = *value;
                output_column += 1;
            }
        }
        output_row += 1;
    }
    result
}

fn invert4(matrix: [[f64; 4]; 4]) -> Result<[[f64; 4]; 4], String> {
    let determinant = det4(&matrix);
    if determinant == 0.0 {
        return Err("solid tetra coordinate matrix is singular".to_string());
    }
    Ok(std::array::from_fn(|row| {
        std::array::from_fn(|column| {
            let sign = if (row + column) % 2 == 0 { 1.0 } else { -1.0 };
            sign * det3(minor3(&matrix, column, row)) / determinant
        })
    }))
}
