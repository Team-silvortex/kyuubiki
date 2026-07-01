pub(crate) fn solve_linear_system(
    matrix: Vec<Vec<f64>>,
    vector: Vec<f64>,
) -> Result<Vec<f64>, String> {
    let size = vector.len();

    if matrix.len() != size || matrix.iter().any(|row| row.len() != size) {
        return Err("matrix dimensions do not match vector".to_string());
    }

    let mut augmented = matrix
        .into_iter()
        .zip(vector)
        .map(|(mut row, value)| {
            row.push(value);
            row
        })
        .collect::<Vec<_>>();

    for pivot in 0..size {
        let max_row = (pivot..size)
            .max_by(|&left, &right| {
                augmented[left][pivot]
                    .abs()
                    .partial_cmp(&augmented[right][pivot].abs())
                    .expect("finite pivot comparisons")
            })
            .expect("pivot range should not be empty");

        augmented.swap(pivot, max_row);

        let pivot_value = augmented[pivot][pivot];
        if pivot_value.abs() < 1.0e-12 {
            return Err("system is singular".to_string());
        }

        for column in pivot..=size {
            augmented[pivot][column] /= pivot_value;
        }

        for row in 0..size {
            if row == pivot {
                continue;
            }

            let factor = augmented[row][pivot];
            for column in pivot..=size {
                augmented[row][column] -= factor * augmented[pivot][column];
            }
        }
    }

    Ok(augmented.into_iter().map(|row| row[size]).collect())
}

pub(crate) fn zero_matrix(size: usize) -> Vec<Vec<f64>> {
    vec![vec![0.0; size]; size]
}
