use super::{ROOTS, SEGMENTS};

// Independent banded LDL inertia of K - lambda*M, without mass normalization or Jacobi.
pub(crate) fn count_below(segments: usize, shift: f64) -> usize {
    let size = 2 * segments;
    let mut matrix = vec![[0.0; 4]; size];
    let element = [
        [12.0, 6.0, -12.0, 6.0],
        [6.0, 4.0, -6.0, 2.0],
        [-12.0, -6.0, 12.0, -6.0],
        [6.0, 2.0, -6.0, 4.0],
    ];
    for segment in 0..segments {
        for (i, values) in element.iter().enumerate() {
            for (j, value) in values.iter().enumerate().take(i + 1) {
                let (row, column) = (2 * segment + i, 2 * segment + j);
                if row >= 2 && column >= 2 {
                    matrix[row - 2][row - column] += value;
                }
            }
        }
    }
    let mut lower = vec![[0.0; 4]; size];
    let mut diagonal = vec![0.0; size];
    for row in 0..size {
        let start = row.saturating_sub(3);
        for column in start..row {
            let mut entry = matrix[row][row - column];
            for (k, pivot) in diagonal.iter().enumerate().take(column).skip(start) {
                entry -= lower[row][row - k] * pivot * lower[column][column - k];
            }
            lower[row][row - column] = entry / diagonal[column];
        }
        let mass =
            (if row % 2 == 0 { 1.0 } else { 1.0 / 12.0 }) * if row >= size - 2 { 0.5 } else { 1.0 };
        diagonal[row] = matrix[row][0] - shift * mass;
        for column in start..row {
            diagonal[row] -= lower[row][row - column].powi(2) * diagonal[column];
        }
        assert!(diagonal[row].is_finite() && diagonal[row] != 0.0);
    }
    diagonal.iter().filter(|value| **value < 0.0).count()
}

#[test]
fn high_precision_bending_roots_have_independent_native_inertia_brackets() {
    for (index, root) in ROOTS.iter().enumerate() {
        assert_eq!(count_below(SEGMENTS, root * (1.0 - 1e-6)), index);
        assert_eq!(count_below(SEGMENTS, root * (1.0 + 1e-6)), index + 1);
    }
}
