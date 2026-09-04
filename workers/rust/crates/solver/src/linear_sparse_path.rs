use super::SparseMatrix;
use crate::chain_tridiagonal::{PreparedTridiagonal, path_forest_order};

pub(super) struct SparseTridiagonal {
    pub(super) diagonal: Vec<f64>,
    pub(super) lower: Vec<f64>,
    pub(super) upper: Vec<f64>,
    pub(super) order: Option<Vec<usize>>,
}

/// Solves a sparse path in linear time, regardless of the caller's node numbering.
/// `None` keeps arbitrary topologies on the general SPD fallback.
pub(crate) fn solve_tridiagonal_system(
    matrix: &SparseMatrix,
    rhs: &[f64],
) -> Option<Result<Vec<f64>, String>> {
    if matrix.size() != rhs.len() {
        return Some(Err("tridiagonal system dimensions must match".to_string()));
    }
    if rhs.is_empty() {
        return Some(Ok(Vec::new()));
    }

    let coefficients = sparse_tridiagonal_coefficients(matrix)?;
    let factor = match PreparedTridiagonal::factor(
        &coefficients.diagonal,
        &coefficients.lower,
        &coefficients.upper,
    ) {
        Ok(factor) => factor,
        Err(error) => return Some(Err(error)),
    };
    Some(solve_factored_tridiagonal(
        &factor,
        coefficients.order.as_deref(),
        rhs,
    ))
}

pub(super) fn sparse_tridiagonal_coefficients(matrix: &SparseMatrix) -> Option<SparseTridiagonal> {
    direct_coefficients(matrix).or_else(|| permuted_coefficients(matrix))
}

pub(super) fn solve_factored_tridiagonal(
    factor: &PreparedTridiagonal,
    order: Option<&[usize]>,
    rhs: &[f64],
) -> Result<Vec<f64>, String> {
    let Some(order) = order else {
        return factor.solve(rhs);
    };
    if order.len() != rhs.len() {
        return Err("permuted tridiagonal system dimensions must match".to_string());
    }

    let ordered_rhs = order.iter().map(|&node| rhs[node]).collect::<Vec<_>>();
    let ordered_solution = factor.solve(&ordered_rhs)?;
    let mut solution = vec![0.0; rhs.len()];
    for (path_index, &node_index) in order.iter().enumerate() {
        solution[node_index] = ordered_solution[path_index];
    }
    Ok(solution)
}

fn direct_coefficients(matrix: &SparseMatrix) -> Option<SparseTridiagonal> {
    let size = matrix.size();
    let mut diagonal = vec![0.0; size];
    let mut lower = vec![0.0; size.saturating_sub(1)];
    let mut upper = vec![0.0; size.saturating_sub(1)];
    for (row, entries) in matrix.rows.iter().enumerate() {
        for &(column, value) in entries {
            if column >= size || row.abs_diff(column) > 1 {
                return None;
            }
            if column == row {
                diagonal[row] = value;
            } else if column < row {
                lower[row - 1] = value;
            } else {
                upper[row] = value;
            }
        }
    }
    Some(SparseTridiagonal {
        diagonal,
        lower,
        upper,
        order: None,
    })
}

fn permuted_coefficients(matrix: &SparseMatrix) -> Option<SparseTridiagonal> {
    let size = matrix.size();
    let edge_count = upper_edges(matrix).count();
    let order = path_forest_order(size, edge_count, upper_edges(matrix))?;
    let mut position = vec![0_usize; size];
    for (path_index, &node_index) in order.iter().enumerate() {
        position[node_index] = path_index;
    }

    let mut diagonal = vec![0.0; size];
    let mut lower = vec![0.0; size - 1];
    let mut upper = vec![0.0; size - 1];
    for (row, entries) in matrix.rows.iter().enumerate() {
        for &(column, value) in entries {
            let ordered_row = position[row];
            let ordered_column = position[column];
            if ordered_row == ordered_column {
                diagonal[ordered_row] = value;
            } else if ordered_column + 1 == ordered_row {
                lower[ordered_column] = value;
            } else if ordered_row + 1 == ordered_column {
                upper[ordered_row] = value;
            } else {
                return None;
            }
        }
    }
    Some(SparseTridiagonal {
        diagonal,
        lower,
        upper,
        order: Some(order),
    })
}

fn upper_edges(matrix: &SparseMatrix) -> impl Iterator<Item = (usize, usize)> + '_ {
    matrix.rows.iter().enumerate().flat_map(|(row, entries)| {
        entries
            .iter()
            .filter_map(move |&(column, _)| (column > row).then_some((row, column)))
    })
}

#[cfg(test)]
mod tests {
    use super::{solve_tridiagonal_system, sparse_tridiagonal_coefficients};
    use crate::linear_algebra::{SparseMatrix, add_at};

    #[test]
    fn solves_a_permuted_forest_as_one_block_tridiagonal_system() {
        let mut matrix = SparseMatrix::with_uniform_row_capacity(6, 3);
        for node in 0..6 {
            add_at(&mut matrix, node, node, 4.0);
        }
        for (first, second) in [(0, 3), (3, 1), (2, 5)] {
            add_at(&mut matrix, first, second, -1.0);
            add_at(&mut matrix, second, first, -1.0);
        }

        let coefficients =
            sparse_tridiagonal_coefficients(&matrix).expect("path forest should be recognized");
        assert_eq!(coefficients.order, Some(vec![0, 3, 1, 2, 5, 4]));

        let solution = solve_tridiagonal_system(&matrix, &[0.0, 4.0, 6.0, 13.0, 20.0, 21.0])
            .expect("path forest should select the direct solver")
            .expect("path forest should solve");
        assert_eq!(solution, vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
    }
}
