use super::*;

#[test]
fn harmonic_fast_division_normal_components_match_independent_small_integer_ratios() {
    for sign in [-1.0, 1.0] {
        let result = divide(
            Complex {
                re: sign * 2.0,
                im: sign * 3.0,
            },
            Complex { re: 4.0, im: 5.0 },
            SolverStage::DenseSubstitutionRange,
        )
        .unwrap();
        assert_eq!(result.re.to_bits(), (sign * (23.0 / 41.0)).to_bits());
        assert_eq!(result.im.to_bits(), (sign * (2.0 / 41.0)).to_bits());
    }
}

#[test]
fn harmonic_fast_division_real_load_matches_independent_normal_response_components() {
    let result = divide(
        Complex::real(1.0),
        Complex { re: 2.0, im: 3.0 },
        SolverStage::TridiagonalSubstitutionRange,
    )
    .unwrap();
    assert_eq!(result.re.to_bits(), (2.0_f64 / 13.0).to_bits());
    assert_eq!(result.im.to_bits(), (-3.0_f64 / 13.0).to_bits());
}

#[test]
fn harmonic_fast_division_signed_scaled_integer_oracle_retains_a_real_fast_path() {
    let mut seed = 0xd29a_b043_a11e_6179_u64;
    let mut next = || {
        seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
        ((seed >> 32) % 63) as i64 - 31
    };
    let mut accepted = 0;
    let mut total = 0;
    for _ in 0..512 {
        let (a, b, c, mut d) = (next(), next(), next(), next());
        if c == 0 && d == 0 {
            d = 1;
        }
        let den = c * c + d * d;
        for power in [-900, -300, 0, 300, 900] {
            let s = 2.0_f64.powi(power / 2);
            let t = 2.0_f64.powi(power / 2 - power);
            let left = Complex {
                re: a as f64 * s,
                im: b as f64 * s,
            };
            let pivot = Complex {
                re: c as f64 * t,
                im: d as f64 * t,
            };
            let expected = [a * c + b * d, b * c - a * d]
                .map(|n| (n as f64 / den as f64) * 2.0_f64.powi(power));
            if let Some(fast) = fast_division::divide(left, pivot) {
                accepted += 1;
                for (actual, reference) in [fast.re, fast.im].into_iter().zip(expected) {
                    if reference == 0.0 {
                        assert_eq!(actual, 0.0);
                    } else {
                        assert_eq!(actual.to_bits(), reference.to_bits());
                    }
                }
            }
            let result = divide(left, pivot, SolverStage::DenseSubstitutionRange).unwrap();
            for (actual, reference) in [result.re, result.im].into_iter().zip(expected) {
                if reference == 0.0 {
                    assert_eq!(actual, 0.0);
                } else {
                    assert_eq!(actual.to_bits(), reference.to_bits());
                }
            }
            total += 1;
        }
    }
    eprintln!("certified fast integer cases: {accepted}/{total}");
    assert!(
        accepted * 4 >= total * 3,
        "not all ordinary cases may fall back"
    );
}

#[test]
fn harmonic_fast_division_range_and_cancellation_routing_remain_fail_closed() {
    use crate::solver_control::{SolverControl, with_solver_observer};
    use std::cell::Cell;
    let stage = SolverStage::DenseSubstitutionRange;
    let pivot = Complex { re: 1.0, im: 1.0 };
    for left in [
        Complex::real(f64::from_bits(1)),
        Complex {
            re: f64::MAX,
            im: f64::MAX,
        },
    ] {
        assert!(fast_division::divide(left, pivot).is_none());
    }
    assert!(divide(Complex::real(f64::from_bits(1)), pivot, stage).is_err());
    assert!(
        divide(
            Complex {
                re: f64::MAX,
                im: f64::MAX
            },
            Complex { re: 0.5, im: 0.5 },
            stage
        )
        .is_err()
    );
    let contrast = Complex {
        re: 1.0,
        im: f64::from_bits(1),
    };
    assert!(fast_division::divide(contrast, contrast).is_none());
    let recovered = divide(contrast, contrast, stage).unwrap();
    assert_eq!((recovered.re, recovered.im), (1.0, 0.0));
    let min = Complex {
        re: f64::MIN_POSITIVE,
        im: f64::MIN_POSITIVE,
    };
    let large = divide(Complex::real(1.0), min, stage).unwrap();
    assert_eq!(large.re, 2.0_f64.powi(1021));
    assert_eq!(large.im, -2.0_f64.powi(1021));
    let tiny = divide(
        Complex::real(1.0),
        Complex {
            re: f64::MAX,
            im: f64::MAX,
        },
        stage,
    )
    .unwrap();
    assert!(tiny.re.is_subnormal() && tiny.im.is_subnormal());
    let delta = 2.0_f64.powi(-30);
    let weak = Complex {
        re: 1.0 + delta,
        im: 1.0,
    };
    assert!(fast_division::divide(weak, pivot).is_none());
    let value = divide(weak, pivot, stage).unwrap();
    assert_eq!((value.re, value.im), (1.0 + delta / 2.0, -delta / 2.0));
    let control = SolverControl::default();
    let cancel = control.clone();
    let error = with_solver_observer(
        &control,
        move |p| {
            if p.stage == stage {
                cancel.request_cancel();
            }
        },
        || divide(weak, pivot, stage),
    )
    .unwrap_err();
    assert!(control.was_interrupted() && error.contains("cancelled"));
    assert_eq!(divide(weak, pivot, stage).unwrap().im, -delta / 2.0);
    let observed = std::rc::Rc::new(Cell::new(0));
    let events = observed.clone();
    with_solver_observer(
        &SolverControl::default(),
        move |_| events.set(events.get() + 1),
        || {
            divide(Complex::real(1.0), Complex { re: 2.0, im: 3.0 }, stage).unwrap();
            divide(Complex::default(), pivot, stage).unwrap();
            Ok::<_, String>(())
        },
    )
    .unwrap();
    assert_eq!(observed.get(), 0);
}

#[test]
fn harmonic_fast_division_both_factorizations_match_the_normal_rational_response() {
    use super::super::{ComplexTridiagonalSystem, solve_complex_system, solve_complex_tridiagonal};
    let coefficient = Complex { re: 2.0, im: 3.0 };
    for result in [
        solve_complex_system(vec![vec![coefficient]], vec![Complex::real(1.0)]),
        solve_complex_tridiagonal(ComplexTridiagonalSystem {
            diagonal: vec![coefficient],
            lower: vec![],
            upper: vec![],
            rhs: vec![Complex::real(1.0)],
        }),
    ] {
        let value = result.unwrap()[0];
        assert_eq!(value.re.to_bits(), (2.0_f64 / 13.0).to_bits());
        assert_eq!(value.im.to_bits(), (-3.0_f64 / 13.0).to_bits());
    }
}

#[test]
fn harmonic_fast_division_full_significand_fixtures_match_independent_rational_bits() {
    // Exact rational evaluation of the four literal binary64 operands, rounded
    // once per component; no production decoder or quotient is the oracle.
    let cases = [
        (
            [
                0xbff3fcdbcd20e6e0,
                0xbffec9fa148fa50f,
                0xbfffa827cef12964,
                0x3ff183788067d880,
            ],
            [0x3fb24ac814e48c3a, 0x3ff0319f7089bae6],
        ),
        (
            [
                0xbff5a051c4200c96,
                0x3ff738bd1d70cfd7,
                0x3ff32627af299728,
                0x3ff165832b955221,
            ],
            [0xbf8f0b728dd658f5, 0x3ff39f89ee6aa308],
        ),
        (
            [
                0xbff144388091e3ce,
                0xbfff417dc99946b2,
                0xbff4f511a1ac022f,
                0xbfffd6e8f04c1659,
            ],
            [0x3fede31da1e7f8c2, 0x3fb28cbc68d60d54],
        ),
        (
            [
                0xbff249195cbd8e0e,
                0xbff73479faf4244b,
                0x3ff70c48ed7b3895,
                0xbff687c17f399cf2,
            ],
            [0x3fb8fb1db313adbf, 0xbfed2a5e486703dc],
        ),
        (
            [
                0x3ff8fdd801539714,
                0x3fff87f22f30e08a,
                0x3ffa82aacfea286e,
                0xbff5038585223042,
            ],
            [0xbf062af30b46c098, 0x3ff307933b7b7c37],
        ),
        (
            [
                0xbff639c0e13637e2,
                0x3ffcb76f711c1f85,
                0x3ff4134b5cb5e7d9,
                0xbffcd99c4c1c1935,
            ],
            [0xbff0825b4905c0e9, 0xbfaad24764c08289],
        ),
        (
            [
                0xbff6460203c29e39,
                0xbffb558b53236111,
                0xbffcda7d5954c314,
                0x3ffe9b0a10b34d88,
            ],
            [0xbfbc0eeaceb7457b, 0x3fea984053950b20],
        ),
        (
            [
                0x3ff80424807addee,
                0x3ff17297a9a036ba,
                0x3ff5d34ef1a4542c,
                0xbffa75cb0bc8169b,
            ],
            [0x3fab32c06cc9be1c, 0x3feba469341c600a],
        ),
        (
            [
                0xbff483612e7c3544,
                0xbff21b4a466ab221,
                0xbfffed31a07b9ed9,
                0x3ffe1329c04f6847,
            ],
            [0x3fad5f2a8157daa6, 0x3fe3e0a5bff5ef56],
        ),
        (
            [
                0xbff1a26460fb26ce,
                0xbff1a67859e2e277,
                0xbff0558180f13a1e,
                0x3ffe2a977a62735d,
            ],
            [0xbfca956869e97067, 0x3fe64e03cd50fd45],
        ),
        (
            [
                0x3ff48460c064a7d2,
                0x3ff0e43279e7ef54,
                0xbff2b191fa6b0263,
                0xbff084c8b917b911,
            ],
            [0xbff108e0e3b0ef30, 0x3fa30cac2395afa3],
        ),
        (
            [
                0x3ffa63cae6bd7a1e,
                0xbffeb6344ec56d1e,
                0x3ff284bddf694ae8,
                0xbff37909595d5f91,
            ],
            [0x3ff8142acd29de18, 0xbfb37217688b9124],
        ),
    ];
    for (input, expected) in cases {
        for power in [-800, 0, 800] {
            let [a, b, c, d] = input.map(|bits| f64::from_bits(bits) * 2.0_f64.powi(power));
            let value = divide(
                Complex { re: a, im: b },
                Complex { re: c, im: d },
                SolverStage::DenseSubstitutionRange,
            )
            .unwrap();
            assert_eq!([value.re.to_bits(), value.im.to_bits()], expected);
        }
    }
}
