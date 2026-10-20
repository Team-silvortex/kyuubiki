use super::hypot;
use crate::modal_test_wide::Wide;

#[test]
fn modal_norm_has_a_fixed_cross_platform_rounding_path() {
    // Minimal inputs on which system libm changes the final bit across platforms.
    // Pin the arithmetic contract, not the host's implementation of hypot.
    let cases = [
        (0x3ff7bfc8db5629ce, 0x3f27bd6df62f9e8f, 0x3ff7bfc8de4d8c38),
        (0x3ff336d9bc2d9e7d, 0x3f6aa06ffcb96c40, 0x3ff336de58ef6409),
        (0x3ff5573cd804fab7, 0x3fa6f199842ac4f4, 0x3ff55a51f8288996),
        (0x3ff476bb2ba13387, 0x3ee60c44da1fcf29, 0x3ff476bb2ba42bae),
        (0x3ff0a98c781a41f7, 0x3faf928d7594cf9e, 0x3ff0b1053117c9d3),
        (0x3ff09568a972f0f1, 0x3f0b9462c425a6ef, 0x3ff09568a9ceacaf),
        (0x3ff090a2ca4bab11, 0x3ec2da7aaa9483b0, 0x3ff090a2ca4bd5fb),
        (0x3ff8d88aa0fdc246, 0x3fabada61dcbba07, 0x3ff8dc6501ebda8c),
    ];
    for (a, b, expected) in cases {
        for (x, y) in [(a, b), (b, a)] {
            for sign in [-1.0, 1.0] {
                assert_eq!(
                    hypot(sign * f64::from_bits(x), -f64::from_bits(y)).to_bits(),
                    expected
                );
            }
        }
    }
}

#[test]
fn modal_norm_checks_accuracy_independently_and_preserves_binary_scaling() {
    let mut state = 17_u64;
    for _ in 0..4096 {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let x = f64::from_bits((1023_u64 << 52) | (state >> 12));
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let y = f64::from_bits(((1023 - state % 64) << 52) | (state >> 12));
        let square = Wide::from(x)
            .mul(Wide::from(x))
            .add(Wide::from(y).mul(Wide::from(y)));
        let mut root = Wide::from(square.rounded().sqrt());
        for _ in 0..2 {
            root = root.add(square.sub(root.mul(root)).div(root.mul(Wide::from(2.0))));
        }
        let norm = hypot(x, y);
        assert!(
            norm.to_bits().abs_diff(root.rounded().to_bits()) <= 1,
            "x={x:e}, y={y:e}, norm={norm:e}, reference={root:?}"
        );
        for exponent in [-900, -500, 0, 500, 900] {
            let scale = f64::from_bits(((exponent + 1023) as u64) << 52);
            assert_eq!(
                hypot(x * scale, y * scale).to_bits(),
                (norm * scale).to_bits()
            );
        }
    }
}

#[test]
fn modal_norm_preserves_subnormal_and_extreme_finite_values() {
    for scale in [
        f64::from_bits(1),
        f64::MIN_POSITIVE,
        1.0,
        f64::from_bits(2043_u64 << 52),
    ] {
        assert_eq!(hypot(3.0 * scale, -4.0 * scale), 5.0 * scale);
    }
    assert_eq!(hypot(f64::MAX, f64::MIN_POSITIVE), f64::MAX);
    assert_eq!(
        hypot(f64::MIN_POSITIVE, f64::from_bits(1)),
        f64::MIN_POSITIVE
    );
    assert_eq!(hypot(f64::MAX, f64::MAX), f64::INFINITY);
    assert_eq!(hypot(-0.0, -0.0).to_bits(), 0);
    assert_eq!(hypot(f64::INFINITY, f64::NAN), f64::INFINITY);
    assert_eq!(hypot(f64::NAN, f64::NEG_INFINITY), f64::INFINITY);
    assert!(hypot(1.0, f64::NAN).is_nan());
    assert!(hypot(f64::NAN, 1.0).is_nan());
}
