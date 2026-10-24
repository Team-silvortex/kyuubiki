use super::*;
use crate::harmonic_spring_linear::arithmetic::InertiaSum;

fn total(inertia: Inertia, stiffness: f64) -> Result<f64, String> {
    let mut sum = InertiaSum::default();
    sum.add_scaled(stiffness, 0)?;
    for part in inertia.parts {
        sum.add_scaled(-part, inertia.exponent)?;
    }
    sum.finish()
}

fn assembled(frequency: f64, mass: f64, stiffness: f64) -> f64 {
    total(inertia_parts(frequency, mass).unwrap(), stiffness).unwrap()
}

#[test]
fn harmonic_inertia_precision_parts_preserve_the_independent_cubic_cancellation() {
    for q in [27, 30, 34] {
        let d = 2.0_f64.powi(-q);
        let stiffness = 1.0 + 3.0 * d + 2.0_f64.powi(-51);
        // Subtracting the exact cubic leaves the dyadic expression below.
        // These exponents keep this expression within 53 bits without rounding.
        let expected = 2.0_f64.powi(-51) - 3.0 * 2.0_f64.powi(-2 * q) - 2.0_f64.powi(-3 * q);
        for power in [-500, 0, 500] {
            assert_eq!(
                assembled(
                    (1.0 + d) * 2.0_f64.powi(power),
                    (1.0 + d) * 2.0_f64.powi(-2 * power),
                    stiffness
                ),
                expected
            );
        }
    }
}

#[test]
fn harmonic_inertia_precision_zero_fma_does_not_certify_an_underflowed_product_tail() {
    let epsilon = f64::EPSILON;
    let frequency = (1.0 + epsilon) * 2.0_f64.powi(-511);
    let square = frequency * frequency;
    assert_eq!(frequency.mul_add(frequency, -square), 0.0);
    let inertia = inertia_parts(frequency, 1.0).unwrap();
    assert!(inertia.parts[1..].iter().any(|part| *part != 0.0));
    assert_eq!(
        assembled(frequency, 2.0_f64.powi(1022), 1.0 + 3.0 * epsilon),
        epsilon - epsilon * epsilon
    );
    let mass = f64::MIN_POSITIVE.next_up();
    let rounded = 2.25 * mass;
    assert_eq!(2.25_f64.mul_add(mass, -rounded), 0.0);
    assert!(
        total(inertia_parts(1.5, mass).unwrap(), rounded)
            .unwrap_err()
            .contains("lose numerical range")
    );
}

#[test]
fn harmonic_inertia_precision_parts_match_an_independent_integer_product_oracle() {
    let mut seed = 0xbd92_e311_769a_6c25_u64;
    for _ in 0..256 {
        seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
        let a = 257 + (seed >> 32) % 767;
        let b = 257 + seed % 767;
        // The integer product is below 2^30, so the single conversion is exact.
        let exact = a * a * b;
        for power in [-500, 0, 500] {
            let frequency = a as f64 * 2.0_f64.powi(power - 10);
            let mass = b as f64 * 2.0_f64.powi(-2 * power - 10);
            let negative = total(inertia_parts(frequency, mass).unwrap(), 0.0).unwrap();
            assert_eq!(-negative, exact as f64 * 2.0_f64.powi(-30));
        }
    }
}

#[test]
fn harmonic_inertia_precision_keeps_exact_products_zero_and_scaled_range_recovery() {
    for (f, m, expected) in [
        (1.0, f64::MAX, f64::MAX),
        (0.5, 1.25, 0.3125),
        (2.0_f64.powi(600), 2.0_f64.powi(-900), 2.0_f64.powi(300)),
        (2.0_f64.powi(-600), 2.0_f64.powi(900), 2.0_f64.powi(-300)),
        (1.0, f64::from_bits(1), f64::from_bits(1)),
        (0.5, 4.0 * f64::from_bits(1), f64::from_bits(1)),
    ] {
        let parts = inertia_parts(f, m).unwrap();
        assert_eq!(-total(parts, 0.0).unwrap(), expected);
        assert!(parts.parts[1..].iter().all(|part| *part == 0.0));
    }
    assert_eq!(inertia_parts(0.0, f64::MAX).unwrap().parts, [0.0; 4]);
    assert_eq!(inertia_parts(f64::MAX, 0.0).unwrap().parts, [0.0; 4]);
}

#[test]
fn harmonic_inertia_precision_full_integer_products_and_weak_differences_match_u128_reference() {
    let mut seed = 0x91d5_073f_9a2c_44b7_u64;
    let mask = (1_u64 << 41) - 1;
    for _ in 0..256 {
        seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
        let a = (1_u64 << 41) | (seed & mask) | 1;
        seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
        let b = (1_u64 << 41) | (seed & mask) | 1;
        let exact = u128::from(a) * u128::from(a) * u128::from(b);
        let rounded = exact as f64;
        let stiffness = rounded * 2.0_f64.powi(-123);
        let delta = ((rounded as i128) - exact as i128) as f64 * 2.0_f64.powi(-123);
        for power in [-500, 0, 500] {
            let frequency = a as f64 * 2.0_f64.powi(power - 41);
            let mass = (b as f64 * 2.0_f64.powi(-41)) * 2.0_f64.powi(-2 * power);
            assert_eq!(
                -total(inertia_parts(frequency, mass).unwrap(), 0.0).unwrap(),
                stiffness
            );
            assert_eq!(
                assembled(frequency, mass, stiffness).to_bits(),
                delta.to_bits()
            );
        }
    }
}

#[test]
fn harmonic_inertia_precision_retains_scaled_tails_and_rejects_true_range_loss() {
    let d = 2.0_f64.powi(-27);
    let inertia = inertia_parts(1.0 + d, (1.0 + d) * 2.0_f64.powi(-1000)).unwrap();
    assert!(
        inertia
            .parts
            .iter()
            .any(|part| *part != 0.0 && restore_checked(*part, inertia.exponent).is_err())
    );
    assert_eq!(total(inertia, 1.0).unwrap(), 1.0);
    assert!(
        inertia_parts(2.0_f64.powi(600), 1.0)
            .unwrap_err()
            .contains("finite numeric range")
    );
    assert!(
        inertia_parts(2.0_f64.powi(-600), 1.0)
            .unwrap_err()
            .contains("lose numerical range")
    );
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(inertia_parts(bad, 0.0).is_err());
        assert!(inertia_parts(0.0, bad).is_err());
    }
    assert_eq!(
        assembled(1.0 + d, 1.0 + d, 1.0 + 3.0 * d + 2.0_f64.powi(-51)),
        5.0 * 2.0_f64.powi(-54) - 2.0_f64.powi(-81)
    );
}
