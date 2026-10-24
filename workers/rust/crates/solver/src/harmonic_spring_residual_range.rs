use super::{Complex, Equation, Product, visit_terms};
use crate::{
    linear_binary_scale::restore,
    solver_control::{SolverStage, checkpoint, checkpoint_chunk},
};
use kyuubiki_protocol::SolveHarmonicSpring1dRequest;

#[path = "harmonic_spring_residual_sum.rs"]
mod sum;
use sum::IntervalSum;

pub(super) fn validate(
    request: &SolveHarmonicSpring1dRequest,
    omega: f64,
    displacement: &[Complex],
    reduced: &[Option<usize>],
    rows: &[Equation],
    tolerance: f64,
) -> Result<(), String> {
    checkpoint(SolverStage::HarmonicResidual, 0)?;
    // Only failed alignments allocate enclosure state. Row enumeration already
    // orders this sparse index; no matrix copy or sorting pass is needed.
    let mut entries = Vec::new();
    for (row, equation) in rows.iter().enumerate() {
        if equation.needs_replay {
            entries.push((row, Enclosure::new(equation.exponent)));
        }
        checkpoint_chunk(SolverStage::HarmonicResidual, row + 1, rows.len())?;
    }
    for (index, node) in request.nodes.iter().enumerate() {
        if let Some(row) = reduced[index].filter(|row| rows[*row].needs_replay) {
            let target = entry(&mut entries, row)?;
            let load = normalized(&Product::new(node.load_x, 1.0)?, target.exponent)?;
            target.real.add(load)?;
            target.add_scale(load.minimum_abs())?;
        }
        checkpoint_chunk(
            SolverStage::HarmonicResidual,
            index + 1,
            request.nodes.len(),
        )?;
    }
    visit_terms(request, omega, displacement, reduced, |row, products| {
        if rows[row].needs_replay {
            entry(&mut entries, row)?.subtract(products)?;
        }
        Ok(())
    })?;
    let count = entries.len();
    for (index, (row, enclosure)) in entries.into_iter().enumerate() {
        enclosure.check(row, tolerance)?;
        checkpoint_chunk(SolverStage::HarmonicResidual, index + 1, count)?;
    }
    Ok(())
}

fn entry(entries: &mut [(usize, Enclosure)], row: usize) -> Result<&mut Enclosure, String> {
    let index = entries
        .binary_search_by_key(&row, |(id, _)| *id)
        .map_err(|_| "harmonic residual enclosure is missing its original row".to_string())?;
    Ok(&mut entries[index].1)
}

#[derive(Clone, Copy, Debug, Default)]
struct Interval {
    lower: f64,
    upper: f64,
}

impl Interval {
    fn checked(self) -> Result<Self, String> {
        if !self.lower.is_finite() || !self.upper.is_finite() || self.lower > self.upper {
            return Err("harmonic residual enclosure contains invalid bounds".into());
        }
        Ok(self)
    }

    fn add(&mut self, other: Self) -> Result<(), String> {
        self.checked()?;
        other.checked()?;
        if other.lower == 0.0 && other.upper == 0.0 {
            return Ok(());
        }
        if self.lower == 0.0 && self.upper == 0.0 {
            *self = other;
            return Ok(());
        }
        *self = Self {
            lower: (self.lower + other.lower).next_down(),
            upper: (self.upper + other.upper).next_up(),
        }
        .checked()?;
        Ok(())
    }

    fn negated(self) -> Self {
        Self {
            lower: -self.upper,
            upper: -self.lower,
        }
    }

    fn minimum_abs(self) -> f64 {
        if self.lower <= 0.0 && self.upper >= 0.0 {
            0.0
        } else {
            self.lower.abs().min(self.upper.abs())
        }
    }

    fn maximum_abs(self) -> f64 {
        self.lower.abs().max(self.upper.abs())
    }
}

fn normalized(product: &Product, exponent: i32) -> Result<Interval, String> {
    let (shift, reverse) = product.shifts(exponent)?;
    let mut total = Interval::default();
    for part in [product.high, product.low] {
        if !part.is_finite() {
            return Err("harmonic residual enclosure contains a non-finite part".into());
        }
        let value = restore(part, shift);
        let bound = if restore(value, reverse) == part {
            Interval {
                lower: value,
                upper: value,
            }
        } else {
            // Binary scaling may lose subnormal bits. Retain neighboring
            // endpoints rather than treating a rounded zero as exact zero.
            Interval {
                lower: if part > 0.0 {
                    value.next_down().max(0.0)
                } else {
                    value.next_down()
                },
                upper: if part < 0.0 {
                    value.next_up().min(0.0)
                } else {
                    value.next_up()
                },
            }
        };
        total.add(bound)?;
    }
    Ok(total)
}

struct Enclosure {
    exponent: i32,
    real: IntervalSum,
    imaginary: IntervalSum,
    scale_lower: f64,
}

impl Enclosure {
    fn new(exponent: i32) -> Self {
        Self {
            exponent,
            real: IntervalSum::default(),
            imaginary: IntervalSum::default(),
            scale_lower: 0.0,
        }
    }

    fn add_scale(&mut self, lower: f64) -> Result<(), String> {
        if !lower.is_finite() || lower < 0.0 {
            return Err("harmonic residual enclosure contains invalid scale".into());
        }
        if lower == 0.0 {
            return Ok(());
        }
        let sum = self.scale_lower + lower;
        if !sum.is_finite() {
            return Err("harmonic residual enclosure scale exceeds range".into());
        }
        self.scale_lower = if self.scale_lower == 0.0 {
            lower
        } else {
            sum.next_down().max(0.0)
        };
        Ok(())
    }

    fn subtract(&mut self, products: [Product; 4]) -> Result<(), String> {
        let mut lower = 0.0_f64;
        for (index, product) in products.iter().enumerate() {
            let bound = normalized(product, self.exponent)?;
            // max |component| is a lower bound for the four-product L2 norm.
            lower = lower.max(bound.minimum_abs());
            let signed = if index == 1 { bound } else { bound.negated() };
            if index < 2 {
                self.real.add(signed)?;
            } else {
                self.imaginary.add(signed)?;
            }
        }
        self.add_scale(lower)
    }

    fn check(&self, row: usize, tolerance: f64) -> Result<(), String> {
        let real = self.real.interval()?;
        let imaginary = self.imaginary.interval()?;
        if !tolerance.is_finite()
            || tolerance < 0.0
            || !self.scale_lower.is_finite()
            || self.scale_lower <= 0.0
        {
            return Err(format!(
                "harmonic residual enclosure cannot certify free row {row}"
            ));
        }
        let a = real.maximum_abs();
        let b = imaginary.maximum_abs();
        // L1 bounds L2 from above. No hypot accuracy assumption is required.
        let upper = if a == 0.0 {
            b
        } else if b == 0.0 {
            a
        } else {
            (a + b).next_up()
        };
        let relative = if upper == 0.0 {
            0.0
        } else {
            (upper / self.scale_lower).next_up()
        };
        if !relative.is_finite() || relative > tolerance {
            return Err(format!(
                "harmonic spring 1d dynamic stiffness failed backward-error enclosure at free row {row} ({relative:.6e})"
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "harmonic_spring_residual_interval_tests.rs"]
mod tests;
