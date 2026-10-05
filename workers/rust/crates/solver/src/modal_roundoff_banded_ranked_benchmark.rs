use super::{Plan, Profile, Receipt, SAMPLES, Selection, peak_rss_kib, run, run_selected};
use std::hint::black_box;

fn assert_comparison(full: &Receipt, ranked: &Receipt, tiny_layer: bool) {
    assert_eq!(ranked.root, full.root);
    assert_eq!(ranked.seed, full.seed);
    assert!(f64::from_bits(ranked.residual) <= 1e-8);
    let factors = if tiny_layer { 2 } else { 1 };
    let certificates = 4 * factors;
    for work in &ranked.work[1..] {
        assert_eq!(work.grid_factors, factors);
        assert_eq!(work.grid_certificates, certificates);
    }
    assert_eq!(ranked.work[0], full.work[0]);
    assert_eq!(ranked.factor_numeric_bytes, full.factor_numeric_bytes);
    assert_eq!(
        ranked.grid_visits_reserved,
        if tiny_layer {
            1_347_092_480
        } else {
            676_003_840
        }
    );
}

#[test]
fn modal_banded_ranked_six_fixture_cost_and_fresh_replay() {
    let mut cases = 0;
    for profile in [Profile::Graded, Profile::Layered] {
        for scale in [1.0, 1e14, 1e-10] {
            let (full, _) = run(profile, scale);
            let (ranked, _) = run_selected(profile, scale, Selection::RankedThenReverse);
            assert_comparison(
                &full,
                &ranked,
                matches!(profile, Profile::Layered) && scale == 1e-10,
            );
            let (replay, _) = run_selected(profile, scale, Selection::RankedThenReverse);
            assert_eq!(replay, ranked);
            cases += 1;
        }
    }
    assert_eq!(cases, 6);
    println!(
        "banded ranked fixtures=6 exact_replays=6 maximum_grid_factors=4 maximum_grid_certificates=16"
    );
}

fn medians(samples: &[[f64; 10]]) -> [f64; 10] {
    std::array::from_fn(|i| {
        let mut values: Vec<_> = samples.iter().map(|v| v[i]).collect();
        values.sort_by(f64::total_cmp);
        values[SAMPLES / 2]
    })
}

pub(super) fn summary(receipt: &Receipt, samples: &[[f64; 10]]) -> serde_json::Value {
    serde_json::json!({
        "internal_factors":receipt.work[1].grid_factors,
        "physical_factors":receipt.work[2].grid_factors,
        "internal_checks":receipt.work[1].grid_certificates,
        "physical_checks":receipt.work[2].grid_certificates,
        "physical_polish_steps":receipt.work[2].physical_polish_steps,
        "grid_visits_reserved":receipt.grid_visits_reserved,
        "independent_readback_residual":f64::from_bits(receipt.residual),
        "phase_samples_ms":samples, "phase_medians_ms":medians(samples),
    })
}

fn benchmark(case: &str, profile: Profile, scale: f64) {
    assert!(!cfg!(debug_assertions), "run with --release");
    let rss_before = peak_rss_kib();
    let (full, _) = run(profile, scale);
    let (ranked, _) = run_selected(profile, scale, Selection::RankedThenReverse);
    assert_comparison(
        &full,
        &ranked,
        matches!(profile, Profile::Layered) && scale == 1e-10,
    );
    let mut full_samples = Vec::with_capacity(SAMPLES);
    let mut ranked_samples = Vec::with_capacity(SAMPLES);
    for sample in 0..SAMPLES {
        // Alternate pair order; both members rebuild their own request and retained pair.
        for selection in if sample % 2 == 0 {
            [Selection::Full, Selection::RankedThenReverse]
        } else {
            [Selection::RankedThenReverse, Selection::Full]
        } {
            let (receipt, phases) = run_selected(black_box(profile), black_box(scale), selection);
            match selection {
                Selection::Full => {
                    assert_eq!(receipt, full);
                    full_samples.push(phases);
                }
                Selection::RankedThenReverse => {
                    assert_eq!(receipt, ranked);
                    ranked_samples.push(phases);
                }
                Selection::Single(_) => unreachable!(),
            }
        }
    }
    let full_plan = Plan::canonical(256).unwrap();
    let ranked_plan = Plan::canonical_selected(256, Selection::RankedThenReverse).unwrap();
    assert_eq!(ranked_plan.component_visits, 673_546_240);
    assert_eq!(ranked_plan.max_attempts, 2);
    assert_eq!(ranked_plan.max_certificates, 13);
    println!(
        "modal-banded-ranked-paired-cost {}",
        serde_json::json!({
            "case":case, "elements":128, "active_dofs":256, "coordinate_scale":scale,
            "warmups_per_strategy":1, "samples_per_strategy":SAMPLES,
            "baseline":summary(&full, &full_samples), "ranked":summary(&ranked, &ranked_samples),
            "baseline_max_grid_visits_reserved":2 * full_plan.component_visits,
            "ranked_max_grid_visits_reserved":2 * ranked_plan.component_visits,
            "grid_payload_per_stage_bytes":ranked_plan.payload_bytes,
            "phase_order":["total", "preparation", "initializer", "internal", "physical", "readback", "input_validation_assembly", "dense_matrix_jacobi", "retained_pair_refinement", "normalized_directions"],
            "process_peak_rss_before_kib":rss_before, "process_peak_rss_after_kib":peak_rss_kib(),
            "scope":"test-only paired full versus ranked-then-reverse candidate chains; no natural strategy or third factor in ranked stages; fresh preparation for each pair member; shared process RSS includes both strategies and is not a per-strategy heap comparison; reservations exclude preparation, initializer, physical polish and independent readback; no fixture-selection heuristic, tolerance change, stacked fallback or production admission; per-case regressions must remain visible"
        })
    );
}

macro_rules! cases {
    ($($name:ident => ($case:literal, $profile:expr, $scale:expr)),+ $(,)?) => {
        $(
            #[test]
            #[ignore = "explicit isolated release paired ranked candidate cost; not production throughput"]
            fn $name() { benchmark($case, $profile, $scale); }
        )+
    };
}

cases! {
    modal_banded_ranked_pipeline_cost_graded_unit => ("graded-unit", Profile::Graded, 1.0),
    modal_banded_ranked_pipeline_cost_graded_large => ("graded-large", Profile::Graded, 1e14),
    modal_banded_ranked_pipeline_cost_graded_tiny => ("graded-tiny", Profile::Graded, 1e-10),
    modal_banded_ranked_pipeline_cost_layered_unit => ("layered-unit", Profile::Layered, 1.0),
    modal_banded_ranked_pipeline_cost_layered_large => ("layered-large", Profile::Layered, 1e14),
    modal_banded_ranked_pipeline_cost_layered_tiny => ("layered-tiny", Profile::Layered, 1e-10),
}
