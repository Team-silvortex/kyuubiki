use super::{
    Complex,
    arithmetic::{CoefficientSum, InertiaSum},
};
use crate::linear_binary_scale::restore;
use crate::solver_control::{SolverStage, check_cancellation, checkpoint, checkpoint_chunk};

// Exact additions need no scratch storage. Retain only their lost tails,
// with scalar cell IDs shared by dense and path assembly layouts.
#[derive(Default)]
pub(super) struct Corrections {
    tails: Vec<(usize, f64)>,
    scaled: Vec<(usize, f64, i32)>,
    failed: bool,
}

impl Corrections {
    pub(super) fn add_scaled_real(
        &mut self,
        cell: usize,
        value: &mut Complex,
        parts: [f64; 4],
        exponent: i32,
    ) -> Result<(), String> {
        if self.failed {
            return Err("harmonic assembly correction is invalid".into());
        }
        if !(-3222..=1023).contains(&exponent) || parts.iter().any(|part| !part.is_finite()) {
            self.failed = true;
            return Err("harmonic scaled assembly contains an invalid part or exponent".into());
        }
        let finite = if exponent == 0 {
            parts
        } else {
            parts.map(|part| restore(part, exponent))
        };
        if exponent == 0
            || finite
                .iter()
                .zip(parts)
                .all(|(&f, p)| f.is_finite() && restore(f, -exponent) == p)
        {
            for part in finite {
                if let Err(error) = self.add_component(2 * cell, &mut value.re, part) {
                    self.failed = true;
                    return Err(error);
                }
            }
        } else {
            for part in parts.into_iter().filter(|part| *part != 0.0) {
                self.scaled.push((2 * cell, part, exponent));
            }
        }
        Ok(())
    }

    pub(super) fn add(
        &mut self,
        cell: usize,
        value: &mut Complex,
        term: Complex,
    ) -> Result<(), String> {
        if self.failed {
            return Err("harmonic assembly correction is invalid".into());
        }
        let result = self
            .add_component(2 * cell, &mut value.re, term.re)
            .and_then(|()| self.add_component(2 * cell + 1, &mut value.im, term.im));
        self.failed = result.is_err();
        result
    }

    fn add_component(&mut self, cell: usize, value: &mut f64, term: f64) -> Result<(), String> {
        if !value.is_finite() || !term.is_finite() {
            return Err("harmonic assembly contains a non-finite coefficient".into());
        }
        if *value == 0.0 || term == 0.0 {
            *value += term;
            return Ok(());
        }
        let sum = *value + term;
        let recovered = sum - *value;
        let error = (*value - (sum - recovered)) + (term - recovered);
        if sum.is_finite() && error.is_finite() {
            if error != 0.0 {
                self.tails.push((cell, error));
            }
            *value = sum;
        } else {
            // Do not discard finite operands when a partial sum or TwoSum
            // intermediate overflows. Final exact rounding decides the range.
            self.tails.push((cell, *value));
            self.tails.push((cell, term));
            *value = 0.0;
        }
        Ok(())
    }

    pub(super) fn finish<'a>(
        mut self,
        cells: impl Iterator<Item = &'a mut Complex>,
        assembled: usize,
    ) -> Result<(), String> {
        if self.failed {
            return Err("harmonic assembly correction is invalid".into());
        }
        if self.tails.is_empty() && self.scaled.is_empty() {
            return Ok(());
        }
        checkpoint(SolverStage::ElementAssembly, assembled.saturating_add(1))?;
        self.tails.sort_unstable_by_key(|(cell, _)| *cell);
        check_cancellation()?;
        self.scaled.sort_unstable_by_key(|(cell, _, _)| *cell);
        checkpoint(SolverStage::ElementAssembly, assembled.saturating_add(2))?;
        let count = self.tails.len().saturating_add(self.scaled.len());
        let mut tails = self.tails.into_iter().peekable();
        let mut scaled = self.scaled.into_iter().peekable();
        let mut processed = 0;
        for (cell, value) in cells.flat_map(|c| [&mut c.re, &mut c.im]).enumerate() {
            if tails.peek().is_some_and(|(index, _)| *index == cell)
                || scaled.peek().is_some_and(|(index, _, _)| *index == cell)
            {
                let result = if scaled.peek().is_some_and(|(index, _, _)| *index == cell) {
                    let mut sum = InertiaSum::default();
                    sum.add_scaled(*value, 0)?;
                    consume(
                        cell,
                        &mut tails,
                        &mut scaled,
                        &mut processed,
                        count,
                        |v, e| sum.add_scaled(v, e),
                    )?;
                    sum.finish()
                } else {
                    let mut sum = CoefficientSum::default();
                    sum.add(*value)?;
                    consume(
                        cell,
                        &mut tails,
                        &mut scaled,
                        &mut processed,
                        count,
                        |v, e| {
                            debug_assert_eq!(e, 0);
                            sum.add(v)
                        },
                    )?;
                    sum.finish()
                };
                *value = result
                    .map_err(|error| format!("harmonic assembly coefficient {cell}: {error}"))?;
            }
            if (cell + 1) % 64 == 0 {
                check_cancellation()?;
            }
            if tails.peek().is_none() && scaled.peek().is_none() {
                checkpoint(SolverStage::ElementAssembly, assembled.saturating_add(3))?;
                return Ok(());
            }
        }
        Err("harmonic assembly correction cell is outside the matrix".into())
    }
}

type FiniteTails = std::iter::Peekable<std::vec::IntoIter<(usize, f64)>>;
type ScaledTails = std::iter::Peekable<std::vec::IntoIter<(usize, f64, i32)>>;

fn consume(
    cell: usize,
    tails: &mut FiniteTails,
    scaled: &mut ScaledTails,
    processed: &mut usize,
    count: usize,
    mut add: impl FnMut(f64, i32) -> Result<(), String>,
) -> Result<(), String> {
    while tails.peek().is_some_and(|(index, _)| *index == cell) {
        add(tails.next().unwrap().1, 0)?;
        *processed += 1;
        checkpoint_chunk(SolverStage::ElementAssembly, *processed, count)?;
    }
    while scaled.peek().is_some_and(|(index, _, _)| *index == cell) {
        let (_, value, exponent) = scaled.next().unwrap();
        add(value, exponent)?;
        *processed += 1;
        checkpoint_chunk(SolverStage::ElementAssembly, *processed, count)?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "harmonic_spring_assembly_sum_tests.rs"]
mod tests;
