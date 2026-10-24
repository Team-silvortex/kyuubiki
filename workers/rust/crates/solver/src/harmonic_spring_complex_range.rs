use super::Complex;
use crate::{
    linear_binary_scale::{restore, split_finite},
    solver_control::{SolverStage, check_cancellation, checkpoint, checkpoint_chunk},
};

#[path = "harmonic_spring_exact_sum.rs"]
mod exact_sum;
use exact_sum::ExactSum;
pub(crate) use exact_sum::{CoefficientSum, InertiaSum};

#[path = "harmonic_spring_quotient.rs"]
mod quotient;

#[path = "harmonic_spring_complex_quotient.rs"]
mod complex_quotient;

#[path = "harmonic_spring_division_product.rs"]
mod division_product;

#[path = "harmonic_spring_fast_division.rs"]
mod fast_division;

#[path = "harmonic_spring_row_update.rs"]
mod row_update;

#[path = "harmonic_spring_factor.rs"]
mod factor;
pub(crate) use factor::EliminationFactor;

#[cfg(test)]
#[path = "harmonic_spring_quotient_tests.rs"]
mod quotient_tests;

#[cfg(test)]
#[path = "harmonic_spring_general_division_tests.rs"]
mod general_division_tests;

#[cfg(test)]
#[path = "harmonic_spring_fast_division_tests.rs"]
mod fast_division_tests;

#[cfg(test)]
#[path = "harmonic_spring_row_update_tests.rs"]
mod row_update_tests;

#[cfg(test)]
#[path = "harmonic_spring_row_quotient_tests.rs"]
mod row_quotient_tests;

pub(crate) fn product(
    left: Complex,
    right: Complex,
    stage: SolverStage,
) -> Result<Complex, String> {
    if let Some(value) = ordinary_product(left, right)? {
        return Ok(value);
    }
    difference(
        Complex::default(),
        Complex {
            re: -left.re,
            im: -left.im,
        },
        right,
        stage,
    )
}

pub(crate) fn difference(
    base: Complex,
    left: Complex,
    right: Complex,
    stage: SolverStage,
) -> Result<Complex, String> {
    let terms = std::iter::once((left, right));
    if let Some(value) = ordinary_numerator(base, terms.clone())? {
        return Ok(value);
    }
    let (real, imaginary) = numerator(base, terms, stage)?;
    Ok(Complex {
        re: real.restore_exact()?,
        im: imaginary.restore_exact()?,
    })
}

pub(crate) fn matrix_difference(
    base: Complex,
    left: Complex,
    right: Complex,
    stage: SolverStage,
) -> Result<Complex, String> {
    let terms = std::iter::once((left, right));
    if let Some(value) = row_update::numerator(base, terms.clone())? {
        return Ok(value);
    }
    let (real, imaginary) = numerator(base, terms, stage)?;
    Ok(Complex {
        re: real.restore_exact()?,
        im: imaginary.restore_exact()?,
    })
}

pub(crate) fn divide(left: Complex, right: Complex, stage: SolverStage) -> Result<Complex, String> {
    validate_pivot(right)?;
    if !left.is_finite() {
        return Err("complex division contains a non-finite numerator".into());
    }
    if let Some(value) = fast_division::divide(left, right) {
        return Ok(value);
    }
    divide_wide(Scaled::from(left.re), Scaled::from(left.im), right, stage)
}

pub(crate) fn solve_row<I>(
    rhs: impl Into<WideComplex>,
    terms: I,
    pivot: Complex,
    stage: SolverStage,
) -> Result<Complex, String>
where
    I: ExactSizeIterator<Item = (Complex, Complex)> + Clone,
{
    validate_pivot(pivot)?;
    let rhs = rhs.into();
    if let Some(finite) = rhs.finite() {
        if let Some(numerator) = row_update::numerator(finite, terms.clone())? {
            return divide(numerator, pivot, stage);
        }
    }
    let (real, imaginary) = accumulate_row(rhs, terms.map(|(a, x)| (a, x.into())), stage)?;
    complex_quotient::divide_accumulated(real, imaginary, pivot, stage)
}

#[derive(Clone, Copy)]
pub(crate) struct WideComplex {
    real: Scaled,
    imaginary: Scaled,
}

impl From<Complex> for WideComplex {
    fn from(value: Complex) -> Self {
        Self {
            real: Scaled::from(value.re),
            imaginary: Scaled::from(value.im),
        }
    }
}

impl WideComplex {
    pub(crate) fn normalized(self, scale: f64) -> Result<Self, String> {
        if !scale.is_finite() || scale <= 0.0 {
            return Err("complex row normalization requires a finite positive scale".into());
        }
        Ok(Self {
            real: self.real.divided_by_real(scale)?,
            imaginary: self.imaginary.divided_by_real(scale)?,
        })
    }

    pub(crate) fn finite(self) -> Option<Complex> {
        Some(Complex {
            re: self.real.restore_exact().ok()?,
            im: self.imaginary.restore_exact().ok()?,
        })
    }

    pub(crate) fn subtract_product(
        self,
        factor: Complex,
        other: Self,
        stage: SolverStage,
    ) -> Result<Self, String> {
        if let (Some(base), Some(value)) = (self.finite(), other.finite()) {
            if let Some(value) = row_update::numerator(base, std::iter::once((factor, value)))? {
                return Ok(value.into());
            }
        }
        let (real, imaginary) = wide_numerator(self, std::iter::once((factor, other)), stage)?;
        Ok(Self { real, imaginary })
    }

    pub(crate) fn divide(self, pivot: Complex, stage: SolverStage) -> Result<Complex, String> {
        if let Some(value) = self.finite() {
            return divide(value, pivot, stage);
        }
        divide_wide(self.real, self.imaginary, pivot, stage)
    }
}

fn ordinary_numerator<I>(rhs: Complex, terms: I) -> Result<Option<Complex>, String>
where
    I: Iterator<Item = (Complex, Complex)>,
{
    if !rhs.is_finite() {
        return Err("complex row contains a non-finite RHS".into());
    }
    let mut value = rhs;
    let mut peak = Complex {
        re: rhs.re.abs(),
        im: rhs.im.abs(),
    };
    let mut safe = true;
    for (index, (left, right)) in terms.enumerate() {
        if let Some(product) = ordinary_product(left, right)? {
            peak.re = peak.re.max(value.re.abs()).max(product.re.abs());
            peak.im = peak.im.max(value.im.abs()).max(product.im.abs());
            value = value - product;
            safe &= ordinary_component(value.re) && ordinary_component(value.im);
            // A gradual decline can expose earlier lost tails without a single large subtraction.
            safe &= !cancellation_sensitive(value.re, peak.re, 0.0)
                && !cancellation_sensitive(value.im, peak.im, 0.0);
        } else {
            safe = false;
        }
        if (index + 1) % 1024 == 0 {
            check_cancellation()?;
        }
    }
    Ok(safe.then_some(value))
}

fn ordinary_product(left: Complex, right: Complex) -> Result<Option<Complex>, String> {
    if !left.is_finite() || !right.is_finite() {
        return Err("complex product contains a non-finite operand".into());
    }
    let products = [
        left.re * right.re,
        left.im * right.im,
        left.re * right.im,
        left.im * right.re,
    ];
    let value = Complex {
        re: products[0] - products[1],
        im: products[2] + products[3],
    };
    Ok((product_ok(left.re, right.re, products[0])
        && product_ok(left.im, right.im, products[1])
        && product_ok(left.re, right.im, products[2])
        && product_ok(left.im, right.re, products[3])
        && ordinary_component(value.re)
        && ordinary_component(value.im)
        && !cancellation_sensitive(value.re, products[0], products[1])
        && !cancellation_sensitive(value.im, products[2], products[3]))
    .then_some(value))
}

fn cancellation_sensitive(value: f64, first: f64, second: f64) -> bool {
    // 2^-26 = sqrt(binary64 epsilon): a replay heuristic, not an admission tolerance.
    const REPLAY_RATIO: f64 = 1.0 / 67_108_864.0;
    let scale = first.abs().max(second.abs());
    scale > 0.0 && value.abs() <= scale * REPLAY_RATIO
}

fn product_ok(left: f64, right: f64, value: f64) -> bool {
    value.is_normal() || left == 0.0 || right == 0.0
}

fn ordinary_component(value: f64) -> bool {
    value.is_normal() || value == 0.0
}

fn validate_pivot(pivot: Complex) -> Result<(), String> {
    if !pivot.is_finite() || pivot.component_scale() == 0.0 {
        return Err("complex division requires a finite nonzero pivot".into());
    }
    Ok(())
}

fn numerator<I>(rhs: Complex, terms: I, stage: SolverStage) -> Result<(Scaled, Scaled), String>
where
    I: ExactSizeIterator<Item = (Complex, Complex)> + Clone,
{
    wide_numerator(rhs.into(), terms.map(|(a, x)| (a, x.into())), stage)
}

fn wide_numerator<I>(
    rhs: WideComplex,
    terms: I,
    stage: SolverStage,
) -> Result<(Scaled, Scaled), String>
where
    I: ExactSizeIterator<Item = (Complex, WideComplex)> + Clone,
{
    let (real, imaginary) = accumulate_row(rhs, terms, stage)?;
    let finish =
        |(sum, exponent): (ExactSum, i32)| sum.finish().map(|value| Scaled { value, exponent });
    Ok((finish(real)?, finish(imaginary)?))
}

type Accumulated = (ExactSum, i32);

fn accumulate_row<I>(
    rhs: WideComplex,
    terms: I,
    stage: SolverStage,
) -> Result<(Accumulated, Accumulated), String>
where
    I: ExactSizeIterator<Item = (Complex, WideComplex)> + Clone,
{
    let length = terms.len().saturating_mul(2).saturating_add(1);
    let real = std::iter::once((rhs.real.value, 1.0, rhs.real.exponent)).chain(
        terms.clone().flat_map(|(a, x)| {
            [
                (-a.re, x.real.value, x.real.exponent),
                (a.im, x.imaginary.value, x.imaginary.exponent),
            ]
        }),
    );
    let imaginary = std::iter::once((rhs.imaginary.value, 1.0, rhs.imaginary.exponent)).chain(
        terms.flat_map(|(a, x)| {
            [
                (-a.re, x.imaginary.value, x.imaginary.exponent),
                (-a.im, x.real.value, x.real.exponent),
            ]
        }),
    );
    Ok((
        accumulate_products(real, length, stage)?,
        accumulate_products(imaginary, length, stage)?,
    ))
}

fn divide_wide(
    real: Scaled,
    imaginary: Scaled,
    pivot: Complex,
    stage: SolverStage,
) -> Result<Complex, String> {
    validate_pivot(pivot)?;
    if pivot.im == 0.0 || pivot.re == 0.0 {
        checkpoint(stage, 0)?;
        let (first, second, divisor) = if pivot.im == 0.0 {
            (real, imaginary, Scaled::from(pivot.re))
        } else {
            (
                imaginary,
                Scaled {
                    value: -real.value,
                    exponent: real.exponent,
                },
                Scaled::from(pivot.im),
            )
        };
        let re = first.quotient(divisor)?;
        checkpoint(stage, 1)?;
        let im = second.quotient(divisor)?;
        checkpoint(stage, 2)?;
        return Ok(Complex { re, im });
    }
    complex_quotient::divide(real, imaginary, pivot, stage)
}

#[derive(Clone, Copy)]
struct Scaled {
    value: f64,
    exponent: i32,
}

impl From<f64> for Scaled {
    fn from(value: f64) -> Self {
        Self { value, exponent: 0 }
    }
}

impl Scaled {
    fn divided_by_real(self, scale: f64) -> Result<Self, String> {
        if !self.value.is_finite() {
            return Err("complex row contains a non-finite RHS".into());
        }
        let ordinary = self.value / scale;
        if self.exponent == 0 && (ordinary.is_normal() || self.value == 0.0) {
            return Ok(Self::from(ordinary));
        }
        let (value, exponent) = split_finite(self.value);
        let (divisor, power) = split_finite(scale);
        Ok(Self {
            value: value / divisor,
            exponent: self.exponent + exponent - power,
        })
    }

    fn restore_exact(self) -> Result<f64, String> {
        if self.exponent == 0 && self.value.is_finite() {
            return Ok(self.value);
        }
        let value = restore(self.value, self.exponent);
        if !value.is_finite() || restore(value, -self.exponent) != self.value {
            return Err("complex product/update restoration would lose numerical range".into());
        }
        Ok(value)
    }

    fn quotient(self, denominator: Self) -> Result<f64, String> {
        quotient::scaled_quotient(
            self.value,
            self.exponent,
            denominator.value,
            denominator.exponent,
        )
    }
}

// Two passes retain FMA tails; aligned parts accumulate without a rounded error balance.
fn accumulate_products(
    entries: impl Iterator<Item = (f64, f64, i32)> + Clone,
    length: usize,
    stage: SolverStage,
) -> Result<Accumulated, String> {
    checkpoint(stage, 0)?;
    let mut exponent = i32::MIN;
    for (index, entry) in entries.clone().enumerate() {
        let (high, _, power) = split_product(entry)?;
        if high != 0.0 {
            exponent = exponent.max(power);
        }
        checkpoint_chunk(stage, index + 1, length)?;
    }
    if exponent == i32::MIN {
        exponent = 0;
    }
    let mut sum = ExactSum::default();
    for (index, entry) in entries.enumerate() {
        let (high, low, power) = split_product(entry)?;
        for part in [high, low] {
            let value = restore(part, power - exponent);
            if !value.is_finite() || restore(value, exponent - power) != part {
                return Err("complex arithmetic normalization would lose numerical range".into());
            }
            sum.add(value)?;
        }
        checkpoint_chunk(stage, length + index + 1, length.saturating_mul(2))?;
    }
    Ok((sum, exponent))
}

fn split_product((left, right, exponent): (f64, f64, i32)) -> Result<(f64, f64, i32), String> {
    if !left.is_finite() || !right.is_finite() {
        return Err("complex arithmetic contains a non-finite operand".into());
    }
    let (left, le) = split_finite(left);
    let (right, re) = split_finite(right);
    let high = left * right;
    Ok((high, left.mul_add(right, -high), exponent + le + re))
}
