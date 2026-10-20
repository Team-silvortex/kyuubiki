use crate::solver_control::{SolverStage, checkpoint};

#[path = "chain_tridiagonal_pivot.rs"]
mod pivot;
use pivot::PivotedTridiagonal;
#[path = "chain_tridiagonal_equilibration.rs"]
mod equilibration;
#[path = "chain_tridiagonal_residual.rs"]
mod residual;
use equilibration::RowEquilibration;

#[cfg(test)]
#[path = "chain_tridiagonal_pivot_tests.rs"]
mod pivot_tests;

#[cfg(test)]
#[path = "chain_tridiagonal_scaling_tests.rs"]
mod scaling_tests;

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
    pivoted: Option<Box<PivotedTridiagonal>>,
    equilibration: Option<Box<RowEquilibration>>,
}

impl PreparedTridiagonal {
    pub(crate) fn factor(diagonal: &[f64], lower: &[f64], upper: &[f64]) -> Result<Self, String> {
        checkpoint(SolverStage::TridiagonalFactor, 0)?;
        validate_coefficient_dimensions(diagonal, lower, upper)?;
        validate_finite(diagonal, lower, upper, &[])?;
        if diagonal.is_empty() {
            return Ok(Self {
                diagonal: Vec::new(),
                lower: Vec::new(),
                upper: Vec::new(),
                factored_diagonal: Vec::new(),
                elimination_factors: Vec::new(),
                pivoted: None,
                equilibration: None,
            });
        }

        let size = diagonal.len();
        let row_scales = row_scales(diagonal, lower, upper);
        if row_scales.contains(&0.0) {
            return Err("tridiagonal system contains a singular row".to_string());
        }
        if equilibration::needed(&row_scales) {
            return Self::factor_equilibrated(diagonal, lower, upper, &row_scales);
        }
        let mut factored_diagonal = diagonal.to_vec();
        let mut elimination_factors = Vec::with_capacity(size.saturating_sub(1));
        for row in 1..size {
            // A zero leading/Schur pivot need not mean rank loss. Keep the
            // no-swap path allocation-free beyond its existing factors.
            if !usable_pivot(factored_diagonal[row - 1], row_scales[row - 1], size)
                || factored_diagonal[row - 1].abs() < lower[row - 1].abs()
            {
                return Self::factor_with_pivoting(diagonal, lower, upper);
            }
            let factor = lower[row - 1] / factored_diagonal[row - 1];
            if !factor.is_finite() {
                return Err("tridiagonal elimination diverged".to_string());
            }
            factored_diagonal[row] = (-factor).mul_add(upper[row - 1], factored_diagonal[row]);
            if !factored_diagonal[row].is_finite() {
                return Err("tridiagonal elimination diverged".to_string());
            }
            elimination_factors.push(factor);
            if row % 256 == 0 {
                checkpoint(SolverStage::TridiagonalFactor, row)?;
            }
        }
        if !usable_pivot(factored_diagonal[size - 1], row_scales[size - 1], size) {
            return Self::factor_with_pivoting(diagonal, lower, upper);
        }
        checkpoint(SolverStage::TridiagonalFactor, size)?;

        Ok(Self {
            diagonal: diagonal.to_vec(),
            lower: lower.to_vec(),
            upper: upper.to_vec(),
            factored_diagonal,
            elimination_factors,
            pivoted: None,
            equilibration: None,
        })
    }

    fn factor_with_pivoting(
        diagonal: &[f64],
        lower: &[f64],
        upper: &[f64],
    ) -> Result<Self, String> {
        Ok(Self {
            pivoted: Some(Box::new(PivotedTridiagonal::factor(
                diagonal, lower, upper,
            )?)),
            diagonal: diagonal.to_vec(),
            lower: lower.to_vec(),
            upper: upper.to_vec(),
            factored_diagonal: Vec::new(),
            elimination_factors: Vec::new(),
            equilibration: None,
        })
    }

    pub(crate) fn solve(&self, rhs: &[f64]) -> Result<Vec<f64>, String> {
        checkpoint(SolverStage::TridiagonalSubstitution, 0)?;
        if rhs.len() != self.diagonal.len() {
            return Err("tridiagonal system dimensions must match".to_string());
        }
        if rhs.iter().any(|value| !value.is_finite()) {
            return Err("tridiagonal system contains a non-finite value".to_string());
        }
        if rhs.is_empty() {
            return Ok(Vec::new());
        }

        let scaled_rhs;
        let factor_rhs = if let Some(scaling) = &self.equilibration {
            scaled_rhs = scaling.scale_rhs(rhs)?;
            scaled_rhs.as_slice()
        } else {
            rhs
        };
        let solution = match &self.pivoted {
            Some(factor) => factor.solve(factor_rhs)?,
            None => self.solve_without_pivoting(factor_rhs)?,
        };
        if solution.iter().any(|value| !value.is_finite()) {
            return Err("tridiagonal back substitution diverged".to_string());
        }
        let tolerance = backward_error_tolerance(rhs.len());
        let backward_error = if self.equilibration.is_some() {
            residual::scaled_backward_error(
                &self.diagonal,
                &self.lower,
                &self.upper,
                rhs,
                &solution,
            )?
        } else {
            maximum_backward_error(
                &self.diagonal,
                &self.lower,
                &self.upper,
                rhs,
                &solution,
                tolerance,
            )?
        };
        if !backward_error.is_finite() || backward_error > tolerance {
            return Err(format!(
                "tridiagonal solution failed backward-error validation ({backward_error:.6e})"
            ));
        }
        Ok(solution)
    }

    fn solve_without_pivoting(&self, rhs: &[f64]) -> Result<Vec<f64>, String> {
        let size = rhs.len();
        let mut rhs_work = rhs.to_vec();
        for row in 1..size {
            rhs_work[row] =
                (-self.elimination_factors[row - 1]).mul_add(rhs_work[row - 1], rhs_work[row]);
            if !rhs_work[row].is_finite() {
                return Err("tridiagonal elimination diverged".to_string());
            }
            if row % 256 == 0 {
                checkpoint(SolverStage::TridiagonalSubstitution, row)?;
            }
        }

        let mut solution = vec![0.0; size];
        let upper = self
            .equilibration
            .as_ref()
            .map_or(self.upper.as_slice(), |scaling| scaling.upper.as_slice());
        solution[size - 1] = rhs_work[size - 1] / self.factored_diagonal[size - 1];
        for row in (0..size - 1).rev() {
            solution[row] = (-upper[row]).mul_add(solution[row + 1], rhs_work[row])
                / self.factored_diagonal[row];
            if row % 256 == 0 {
                checkpoint(SolverStage::TridiagonalSubstitution, size + (size - row))?;
            }
        }
        checkpoint(SolverStage::TridiagonalSubstitution, size.saturating_mul(2))?;
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

pub(crate) fn solve_path_with_prescribed<Element>(
    node_count: usize,
    elements: &[Element],
    endpoints: impl Fn(&Element) -> (usize, usize),
    local_matrix: impl Fn(&Element) -> Result<[[f64; 2]; 2], String>,
    rhs: &[f64],
    prescribed: &[(usize, f64)],
) -> Option<Result<Vec<f64>, String>> {
    let order = path_order(node_count, elements.len(), elements.iter().map(&endpoints))?;
    Some(solve_ordered_path(
        node_count,
        elements,
        &endpoints,
        local_matrix,
        rhs,
        prescribed,
        &order,
    ))
}

pub(crate) fn path_order(
    node_count: usize,
    edge_count: usize,
    edges: impl IntoIterator<Item = (usize, usize)>,
) -> Option<Vec<usize>> {
    if node_count < 2 || edge_count != node_count - 1 {
        return None;
    }

    path_forest_order(node_count, edge_count, edges)
}

pub(crate) fn path_forest_order(
    node_count: usize,
    edge_count: usize,
    edges: impl IntoIterator<Item = (usize, usize)>,
) -> Option<Vec<usize>> {
    const MISSING: usize = usize::MAX;

    if node_count == 0 {
        return (edge_count == 0).then(Vec::new);
    }
    if edge_count >= node_count {
        return None;
    }

    let mut neighbors = vec![[MISSING; 2]; node_count];
    let mut observed_edges = 0_usize;
    for (first, second) in edges {
        observed_edges = observed_edges.checked_add(1)?;
        if first >= node_count || second >= node_count || first == second {
            return None;
        }
        if !add_neighbor(&mut neighbors[first], second)
            || !add_neighbor(&mut neighbors[second], first)
        {
            return None;
        }
    }
    if observed_edges != edge_count {
        return None;
    }

    let mut order = Vec::with_capacity(node_count);
    let mut visited = vec![false; node_count];
    for start in 0..node_count {
        if visited[start] || neighbors[start][1] != MISSING {
            continue;
        }

        let mut previous = MISSING;
        let mut current = start;
        loop {
            if visited[current] {
                return None;
            }
            visited[current] = true;
            order.push(current);

            let next = neighbors[current]
                .into_iter()
                .filter(|&neighbor| neighbor != MISSING)
                .find(|&neighbor| neighbor != previous);
            let Some(next) = next else {
                break;
            };
            previous = current;
            current = next;
        }
    }

    (order.len() == node_count).then_some(order)
}

fn add_neighbor(neighbors: &mut [usize; 2], neighbor: usize) -> bool {
    const MISSING: usize = usize::MAX;

    if neighbors.contains(&neighbor) {
        return false;
    }
    if neighbors[0] == MISSING {
        neighbors[0] = neighbor;
        true
    } else if neighbors[1] == MISSING {
        neighbors[1] = neighbor;
        true
    } else {
        false
    }
}

fn solve_ordered_path<Element>(
    node_count: usize,
    elements: &[Element],
    endpoints: &impl Fn(&Element) -> (usize, usize),
    local_matrix: impl Fn(&Element) -> Result<[[f64; 2]; 2], String>,
    rhs: &[f64],
    prescribed: &[(usize, f64)],
    order: &[usize],
) -> Result<Vec<f64>, String> {
    if rhs.len() != node_count {
        return Err("1d path solver dimensions must match".to_string());
    }

    let mut position = vec![0_usize; node_count];
    for (path_index, &node_index) in order.iter().enumerate() {
        position[node_index] = path_index;
    }

    let mut diagonal = vec![0.0; node_count];
    let mut lower = vec![0.0; node_count - 1];
    let mut upper = vec![0.0; node_count - 1];
    for element in elements {
        let (node_i, node_j) = endpoints(element);
        let local = local_matrix(element)?;
        let map = [position[node_i], position[node_j]];
        if map[0].abs_diff(map[1]) != 1 {
            return Err("1d path solver received inconsistent topology".to_string());
        }
        for row in 0..2 {
            for column in 0..2 {
                let row_index = map[row];
                let column_index = map[column];
                if row_index == column_index {
                    diagonal[row_index] += local[row][column];
                } else if column_index == row_index + 1 {
                    upper[row_index] += local[row][column];
                } else {
                    lower[column_index] += local[row][column];
                }
            }
        }
    }

    let ordered_rhs = order.iter().map(|&node| rhs[node]).collect::<Vec<_>>();
    let mut ordered_prescribed = Vec::with_capacity(prescribed.len());
    for &(node, value) in prescribed {
        if node >= node_count {
            return Err("1d path solver received an out-of-range prescribed value".to_string());
        }
        ordered_prescribed.push((position[node], value));
    }
    let ordered_values =
        solve_with_prescribed(&diagonal, &lower, &upper, &ordered_rhs, &ordered_prescribed)?;

    let mut values = vec![0.0; node_count];
    for (path_index, &node_index) in order.iter().enumerate() {
        values[node_index] = ordered_values[path_index];
    }
    Ok(values)
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
    if !usable_pivot(pivot, row_scale, size) {
        return Err("tridiagonal system has a singular pivot".to_string());
    }
    Ok(())
}

fn usable_pivot(pivot: f64, row_scale: f64, size: usize) -> bool {
    let relative_floor = (64.0 * f64::EPSILON * size.max(1) as f64).min(1.0e-8);
    pivot.is_finite() && row_scale > 0.0 && pivot.abs() / row_scale > relative_floor
}

fn maximum_backward_error(
    diagonal: &[f64],
    lower: &[f64],
    upper: &[f64],
    rhs: &[f64],
    solution: &[f64],
    tolerance: f64,
) -> Result<f64, String> {
    checkpoint(SolverStage::SparseResidual, 0)?;
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
        if !residual.is_finite() || !scale.is_finite() {
            // Never let infinity in the denominator or NaN in f64::max
            // turn an unchecked equation into a zero relative error.
            drop(rows);
            return residual::scaled_backward_error(diagonal, lower, upper, rhs, solution);
        }
        global_scale = global_scale.max(scale);
        rows.push((residual, scale));
        if (row + 1) % 256 == 0 {
            checkpoint(SolverStage::SparseResidual, row + 1)?;
        }
    }

    let roundoff_floor = global_scale * f64::EPSILON / tolerance;
    let mut maximum = 0.0_f64;
    for (row, (residual, scale)) in rows.into_iter().enumerate() {
        let effective_scale = scale.max(roundoff_floor);
        let relative = if effective_scale == 0.0 {
            if residual == 0.0 { 0.0 } else { f64::INFINITY }
        } else {
            residual / effective_scale
        };
        maximum = maximum.max(relative);
        if (row + 1) % 256 == 0 {
            checkpoint(SolverStage::SparseResidual, diagonal.len() + row + 1)?;
        }
    }
    checkpoint(
        SolverStage::SparseResidual,
        diagonal.len().saturating_mul(2),
    )?;
    Ok(maximum)
}

fn backward_error_tolerance(size: usize) -> f64 {
    (128.0 * f64::EPSILON * size.max(1) as f64)
        .clamp(MIN_BACKWARD_ERROR_TOLERANCE, MAX_BACKWARD_ERROR_TOLERANCE)
}

#[cfg(test)]
mod tests {
    use super::{
        path_forest_order, path_order, solve_path_with_prescribed, solve_tridiagonal,
        solve_with_prescribed,
    };

    #[test]
    fn recognizes_a_path_independent_of_node_numbering() {
        assert_eq!(
            path_order(4, 3, [(2, 1), (0, 2), (1, 3)]),
            Some(vec![0, 2, 1, 3])
        );
        assert_eq!(path_order(4, 3, [(0, 1), (1, 2), (1, 3)]), None);
        assert_eq!(path_order(4, 3, [(0, 1), (1, 0), (2, 3)]), None);
    }

    #[test]
    fn recognizes_disconnected_path_segments_and_isolated_nodes() {
        assert_eq!(
            path_forest_order(6, 3, [(0, 3), (3, 1), (2, 5)]),
            Some(vec![0, 3, 1, 2, 5, 4])
        );
        assert_eq!(path_forest_order(4, 3, [(0, 1), (1, 2), (2, 0)]), None);
        assert_eq!(path_forest_order(4, 3, [(0, 1), (1, 2), (1, 3)]), None);
    }

    #[test]
    fn restores_node_order_and_preserves_asymmetric_element_orientation() {
        let elements = [
            (2, 1, [[5.0, -2.0], [-4.0, 6.0]]),
            (0, 2, [[2.0, -1.0], [-3.0, 4.0]]),
        ];
        let values = solve_path_with_prescribed(
            3,
            &elements,
            |element| (element.0, element.1),
            |element| Ok(element.2),
            &[0.0, 0.0, 0.0],
            &[(0, 1.0), (1, 3.0)],
        )
        .expect("permuted topology should be a path")
        .expect("permuted path should solve");

        assert_eq!(values, vec![1.0, 3.0, 1.0]);
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
