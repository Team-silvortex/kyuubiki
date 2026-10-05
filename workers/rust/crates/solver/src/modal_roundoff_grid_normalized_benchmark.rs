use super::{Fixture, Plan, Prepared, Profile, search_canonical_direction};
use crate::solver_control::{SolverControl, SolverStage, with_solver_observer};
use std::{cell::Cell, hint::black_box, rc::Rc, time::Instant};

const SAMPLES: usize = 3;

#[derive(Debug, PartialEq, Eq)]
struct Receipt {
    root_bits: u64,
    internal_bits: Option<Vec<u64>>,
    physical_bits: Option<Vec<u64>>,
    internal_factors: usize,
    internal_checks: usize,
    physical_factors: usize,
    physical_checks: usize,
    grid_visits_reserved: usize,
}

pub(super) fn peak_rss_kib() -> Option<u64> {
    #[cfg(unix)]
    {
        let mut usage = std::mem::MaybeUninit::<libc::rusage>::uninit();
        // Native process high water, not incremental heap or a stage-local peak.
        if unsafe { libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) } != 0 {
            return None;
        }
        let peak = u64::try_from(unsafe { usage.assume_init() }.ru_maxrss).ok()?;
        Some(if cfg!(target_os = "macos") {
            peak / 1024
        } else {
            peak
        })
    }
    #[cfg(not(unix))]
    {
        None
    }
}

fn run(profile: Profile, scale: f64) -> (Receipt, [f64; 9], Option<f64>) {
    let started = Instant::now();
    let (fixture, preparation) = Prepared::new_measured(profile, scale);
    let preparation_ms = started.elapsed().as_secs_f64() * 1000.0;
    let phase = Instant::now();
    let factors = Rc::new(Cell::new(0));
    let observer = factors.clone();
    let mut calls = 0;
    let internal = with_solver_observer(
        &SolverControl::default(),
        move |p| {
            if p.stage == SolverStage::DenseFactor && p.completed_steps == 0 {
                observer.set(observer.get() + 1);
            }
        },
        || {
            search_canonical_direction(&fixture.directions, &fixture.seed, 1e-8, |v| {
                calls += 1;
                fixture.checked(v)
            })
        },
    );
    let internal_ms = phase.elapsed().as_secs_f64() * 1000.0;
    let plan = Plan::canonical(fixture.seed.len()).unwrap();
    let mut receipt = Receipt {
        root_bits: fixture.value.to_bits(),
        internal_bits: None,
        physical_bits: None,
        internal_factors: factors.get(),
        internal_checks: calls,
        physical_factors: 0,
        physical_checks: 0,
        grid_visits_reserved: plan.preflight_visits + factors.get() * plan.fit_visits,
    };
    let (mut physical_ms, mut readback_ms, mut residual) = (0.0, 0.0, None);
    match internal {
        Ok(internal) => {
            assert_eq!(internal.usage.certificates, calls);
            assert_eq!(internal.usage.attempts, factors.get());
            assert_eq!(
                internal.usage.component_visits,
                receipt.grid_visits_reserved
            );
            let phase = Instant::now();
            let published = fixture
                .physical_candidate(black_box(&internal.shape))
                .unwrap();
            physical_ms = phase.elapsed().as_secs_f64() * 1000.0;
            let phase = Instant::now();
            residual = Some(Fixture::check_recovered_readback(
                &fixture.input,
                fixture.total_mass,
                fixture.value,
                &published.shape,
            ));
            readback_ms = phase.elapsed().as_secs_f64() * 1000.0;
            receipt.internal_bits = Some(internal.shape.iter().map(|v| v.to_bits()).collect());
            receipt.physical_bits = Some(published.shape.iter().map(|v| v.to_bits()).collect());
            receipt.physical_factors = published.usage.attempts;
            receipt.physical_checks = published.usage.certificates;
            receipt.grid_visits_reserved += published.usage.component_visits;
        }
        Err(error) => {
            assert!(error.contains("exhausted its bounded policies"), "{error}");
        }
    }
    let total_ms = started.elapsed().as_secs_f64() * 1000.0;
    assert!(receipt.internal_factors <= 3 && receipt.internal_checks <= 19);
    assert!(receipt.physical_factors <= 3 && receipt.physical_checks <= 19);
    assert!(receipt.grid_visits_reserved <= 2 * plan.component_visits);
    let phases = [
        total_ms,
        preparation_ms,
        internal_ms,
        physical_ms,
        readback_ms,
        preparation[0],
        preparation[1],
        preparation[2],
        preparation[3],
    ];
    assert!(phases.iter().all(|v| v.is_finite() && *v >= 0.0));
    assert!(phases[1..5].iter().sum::<f64>() <= total_ms);
    assert!(preparation.iter().sum::<f64>() <= preparation_ms);
    (receipt, phases, residual)
}

fn pipeline_cost_benchmark(case: &str) {
    assert!(!cfg!(debug_assertions), "run with --release");
    let (profile, scale, accepted) = match case {
        "graded-unit" => (Profile::Graded, 1.0, true),
        "graded-large" => (Profile::Graded, 1e14, false),
        "graded-tiny" => (Profile::Graded, 1e-10, true),
        "layered-unit" => (Profile::Layered, 1.0, true),
        "layered-large" => (Profile::Layered, 1e14, true),
        "layered-tiny" => (Profile::Layered, 1e-10, true),
        _ => panic!("unknown normalized candidate benchmark case"),
    };
    let rss_before = peak_rss_kib();
    let (baseline, _, _) = run(profile, scale);
    assert_eq!(baseline.physical_bits.is_some(), accepted);
    let mut samples = Vec::with_capacity(SAMPLES);
    let mut residuals = Vec::with_capacity(SAMPLES);
    for _ in 0..SAMPLES {
        let (receipt, phases, residual) = run(black_box(profile), black_box(scale));
        assert_eq!(receipt, baseline);
        samples.push(phases);
        residuals.push(residual);
    }
    let mut medians = [0.0; 9];
    for (i, median) in medians.iter_mut().enumerate() {
        let mut values: Vec<_> = samples.iter().map(|v| v[i]).collect();
        values.sort_by(f64::total_cmp);
        *median = values[SAMPLES / 2];
    }
    println!(
        "modal-normalized-pipeline-cost {}",
        serde_json::json!({
            "case":case, "profile":profile.name(), "coordinate_scale":scale,
            "elements":128, "active_dofs":256, "warmups":1, "samples":SAMPLES,
            "accepted":accepted, "internal_factors":baseline.internal_factors,
            "internal_checks":baseline.internal_checks,
            "physical_factors":baseline.physical_factors,
            "physical_checks":baseline.physical_checks,
            "grid_visits_reserved":baseline.grid_visits_reserved,
            "grid_payload_per_stage_bytes":Plan::canonical(256).unwrap().payload_bytes,
            "phase_order":["total", "preparation", "internal", "physical", "readback", "input_validation_assembly", "dense_matrix_jacobi", "retained_pair_refinement", "normalized_directions"],
            "phase_samples_ms":samples, "phase_medians_ms":medians,
            "independent_readback_residuals":residuals,
            "process_peak_rss_before_kib":rss_before,
            "process_peak_rss_after_kib":peak_rss_kib(),
            "scope":"fresh generated input per run; validation, assembly, Jacobi, failed four-step refinement retaining its pair, direction preparation, canonical recovery, ordinary physical proposal, physical grid, constructed result JSON and independent reassembly; excludes fixture destruction, independent inertia diagnostics and original production-failure comparisons; phase medians are not additive; RSS is cumulative native process high water including warmup, not incremental heap; not public-runtime or Agent throughput"
        })
    );
}

macro_rules! pipeline_cases {
    ($($name:ident => $case:literal),+ $(,)?) => {
        $(
            #[test]
            #[ignore = "explicit isolated release candidate pipeline cost; not public solver throughput"]
            fn $name() {
                pipeline_cost_benchmark($case);
            }
        )+
    };
}

pipeline_cases! {
    triangular_grid_normalized_pipeline_cost_graded_unit => "graded-unit",
    triangular_grid_normalized_pipeline_cost_graded_large => "graded-large",
    triangular_grid_normalized_pipeline_cost_graded_tiny => "graded-tiny",
    triangular_grid_normalized_pipeline_cost_layered_unit => "layered-unit",
    triangular_grid_normalized_pipeline_cost_layered_large => "layered-large",
    triangular_grid_normalized_pipeline_cost_layered_tiny => "layered-tiny",
}
