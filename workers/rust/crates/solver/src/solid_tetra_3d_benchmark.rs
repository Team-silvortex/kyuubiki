use std::hint::black_box;
use std::time::Instant;

use super::geometry_tests::{POINTS, compare_kernels, element};
use super::{SolidTetra3dElementKernel, geometry, reference};
use kyuubiki_protocol::SolidTetra3dElementInput;

#[inline(never)]
fn current(points: [[f64; 3]; 4], element: &SolidTetra3dElementInput) -> SolidTetra3dElementKernel {
    SolidTetra3dElementKernel::new(points, element).unwrap()
}

#[inline(never)]
fn previous(
    points: [[f64; 3]; 4],
    element: &SolidTetra3dElementInput,
) -> SolidTetra3dElementKernel {
    reference::kernel(points, element).unwrap()
}

#[inline(never)]
fn current_geometry(points: [[f64; 3]; 4]) -> (f64, f64, [[f64; 12]; 6]) {
    geometry(points, "benchmark").unwrap()
}

#[inline(never)]
fn previous_geometry(points: [[f64; 3]; 4]) -> (f64, f64, [[f64; 12]; 6]) {
    reference::geometry(points, "benchmark").unwrap()
}

fn elapsed<T>(iterations: usize, mut build: impl FnMut(usize) -> T) -> f64 {
    let start = Instant::now();
    for index in 0..iterations {
        black_box(build(index));
    }
    start.elapsed().as_secs_f64() * 1e9 / iterations as f64
}

#[test]
#[ignore = "paired release microbenchmark; run explicitly on the benchmark host"]
fn tetra_precompute_paired_benchmark() {
    assert!(!cfg!(debug_assertions), "run this benchmark with --release");
    let fixtures: Vec<_> = (0..16)
        .map(|i| {
            let shear = (i as f64 - 8.0) / 16.0;
            let scale = 0.5 + i as f64 / 8.0;
            POINTS.map(|[x, y, z]| [scale * (x + shear * y), scale * y, scale * z])
        })
        .collect();
    let element = element();
    for &points in &fixtures {
        compare_kernels(&current(points, &element), &previous(points, &element));
    }
    let iterations = 100_000;
    let warmups = 3;
    let samples = 9;
    for geometry_only in [true, false] {
        let mut previous_samples = Vec::new();
        let mut current_samples = Vec::new();
        for sample in 0..warmups + samples {
            // Alternate order to avoid consistently charging one path for warm-up.
            for old in if sample % 2 == 0 {
                [true, false]
            } else {
                [false, true]
            } {
                let ns = if geometry_only {
                    let build = if old {
                        previous_geometry
                    } else {
                        current_geometry
                    };
                    elapsed(iterations, |i| {
                        build(black_box(fixtures[i % fixtures.len()]))
                    })
                } else {
                    let build = if old { previous } else { current };
                    elapsed(iterations, |i| {
                        build(black_box(fixtures[i % fixtures.len()]), black_box(&element))
                    })
                };
                if sample >= warmups {
                    if old {
                        previous_samples.push(ns);
                    } else {
                        current_samples.push(ns);
                    }
                }
            }
        }
        println!(
            "tetra-precompute geometry_only={geometry_only} iterations={iterations} warmups={warmups} samples={samples}"
        );
        println!("previous_ns={previous_samples:?}");
        println!("current_ns={current_samples:?}");
        previous_samples.sort_by(f64::total_cmp);
        current_samples.sort_by(f64::total_cmp);
        let old = previous_samples[samples / 2];
        let new = current_samples[samples / 2];
        println!(
            "median previous_ns={old:.3} current_ns={new:.3} speedup={:.3} reduction_pct={:.2}",
            old / new,
            (1.0 - new / old) * 100.0
        );
    }
}
