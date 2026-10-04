use super::{
    hybrid_tests::{check_published, scaled_bending_parts},
    triangular_grid::{GridFit, Policy},
    *,
};

#[test]
fn triangular_grid_long_bending_compares_order_radius_and_rounding_stage() {
    for step in [1.0_f64, 1e14, 1e-10] {
        let (system, physical) = scaled_bending_parts(128, step);
        let spectrum = crate::modal_frame_spectrum::frame_eigenpairs(&system, Some(1)).unwrap();
        let (value, vector) = &spectrum.pairs[0];
        let (shape, _) = crate::modal_frame_spectrum::checked_mode_shape(
            vector,
            &system.mass,
            &(0..256).collect::<Vec<_>>(),
            256,
        )
        .unwrap();
        let shape = system
            .operator
            .roundoff_comparison_seed(*value, &shape, &system.mass)
            .unwrap();
        let checked = |v: &[f64]| {
            let applied = system.operator.apply_physical_compensated(v)?;
            system
                .operator
                .physical_residual(*value, v, &system.mass, &applied)
        };
        let (initial, residual) = checked(&shape).unwrap();
        let squares: Vec<_> = residual.iter().map(|r| r * r).collect();
        let translation = squares.iter().step_by(2).sum::<f64>() / squares.iter().sum::<f64>();
        assert!(
            (0.88..0.92).contains(&translation),
            "{step:e}: {translation:e}"
        );
        let mut rows: Vec<_> = (0..256).collect();
        rows.sort_by(|&a, &b| squares[b].total_cmp(&squares[a]));
        println!(
            "grid distribution step={step:e} initial={initial:e} translation_share={translation:e} top_rows={:?}",
            &rows[..8]
        );
        let directions =
            wide_projection::physical_directions(&physical, &system.mass, *value).unwrap();
        let anchor = shape
            .iter()
            .zip(&system.mass)
            .enumerate()
            .max_by(|(_, (a, ma)), (_, (b, mb))| {
                (a.abs() * ma.sqrt()).total_cmp(&(b.abs() * mb.sqrt()))
            })
            .unwrap()
            .0;
        let natural: Vec<_> = (0..256).filter(|&i| i != anchor).collect();
        let mut by_norm = natural.clone();
        by_norm.sort_by(|&a, &b| {
            let norm = |j: usize| {
                directions
                    .iter()
                    .map(|r| r[j].rounded().powi(2))
                    .sum::<f64>()
                    .sqrt()
                    * (shape[j].next_up() - shape[j])
            };
            norm(a).total_cmp(&norm(b)).then(a.cmp(&b))
        });
        for (name, order) in [
            ("natural", natural.clone()),
            ("reverse", natural.into_iter().rev().collect()),
            ("grid-norm", by_norm),
        ] {
            let columns: Vec<Vec<_>> = order
                .iter()
                .map(|&j| {
                    directions
                        .iter()
                        .map(|r| {
                            r[j].mul(wide::Wide::from(shape[j].next_up() - shape[j]))
                                .rounded()
                        })
                        .collect()
                })
                .collect();
            let diagnostic = qr::QrFit::factor(&columns).unwrap();
            let rhs: Vec<_> = residual.iter().map(|v| -v).collect();
            let continuous = diagnostic
                .solve(&residual.iter().map(|v| -v).collect::<Vec<_>>())
                .unwrap();
            let maximum = continuous.iter().map(|v| v.abs()).fold(0.0_f64, f64::max);
            println!(
                "grid coefficients step={step:e} order={name} maximum={maximum:e} outside64={}",
                continuous.iter().filter(|v| v.abs() > 64.0).count()
            );
            let integer = diagnostic
                .quantized_solve(&rhs, qr::MAX_GRID_RADIUS)
                .unwrap();
            println!(
                "grid backsolve decisions step={step:e} order={name} maximum={:e}",
                integer.iter().map(|v| v.abs()).fold(0.0_f64, f64::max)
            );
            let fit = GridFit::prepare(&directions, &shape, anchor, &order).unwrap();
            let mut radii = vec![1, 4, 16, 64, 256, 1024, qr::MAX_GRID_RADIUS];
            if name == "reverse" && step != 1.0 {
                radii.insert(6, 65_536);
            }
            for radius in radii {
                for policy in [Policy::Independent, Policy::InBacksolve] {
                    let mut best = initial;
                    let mut calls = 0;
                    let result = fit.correct(radius, policy, 1e-8, |v| {
                        calls += 1;
                        let result = checked(v)?;
                        best = best.min(result.0);
                        Ok(result)
                    });
                    println!(
                        "grid step={step:e} order={name} radius={radius} policy={policy:?} calls={calls} best={best:e}: {:?}",
                        result.as_ref().map(|_| "pass")
                    );
                    let expected_pass = (radius == qr::MAX_GRID_RADIUS || radius == 65_536)
                        && matches!(policy, Policy::InBacksolve)
                        && (name == "reverse" || (name == "grid-norm" && step != 1e-10));
                    assert_eq!(
                        result.is_ok(),
                        expected_pass,
                        "{step:e}/{name}/{radius}/{policy:?}"
                    );
                    assert!(calls <= triangular_grid::MAX_CERTIFICATES);
                    if expected_pass {
                        let candidate = result.unwrap();
                        assert_eq!(calls, 3);
                        assert_eq!(candidate[anchor].to_bits(), shape[anchor].to_bits());
                        assert!(checked(&candidate).unwrap().0 <= 1e-8);
                        check_published(128, step, *value, &candidate);
                    } else {
                        let error = result.unwrap_err();
                        assert!(error.contains("unchanged residual gate"), "{error}");
                        let relative: f64 = error
                            .split("relative=")
                            .nth(1)
                            .unwrap()
                            .trim_end_matches(')')
                            .parse()
                            .unwrap();
                        assert_eq!(relative, best);
                        if step == 1e-10
                            && name == "grid-norm"
                            && radius == qr::MAX_GRID_RADIUS
                            && matches!(policy, Policy::InBacksolve)
                        {
                            assert!((1.07e-8..1.08e-8).contains(&relative));
                            assert_eq!(calls, 4);
                        } else {
                            assert_eq!(relative, initial);
                            assert_eq!(calls, 3);
                        }
                    }
                }
            }
        }
    }
}
