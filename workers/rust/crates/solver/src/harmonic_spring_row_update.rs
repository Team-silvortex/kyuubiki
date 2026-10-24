use super::{Complex, cancellation_sensitive, ordinary_component, product_ok};
use crate::solver_control::check_cancellation;

// This budget selects replay, never physical acceptance. It bounds local FMA
// rounding relative to the represented numerator, not upstream solve error.
const REPLAY_BUDGET: f64 = 8.0 * f64::EPSILON;

pub(super) fn numerator(
    rhs: Complex,
    terms: impl Iterator<Item = (Complex, Complex)>,
) -> Result<Option<Complex>, String> {
    if !rhs.is_finite() {
        return Err("complex row contains a non-finite RHS".into());
    }
    let mut value = rhs;
    let mut peak = Complex {
        re: rhs.re.abs(),
        im: rhs.im.abs(),
    };
    let mut rounding = Complex::default();
    let mut safe = true;
    for (index, (a, x)) in terms.enumerate() {
        if !a.is_finite() || !x.is_finite() {
            return Err("complex product contains a non-finite operand".into());
        }
        let products = [a.re * x.re, a.im * x.im, a.re * x.im, a.im * x.re];
        safe &= product_ok(a.re, x.re, products[0])
            && product_ok(a.im, x.im, products[1])
            && product_ok(a.re, x.im, products[2])
            && product_ok(a.im, x.re, products[3]);
        let product = Complex {
            re: products[0] - products[1],
            im: products[2] + products[3],
        };
        safe &= product.is_finite();
        peak.re = peak.re.max(value.re.abs()).max(product.re.abs());
        peak.im = peak.im.max(value.im.abs()).max(product.im.abs());
        let first_re = fused(value.re, -a.re, x.re, &mut rounding.re);
        let first_im = fused(value.im, -a.re, x.im, &mut rounding.im);
        value.re = fused(first_re, a.im, x.im, &mut rounding.re);
        value.im = fused(first_im, -a.im, x.re, &mut rounding.im);
        safe &= ordinary_component(first_re)
            && ordinary_component(first_im)
            && ordinary_component(value.re)
            && ordinary_component(value.im)
            && !cancellation_sensitive(value.re, peak.re, 0.0)
            && !cancellation_sensitive(value.im, peak.im, 0.0);
        if (index + 1) % 1024 == 0 {
            check_cancellation()?;
        }
    }
    safe &= within_budget(value.re, rounding.re) && within_budget(value.im, rounding.im);
    Ok(safe.then_some(value))
}

fn fused(base: f64, a: f64, x: f64, rounding: &mut f64) -> f64 {
    if a == 0.0 || x == 0.0 {
        return base;
    }
    let value = a.mul_add(x, base);
    // Upward padding also covers a rounded-zero FMA. Subnormal intermediate
    // results are independently rejected from this ordinary path.
    let local = (value.abs() * f64::EPSILON).next_up();
    *rounding = (*rounding + local).next_up();
    value
}

fn within_budget(value: f64, rounding: f64) -> bool {
    rounding == 0.0 || (value != 0.0 && rounding <= value.abs() * REPLAY_BUDGET)
}
