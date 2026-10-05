use super::*;

#[test]
fn triangular_grid_wide_givens_analytic_solve_range_and_payload() {
    let plan = super::super::super::Plan::givens(256).unwrap();
    assert_eq!(plan.payload_bytes, 7_864_320);
    assert_eq!(plan.component_visits, 335_544_320);
    for size in [0, 1, 257, usize::MAX] {
        assert!(super::super::super::Plan::givens(size).is_err());
    }
    for power in [-100, 0, 100] {
        let scale = 2.0_f64.powi(power);
        for order in [[0, 1, 2], [2, 0, 1], [1, 2, 0]] {
            let columns: Vec<Vec<_>> = [[2.0, 0.0, 1.0], [1.0, 3.0, 0.0]]
                .iter()
                .map(|c| order.iter().map(|&i| Wide::from(c[i] * scale)).collect())
                .collect();
            let fit = QrFit::factor_givens(&columns).unwrap();
            for sign in [1.0, -1.0] {
                let rhs = order.map(|i| sign * [8.0, 6.0, 3.0][i] * scale);
                assert_eq!(
                    fit.quantized_solve(&rhs, 4).unwrap(),
                    [sign * 3.0, sign * 2.0]
                );
            }
        }
    }
    for columns in [
        vec![],
        vec![vec![]],
        vec![vec![Wide::from(1.0); 2]; 2],
        vec![vec![Wide::default(); 3]],
        vec![vec![Wide::from(1.0); 3]; 2],
        vec![vec![Wide::from(1e-101); 3]],
        vec![vec![Wide::from(1e101); 3]],
        vec![vec![
            Wide {
                high: 1.0,
                low: f64::NAN
            };
            3
        ]],
    ] {
        assert!(QrFit::factor_givens(&columns).is_err());
    }
    let (rotation, diagonal) = Rotation::new(0, 1, Wide::from(3.0), Wide::from(4.0)).unwrap();
    assert_eq!(diagonal.rounded(), 5.0);
    let (left, right) = rotation.apply(Wide::from(3.0), Wide::from(4.0)).unwrap();
    assert!((left.rounded() - 5.0).abs() < 1e-30 && right.rounded().abs() < 1e-30);
    assert!(rotation.apply(Wide::from(1e-101), Wide::default()).is_err());
    // Valid nonzero inputs can generate an out-of-range retained fill-in.
    let (rotation, _) = Rotation::new(0, 1, Wide::from(1.0), Wide::from(1e-80)).unwrap();
    assert!(rotation.apply(Wide::from(1e-80), Wide::default()).is_err());
    assert!(
        Rotation::new(
            0,
            1,
            Wide::default(),
            Wide {
                high: 1.0,
                low: -1.0
            }
        )
        .is_err()
    );
    assert!(Rotation::new(0, 1, Wide::from(1e100), Wide::from(1e-100)).is_err());
    println!(
        "wide givens analytic_solves=18 range_guard_unchanged=1 payload_bytes=7864320 component_visits=335544320"
    );
}

#[test]
fn triangular_grid_wide_givens_beam_exhaustive_reference_and_sign_scale_equivariance() {
    for order in [[0, 1, 2], [2, 0, 1], [1, 2, 0]] {
        for power in [-100, 0, 100] {
            let scale = 2.0_f64.powi(power);
            let columns: Vec<Vec<_>> = [[1.0, 0.0, 0.0], [4.4, 1.0, 0.0]]
                .iter()
                .map(|c| order.iter().map(|&i| Wide::from(c[i] * scale)).collect())
                .collect();
            let factor = QrFit::factor_givens(&columns).unwrap();
            let mut baseline = None;
            for sign in [1.0, -1.0] {
                let rhs = order.map(|i| sign * [2.49, 0.49, 0.0][i] * scale);
                let greedy = factor.quantized_solve(&rhs, 4).unwrap();
                assert_eq!(greedy, [sign * 2.0, 0.0]);
                let proposals = factor.beam_solve(&rhs, 4).unwrap();
                assert_eq!(proposals.len(), super::super::BEAM_WIDTH);
                let square = |a: f64, b: f64| {
                    (a + 4.4 * b - sign * 2.49).powi(2) + (b - sign * 0.49).powi(2)
                };
                let selected = proposals
                    .iter()
                    .min_by(|a, b| square(a[0], a[1]).total_cmp(&square(b[0], b[1])))
                    .unwrap();
                assert_eq!(selected, &[sign * -2.0, sign]);
                assert!(square(selected[0], selected[1]) < square(greedy[0], greedy[1]));
                for a in -4..=4 {
                    for b in -4..=4 {
                        assert!(square(selected[0], selected[1]) <= square(a as f64, b as f64));
                    }
                }
                let positive: Vec<Vec<_>> = proposals
                    .iter()
                    .map(|p| p.iter().map(|v| sign * v).collect())
                    .collect();
                if let Some(reference) = &baseline {
                    assert_eq!(&positive, reference);
                } else {
                    baseline = Some(positive);
                }
            }
        }
    }
    println!("wide givens exhaustive_beam_solves=18 enumerated_integer_pairs_per_solve=81");
}
