use crate::solver_control::{SolverStage, checkpoint, checkpoint_chunk};

#[path = "harmonic_spring_complex_range.rs"]
pub(super) mod arithmetic;
use arithmetic::{difference, divide, solve_row};

#[cfg(test)]
#[path = "harmonic_spring_complex_range_tests.rs"]
mod range_tests;

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Complex {
    pub(super) re: f64,
    pub(super) im: f64,
}

pub(super) struct ComplexTridiagonalSystem {
    pub(super) diagonal: Vec<Complex>,
    pub(super) lower: Vec<Complex>,
    pub(super) upper: Vec<Complex>,
    pub(super) rhs: Vec<Complex>,
}

pub(super) fn solve_complex_tridiagonal(
    system: ComplexTridiagonalSystem,
) -> Result<Vec<Complex>, String> {
    checkpoint(SolverStage::LinearPrepare, 0)?;
    let ComplexTridiagonalSystem {
        mut diagonal,
        mut lower,
        mut upper,
        mut rhs,
    } = system;
    let size = diagonal.len();
    if size == 0 || rhs.len() != size || lower.len() != size - 1 || upper.len() != size - 1 {
        return Err("harmonic spring 1d dynamic stiffness dimensions do not match".to_string());
    }
    if diagonal
        .iter()
        .chain(&lower)
        .chain(&upper)
        .chain(&rhs)
        .any(|value| !value.is_finite())
    {
        return Err("harmonic spring 1d dynamic stiffness contains a non-finite value".to_string());
    }

    let row_scales = (0..size)
        .map(|row| {
            checkpoint_chunk(SolverStage::LinearPrepare, row + 1, size)?;
            let mut scale = diagonal[row].component_scale();
            if row > 0 {
                scale = scale.max(lower[row - 1].component_scale());
            }
            if row + 1 < size {
                scale = scale.max(upper[row].component_scale());
            }
            Ok(scale)
        })
        .collect::<Result<Vec<_>, String>>()?;
    if row_scales.contains(&0.0) {
        return Err("harmonic spring 1d dynamic stiffness is singular".to_string());
    }
    for row in 0..size {
        let scale = row_scales[row];
        diagonal[row] = diagonal[row].scaled(scale);
        rhs[row] = rhs[row].scaled(scale);
        if row > 0 {
            lower[row - 1] = lower[row - 1].scaled(scale);
        }
        if row + 1 < size {
            upper[row] = upper[row].scaled(scale);
        }
        checkpoint_chunk(SolverStage::LinearPrepare, row + 1, size)?;
    }

    let relative_pivot_floor = 32.0 * f64::EPSILON * size as f64;
    checkpoint(SolverStage::TridiagonalFactor, 0)?;
    for row in 0..size - 1 {
        let diagonal_amplitude = diagonal[row].amplitude();
        let lower_amplitude = lower[row].amplitude();
        if diagonal_amplitude >= lower_amplitude {
            if diagonal_amplitude <= relative_pivot_floor {
                return Err("harmonic spring 1d dynamic stiffness is singular".to_string());
            }
            let factor = divide(lower[row], diagonal[row], SolverStage::TridiagonalFactor)?;
            diagonal[row + 1] = difference(
                diagonal[row + 1],
                factor,
                upper[row],
                SolverStage::TridiagonalFactor,
            )?;
            rhs[row + 1] = difference(
                rhs[row + 1],
                factor,
                rhs[row],
                SolverStage::TridiagonalFactor,
            )?;
            lower[row] = Complex::default();
        } else {
            if lower_amplitude <= relative_pivot_floor {
                return Err("harmonic spring 1d dynamic stiffness is singular".to_string());
            }
            let factor = divide(diagonal[row], lower[row], SolverStage::TridiagonalFactor)?;
            let next_diagonal = diagonal[row + 1];
            diagonal[row] = lower[row];
            diagonal[row + 1] = difference(
                upper[row],
                factor,
                next_diagonal,
                SolverStage::TridiagonalFactor,
            )?;
            upper[row] = next_diagonal;
            let current_rhs = rhs[row];
            rhs[row] = rhs[row + 1];
            rhs[row + 1] = difference(
                current_rhs,
                factor,
                rhs[row + 1],
                SolverStage::TridiagonalFactor,
            )?;
            if row + 1 < size - 1 {
                lower[row] = upper[row + 1];
                upper[row + 1] = difference(
                    Complex::default(),
                    factor,
                    lower[row],
                    SolverStage::TridiagonalFactor,
                )?;
            } else {
                lower[row] = Complex::default();
            }
        }
        if !diagonal[row + 1].is_finite() || !rhs[row + 1].is_finite() {
            return Err("harmonic spring 1d dynamic stiffness elimination diverged".to_string());
        }
        checkpoint_chunk(SolverStage::TridiagonalFactor, row + 1, size - 1)?;
    }
    if diagonal[size - 1].amplitude() <= relative_pivot_floor {
        return Err("harmonic spring 1d dynamic stiffness is singular".to_string());
    }

    checkpoint(SolverStage::TridiagonalSubstitution, 0)?;
    rhs[size - 1] = divide(
        rhs[size - 1],
        diagonal[size - 1],
        SolverStage::TridiagonalSubstitutionRange,
    )?;
    if size > 1 {
        rhs[size - 2] = solve_row(
            rhs[size - 2],
            std::iter::once((upper[size - 2], rhs[size - 1])),
            diagonal[size - 2],
            SolverStage::TridiagonalSubstitutionRange,
        )?;
    }
    for row in (0..size.saturating_sub(2)).rev() {
        rhs[row] = solve_row(
            rhs[row],
            [(upper[row], rhs[row + 1]), (lower[row], rhs[row + 2])].into_iter(),
            diagonal[row],
            SolverStage::TridiagonalSubstitutionRange,
        )?;
        checkpoint_chunk(SolverStage::TridiagonalSubstitution, size - row, size)?;
    }
    checkpoint(SolverStage::TridiagonalSubstitution, size)?;
    if rhs.iter().any(|value| !value.is_finite()) {
        return Err("harmonic spring 1d dynamic stiffness back substitution diverged".to_string());
    }
    Ok(rhs)
}

pub(super) fn solve_complex_system(
    mut matrix: Vec<Vec<Complex>>,
    mut rhs: Vec<Complex>,
) -> Result<Vec<Complex>, String> {
    checkpoint(SolverStage::LinearPrepare, 0)?;
    let size = rhs.len();
    if size == 0 || matrix.len() != size || matrix.iter().any(|row| row.len() != size) {
        return Err("harmonic spring 1d dynamic stiffness dimensions do not match".to_string());
    }
    if matrix.iter().flatten().any(|value| !value.is_finite())
        || rhs.iter().any(|value| !value.is_finite())
    {
        return Err("harmonic spring 1d dynamic stiffness contains a non-finite value".to_string());
    }
    for (index, (row, rhs)) in matrix.iter_mut().zip(&mut rhs).enumerate() {
        let scale = row
            .iter()
            .map(|value| value.component_scale())
            .fold(0.0, f64::max);
        if scale == 0.0 {
            return Err("harmonic spring 1d dynamic stiffness is singular".to_string());
        }
        for value in row {
            *value = value.scaled(scale);
        }
        *rhs = rhs.scaled(scale);
        checkpoint(SolverStage::LinearPrepare, index + 1)?;
    }
    let relative_pivot_floor = 32.0 * f64::EPSILON * size as f64;

    for pivot in 0..size {
        checkpoint(SolverStage::DenseFactor, pivot)?;
        let best = (pivot..size)
            .max_by(|&a, &b| {
                matrix[a][pivot]
                    .amplitude()
                    .total_cmp(&matrix[b][pivot].amplitude())
            })
            .expect("pivot range is non-empty");
        matrix.swap(pivot, best);
        rhs.swap(pivot, best);
        if matrix[pivot][pivot].amplitude() <= relative_pivot_floor {
            return Err("harmonic spring 1d dynamic stiffness is singular".to_string());
        }

        for row in (pivot + 1)..size {
            let factor = divide(
                matrix[row][pivot],
                matrix[pivot][pivot],
                SolverStage::DenseFactor,
            )?;
            if !factor.is_finite() {
                return Err("harmonic spring 1d dynamic stiffness elimination diverged".to_string());
            }
            matrix[row][pivot] = Complex::default();
            for column in (pivot + 1)..size {
                matrix[row][column] = difference(
                    matrix[row][column],
                    factor,
                    matrix[pivot][column],
                    SolverStage::DenseFactor,
                )?;
                if !matrix[row][column].is_finite() {
                    return Err(
                        "harmonic spring 1d dynamic stiffness elimination diverged".to_string()
                    );
                }
            }
            rhs[row] = difference(rhs[row], factor, rhs[pivot], SolverStage::DenseFactor)?;
            if !rhs[row].is_finite() {
                return Err("harmonic spring 1d dynamic stiffness elimination diverged".to_string());
            }
        }
    }

    let mut result = vec![Complex::default(); size];
    for row in (0..size).rev() {
        checkpoint(SolverStage::DenseSubstitution, size - row - 1)?;
        result[row] = solve_row(
            rhs[row],
            matrix[row]
                .iter()
                .copied()
                .skip(row + 1)
                .zip(result.iter().copied().skip(row + 1)),
            matrix[row][row],
            SolverStage::DenseSubstitutionRange,
        )?;
        if !result[row].is_finite() {
            return Err(
                "harmonic spring 1d dynamic stiffness back substitution diverged".to_string(),
            );
        }
    }
    Ok(result)
}

impl Complex {
    pub(super) fn component_scale(self) -> f64 {
        self.re.abs().max(self.im.abs())
    }

    pub(super) fn scaled(self, scale: f64) -> Self {
        Self {
            re: self.re / scale,
            im: self.im / scale,
        }
    }

    pub(super) fn real(value: f64) -> Self {
        Self { re: value, im: 0.0 }
    }

    pub(super) fn amplitude(self) -> f64 {
        self.re.hypot(self.im)
    }

    pub(super) fn is_finite(self) -> bool {
        self.re.is_finite() && self.im.is_finite()
    }

    pub(super) fn phase_deg(self) -> f64 {
        self.im.atan2(self.re).to_degrees()
    }
}

impl std::ops::Add for Complex {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            re: self.re + rhs.re,
            im: self.im + rhs.im,
        }
    }
}

impl std::ops::Sub for Complex {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            re: self.re - rhs.re,
            im: self.im - rhs.im,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complex_division_preserves_large_finite_quotients() {
        for divisor in [0.6, 0.9, 1.0] {
            let value = divide(
                Complex {
                    re: 1.0e308,
                    im: 1.0e308,
                },
                Complex {
                    re: divisor,
                    im: divisor,
                },
                SolverStage::LinearPrepare,
            )
            .unwrap();
            assert!(value.is_finite(), "divisor={divisor}: {value:?}");
            assert!((value.re / (1.0e308 / divisor) - 1.0).abs() < 1.0e-14);
            assert!(value.im.abs() < 1.0e294);
        }
        for scale in [1.0e-300, 1.0, 1.0e300] {
            let value = divide(
                Complex {
                    re: 2.0 * scale,
                    im: 3.0 * scale,
                },
                Complex {
                    re: 4.0 * scale,
                    im: -5.0 * scale,
                },
                SolverStage::LinearPrepare,
            )
            .unwrap();
            assert!((value.re + 7.0 / 41.0).abs() < 1.0e-14);
            assert!((value.im - 22.0 / 41.0).abs() < 1.0e-14);
        }
    }

    #[test]
    fn complex_division_weights_before_an_unrepresentable_intermediate_quotient() {
        let value = divide(
            Complex::real(1.0e308),
            Complex { re: 0.5, im: 0.5 },
            SolverStage::LinearPrepare,
        )
        .unwrap();
        assert!(value.is_finite(), "{value:?}");
        assert!((value.re / 1.0e308 - 1.0).abs() < 1.0e-14);
        assert!((value.im / -1.0e308 - 1.0).abs() < 1.0e-14);
    }
}
