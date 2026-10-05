use super::*;
use crate::modal_frame_spectrum::refinement::roundoff::banded_grid::checked_passes;

const CHARTS: usize = 32;

// Separate cold research charts, not an admitted runtime retry budget.
fn amplitude(index: usize) -> f64 {
    0.75 + index as f64 / 32.0
}

#[test]
fn modal_material_amplitude_isolated_three_layer_charts() {
    let mut acceptances = [0; 4];
    let mut hybrid_acceptances = [0; 4];
    for (case, (members, scale)) in [(100, 1e-10), (128, 1.0), (128, 1e14), (128, 1e-10)]
        .into_iter()
        .enumerate()
    {
        let input = renumber(&Recipe::ThreeLayers.request(members, scale), 7);
        let layout = Rebuilt::prepare_bounded(&input).unwrap();
        let (fixture, refined) = prepare(layout.canonical).unwrap();
        assert!(!refined);
        let root = fixture.value.to_bits();
        let original = bits(&fixture.seed);
        let wide = Factor::prepare_shifted(&fixture.directions, fixture.value)
            .unwrap()
            .final_direction(&fixture.seed, 4)
            .unwrap();
        for index in 0..CHARTS {
            let scale = amplitude(index);
            let center: Vec<_> = wide.iter().map(|v| v.mul(Wide::from(scale))).collect();
            let direction = rounded(&center);
            assert!((0.25..=2.0).contains(&vector_norm(direction.iter().copied()).unwrap()));
            let frozen = bits(&direction);
            let mut calls = 0;
            let mut best = f64::INFINITY;
            let mut retained = Vec::new();
            let (actual, factors) = measured(|| {
                search_canonical_direction_ranked(&fixture.directions, &direction, 1e-8, |v| {
                    calls += 1;
                    let receipt = fixture.checked(v)?;
                    if receipt.0 < best {
                        best = receipt.0;
                        retained = v.to_vec();
                    }
                    Ok(receipt)
                })
            });
            assert_eq!(actual.is_ok(), case == 0 && index == 9);
            assert!(factors <= 2 && calls <= 13);
            if let Ok(accepted) = &actual {
                assert!(fixture.checked(&accepted.shape).unwrap().0 <= 1e-8);
                assert_eq!(
                    accepted.shape[accepted.anchor].to_bits(),
                    direction[accepted.anchor].to_bits()
                );
                assert_eq!(accepted.usage.attempts, factors);
                assert_eq!(accepted.usage.certificates, calls);
                acceptances[case] += 1;
            } else {
                assert!(
                    actual
                        .as_ref()
                        .unwrap_err()
                        .contains("exhausted its bounded policies"),
                    "{actual:?}"
                );
            }
            assert_eq!(bits(&direction), frozen);
            println!(
                "material amplitude isolated members={members} input_scale={:e} chart={index} amplitude={scale} accepted={} best={best:e} factors={factors} certificates={calls}",
                fixture.input.nodes[1].x,
                actual.is_ok()
            );
            let center: Vec<_> = retained.iter().copied().map(Wide::from).collect();
            let anchor = (0..direction.len())
                .max_by(|&a, &b| direction[a].abs().total_cmp(&direction[b].abs()))
                .unwrap();
            assert_eq!(retained[anchor].to_bits(), direction[anchor].to_bits());
            let mut checks = 0;
            let mut after = f64::INFINITY;
            let result = checked_passes(&fixture.directions, &center, anchor, 4, |v| {
                checks += 1;
                let receipt = fixture.checked(v)?;
                after = after.min(receipt.0);
                Ok(receipt)
            });
            if result.is_ok() {
                hybrid_acceptances[case] += 1;
            }
            assert_eq!(result.is_ok(), case == 0 && index == 9);
            println!(
                "material amplitude hybrid members={members} input_scale={:e} chart={index} amplitude={scale} accepted={} before={best:e} after={after:e} qr_certificates={calls} dp_certificates={checks}",
                fixture.input.nodes[1].x,
                result.is_ok()
            );
        }
        assert_eq!(fixture.value.to_bits(), root);
        assert_eq!(bits(&fixture.seed), original);
    }
    assert_eq!(acceptances, [1, 0, 0, 0]);
    assert_eq!(hybrid_acceptances, acceptances);
    println!(
        "material amplitude difficult_inputs=4 cold_charts=128 accepted_by_input={acceptances:?} hybrid_accepted_by_input={hybrid_acceptances:?} physical_publications=0 production_admission=unchanged"
    );
}
