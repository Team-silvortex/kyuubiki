use std::hint::black_box;
use std::time::Instant;

use super::kernel_tests::{POINTS, compare};
use super::{PlaneQuadComputed, precompute_plane_quad_element_from_coordinates, reference};

#[inline(never)]
fn current(points: [[f64; 2]; 4], material: [f64; 3]) -> PlaneQuadComputed {
    precompute_plane_quad_element_from_coordinates(points, material[0], material[1], material[2])
        .unwrap()
}

#[inline(never)]
fn previous(points: [[f64; 2]; 4], material: [f64; 3]) -> PlaneQuadComputed {
    reference::compute(points, material[0], material[1], material[2]).unwrap()
}

#[test]
#[ignore = "paired release microbenchmark; run explicitly on the benchmark host"]
fn q4_precompute_paired_benchmark() {
    assert!(!cfg!(debug_assertions), "run this benchmark with --release");
    let fixtures: Vec<_> = (0..16)
        .map(|i| {
            let shear = (i as f64 - 8.0) / 16.0;
            let scale = 0.5 + i as f64 / 8.0;
            POINTS.map(|[x, y]| [scale * (x + shear * y), scale * y])
        })
        .collect();
    let material = [0.5, 1200.0, 0.25];
    for &points in &fixtures {
        compare(&current(points, material), &previous(points, material));
    }
    let iterations = 100_000;
    let warmups = 3;
    let samples = 9;
    let mut previous_samples = Vec::new();
    let mut current_samples = Vec::new();
    for sample in 0..warmups + samples {
        for old in if sample % 2 == 0 {
            [true, false]
        } else {
            [false, true]
        } {
            let build = if old { previous } else { current };
            let start = Instant::now();
            for i in 0..iterations {
                black_box(build(
                    black_box(fixtures[i % fixtures.len()]),
                    black_box(material),
                ));
            }
            let ns = start.elapsed().as_secs_f64() * 1e9 / iterations as f64;
            if sample >= warmups {
                if old {
                    previous_samples.push(ns);
                } else {
                    current_samples.push(ns);
                }
            }
        }
    }
    println!("q4-precompute iterations={iterations} warmups={warmups} samples={samples}");
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
