// A fixed IEEE arithmetic path, rather than platform libm: last-bit changes in
// rotations can change the recovery path of ill-conditioned modal systems.
pub(crate) fn hypot(left: f64, right: f64) -> f64 {
    let (left, right) = (left.abs(), right.abs());
    if left.is_infinite() || right.is_infinite() {
        return f64::INFINITY;
    }
    if left.is_nan() || right.is_nan() {
        return f64::NAN;
    }
    let (a, b) = (left.max(right), left.min(right));
    if b <= a * f64::EPSILON {
        return a;
    }
    // Scale by an exact power of two, including for subnormal inputs.
    let scale = f64::from_bits((a.to_bits() & (0x7ff_u64 << 52)).max(1_u64 << 52));
    let (x, y) = (a / scale, b / scale);
    y.mul_add(y, x * x).sqrt() * scale
}

#[cfg(test)]
#[path = "modal_norm_tests.rs"]
mod tests;
