use super::*;

#[test]
fn triangular_grid_wide_qr_low_tail_integer_rounding_and_analytic_solve() {
    for sign in [1.0, -1.0] {
        for (low, expected) in [
            (-2.0_f64.powi(-55), 0.0),
            (0.0, 1.0),
            (2.0_f64.powi(-55), 1.0),
        ] {
            let value = Wide {
                high: sign * 0.5,
                low: sign * low,
            };
            assert_eq!(round_grid(value, 4), sign * expected);
        }
        assert_eq!(round_grid(Wide::from(sign * 1e10), 4), sign * 4.0);
    }
    let columns = vec![
        vec![Wide::from(2.0), Wide::default(), Wide::default()],
        vec![Wide::from(1.0), Wide::from(3.0), Wide::default()],
    ];
    let factor = QrFit::factor(&columns).unwrap();
    assert_eq!(
        factor.quantized_solve(&[8.0, 6.0, 0.0], 4).unwrap(),
        [3.0, 2.0]
    );
    let plan = super::super::Plan::wide(256).unwrap();
    assert_eq!(plan.payload_bytes, 6_815_744);
    assert_eq!(plan.component_visits, 335_544_320);
}

#[test]
fn triangular_grid_wide_qr_invalid_input_and_rank_fail_closed() {
    for size in [0, 1, 257, usize::MAX] {
        assert!(super::super::Plan::wide(size).is_err());
    }
    for columns in [
        vec![],
        vec![vec![]],
        vec![vec![Wide::from(1.0); 2]; 2],
        vec![vec![Wide::default(); 3]],
        vec![vec![Wide::from(1.0); 3]; 2],
        vec![vec![
            Wide {
                high: 1.0,
                low: f64::NAN
            };
            3
        ]],
        vec![vec![Wide::from(1.0); 3], vec![Wide::from(1.0); 2]],
    ] {
        assert!(QrFit::factor(&columns).is_err());
    }
    let factor = QrFit::factor(&[vec![Wide::from(1.0), Wide::default(), Wide::default()]]).unwrap();
    for rhs in [
        vec![1.0; 2],
        vec![f64::NAN; 3],
        vec![f64::INFINITY; 3],
        vec![1e51; 3],
    ] {
        assert!(factor.quantized_solve(&rhs, 4).is_err());
        assert!(factor.beam_solve(&rhs, 4).is_err());
    }
    for radius in [0, MAX_GRID_RADIUS + 1, usize::MAX] {
        assert!(factor.quantized_solve(&[1.0; 3], radius).is_err());
        assert!(factor.beam_solve(&[1.0; 3], radius).is_err());
    }
}

#[test]
fn triangular_grid_wide_beam_crosses_a_greedy_barrier_with_an_exhaustive_reference() {
    for order in [[0, 1, 2], [2, 0, 1], [1, 2, 0]] {
        for power in [-100, 0, 100] {
            let scale = 2.0_f64.powi(power);
            let columns: Vec<Vec<_>> = [[1.0, 0.0, 0.0], [4.4, 1.0, 0.0]]
                .iter()
                .map(|r| order.iter().map(|&i| Wide::from(r[i] * scale)).collect())
                .collect();
            let factor = QrFit::factor(&columns).unwrap();
            let mut baseline = None;
            for sign in [1.0, -1.0] {
                let rhs = order.map(|i| sign * [2.49, 0.49, 0.0][i] * scale);
                let greedy = factor.quantized_solve(&rhs, 4).unwrap();
                assert_eq!(greedy, [sign * 2.0, 0.0]);
                let proposals = factor.beam_solve(&rhs, 4).unwrap();
                assert_eq!(proposals.len(), BEAM_WIDTH);
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
    let tied = QrFit::factor(&[
        vec![Wide::from(1.0), Wide::default(), Wide::default()],
        vec![Wide::default(), Wide::from(1.0), Wide::default()],
    ])
    .unwrap();
    let positive = tied.beam_solve(&[0.5, 0.5, 0.0], 4).unwrap();
    let negative: Vec<Vec<_>> = tied
        .beam_solve(&[-0.5, -0.5, 0.0], 4)
        .unwrap()
        .into_iter()
        .map(|p| p.into_iter().map(|v| -v).collect())
        .collect();
    assert_eq!(positive, negative);
}
