use super::*;

#[test]
fn modal_rounded_beam_qr_analytic_and_exhaustive_control() {
    let columns = vec![vec![1.0, 0.0, 1.0], vec![0.0, 1.0, 1.0]];
    let fit = QrFit::factor(&columns).unwrap();
    assert_eq!(
        fit.quantized_beam(&[2.0, -1.0, 1.0], 4, 64).unwrap()[0],
        [2.0, -1.0]
    );
    for width in [1, 4, 16, 64] {
        assert!(
            fit.quantized_beam(&[0.25, -0.375, 0.625], 4, width)
                .unwrap()
                .len()
                <= width
        );
    }
    for width in [0, 65, usize::MAX] {
        assert!(fit.quantized_beam(&[1.0; 3], 4, width).is_err());
    }
    for a in [-1.5, -0.375, 0.25, 1.875] {
        for b in [-1.25, -0.125, 0.75] {
            let rhs = [a, b, a + 0.5 * b];
            let proposals = fit.quantized_beam(&rhs, 4, 64).unwrap();
            let objective = |v: &[f64]| {
                (rhs[0] - v[0]).powi(2) + (rhs[1] - v[1]).powi(2) + (rhs[2] - v[0] - v[1]).powi(2)
            };
            let exhaustive = (-4..=4)
                .flat_map(|x| (-4..=4).map(move |y| [x as f64, y as f64]))
                .map(|v| objective(&v))
                .fold(f64::INFINITY, f64::min);
            assert!((objective(&proposals[0]) - exhaustive).abs() < 1e-13);
            let negative = fit.quantized_beam(&rhs.map(|v| -v), 4, 64).unwrap();
            assert_eq!(
                negative,
                proposals
                    .iter()
                    .map(|v| v.iter().map(|x| -x).collect::<Vec<_>>())
                    .collect::<Vec<_>>()
            );
            for exponent in [-80, 80] {
                let scale = 2.0_f64.powi(exponent);
                let columns: Vec<_> = columns
                    .iter()
                    .map(|v| v.iter().map(|x| x * scale).collect())
                    .collect();
                let scaled = QrFit::factor(&columns).unwrap();
                assert_eq!(
                    scaled
                        .quantized_beam(&rhs.map(|v| v * scale), 4, 64)
                        .unwrap(),
                    proposals
                );
            }
        }
    }
    for rhs in [
        vec![1.0; 2],
        vec![f64::NAN; 3],
        vec![1e51; 3],
        vec![1e-200, 2e-200, -1e-200],
    ] {
        assert!(fit.quantized_beam(&rhs, 4, 64).is_err());
    }
    for radius in [0, MAX_GRID_RADIUS + 1, usize::MAX] {
        assert!(fit.quantized_beam(&[1.0; 3], radius, 64).is_err());
    }
    println!(
        "rounded beam QR controls exhaustive_inputs=12 integer_pairs_per_input=81 sign_controls=12 binary_matrix_controls=24 malformed_rhs=4 invalid_widths=3 invalid_radii=3"
    );
}
