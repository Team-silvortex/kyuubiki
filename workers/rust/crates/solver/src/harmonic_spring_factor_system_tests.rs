use super::*;

#[test]
fn harmonic_factor_compensation_real_path_retains_the_non_dyadic_factor_tail() {
    let w = 2.0_f64.powi(-20);
    let coefficient = 1.0 / 3.0 + w;
    let result = solve_complex_tridiagonal(ComplexTridiagonalSystem {
        diagonal: vec![Complex::real(3.0), Complex::real(coefficient)],
        lower: vec![Complex::real(-1.0)],
        upper: vec![Complex::real(-1.0)],
        rhs: vec![Complex::default(), Complex::real(w)],
    })
    .unwrap();
    let expected = 3.0 * w / (3.0 * w - 2.0_f64.powi(-54));
    assert!(
        (result[1].re / expected - 1.0).abs() <= 4.0 * f64::EPSILON,
        "{result:?} != {expected}"
    );
}

#[test]
fn harmonic_factor_compensation_complex_leading_phase_reaches_both_factorizations() {
    for power in [-300, 0, 300] {
        for swapped in [false, true] {
            let w = 2.0_f64.powi(-20);
            let z = Complex::default();
            let mut matrix = vec![
                vec![Complex { re: 3.0, im: 3.0 }, Complex { re: -1.0, im: -1.0 }],
                vec![Complex::real(-1.0), Complex::real(1.0 / 3.0 + w)],
            ];
            let mut rhs = vec![z, Complex::real(w)];
            let scale = 2.0_f64.powi(power);
            for value in matrix.iter_mut().flatten().chain(rhs.iter_mut()) {
                value.re *= scale;
                value.im *= scale;
            }
            if swapped {
                matrix.swap(0, 1);
                rhs.swap(0, 1);
            }
            let path = ComplexTridiagonalSystem {
                diagonal: vec![matrix[0][0], matrix[1][1]],
                lower: vec![matrix[1][0]],
                upper: vec![matrix[0][1]],
                rhs: rhs.clone(),
            };
            for result in [
                solve_complex_system(matrix, rhs),
                solve_complex_tridiagonal(path),
            ] {
                let result = result.unwrap();
                let expected = 3.0 * w / (3.0 * w - 2.0_f64.powi(-54));
                assert!(
                    (result[1].re / expected - 1.0).abs() <= 4.0 * f64::EPSILON,
                    "{result:?} != {expected}"
                );
                assert!((result[0].re / (expected / 3.0) - 1.0).abs() <= 4.0 * f64::EPSILON);
                assert!(
                    result
                        .iter()
                        .all(|x| x.im.abs() < 4.0 * f64::EPSILON * expected)
                );
            }
        }
    }
}

#[test]
fn harmonic_factor_compensation_adjacent_swap_keeps_the_generated_fill_and_rhs_consistent() {
    let w = 2.0_f64.powi(-20);
    let z = Complex::default();
    let matrix = vec![
        vec![Complex::real(-1.0), Complex::real(1.0 / 3.0 + w), z],
        vec![
            Complex { re: 3.0, im: 3.0 },
            Complex { re: -1.0, im: -1.0 },
            Complex { re: 0.75, im: 0.75 },
        ],
        vec![z, z, Complex::real(1.0)],
    ];
    let rhs = vec![
        Complex::real(w),
        Complex { re: 0.75, im: 0.75 },
        Complex::real(1.0),
    ];
    let path = ComplexTridiagonalSystem {
        diagonal: vec![matrix[0][0], matrix[1][1], matrix[2][2]],
        lower: vec![matrix[1][0], z],
        upper: vec![matrix[0][1], matrix[1][2]],
        rhs: rhs.clone(),
    };
    for result in [
        solve_complex_system(matrix, rhs),
        solve_complex_tridiagonal(path),
    ] {
        let result = result.unwrap();
        let expected = 3.0 * w / (3.0 * w - 2.0_f64.powi(-54));
        assert!(
            (result[1].re / expected - 1.0).abs() <= 4.0 * f64::EPSILON,
            "{result:?}"
        );
        assert!((result[0].re / (expected / 3.0) - 1.0).abs() <= 4.0 * f64::EPSILON);
        assert_eq!(result[2].re, 1.0);
    }
}
