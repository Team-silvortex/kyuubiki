use crate::frame_3d_element::{finite_fields, norm3};
use crate::solver_control::{SolverStage, checkpoint};

const DIRECTION_TOLERANCE: f64 = 1.0e-10;

// N = Q R, with the normalized support directions as columns of N.
// Stack-sized factors avoid a persistent per-node matrix or normal equations.
struct ConstraintBasis {
    axes: [[f64; 3]; 3],
    upper: [[f64; 3]; 3],
    rank: usize,
}

impl ConstraintBasis {
    fn factor(directions: &[[f64; 3]]) -> Result<Self, String> {
        if directions.len() > 3 {
            return Err(
                "contains linearly dependent directions (more than three per block)".into(),
            );
        }
        let mut basis = Self {
            axes: [[0.0; 3]; 3],
            upper: [[0.0; 3]; 3],
            rank: directions.len(),
        };
        for (column, &direction) in directions.iter().enumerate() {
            let (candidate, coefficients) = orthogonalize(direction, &basis.axes[..column]);
            let norm = norm3(candidate);
            if !norm.is_finite() || norm <= DIRECTION_TOLERANCE {
                return Err("contains linearly dependent directions".into());
            }
            basis.axes[column] = candidate.map(|v| v / norm);
            for (row, coefficient) in coefficients.into_iter().enumerate().take(column) {
                basis.upper[row][column] = coefficient;
            }
            basis.upper[column][column] = norm;
        }
        Ok(basis)
    }

    fn free_basis(mut self) -> Result<Vec<[f64; 3]>, String> {
        let mut count = self.rank;
        let mut free = Vec::with_capacity(3 - self.rank);
        for axis in 0..3 {
            if count == 3 {
                break;
            }
            let direction = std::array::from_fn(|index| (index == axis) as u8 as f64);
            let (candidate, _) = orthogonalize(direction, &self.axes[..count]);
            let norm = norm3(candidate);
            if norm > DIRECTION_TOLERANCE {
                let unit = candidate.map(|v| v / norm);
                self.axes[count] = unit;
                free.push(unit);
                count += 1;
            }
        }
        if count != 3 {
            return Err("could not construct an orthogonal free basis".into());
        }
        Ok(free)
    }
}

pub(super) fn free_basis(directions: &[[f64; 3]]) -> Result<Vec<[f64; 3]>, String> {
    ConstraintBasis::factor(directions)?.free_basis()
}

pub(super) fn reaction_coefficients(
    directions: &[[f64; 3]],
    residual: [f64; 3],
) -> Result<Vec<f64>, String> {
    if directions.is_empty() {
        return Ok(Vec::new());
    }
    checkpoint(SolverStage::DenseFactor, 0)?;
    let basis = ConstraintBasis::factor(directions)
        .map_err(|error| format!("could not recover exact constraint reactions: {error}"))?;
    checkpoint(SolverStage::DenseFactor, basis.rank)?;
    checkpoint(SolverStage::DenseSubstitution, 0)?;
    let mut coefficients = vec![0.0; basis.rank];
    for row in (0..basis.rank).rev() {
        let mut value = dot(basis.axes[row], residual);
        for (column, &coefficient) in coefficients.iter().enumerate().skip(row + 1) {
            value = (-basis.upper[row][column]).mul_add(coefficient, value);
        }
        coefficients[row] = value / basis.upper[row][row];
        finite_fields("constraint reaction", "coefficient", &[coefficients[row]])?;
        checkpoint(SolverStage::DenseSubstitution, basis.rank - row)?;
    }
    Ok(coefficients)
}

fn orthogonalize(mut vector: [f64; 3], basis: &[[f64; 3]]) -> ([f64; 3], [f64; 3]) {
    let mut coefficients = [0.0; 3];
    // Reorthogonalize before normalizing a small remainder. A single pass can
    // turn roundoff along an existing axis into a spurious independent DOF.
    for _ in 0..2 {
        for (column, &axis) in basis.iter().enumerate() {
            let projection = dot(vector, axis);
            coefficients[column] += projection;
            for index in 0..3 {
                vector[index] = (-projection).mul_add(axis[index], vector[index]);
            }
        }
    }
    (vector, coefficients)
}

pub(super) fn dot(left: [f64; 3], right: [f64; 3]) -> f64 {
    left.into_iter().zip(right).map(|(a, b)| a * b).sum()
}
