use super::{
    hybrid_tests::{check_published, scaled_bending_parts},
    *,
};

#[test]
fn physical_neighborhood_modes_compare_polisher_and_checked_block_seeds() {
    for step in [1.0, 1e14, 1e-10] {
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
        let directions =
            wide_projection::physical_directions(&physical, &system.mass, *value).unwrap();
        let rounded = directions
            .iter()
            .map(|r| r.iter().map(|v| v.rounded()).collect())
            .collect();
        let block = BlockFit::prepare_automatic(rounded, &shape).unwrap();
        let checked = |v: &[f64]| {
            let applied = system.operator.apply_physical_compensated(v)?;
            system
                .operator
                .physical_residual(*value, v, &system.mass, &applied)
        };
        let mut warm = shape.clone();
        let mut previous = checked(&shape).unwrap().0;
        let baseline = block.correct(&shape, 1e-8, |v| {
            let certificate = checked(v)?;
            if certificate.0 < previous {
                previous = certificate.0;
                warm = v.to_vec();
            }
            Ok(certificate)
        });
        assert_eq!(baseline.is_ok(), step != 1e-10);
        for width in [1, 2, 3, 4] {
            let fit =
                physical_neighborhood::Neighborhood::prepare(directions.clone(), width).unwrap();
            for (name, seed) in [("polisher", &shape), ("checked-block", &warm)] {
                let mut best = checked(seed).unwrap().0;
                let mut calls = 0;
                let result = fit.correct(seed, 1e-8, |v| {
                    calls += 1;
                    let certificate = checked(v)?;
                    best = best.min(certificate.0);
                    Ok(certificate)
                });
                assert!(calls <= physical_neighborhood::MAX_CERTIFICATES);
                if name == "checked-block" && step != 1e-10 {
                    let candidate = result.unwrap();
                    assert_eq!(candidate, warm, "already-qualified seeds require no search");
                    assert_eq!(calls, 2);
                    check_published(128, step, *value, &candidate);
                    println!(
                        "physical neighborhood {name} step={step:e} width={width}: pass {best:e}"
                    );
                } else {
                    let error = result.unwrap_err();
                    assert!(error.contains("unchanged residual gate"), "{error}");
                    let relative = error
                        .split("relative=")
                        .nth(1)
                        .unwrap()
                        .trim_end_matches(')')
                        .parse::<f64>()
                        .unwrap();
                    let band = if step == 1e-10 {
                        1.73e-8..1.75e-8
                    } else if step == 1.0 {
                        2.17e-8..2.18e-8
                    } else {
                        2.01e-8..2.02e-8
                    };
                    assert!(band.contains(&relative), "{error}");
                    assert_eq!(
                        relative, best,
                        "failed search retains its best true certificate"
                    );
                    println!(
                        "physical neighborhood {name} step={step:e} width={width}: reject {relative:e}"
                    );
                }
            }
        }
    }
}
