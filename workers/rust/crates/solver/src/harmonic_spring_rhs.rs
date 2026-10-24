use super::{Complex, arithmetic::WideComplex};
use crate::solver_control::{SolverStage, checkpoint, checkpoint_chunk};

// Ordinary systems keep their original allocation; only exceptional RHS values promote it.
pub(super) enum ComplexRhs {
    Finite(Vec<Complex>),
    Wide(Vec<WideComplex>),
}

impl ComplexRhs {
    pub(super) fn new(values: Vec<Complex>) -> Self {
        Self::Finite(values)
    }

    pub(super) fn get(&self, row: usize) -> WideComplex {
        match self {
            Self::Finite(values) => values[row].into(),
            Self::Wide(values) => values[row],
        }
    }

    pub(super) fn solution(&self, row: usize) -> Result<Complex, String> {
        self.get(row)
            .finite()
            .ok_or_else(|| "complex RHS back substitution did not produce a finite solution".into())
    }

    pub(super) fn swap(&mut self, first: usize, second: usize) {
        match self {
            Self::Finite(values) => values.swap(first, second),
            Self::Wide(values) => values.swap(first, second),
        }
    }

    pub(super) fn normalize(&mut self, row: usize, scale: f64) -> Result<(), String> {
        self.set(
            row,
            self.get(row).normalized(scale)?,
            SolverStage::LinearPrepare,
        )
    }

    pub(super) fn set(
        &mut self,
        row: usize,
        value: WideComplex,
        stage: SolverStage,
    ) -> Result<(), String> {
        match self {
            Self::Wide(values) => {
                values[row] = value;
            }
            Self::Finite(values) => {
                if let Some(finite) = value.finite() {
                    values[row] = finite;
                    return Ok(());
                }
                checkpoint(stage, 0)?;
                let size = values.len();
                let mut wide = Vec::with_capacity(size);
                // Keep the finite source intact until every fallible checkpoint passes.
                for (index, entry) in values.iter().copied().enumerate() {
                    wide.push(entry.into());
                    checkpoint_chunk(stage, index + 1, size)?;
                }
                wide[row] = value;
                *self = Self::Wide(wide);
            }
        }
        Ok(())
    }

    pub(super) fn into_solutions(self) -> Result<Vec<Complex>, String> {
        match self {
            Self::Finite(values) => Ok(values),
            Self::Wide(values) => {
                let size = values.len();
                let mut solutions = Vec::with_capacity(size);
                for (index, value) in values.into_iter().enumerate() {
                    solutions.push(value.finite().ok_or_else(|| {
                        "complex RHS back substitution did not produce a finite solution"
                            .to_string()
                    })?);
                    checkpoint_chunk(SolverStage::TridiagonalSubstitution, index + 1, size)?;
                }
                Ok(solutions)
            }
        }
    }
}

#[cfg(test)]
#[path = "harmonic_spring_rhs_tests.rs"]
mod tests;
