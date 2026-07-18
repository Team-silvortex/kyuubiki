#[derive(Debug, Clone)]
pub(crate) struct IncompleteCholesky {
    diagonal: Vec<f64>,
    lower_columns: Vec<u32>,
    lower_offsets: Vec<usize>,
    lower_values: Vec<f64>,
    transpose_offsets: Vec<usize>,
    transpose_entries: Vec<u32>,
    transpose_rows: Vec<u32>,
}

impl IncompleteCholesky {
    pub(crate) fn build(
        row_offsets: &[usize],
        lower_end_offsets: &[usize],
        columns: &[usize],
        values: &[f64],
        diagonal: &[f64],
    ) -> Self {
        let size = diagonal.len();
        assert!(
            size <= u32::MAX as usize,
            "IC(0) index storage supports at most u32::MAX rows"
        );
        let lower_count = lower_end_offsets
            .iter()
            .enumerate()
            .map(|(row, &end)| end - row_offsets[row])
            .sum();
        let mut lower_columns = Vec::with_capacity(lower_count);
        let mut lower_offsets = Vec::with_capacity(size + 1);
        let mut lower_values = Vec::with_capacity(lower_count);
        let mut factor_diagonal = vec![0.0; size];
        lower_offsets.push(0);

        for row in 0..size {
            let row_start = lower_columns.len();
            for entry in row_offsets[row]..lower_end_offsets[row] {
                let column = columns[entry];
                let correction = lower_dot_until(
                    row_start,
                    lower_columns.len(),
                    column,
                    &lower_offsets,
                    &lower_columns,
                    &lower_values,
                );
                lower_columns.push(column as u32);
                lower_values.push((values[entry] - correction) / factor_diagonal[column]);
            }

            let lower_square_sum = lower_values[row_start..]
                .iter()
                .map(|value| value * value)
                .sum::<f64>();
            // A small positive floor keeps the preconditioner usable for nearly
            // singular assembled systems while the outer solver still checks residuals.
            factor_diagonal[row] = (diagonal[row] - lower_square_sum).max(1.0e-18).sqrt();
            lower_offsets.push(lower_columns.len());
        }

        let (transpose_offsets, transpose_entries, transpose_rows) =
            transpose_lower(size, &lower_offsets, &lower_columns);
        Self {
            diagonal: factor_diagonal,
            lower_columns,
            lower_offsets,
            lower_values,
            transpose_offsets,
            transpose_entries,
            transpose_rows,
        }
    }

    pub(crate) fn apply(&self, residual: &[f64], result: &mut [f64], forward: &mut [f64]) {
        for row in 0..self.diagonal.len() {
            let mut sum = residual[row];
            for entry in self.lower_offsets[row]..self.lower_offsets[row + 1] {
                sum -= self.lower_values[entry] * forward[self.lower_columns[entry] as usize];
            }
            forward[row] = sum / self.diagonal[row];
        }

        for row in (0..self.diagonal.len()).rev() {
            let mut sum = forward[row];
            for entry in self.transpose_offsets[row]..self.transpose_offsets[row + 1] {
                let factor_entry = self.transpose_entries[entry] as usize;
                sum -=
                    self.lower_values[factor_entry] * result[self.transpose_rows[entry] as usize];
            }
            result[row] = sum / self.diagonal[row];
        }
    }
}

fn lower_dot_until(
    row_start: usize,
    row_end: usize,
    column: usize,
    lower_offsets: &[usize],
    lower_columns: &[u32],
    lower_values: &[f64],
) -> f64 {
    let mut left = row_start;
    let mut right = lower_offsets[column];
    let right_end = lower_offsets[column + 1];
    let mut sum = 0.0;
    while left < row_end && right < right_end {
        let left_column = lower_columns[left];
        let right_column = lower_columns[right];
        if left_column == right_column {
            sum += lower_values[left] * lower_values[right];
            left += 1;
            right += 1;
        } else if left_column < right_column {
            left += 1;
        } else {
            right += 1;
        }
    }
    sum
}

fn transpose_lower(
    size: usize,
    lower_offsets: &[usize],
    lower_columns: &[u32],
) -> (Vec<usize>, Vec<u32>, Vec<u32>) {
    let mut counts = vec![0usize; size];
    for row in 0..size {
        for entry in lower_offsets[row]..lower_offsets[row + 1] {
            counts[lower_columns[entry] as usize] += 1;
        }
    }
    let mut offsets = Vec::with_capacity(size + 1);
    offsets.push(0);
    for count in &counts {
        offsets.push(offsets.last().copied().unwrap_or(0) + count);
    }
    let mut next = offsets[..size].to_vec();
    let mut entries = vec![0u32; offsets[size]];
    let mut rows = vec![0u32; offsets[size]];
    for row in 0..size {
        for entry in lower_offsets[row]..lower_offsets[row + 1] {
            let column = lower_columns[entry] as usize;
            let target = next[column];
            entries[target] = entry as u32;
            rows[target] = row as u32;
            next[column] += 1;
        }
    }
    (offsets, entries, rows)
}
