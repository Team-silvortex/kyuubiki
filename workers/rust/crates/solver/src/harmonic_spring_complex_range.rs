use super::Complex;
use crate::{
    linear_binary_scale::{restore, split_finite},
    solver_control::{SolverStage, check_cancellation, checkpoint, checkpoint_chunk},
};

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

pub(crate) fn divide(left: Complex, right: Complex, stage: SolverStage) -> Result<Complex, String> {
    validate_pivot(right)?;
    if !left.is_finite() {
        return Err("complex division contains a non-finite numerator".into());
    }
    let scale = right.component_scale();
    let re = right.re / scale;
    let im = right.im / scale;
    let denominator = re * re + im * im;
    let lr = left.re / scale;
    let li = left.im / scale;
    let rw = re / denominator;
    let iw = im / denominator;
    let products = [lr * rw, li * iw, li * rw, lr * iw];
    let value = Complex {
        re: products[0] + products[1],
        im: products[2] - products[3],
    };
    if scaled_component_ok(right.re, re)
        && scaled_component_ok(right.im, im)
        && scaled_component_ok(left.re, lr)
        && scaled_component_ok(left.im, li)
        && product_ok(lr, rw, products[0])
        && product_ok(li, iw, products[1])
        && product_ok(li, rw, products[2])
        && product_ok(lr, iw, products[3])
        && ordinary_component(value.re)
        && ordinary_component(value.im)
    {
        return Ok(value);
    }
    divide_wide(Scaled::from(left.re), Scaled::from(left.im), right, stage)
}

pub(crate) fn solve_row<I>(
    rhs: Complex,
    terms: I,
    pivot: Complex,
    stage: SolverStage,
) -> Result<Complex, String>
where
    I: ExactSizeIterator<Item = (Complex, Complex)> + Clone,
{
    validate_pivot(pivot)?;
    if let Some(numerator) = ordinary_numerator(rhs, terms.clone())? {
        return divide(numerator, pivot, stage);
    }
    let (real, imaginary) = numerator(rhs, terms, stage)?;
    divide_wide(real, imaginary, pivot, stage)
}

fn ordinary_numerator<I>(rhs: Complex, terms: I) -> Result<Option<Complex>, String>
where
    I: Iterator<Item = (Complex, Complex)>,
{
    if !rhs.is_finite() {
        return Err("complex row contains a non-finite RHS".into());
    }
    let mut value = rhs;
    let mut safe = true;
    for (index, (left, right)) in terms.enumerate() {
        if let Some(product) = ordinary_product(left, right)? {
            value = value - product;
            safe &= ordinary_component(value.re) && ordinary_component(value.im);
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
        && ordinary_component(value.im))
    .then_some(value))
}

fn product_ok(left: f64, right: f64, value: f64) -> bool {
    value.is_normal() || left == 0.0 || right == 0.0
}

fn ordinary_component(value: f64) -> bool {
    value.is_normal() || value == 0.0
}

fn scaled_component_ok(original: f64, scaled: f64) -> bool {
    scaled.is_normal() || original == 0.0
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
    let length = terms.len().saturating_mul(2).saturating_add(1);
    let real = std::iter::once((rhs.re, 1.0, 0)).chain(
        terms
            .clone()
            .flat_map(|(a, x)| [(-a.re, x.re, 0), (a.im, x.im, 0)]),
    );
    let imaginary = std::iter::once((rhs.im, 1.0, 0))
        .chain(terms.flat_map(|(a, x)| [(-a.re, x.im, 0), (-a.im, x.re, 0)]));
    Ok((
        sum_products(real, length, stage)?,
        sum_products(imaginary, length, stage)?,
    ))
}

fn divide_wide(
    real: Scaled,
    imaginary: Scaled,
    pivot: Complex,
    stage: SolverStage,
) -> Result<Complex, String> {
    validate_pivot(pivot)?;
    let denominator = sum_products(
        [(pivot.re, pivot.re, 0), (pivot.im, pivot.im, 0)].into_iter(),
        2,
        stage,
    )?;
    let nr = sum_products(
        [
            (real.value, pivot.re, real.exponent),
            (imaginary.value, pivot.im, imaginary.exponent),
        ]
        .into_iter(),
        2,
        stage,
    )?;
    let ni = sum_products(
        [
            (imaginary.value, pivot.re, imaginary.exponent),
            (-real.value, pivot.im, real.exponent),
        ]
        .into_iter(),
        2,
        stage,
    )?;
    Ok(Complex {
        re: nr.quotient(denominator)?,
        im: ni.quotient(denominator)?,
    })
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
    fn restore_exact(self) -> Result<f64, String> {
        let value = restore(self.value, self.exponent);
        if !value.is_finite() || restore(value, -self.exponent) != self.value {
            return Err("complex product/update restoration would lose numerical range".into());
        }
        Ok(value)
    }

    fn quotient(self, denominator: Self) -> Result<f64, String> {
        let (numerator, ne) = split_finite(self.value);
        let (divisor, de) = split_finite(denominator.value);
        let value = restore(
            numerator / divisor,
            self.exponent + ne - denominator.exponent - de,
        );
        // Rounded subnormal unknowns remain subject to original-equation admission.
        if !value.is_finite() || (self.value != 0.0 && value == 0.0) {
            return Err("complex solution is outside the representable range".into());
        }
        Ok(value)
    }
}

// Two passes over scalar products, retaining FMA tails without an unscaled dot product.
fn sum_products(
    entries: impl Iterator<Item = (f64, f64, i32)> + Clone,
    length: usize,
    stage: SolverStage,
) -> Result<Scaled, String> {
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
    let mut sum = 0.0_f64;
    let mut correction = 0.0_f64;
    for (index, entry) in entries.enumerate() {
        let (high, low, power) = split_product(entry)?;
        for part in [high, low] {
            let value = restore(part, power - exponent);
            if !value.is_finite() || restore(value, exponent - power) != part {
                return Err("complex arithmetic normalization would lose numerical range".into());
            }
            let next = sum + value;
            correction += if sum.abs() >= value.abs() {
                (sum - next) + value
            } else {
                (value - next) + sum
            };
            sum = next;
        }
        checkpoint_chunk(stage, length + index + 1, length.saturating_mul(2))?;
    }
    let value = sum + correction;
    if !value.is_finite() {
        return Err("complex arithmetic scaled accumulation diverged".into());
    }
    Ok(Scaled { value, exponent })
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
