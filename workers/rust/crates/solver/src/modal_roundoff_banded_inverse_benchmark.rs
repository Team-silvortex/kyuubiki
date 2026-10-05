use super::super::{
    Order, Plan, Selection, benchmark::peak_rss_kib, search_canonical_direction_ranked,
};
use super::{Prepared, Profile, bits, reference::Factor, rounded, search_canonical_direction};
use crate::solver_control::{SolverControl, SolverStage, with_solver_observer};
use std::{cell::RefCell, hint::black_box, rc::Rc, time::Instant};

const SAMPLES: usize = 3;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Work {
    banded_factors: usize,
    banded_solves: usize,
    grid_factors: usize,
    grid_certificates: usize,
    physical_polish_steps: usize,
}

fn observed<T>(operation: impl FnOnce() -> T) -> (T, Work) {
    let work = Rc::new(RefCell::new(Work::default()));
    let counts = work.clone();
    let result = with_solver_observer(
        &SolverControl::default(),
        move |p| {
            let mut counts = counts.borrow_mut();
            match (p.stage, p.completed_steps) {
                (SolverStage::BandedFactor, 0) => counts.banded_factors += 1,
                (SolverStage::BandedSubstitution, 0) => counts.banded_solves += 1,
                (SolverStage::DenseFactor, 0) => counts.grid_factors += 1,
                (SolverStage::ModalValidation, _) => counts.physical_polish_steps += 1,
                _ => {}
            }
        },
        || Ok::<_, String>(operation()),
    )
    .unwrap();
    let work = work.borrow().clone();
    (result, work)
}

#[derive(Debug, PartialEq, Eq)]
struct Receipt {
    root: u64,
    seed: Vec<u64>,
    internal: Vec<u64>,
    physical: Vec<u64>,
    residual: u64,
    work: [Work; 3],
    grid_visits_reserved: usize,
    factor_numeric_bytes: usize,
    orders: [Order; 2],
}

#[derive(Clone, Copy)]
enum PhysicalPolicy {
    Independent,
    CheckedOrder,
}

fn run(profile: Profile, scale: f64) -> (Receipt, [f64; 10]) {
    run_selected(profile, scale, Selection::Full)
}

fn run_selected(profile: Profile, scale: f64, selection: Selection) -> (Receipt, [f64; 10]) {
    run_pipeline(profile, scale, selection, PhysicalPolicy::Independent)
}

fn run_handoff(profile: Profile, scale: f64) -> (Receipt, [f64; 10]) {
    run_pipeline(
        profile,
        scale,
        Selection::RankedThenReverse,
        PhysicalPolicy::CheckedOrder,
    )
}

fn run_pipeline(
    profile: Profile,
    scale: f64,
    selection: Selection,
    physical_policy: PhysicalPolicy,
) -> (Receipt, [f64; 10]) {
    let started = Instant::now();
    let (fixture, preparation) = Prepared::new_measured(profile, scale);
    let preparation_ms = started.elapsed().as_secs_f64() * 1000.0;
    let root = fixture.value.to_bits();
    let original = bits(&fixture.seed);
    let size = fixture.seed.len();
    let plan = Plan::canonical_selected(size, selection).unwrap();

    let phase = Instant::now();
    let ((seed, factor_numeric_bytes), initializer_work) = observed(|| {
        let factor = Factor::prepare_shifted(&fixture.directions, fixture.value).unwrap();
        let factor_numeric_bytes = factor.owned_numeric_bytes();
        assert_eq!(factor_numeric_bytes, 64 * size + 16);
        let direction = factor.final_direction(&fixture.seed, 4).unwrap();
        (rounded(&direction), factor_numeric_bytes)
    });
    // Factor and wide iterate are destroyed before either dense grid stage starts.
    assert_eq!(initializer_work.banded_factors, 1);
    assert_eq!(initializer_work.banded_solves, 4);
    assert_eq!(initializer_work.grid_factors, 0);
    let initializer_ms = phase.elapsed().as_secs_f64() * 1000.0;
    let seed_bits = bits(&seed);

    let phase = Instant::now();
    let mut calls = 0;
    let (internal, mut internal_work) = observed(|| {
        let checked = |v: &[f64]| {
            calls += 1;
            fixture.checked(v)
        };
        match selection {
            Selection::Full => {
                search_canonical_direction(&fixture.directions, &seed, 1e-8, checked)
            }
            Selection::RankedThenReverse => {
                search_canonical_direction_ranked(&fixture.directions, &seed, 1e-8, checked)
            }
            Selection::Single(_) => unreachable!("single strategies are isolated order controls"),
        }
        .unwrap()
    });
    let internal_ms = phase.elapsed().as_secs_f64() * 1000.0;
    // Nested fit and portfolio search checkpoints overlap; count actual callbacks.
    internal_work.grid_certificates = calls;
    assert_eq!(internal.usage.certificates, calls);
    assert_eq!(internal.usage.attempts, internal_work.grid_factors);
    assert_eq!(
        internal.shape[internal.anchor].to_bits(),
        seed[internal.anchor].to_bits()
    );
    assert_eq!(bits(&seed), seed_bits);
    drop(seed);

    let phase = Instant::now();
    let physical_selection = match physical_policy {
        PhysicalPolicy::Independent => selection,
        // A checked internal ordering is a proposal, never a physical certificate.
        PhysicalPolicy::CheckedOrder => Selection::Single(internal.order),
    };
    let physical_plan = Plan::canonical_selected(size, physical_selection).unwrap();
    let mut physical_calls = 0;
    let (physical, mut physical_work) = observed(|| {
        fixture
            .physical_candidate_ordered(&internal.shape, physical_selection, || physical_calls += 1)
            .unwrap()
    });
    let physical_ms = phase.elapsed().as_secs_f64() * 1000.0;
    physical_work.grid_certificates = physical_calls;
    assert_eq!(physical.usage.attempts, physical_work.grid_factors);
    assert_eq!(physical.usage.certificates, physical_work.grid_certificates);
    assert!(physical_work.physical_polish_steps <= 4);
    let grid_visits_reserved = internal.usage.component_visits + physical.usage.component_visits;
    for (accepted, work, plan) in [
        (&internal, &internal_work, &plan),
        (&physical, &physical_work, &physical_plan),
    ] {
        assert_eq!(
            accepted.usage.component_visits,
            plan.preflight_visits + work.grid_factors * plan.fit_visits
        );
        assert!(
            work.grid_factors <= plan.max_attempts
                && work.grid_certificates <= plan.max_certificates
        );
    }
    assert!(
        internal_work.grid_factors + physical_work.grid_factors
            <= plan.max_attempts + physical_plan.max_attempts
    );
    assert!(
        internal_work.grid_certificates + physical_work.grid_certificates
            <= plan.max_certificates + physical_plan.max_certificates
    );
    assert!(grid_visits_reserved <= plan.component_visits + physical_plan.component_visits);

    let phase = Instant::now();
    let readback = super::super::Fixture::check_recovered_readback(
        &fixture.input,
        fixture.total_mass,
        fixture.value,
        &physical.shape,
    );
    let readback_ms = phase.elapsed().as_secs_f64() * 1000.0;
    assert!(readback <= 1e-8);
    assert_eq!(fixture.value.to_bits(), root);
    assert_eq!(bits(&fixture.seed), original);
    let total_ms = started.elapsed().as_secs_f64() * 1000.0;
    let phases = [
        total_ms,
        preparation_ms,
        initializer_ms,
        internal_ms,
        physical_ms,
        readback_ms,
        preparation[0],
        preparation[1],
        preparation[2],
        preparation[3],
    ];
    assert!(phases.iter().all(|v| v.is_finite() && *v >= 0.0));
    assert!(phases[1..6].iter().sum::<f64>() <= total_ms);
    assert!(preparation.iter().sum::<f64>() <= preparation_ms);
    (
        Receipt {
            root,
            seed: seed_bits,
            internal: bits(&internal.shape),
            physical: bits(&physical.shape),
            residual: readback.to_bits(),
            work: [initializer_work, internal_work, physical_work],
            grid_visits_reserved,
            factor_numeric_bytes,
            orders: [internal.order, physical.order],
        },
        phases,
    )
}

#[path = "modal_roundoff_banded_ranked_benchmark.rs"]
mod ranked;

#[path = "modal_roundoff_banded_handoff_benchmark.rs"]
mod handoff;

fn benchmark(case: &str, profile: Profile, scale: f64) {
    assert!(!cfg!(debug_assertions), "run with --release");
    let rss_before = peak_rss_kib();
    let (baseline, _) = run(profile, scale);
    let mut samples = Vec::with_capacity(SAMPLES);
    for _ in 0..SAMPLES {
        let (receipt, phases) = run(black_box(profile), black_box(scale));
        assert_eq!(receipt, baseline);
        samples.push(phases);
    }
    let mut medians = [0.0; 10];
    for (i, median) in medians.iter_mut().enumerate() {
        let mut values: Vec<_> = samples.iter().map(|v| v[i]).collect();
        values.sort_by(f64::total_cmp);
        *median = values[SAMPLES / 2];
    }
    println!(
        "modal-banded-pipeline-cost {}",
        serde_json::json!({
            "case":case, "coordinate_scale":scale, "elements":128, "active_dofs":256,
            "warmups":1, "samples":SAMPLES, "accepted":true,
            "initializer_factors":baseline.work[0].banded_factors,
            "initializer_solves":baseline.work[0].banded_solves,
            "internal_factors":baseline.work[1].grid_factors,
            "internal_checks":baseline.work[1].grid_certificates,
            "physical_factors":baseline.work[2].grid_factors,
            "physical_checks":baseline.work[2].grid_certificates,
            "physical_polish_steps":baseline.work[2].physical_polish_steps,
            "recovery_grid_factors":baseline.work[1].grid_factors + baseline.work[2].grid_factors,
            "recovery_grid_checks":baseline.work[1].grid_certificates + baseline.work[2].grid_certificates,
            "grid_visits_reserved":baseline.grid_visits_reserved,
            "grid_visits_max_reserved":2 * Plan::canonical(256).unwrap().component_visits,
            "grid_payload_per_stage_bytes":Plan::canonical(256).unwrap().payload_bytes,
            "initializer_factor_numeric_bytes":baseline.factor_numeric_bytes,
            "initializer_owned_peak_numeric_bytes":baseline.factor_numeric_bytes + 2 * 16 * 256,
            "avoided_dense_clone_numeric_bytes":16 * 256 * 256,
            "avoided_direction_history_numeric_bytes":4 * 16 * 256,
            "caller_retained_dense_numeric_bytes":24 * 256 * 256,
            "phase_order":["total", "preparation", "initializer", "internal", "physical", "readback", "input_validation_assembly", "dense_matrix_jacobi", "retained_pair_refinement", "normalized_directions"],
            "phase_samples_ms":samples, "phase_medians_ms":medians,
            "independent_readback_residual":f64::from_bits(baseline.residual),
            "process_peak_rss_before_kib":rss_before, "process_peak_rss_after_kib":peak_rss_kib(),
            "scope":"test-only fresh complete six-fixture banded candidate timing; recovery counts start after retained-pair preparation; certificates include fresh final checks but exclude ordinary physical polish and independent JSON reassembly; grid visit reservations exclude initializer, preparation, physical polish and independent reference work; numeric payloads exclude vector headers, allocator overhead, sparse operator, request, JSON and reference storage; stage payloads are neither additive peak RSS nor complete production memory bounds; process RSS includes warmup and samples; excludes fixture destruction, inertia and public rejection comparisons; no stacked old Gram or direct-grid retries; not production or Agent throughput"
        })
    );
}

#[test]
fn modal_banded_inverse_recovery_work_receipts_replay() {
    for profile in [Profile::Graded, Profile::Layered] {
        for scale in [1.0, 1e14, 1e-10] {
            let (baseline, _) = run(profile, scale);
            let (replay, _) = run(profile, scale);
            assert_eq!(replay, baseline);
        }
    }
    println!("banded inverse fresh_work_receipts=12 exact_candidate_replays=6");
}

macro_rules! cases {
    ($($name:ident => ($case:literal, $profile:expr, $scale:expr)),+ $(,)?) => {
        $(
            #[test]
            #[ignore = "explicit isolated release banded candidate cost; not public solver throughput"]
            fn $name() { benchmark($case, $profile, $scale); }
        )+
    };
}

cases! {
    modal_banded_pipeline_cost_graded_unit => ("graded-unit", Profile::Graded, 1.0),
    modal_banded_pipeline_cost_graded_large => ("graded-large", Profile::Graded, 1e14),
    modal_banded_pipeline_cost_graded_tiny => ("graded-tiny", Profile::Graded, 1e-10),
    modal_banded_pipeline_cost_layered_unit => ("layered-unit", Profile::Layered, 1.0),
    modal_banded_pipeline_cost_layered_large => ("layered-large", Profile::Layered, 1e14),
    modal_banded_pipeline_cost_layered_tiny => ("layered-tiny", Profile::Layered, 1e-10),
}
