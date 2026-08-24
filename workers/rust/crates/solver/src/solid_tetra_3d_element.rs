use kyuubiki_protocol::{SolidTetra3dElementInput, SolidTetra3dElementResult};

use crate::linear_algebra::{MatrixAssembler, add_at};

pub(crate) struct SolidTetra3dElementKernel {
    volume: f64,
    mean_ratio_quality: f64,
    b: [[f64; 12]; 6],
    d: [[f64; 6]; 6],
    stiffness: [[f64; 12]; 12],
}

impl SolidTetra3dElementKernel {
    pub(crate) fn new(
        points: [[f64; 3]; 4],
        element: &SolidTetra3dElementInput,
    ) -> Result<Self, String> {
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
        Ok(Self {
            volume,
            mean_ratio_quality,
            b,
            d,
            stiffness,
        })
    }

    pub(crate) fn assemble<M: MatrixAssembler + ?Sized>(
        &self,
        dofs: &[usize; 12],
        displacements: &[f64],
        internal_forces: &mut [f64],
        tangent: &mut M,
    ) {
        for row in 0..12 {
            let internal = (0..12)
                .map(|column| self.stiffness[row][column] * displacements[dofs[column]])
                .sum::<f64>();
            internal_forces[dofs[row]] += internal;
            for column in 0..12 {
                add_at(
                    tangent,
                    dofs[row],
                    dofs[column],
                    self.stiffness[row][column],
                );
            }
        }
    }

    pub(crate) fn result(
        &self,
        index: usize,
        element: &SolidTetra3dElementInput,
        dofs: &[usize; 12],
        displacements: &[f64],
    ) -> SolidTetra3dElementResult {
        let local = std::array::from_fn(|i| displacements[dofs[i]]);
        let strain = multiply_6x12_12(&self.b, &local);
        let stress = multiply_6x6_6(&self.d, &strain);
        SolidTetra3dElementResult {
            index,
            id: element.id.clone(),
            node_a: element.node_a,
            node_b: element.node_b,
            node_c: element.node_c,
            node_d: element.node_d,
            volume: self.volume,
            strain_x: strain[0],
            strain_y: strain[1],
            strain_z: strain[2],
            gamma_xy: strain[3],
            gamma_yz: strain[4],
            gamma_zx: strain[5],
            stress_x: stress[0],
            stress_y: stress[1],
            stress_z: stress[2],
            shear_xy: stress[3],
            shear_yz: stress[4],
            shear_zx: stress[5],
            von_mises_stress: von_mises_stress(&stress),
            strain_energy_density: strain_energy_density(&stress, &strain),
            mean_ratio_quality: self.mean_ratio_quality,
        }
    }
}

pub(crate) fn element_dof_map(element: &SolidTetra3dElementInput) -> [usize; 12] {
    let nodes = [
        element.node_a,
        element.node_b,
        element.node_c,
        element.node_d,
    ];
    std::array::from_fn(|local_dof| nodes[local_dof / 3] * 3 + local_dof % 3)
}

fn validate_properties(element: &SolidTetra3dElementInput) -> Result<(), String> {
    if !element.youngs_modulus.is_finite() || element.youngs_modulus <= 0.0 {
        return Err(format!(
            "solid tetra element {} must have finite positive youngs_modulus",
            element.id
        ));
    }
    if !(element.poisson_ratio.is_finite()
        && element.poisson_ratio > -1.0
        && element.poisson_ratio < 0.5)
    {
        return Err(format!(
            "solid tetra element {} must have poisson_ratio in (-1, 0.5)",
            element.id
        ));
    }
    Ok(())
}

fn geometry(points: [[f64; 3]; 4], id: &str) -> Result<(f64, f64, [[f64; 12]; 6]), String> {
    let matrix = points.map(|point| [1.0, point[0], point[1], point[2]]);
    let determinant = det4(&matrix);
    let volume = determinant.abs() / 6.0;
    if volume == 0.0 {
        return Err(format!("solid tetra element {id} has zero volume"));
    }
    let mean_ratio_quality = tetra_mean_ratio_quality(&points, volume);
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

fn tetra_mean_ratio_quality(points: &[[f64; 3]; 4], volume: f64) -> f64 {
    let edge_squared_sum = [(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)]
        .into_iter()
        .map(|(a, b)| {
            (0..3)
                .map(|axis| (points[a][axis] - points[b][axis]).powi(2))
                .sum::<f64>()
        })
        .sum::<f64>();
    12.0 * (3.0 * volume).powf(2.0 / 3.0) / edge_squared_sum
}

fn elasticity_matrix(youngs_modulus: f64, poisson_ratio: f64) -> [[f64; 6]; 6] {
    let factor = youngs_modulus / ((1.0 + poisson_ratio) * (1.0 - 2.0 * poisson_ratio));
    let normal = 1.0 - poisson_ratio;
    let shear = (1.0 - 2.0 * poisson_ratio) * 0.5;
    [
        [
            factor * normal,
            factor * poisson_ratio,
            factor * poisson_ratio,
            0.0,
            0.0,
            0.0,
        ],
        [
            factor * poisson_ratio,
            factor * normal,
            factor * poisson_ratio,
            0.0,
            0.0,
            0.0,
        ],
        [
            factor * poisson_ratio,
            factor * poisson_ratio,
            factor * normal,
            0.0,
            0.0,
            0.0,
        ],
        [0.0, 0.0, 0.0, factor * shear, 0.0, 0.0],
        [0.0, 0.0, 0.0, 0.0, factor * shear, 0.0],
        [0.0, 0.0, 0.0, 0.0, 0.0, factor * shear],
    ]
}

fn multiply_6x6_6x12(a: &[[f64; 6]; 6], b: &[[f64; 12]; 6]) -> [[f64; 12]; 6] {
    std::array::from_fn(|row| {
        std::array::from_fn(|column| (0..6).map(|k| a[row][k] * b[k][column]).sum())
    })
}

fn multiply_12x6_6x12(b: &[[f64; 12]; 6], db: &[[f64; 12]; 6], volume: f64) -> [[f64; 12]; 12] {
    std::array::from_fn(|row| {
        std::array::from_fn(|column| {
            (0..6).map(|k| b[k][row] * db[k][column]).sum::<f64>() * volume
        })
    })
}

fn multiply_6x12_12(a: &[[f64; 12]; 6], vector: &[f64; 12]) -> [f64; 6] {
    std::array::from_fn(|row| (0..12).map(|column| a[row][column] * vector[column]).sum())
}

fn multiply_6x6_6(a: &[[f64; 6]; 6], vector: &[f64; 6]) -> [f64; 6] {
    std::array::from_fn(|row| (0..6).map(|column| a[row][column] * vector[column]).sum())
}

fn von_mises_stress(stress: &[f64; 6]) -> f64 {
    let [sx, sy, sz, txy, tyz, tzx] = *stress;
    (0.5 * ((sx - sy).powi(2) + (sy - sz).powi(2) + (sz - sx).powi(2))
        + 3.0 * (txy * txy + tyz * tyz + tzx * tzx))
        .sqrt()
}

fn strain_energy_density(stress: &[f64; 6], strain: &[f64; 6]) -> f64 {
    0.5 * (0..6)
        .map(|index| stress[index] * strain[index])
        .sum::<f64>()
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

#[cfg(test)]
mod tests {
    use super::tetra_mean_ratio_quality;

    #[test]
    fn mean_ratio_is_one_for_a_regular_tetra_and_scale_invariant() {
        let regular = [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.5, 3.0_f64.sqrt() / 2.0, 0.0],
            [0.5, 3.0_f64.sqrt() / 6.0, (2.0_f64 / 3.0).sqrt()],
        ];
        let volume = 2.0_f64.sqrt() / 12.0;
        let quality = tetra_mean_ratio_quality(&regular, volume);
        assert!((quality - 1.0).abs() <= 1.0e-14);

        let scale = 1.0e-9;
        let microscopic = regular.map(|point| point.map(|coordinate| coordinate * scale));
        let microscopic_quality = tetra_mean_ratio_quality(&microscopic, volume * scale.powi(3));
        assert!((microscopic_quality - quality).abs() <= 1.0e-14);
    }
}
