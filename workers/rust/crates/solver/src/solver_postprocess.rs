use crate::solver_control::{SolverStage, checkpoint};

const CHUNK: usize = 64;
const MAX_CHUNK: usize = 1024;

pub(crate) fn collect_results<T>(
    stage: SolverStage,
    mut values: impl ExactSizeIterator<Item = T>,
) -> Result<Vec<T>, String> {
    checkpoint(stage, 0)?;
    let count = values.len();
    let mut result = Vec::with_capacity(count);
    for start in (0..count).step_by(CHUNK) {
        let end = (start + CHUNK).min(count);
        result.extend(values.by_ref().take(end - start));
        checkpoint(stage, end)?;
    }
    Ok(result)
}

pub(crate) fn fold_results<T, A>(
    stage: SolverStage,
    mut values: impl ExactSizeIterator<Item = T>,
    mut accumulated: A,
    mut fold: impl FnMut(A, T) -> A,
) -> Result<A, String> {
    checkpoint(stage, 0)?;
    let count = values.len();
    for start in (0..count).step_by(CHUNK) {
        let end = (start + CHUNK).min(count);
        // Carry the same accumulator across blocks; never regroup physical sums.
        accumulated = values
            .by_ref()
            .take(end - start)
            .fold(accumulated, &mut fold);
        checkpoint(stage, end)?;
    }
    Ok(accumulated)
}

pub(crate) fn max_results<T>(
    stage: SolverStage,
    values: &[T],
    mut project: impl FnMut(&T) -> f64,
) -> Result<f64, String> {
    checkpoint(stage, 0)?;
    let mut maximum = 0.0_f64;
    for (block, values) in values.chunks(MAX_CHUNK).enumerate() {
        maximum = values.iter().map(&mut project).fold(maximum, f64::max);
        checkpoint(stage, block * MAX_CHUNK + values.len())?;
    }
    Ok(maximum)
}

pub(crate) fn restore_solution(
    count: usize,
    prescribed: &[(usize, f64)],
    free: &[usize],
    reduced: &[f64],
) -> Result<Vec<f64>, String> {
    debug_assert_eq!(free.len(), reduced.len());
    checkpoint(SolverStage::ResultPrescribed, 0)?;
    let mut full = vec![0.0; count];
    for (block, values) in prescribed.chunks(CHUNK).enumerate() {
        for &(index, value) in values {
            full[index] = value;
        }
        checkpoint(SolverStage::ResultPrescribed, block * CHUNK + values.len())?;
    }
    fold_results(
        SolverStage::ResultFreeDofs,
        free.iter().enumerate(),
        (),
        |(), (index, &dof)| {
            full[dof] = reduced[index];
        },
    )?;
    Ok(full)
}

#[cfg(test)]
#[path = "solver_postprocess_kernel_tests.rs"]
mod tests;
