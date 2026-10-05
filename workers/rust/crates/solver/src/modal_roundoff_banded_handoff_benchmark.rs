use super::{
    Order, Plan, Profile, Receipt, SAMPLES, Selection, peak_rss_kib, ranked::summary, run_handoff,
    run_selected,
};

fn assert_comparison(independent: &Receipt, handoff: &Receipt, tiny_layer: bool) {
    assert_eq!(handoff.root, independent.root);
    assert_eq!(handoff.seed, independent.seed);
    assert_eq!(handoff.internal, independent.internal);
    assert_eq!(handoff.physical, independent.physical);
    assert_eq!(handoff.residual, independent.residual);
    assert_eq!(handoff.orders, independent.orders);
    assert_eq!(handoff.orders[0], handoff.orders[1]);
    assert_eq!(handoff.work[..2], independent.work[..2]);
    assert_eq!(
        handoff.factor_numeric_bytes,
        independent.factor_numeric_bytes
    );
    assert_eq!(handoff.work[2].grid_factors, 1);
    assert_eq!(handoff.work[2].grid_certificates, 4);
    assert_eq!(
        handoff.grid_visits_reserved,
        if tiny_layer {
            1_011_548_160
        } else {
            676_003_840
        }
    );
    if tiny_layer {
        assert_eq!(handoff.orders, [Order::Reverse; 2]);
        assert_eq!(handoff.work[1].grid_factors, 2);
        assert_eq!(handoff.work[1].grid_certificates, 8);
        assert_eq!(independent.work[2].grid_factors, 2);
    } else {
        assert_eq!(handoff.orders, [Order::GridNorm; 2]);
        assert_eq!(handoff.work[1].grid_factors, 1);
        assert_eq!(handoff.work[1].grid_certificates, 4);
    }
}

#[test]
fn modal_banded_handoff_six_fixture_readback_and_exact_replay() {
    let mut cases = 0;
    for profile in [Profile::Graded, Profile::Layered] {
        for scale in [1.0, 1e14, 1e-10] {
            let (independent, _) = run_selected(profile, scale, Selection::RankedThenReverse);
            let (handoff, _) = run_handoff(profile, scale);
            assert_comparison(
                &independent,
                &handoff,
                matches!(profile, Profile::Layered) && scale == 1e-10,
            );
            let (replay, _) = run_handoff(profile, scale);
            assert_eq!(replay, handoff);
            cases += 1;
        }
    }
    assert_eq!(cases, 6);
    println!(
        "banded handoff fixtures=6 exact_replays=6 maximum_grid_factors=3 maximum_grid_certificates=12 physical_factors=1"
    );
}

fn benchmark(case: &str, profile: Profile, scale: f64) {
    assert!(!cfg!(debug_assertions), "run with --release");
    let rss_before = peak_rss_kib();
    let (independent, _) = run_selected(profile, scale, Selection::RankedThenReverse);
    let (handoff, _) = run_handoff(profile, scale);
    assert_comparison(
        &independent,
        &handoff,
        matches!(profile, Profile::Layered) && scale == 1e-10,
    );
    let mut independent_samples = Vec::with_capacity(SAMPLES);
    let mut handoff_samples = Vec::with_capacity(SAMPLES);
    for sample in 0..SAMPLES {
        for checked_order in if sample % 2 == 0 {
            [false, true]
        } else {
            [true, false]
        } {
            let (receipt, phases) = if checked_order {
                run_handoff(profile, scale)
            } else {
                run_selected(profile, scale, Selection::RankedThenReverse)
            };
            if checked_order {
                assert_eq!(receipt, handoff);
                handoff_samples.push(phases);
            } else {
                assert_eq!(receipt, independent);
                independent_samples.push(phases);
            }
        }
    }
    let internal_plan = Plan::canonical_selected(256, Selection::RankedThenReverse).unwrap();
    let physical_plan =
        Plan::canonical_selected(256, Selection::Single(handoff.orders[0])).unwrap();
    assert_eq!(internal_plan.max_attempts + physical_plan.max_attempts, 3);
    assert_eq!(
        internal_plan.max_certificates + physical_plan.max_certificates,
        20
    );
    println!(
        "modal-banded-handoff-paired-cost {}",
        serde_json::json!({
            "case":case, "elements":128, "active_dofs":256, "coordinate_scale":scale,
            "warmups_per_strategy":1, "samples_per_strategy":SAMPLES,
            "independent_ranked":summary(&independent, &independent_samples),
            "checked_order_handoff":summary(&handoff, &handoff_samples),
            "internal_order":format!("{:?}", handoff.orders[0]),
            "independent_max_grid_visits_reserved":2 * internal_plan.component_visits,
            "handoff_max_grid_visits_reserved":internal_plan.component_visits + physical_plan.component_visits,
            "handoff_max_grid_factors":3, "handoff_max_grid_certificates":20,
            "grid_payload_per_stage_bytes":internal_plan.payload_bytes,
            "phase_order":["total", "preparation", "initializer", "internal", "physical", "readback", "input_validation_assembly", "dense_matrix_jacobi", "retained_pair_refinement", "normalized_directions"],
            "process_peak_rss_before_kib":rss_before, "process_peak_rss_after_kib":peak_rss_kib(),
            "scope":"test-only checked-order physical proposal, not physical admission from internal success; one independently checked physical fit with no fallback on rejection or faults; fresh pair preparation and exact final bits; process RSS includes both strategies; scoped grid reservations exclude preparation, initializer, physical polish and JSON reference; not complete production budget or universal latency/SLO qualification"
        })
    );
}

macro_rules! cases {
    ($($name:ident => ($case:literal, $profile:expr, $scale:expr)),+ $(,)?) => {
        $(
            #[test]
            #[ignore = "explicit isolated release checked-order handoff cost; not production throughput"]
            fn $name() { benchmark($case, $profile, $scale); }
        )+
    };
}

cases! {
    modal_banded_handoff_pipeline_cost_graded_unit => ("graded-unit", Profile::Graded, 1.0),
    modal_banded_handoff_pipeline_cost_graded_large => ("graded-large", Profile::Graded, 1e14),
    modal_banded_handoff_pipeline_cost_graded_tiny => ("graded-tiny", Profile::Graded, 1e-10),
    modal_banded_handoff_pipeline_cost_layered_unit => ("layered-unit", Profile::Layered, 1.0),
    modal_banded_handoff_pipeline_cost_layered_large => ("layered-large", Profile::Layered, 1e14),
    modal_banded_handoff_pipeline_cost_layered_tiny => ("layered-tiny", Profile::Layered, 1e-10),
}
