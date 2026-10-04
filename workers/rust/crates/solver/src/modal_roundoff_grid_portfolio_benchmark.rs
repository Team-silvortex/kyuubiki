use super::super::{hybrid_tests::check_published, triangular_grid_robustness_tests::Fixture};
use super::*;
use std::{hint::black_box, time::Instant};

const SAMPLES: usize = 5;

#[derive(Clone, Copy)]
enum Mode {
    Single,
    Portfolio,
    Canonical,
}

#[derive(Debug, PartialEq, Eq)]
struct Outcome {
    accepted: bool,
    calls: usize,
}

fn run(
    mode: Mode,
    fixture: &Fixture,
    permutation: &[usize],
    matrix: &[Vec<Wide>],
    seed: &[f64],
    mass: &[f64],
) -> (Outcome, Option<Vec<f64>>, f64) {
    let restore = |v: &[f64]| {
        let mut original = vec![0.0; v.len()];
        for (&i, &a) in permutation.iter().zip(v) {
            original[i] = a;
        }
        original
    };
    let mut calls = 0;
    let mut checked = |v: &[f64]| {
        calls += 1;
        let (relative, residual) = fixture.checked(&restore(v))?;
        Ok((relative, permutation.iter().map(|&i| residual[i]).collect()))
    };
    let started = Instant::now();
    let shape = if !matches!(mode, Mode::Single) {
        let result = match mode {
            Mode::Portfolio => search_unit_shape(matrix, seed, mass, 1e-8, &mut checked),
            Mode::Canonical => {
                canonical::search_canonical_unit_shape(matrix, seed, mass, 1e-8, &mut checked)
            }
            Mode::Single => unreachable!(),
        };
        match result {
            Ok(result) => Some(result.shape),
            Err(error) => {
                assert!(error.contains("exhausted its bounded policies"), "{error}");
                None
            }
        }
    } else {
        let mut anchor = 0;
        for i in 1..seed.len() {
            if seed[i].abs() * mass[i].sqrt() > seed[anchor].abs() * mass[anchor].sqrt() {
                anchor = i;
            }
        }
        let order: Vec<_> = (0..seed.len()).rev().filter(|&i| i != anchor).collect();
        let fit = GridFit::prepare(matrix, seed, anchor, &order).unwrap();
        match fit
            .attempt_unit_shape(MAX_GRID_RADIUS, 1e-8, &mut checked)
            .unwrap()
        {
            Attempt::Accepted(shape) => Some(shape),
            Attempt::Rejected(_) => None,
        }
    };
    let milliseconds = started.elapsed().as_secs_f64() * 1000.0;
    assert!(
        calls
            <= if !matches!(mode, Mode::Single) {
                MAX_TOTAL_CERTIFICATES
            } else {
                MAX_CERTIFICATES
            }
    );
    (
        Outcome {
            accepted: shape.is_some(),
            calls,
        },
        shape.map(|shape| restore(&shape)),
        milliseconds,
    )
}

fn peak_rss_kib() -> Option<u64> {
    #[cfg(unix)]
    {
        let mut usage = std::mem::MaybeUninit::<libc::rusage>::uninit();
        // getrusage initializes the struct on success; this is process high water.
        if unsafe { libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) } != 0 {
            return None;
        }
        let peak = unsafe { usage.assume_init() }.ru_maxrss;
        let bytes_or_kib = u64::try_from(peak).ok()?;
        if cfg!(target_os = "macos") {
            Some(bytes_or_kib / 1024)
        } else {
            Some(bytes_or_kib)
        }
    }
    #[cfg(not(unix))]
    {
        None
    }
}

#[test]
#[ignore = "explicit isolated release-mode candidate benchmark; not production throughput"]
fn triangular_grid_portfolio_cost_benchmark() {
    assert!(!cfg!(debug_assertions), "run this benchmark with --release");
    let mode_name =
        std::env::var("KYUUBIKI_MODAL_GRID_BENCH_MODE").unwrap_or_else(|_| "portfolio".into());
    let mode = match mode_name.as_str() {
        "portfolio" => Mode::Portfolio,
        "canonical" => Mode::Canonical,
        "single" => Mode::Single,
        _ => panic!("KYUUBIKI_MODAL_GRID_BENCH_MODE must be single, portfolio or canonical"),
    };
    for step in [1.0, 1e14, 1e-10] {
        let fixture = Fixture::new(step);
        let size = fixture.shape.len();
        for (layout, permutation) in [
            ("identity", (0..size).collect::<Vec<_>>()),
            ("reversed", (0..size).rev().collect()),
            ("shuffle-7", super::tests::shuffled(size, 7)),
        ] {
            let matrix: Vec<Vec<_>> = permutation
                .iter()
                .map(|&i| {
                    permutation
                        .iter()
                        .map(|&j| fixture.directions[i][j])
                        .collect()
                })
                .collect();
            let seed: Vec<_> = permutation.iter().map(|&i| fixture.shape[i]).collect();
            let mass: Vec<_> = permutation
                .iter()
                .map(|&i| fixture.system.mass[i])
                .collect();
            let rss_before = peak_rss_kib();
            let (expected, shape, _) = run(mode, &fixture, &permutation, &matrix, &seed, &mass);
            if let Some(shape) = shape {
                check_published(128, step, fixture.value, &shape);
            }
            let mut samples = Vec::with_capacity(SAMPLES);
            for _ in 0..SAMPLES {
                let (outcome, shape, milliseconds) = run(
                    black_box(mode),
                    &fixture,
                    &permutation,
                    black_box(&matrix),
                    black_box(&seed),
                    black_box(&mass),
                );
                assert_eq!(outcome, expected);
                if let Some(shape) = shape {
                    check_published(128, step, fixture.value, &shape);
                }
                samples.push(milliseconds);
            }
            samples.sort_by(f64::total_cmp);
            println!(
                "modal-grid-cost {}",
                serde_json::json!({
                    "mode": mode_name, "layout": layout, "segment_length": step,
                    "active_dofs": size, "samples": SAMPLES, "warmups": 1,
                    "accepted": expected.accepted, "certificates": expected.calls,
                    "min_ms": samples[0], "median_ms": samples[SAMPLES / 2],
                    "max_ms": samples[SAMPLES - 1], "process_peak_rss_before_kib": rss_before,
                    "process_peak_rss_after_kib": peak_rss_kib(),
                    "scope": "cached input; includes order preparation, factorization, correction and actual certificates; excludes fixture/direction assembly and independent publication recheck; process RSS is cumulative, not incremental heap"
                })
            );
        }
    }
}
