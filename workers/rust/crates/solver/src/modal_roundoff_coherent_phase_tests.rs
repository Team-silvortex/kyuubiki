use super::{
    hybrid_tests::{check_published, scaled_bending_parts},
    *,
};

#[test]
fn coherent_phase_long_bending_comparison_preserves_the_unresolved_tiny_gate() {
    for step in [1.0_f64, 1e14, 1e-10] {
        let (system, physical) = scaled_bending_parts(128, step);
        let scales: Vec<_> = (0..256)
            .map(|i| if i % 2 == 0 { 1.0 } else { step.recip() })
            .collect();
        let proposal = wide::WideProposal::prepare(&physical, &system.mass, &scales).unwrap();
        let (value, physical_shape) = proposal.first_mode().unwrap();
        let center = wide::unit_shape_parts(&physical_shape).unwrap();
        let rounded_unit = wide::unit_shape(&physical_shape).unwrap();
        assert!(
            center
                .iter()
                .zip(&rounded_unit)
                .all(|(a, b)| a.rounded().to_bits() == b.to_bits())
        );
        let directions =
            wide_projection::physical_directions(&physical, &system.mass, value).unwrap();
        let search = coherent_phase::Search::prepare(directions.clone()).unwrap();
        let spectrum = crate::modal_frame_spectrum::frame_eigenpairs(&system, Some(1)).unwrap();
        let (shape, _) = crate::modal_frame_spectrum::checked_mode_shape(
            &spectrum.pairs[0].1,
            &system.mass,
            &(0..256).collect::<Vec<_>>(),
            256,
        )
        .unwrap();
        let polished = system
            .operator
            .roundoff_comparison_seed(value, &shape, &system.mass)
            .unwrap();
        let translated: Vec<_> = center
            .iter()
            .zip(&polished)
            .map(|(&v, &s)| wide::Wide::from(s).add(v.sub(wide::Wide::from(v.rounded()))))
            .collect();
        assert!(
            translated
                .iter()
                .zip(&polished)
                .all(|(a, b)| a.rounded().to_bits() == b.to_bits())
        );
        for (origin, center) in [("wide-unit", &center), ("translated-polisher", &translated)] {
            for partitioned in [false, true] {
                let mut best = f64::INFINITY;
                let mut best_shape = Vec::new();
                let mut calls = 0;
                let checked = |v: &[f64]| {
                    calls += 1;
                    let applied = system.operator.apply_physical_compensated(v)?;
                    let certificate =
                        system
                            .operator
                            .physical_residual(value, v, &system.mass, &applied)?;
                    if certificate.0 < best {
                        best = certificate.0;
                        best_shape = v.to_vec();
                    }
                    Ok(certificate)
                };
                let result = if partitioned {
                    search.correct_partitioned(
                        center,
                        &(0..256).step_by(2).collect::<Vec<_>>(),
                        1e-8,
                        checked,
                    )
                } else {
                    search.correct(center, 1e-8, checked)
                };
                println!(
                    "coherent phase {origin} partitioned={partitioned} step={step:e} calls={calls} best={best:e}: {:?}",
                    result.as_ref().map(|_| "pass")
                );
                let error = result.unwrap_err();
                assert!(error.contains("unchanged residual gate"), "{error}");
                assert_eq!(calls, coherent_phase::MAX_CERTIFICATES);
                let band = if origin == "translated-polisher" {
                    if step == 1.0 {
                        2.17e-8..2.18e-8
                    } else if step == 1e14 {
                        2.01e-8..2.02e-8
                    } else {
                        1.73e-8..1.75e-8
                    }
                } else if step == 1.0 {
                    4.16e-8..4.18e-8
                } else if step == 1e14 {
                    4.23e-8..4.26e-8
                } else {
                    3.91e-8..3.94e-8
                };
                assert!(band.contains(&best), "{origin} {step:e}: {best:e}");
                assert_eq!(reported_relative(&error), best);
                let rounded = directions
                    .iter()
                    .map(|r| r.iter().map(|v| v.rounded()).collect())
                    .collect();
                let fit = BlockFit::prepare_automatic(rounded, &best_shape).unwrap();
                let mut block_calls = 0;
                let corrected = fit.correct(&best_shape, 1e-8, |v| {
                    block_calls += 1;
                    let applied = system.operator.apply_physical_compensated(v)?;
                    system
                        .operator
                        .physical_residual(value, v, &system.mass, &applied)
                });
                println!(
                    "coherent phase {origin} partitioned={partitioned} step={step:e} block={:?}",
                    corrected.as_ref().map(|v| {
                        let applied = system.operator.apply_physical_compensated(v).unwrap();
                        system
                            .operator
                            .physical_residual(value, v, &system.mass, &applied)
                            .unwrap()
                            .0
                    })
                );
                assert!(block_calls <= budget::MAX_CHECKS);
                if step == 1e-10 {
                    let error = corrected.unwrap_err();
                    assert!(error.contains("unchanged residual gate"), "{error}");
                    let band = if origin == "wide-unit" {
                        3.90e-8..3.93e-8
                    } else {
                        1.73e-8..1.75e-8
                    };
                    assert!(band.contains(&reported_relative(&error)), "{error}");
                } else {
                    check_published(128, step, value, &corrected.unwrap());
                }
            }
        }
    }
}

fn reported_relative(error: &str) -> f64 {
    error
        .split("relative=")
        .nth(1)
        .unwrap()
        .trim_end_matches(')')
        .parse()
        .unwrap()
}
