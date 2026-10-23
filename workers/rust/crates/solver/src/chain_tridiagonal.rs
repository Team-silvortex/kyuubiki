use crate::solver_control::{SolverStage, checkpoint};

#[path = "chain_tridiagonal_pivot.rs"]
mod pivot;
use pivot::PivotedTridiagonal;
#[path = "chain_tridiagonal_equilibration.rs"]
mod equilibration;
#[path = "chain_tridiagonal_forward_range.rs"]
mod forward_range;
#[path = "chain_tridiagonal_prescribed.rs"]
mod prescribed;
#[path = "chain_tridiagonal_range_value.rs"]
mod range_value;
#[path = "chain_tridiagonal_residual.rs"]
mod residual;
use equilibration::RowEquilibration;
use prescribed::solve_with_prescribed;
use residual::maximum_backward_error;

#[cfg(test)]
#[path = "chain_tridiagonal_pivot_tests.rs"]
mod pivot_tests;

#[cfg(test)]
#[path = "chain_tridiagonal_scaling_tests.rs"]
mod scaling_tests;

#[cfg(test)]
#[path = "chain_tridiagonal_validation_tests.rs"]
mod validation_tests;

#[cfg(test)]
#[path = "chain_tridiagonal_substitution_tests.rs"]
mod substitution_tests;

#[cfg(test)]
#[path = "chain_tridiagonal_forward_range_tests.rs"]
mod forward_range_tests;

#[cfg(test)]
#[path = "chain_tridiagonal_forward_underflow_tests.rs"]
mod forward_underflow_tests;

#[cfg(test)]
#[path = "chain_tridiagonal_back_underflow_tests.rs"]
mod back_underflow_tests;

#[cfg(test)]
#[path = "chain_tridiagonal_prescribed_range_tests.rs"]
mod prescribed_range_tests;

#[cfg(test)]
#[path = "chain_tridiagonal_prescribed_cancellation_tests.rs"]
mod prescribed_cancellation_tests;

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
        let backward_error = maximum_backward_error(
            &self.diagonal,
            &self.lower,
            &self.upper,
            rhs,
            &solution,
            tolerance,
        )?;
        if !backward_error.is_finite() || backward_error > tolerance {
            return Err(format!(
                "tridiagonal solution failed backward-error validation ({backward_error:.6e})"
            ));
        }
        Ok(solution)
    }

    fn solve_without_pivoting(&self, rhs: &[f64]) -> Result<Vec<f64>, String> {
        let size = rhs.len();
        let upper = self
            .equilibration
            .as_ref()
            .map_or(self.upper.as_slice(), |scaling| scaling.upper.as_slice());
        let mut rhs_work = rhs.to_vec();
        for row in 1..size {
            let original = rhs_work[row];
            let previous = rhs_work[row - 1];
            let coefficient = self.elimination_factors[row - 1];
            rhs_work[row] = (-coefficient).mul_add(previous, original);
            if range_value::update_needs_range(original, coefficient, previous, rhs_work[row])? {
                drop(rhs_work);
                return forward_range::solve(
                    &self.factored_diagonal,
                    upper,
                    &[],
                    &self.elimination_factors,
                    &[],
                    rhs,
                );
            }
            if row % 256 == 0 {
                checkpoint(SolverStage::TridiagonalSubstitution, row)?;
            }
        }

        let mut solution = vec![0.0; size];
        solution[size - 1] = finish_back_substitution(
            rhs_work[size - 1],
            self.factored_diagonal[size - 1],
            std::iter::empty(),
        )?;
        for row in (0..size - 1).rev() {
            solution[row] = finish_back_substitution(
                rhs_work[row],
                self.factored_diagonal[row],
                std::iter::once((upper[row], solution[row + 1])),
            )?;
            if row % 256 == 0 {
                checkpoint(SolverStage::TridiagonalSubstitution, size + (size - row))?;
            }
        }
        checkpoint(SolverStage::TridiagonalSubstitution, size.saturating_mul(2))?;
        Ok(solution)
    }
}

fn finish_back_substitution<I>(rhs: f64, pivot: f64, operands: I) -> Result<f64, String>
where
    I: ExactSizeIterator<Item = (f64, f64)> + Clone,
{
    let mut numerator = rhs;
    let mut needs_range = false;
    for (coefficient, solved) in operands.clone() {
        let updated = (-coefficient).mul_add(solved, numerator);
        if !needs_range {
            needs_range = range_value::update_needs_range(numerator, coefficient, solved, updated)?;
        }
        numerator = updated;
    }
    let result = numerator / pivot;
    if result.is_finite() {
        if needs_range {
            return crate::linear_substitution::solve_scaled_rhs_row_with_policy(
                crate::linear_binary_scale::split_finite(rhs),
                operands,
                pivot,
                SolverStage::TridiagonalSubstitutionRange,
                crate::linear_substitution::SubnormalResult::ValidateOriginalEquations,
            );
        }
        // Retain ordered FMA bits and final quantized tails unless an update lost range.
        return Ok(result);
    }
    crate::linear_substitution::solve_row(
        rhs,
        operands,
        pivot,
        SolverStage::TridiagonalSubstitutionRange,
    )
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
