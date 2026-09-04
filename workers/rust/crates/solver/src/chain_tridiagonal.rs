struct TridiagonalSystem<'a> {
    diagonal: &'a [f64],
    lower: &'a [f64],
    upper: &'a [f64],
    rhs: &'a [f64],
}

const MIN_BACKWARD_ERROR_TOLERANCE: f64 = 1.0e-10;
const MAX_BACKWARD_ERROR_TOLERANCE: f64 = 1.0e-7;

pub(crate) struct PreparedTridiagonal {
    diagonal: Vec<f64>,
    lower: Vec<f64>,
    upper: Vec<f64>,
    factored_diagonal: Vec<f64>,
    elimination_factors: Vec<f64>,
}

impl PreparedTridiagonal {
    pub(crate) fn factor(diagonal: &[f64], lower: &[f64], upper: &[f64]) -> Result<Self, String> {
        validate_coefficient_dimensions(diagonal, lower, upper)?;
        validate_finite(diagonal, lower, upper, &[])?;
        if diagonal.is_empty() {
            return Ok(Self {
                diagonal: Vec::new(),
                lower: Vec::new(),
                upper: Vec::new(),
                factored_diagonal: Vec::new(),
                elimination_factors: Vec::new(),
            });
        }

        let size = diagonal.len();
        let row_scales = row_scales(diagonal, lower, upper);
        if row_scales.contains(&0.0) {
            return Err("tridiagonal system contains a singular row".to_string());
        }
        let mut factored_diagonal = diagonal.to_vec();
        let mut elimination_factors = Vec::with_capacity(size.saturating_sub(1));
        for row in 1..size {
            ensure_usable_pivot(factored_diagonal[row - 1], row_scales[row - 1], size)?;
            let factor = lower[row - 1] / factored_diagonal[row - 1];
            if !factor.is_finite() {
                return Err("tridiagonal elimination diverged".to_string());
            }
            factored_diagonal[row] = (-factor).mul_add(upper[row - 1], factored_diagonal[row]);
            if !factored_diagonal[row].is_finite() {
                return Err("tridiagonal elimination diverged".to_string());
            }
            elimination_factors.push(factor);
        }
        ensure_usable_pivot(factored_diagonal[size - 1], row_scales[size - 1], size)?;

        Ok(Self {
            diagonal: diagonal.to_vec(),
            lower: lower.to_vec(),
            upper: upper.to_vec(),
            factored_diagonal,
            elimination_factors,
        })
    }

    pub(crate) fn solve(&self, rhs: &[f64]) -> Result<Vec<f64>, String> {
        if rhs.len() != self.diagonal.len() {
            return Err("tridiagonal system dimensions must match".to_string());
        }
        if rhs.iter().any(|value| !value.is_finite()) {
            return Err("tridiagonal system contains a non-finite value".to_string());
        }
        if rhs.is_empty() {
            return Ok(Vec::new());
        }

        let size = rhs.len();
        let mut rhs_work = rhs.to_vec();
        for row in 1..size {
            rhs_work[row] =
                (-self.elimination_factors[row - 1]).mul_add(rhs_work[row - 1], rhs_work[row]);
            if !rhs_work[row].is_finite() {
                return Err("tridiagonal elimination diverged".to_string());
            }
        }

        let mut solution = vec![0.0; size];
        solution[size - 1] = rhs_work[size - 1] / self.factored_diagonal[size - 1];
        for row in (0..size - 1).rev() {
            solution[row] = (-self.upper[row]).mul_add(solution[row + 1], rhs_work[row])
                / self.factored_diagonal[row];
        }
        if solution.iter().any(|value| !value.is_finite()) {
            return Err("tridiagonal back substitution diverged".to_string());
        }

        let tolerance = backward_error_tolerance(size);
        let backward_error = maximum_backward_error(
            &self.diagonal,
            &self.lower,
            &self.upper,
            rhs,
            &solution,
            tolerance,
        );
        if !backward_error.is_finite() || backward_error > tolerance {
            return Err(format!(
                "tridiagonal solution failed backward-error validation ({backward_error:.6e})"
            ));
        }
        Ok(solution)
    }
}

pub(crate) fn solve_tridiagonal(
    diagonal: &[f64],
    lower: &[f64],
    upper: &[f64],
    rhs: &[f64],
) -> Result<Vec<f64>, String> {
    validate_dimensions(diagonal, lower, upper, rhs)?;
    PreparedTridiagonal::factor(diagonal, lower, upper)?.solve(rhs)
}

pub(crate) fn is_indexed_chain(
    node_count: usize,
    edges: impl IntoIterator<Item = (usize, usize)>,
) -> bool {
    if node_count < 2 {
        return false;
    }

    let mut spans = vec![false; node_count - 1];
    let mut edge_count = 0;
    for (first, second) in edges {
        let (left, right) = if first < second {
            (first, second)
        } else {
            (second, first)
        };
        if right != left + 1 || left >= spans.len() || spans[left] {
            return false;
        }
        spans[left] = true;
        edge_count += 1;
    }

    edge_count == spans.len() && spans.into_iter().all(|present| present)
}

pub(crate) fn solve_with_prescribed(
    diagonal: &[f64],
    lower: &[f64],
    upper: &[f64],
    rhs: &[f64],
    prescribed: &[(usize, f64)],
) -> Result<Vec<f64>, String> {
    validate_dimensions(diagonal, lower, upper, rhs)
        .map_err(|error| format!("1d chain solver failed: {error}"))?;
    validate_finite(diagonal, lower, upper, rhs)
        .map_err(|error| format!("1d chain solver failed: {error}"))?;
    let node_count = diagonal.len();
    if node_count == 0 {
        return Err("1d chain solver received inconsistent tridiagonal dimensions".to_string());
    }

    let mut values = vec![0.0; node_count];
    let mut fixed = vec![false; node_count];
    for &(index, value) in prescribed {
        if index >= node_count {
            return Err("1d chain solver received an out-of-range prescribed value".to_string());
        }
        if !value.is_finite() {
            return Err("1d chain solver received a non-finite prescribed value".to_string());
        }
        if fixed[index] && (values[index] - value).abs() > 1.0e-12 {
            return Err("1d chain solver received conflicting prescribed values".to_string());
        }
        fixed[index] = true;
        values[index] = value;
    }

    let mut cursor = 0;
    while cursor < node_count {
        if fixed[cursor] {
            cursor += 1;
            continue;
        }
        let start = cursor;
        while cursor < node_count && !fixed[cursor] {
            cursor += 1;
        }
        solve_free_segment(
            start..cursor,
            &TridiagonalSystem {
                diagonal,
                lower,
                upper,
                rhs,
            },
            &mut values,
            &fixed,
        )?;
    }
    Ok(values)
}

fn solve_free_segment(
    range: std::ops::Range<usize>,
    system: &TridiagonalSystem<'_>,
    values: &mut [f64],
    fixed: &[bool],
) -> Result<(), String> {
    let start = range.start;
    let end = range.end;
    let count = end - start;
    let mut rhs_work = system.rhs[start..end].to_vec();

    if start > 0 && fixed[start - 1] {
        rhs_work[0] -= system.lower[start - 1] * values[start - 1];
    }
    if end < values.len() && fixed[end] {
        rhs_work[count - 1] -= system.upper[end - 1] * values[end];
    }

    let solved = solve_tridiagonal(
        &system.diagonal[start..end],
        &system.lower[start..end - 1],
        &system.upper[start..end - 1],
        &rhs_work,
    )
    .map_err(|error| format!("1d chain solver failed: {error}"))?;
    values[start..end].copy_from_slice(&solved);
    Ok(())
}

fn validate_dimensions(
    diagonal: &[f64],
    lower: &[f64],
    upper: &[f64],
    rhs: &[f64],
) -> Result<(), String> {
    validate_coefficient_dimensions(diagonal, lower, upper)?;
    if rhs.len() != diagonal.len() {
        return Err("tridiagonal system dimensions must match".to_string());
    }
    Ok(())
}

fn validate_coefficient_dimensions(
    diagonal: &[f64],
    lower: &[f64],
    upper: &[f64],
) -> Result<(), String> {
    let size = diagonal.len();
    if lower.len() != size.saturating_sub(1) || upper.len() != size.saturating_sub(1) {
        return Err("tridiagonal system dimensions must match".to_string());
    }
    Ok(())
}

fn row_scales(diagonal: &[f64], lower: &[f64], upper: &[f64]) -> Vec<f64> {
    (0..diagonal.len())
        .map(|row| {
            let mut scale = diagonal[row].abs();
            if row > 0 {
                scale = scale.max(lower[row - 1].abs());
            }
            if row + 1 < diagonal.len() {
                scale = scale.max(upper[row].abs());
            }
            scale
        })
        .collect()
}

fn validate_finite(
    diagonal: &[f64],
    lower: &[f64],
    upper: &[f64],
    rhs: &[f64],
) -> Result<(), String> {
    if diagonal
        .iter()
        .chain(lower)
        .chain(upper)
        .chain(rhs)
        .any(|value| !value.is_finite())
    {
        return Err("tridiagonal system contains a non-finite value".to_string());
    }
    Ok(())
}

fn ensure_usable_pivot(pivot: f64, row_scale: f64, size: usize) -> Result<(), String> {
    let relative_floor = (64.0 * f64::EPSILON * size.max(1) as f64).min(1.0e-8);
    if !pivot.is_finite() || pivot.abs() / row_scale <= relative_floor {
        return Err("tridiagonal system has a singular pivot".to_string());
    }
    Ok(())
}

fn maximum_backward_error(
    diagonal: &[f64],
    lower: &[f64],
    upper: &[f64],
    rhs: &[f64],
    solution: &[f64],
    tolerance: f64,
) -> f64 {
    let mut rows = Vec::with_capacity(diagonal.len());
    let mut global_scale = 0.0_f64;
    for row in 0..diagonal.len() {
        let diagonal_term = diagonal[row] * solution[row];
        let mut actual = diagonal_term;
        let mut scale = rhs[row].abs() + diagonal_term.abs();
        if row > 0 {
            let lower_term = lower[row - 1] * solution[row - 1];
            actual += lower_term;
            scale += lower_term.abs();
        }
        if row + 1 < diagonal.len() {
            let upper_term = upper[row] * solution[row + 1];
            actual += upper_term;
            scale += upper_term.abs();
        }
        let residual = (rhs[row] - actual).abs();
        global_scale = global_scale.max(scale);
        rows.push((residual, scale));
    }

    let roundoff_floor = global_scale * f64::EPSILON / tolerance;
    rows.into_iter()
        .fold(0.0_f64, |maximum, (residual, scale)| {
            let effective_scale = scale.max(roundoff_floor);
            let relative = if effective_scale == 0.0 {
                if residual == 0.0 { 0.0 } else { f64::INFINITY }
            } else {
                residual / effective_scale
            };
            maximum.max(relative)
        })
}

fn backward_error_tolerance(size: usize) -> f64 {
    (128.0 * f64::EPSILON * size.max(1) as f64)
        .clamp(MIN_BACKWARD_ERROR_TOLERANCE, MAX_BACKWARD_ERROR_TOLERANCE)
}

#[cfg(test)]
mod tests {
    use super::{is_indexed_chain, solve_tridiagonal, solve_with_prescribed};

    #[test]
    fn recognizes_a_contiguous_chain_only_once_per_span() {
        assert!(is_indexed_chain(4, [(0, 1), (2, 1), (2, 3)]));
        assert!(!is_indexed_chain(4, [(0, 1), (1, 2), (1, 2)]));
    }

    #[test]
    fn solves_segments_around_prescribed_values() {
        let values = solve_with_prescribed(
            &[1.0, 2.0, 2.0, 1.0],
            &[-1.0, -1.0, -1.0],
            &[-1.0, -1.0, -1.0],
            &[0.0, 0.0, 0.0, 0.0],
            &[(0, 0.0), (3, 3.0)],
        )
        .expect("tridiagonal system should solve");
        assert_eq!(values, vec![0.0, 1.0, 2.0, 3.0]);
    }

    #[test]
    fn preserves_uniformly_tiny_coefficients() {
        let solution = solve_tridiagonal(
            &[2.0e-24, 2.0e-24],
            &[-1.0e-24],
            &[-1.0e-24],
            &[1.0e-24, 0.0],
        )
        .expect("uniformly tiny tridiagonal system should solve");

        assert!((solution[0] - 2.0 / 3.0).abs() < 1.0e-12);
        assert!((solution[1] - 1.0 / 3.0).abs() < 1.0e-12);
    }

    #[test]
    fn rejects_scale_relative_rank_loss() {
        let error = solve_tridiagonal(
            &[1.0, 1.0 + f64::EPSILON],
            &[1.0],
            &[1.0],
            &[2.0, 2.0 + f64::EPSILON],
        )
        .expect_err("rank-deficient tridiagonal system should fail closed");

        assert!(error.contains("singular pivot"));
    }
}
