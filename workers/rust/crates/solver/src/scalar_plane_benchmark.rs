use crate::electrostatic_plane_2d_element::precompute_electrostatic_plane_quad_element;
use crate::magnetostatic_plane_2d_element::precompute_quad_element;
use kyuubiki_protocol::{
    SolveElectrostaticPlaneQuad2dRequest, SolveMagnetostaticPlaneQuad2dRequest,
};
use serde_json::json;
use std::{hint::black_box, time::Instant};

#[test]
#[ignore = "release-mode scalar element precompute microbenchmark"]
fn scalar_plane_quad_precompute_benchmark() {
    let nodes = [[0.0, 0.0], [1.5, 0.5], [1.75, 1.75], [0.25, 1.25]]
        .into_iter()
        .enumerate()
        .map(|(index, [x, y])| {
            json!({"id": format!("node-{index}"), "x": x, "y": y,
            "fix_potential": true, "fix_vector_potential": true})
        })
        .collect::<Vec<_>>();
    let input = json!({"nodes": nodes, "elements": [{"id": "representative-scalar-plane-cell",
        "node_i": 0, "node_j": 1, "node_k": 2, "node_l": 3,
        "thickness": 0.25, "permittivity": 2.5, "permeability": 0.4}]});
    let electric: SolveElectrostaticPlaneQuad2dRequest =
        serde_json::from_value(input.clone()).unwrap();
    let magnetic: SolveMagnetostaticPlaneQuad2dRequest = serde_json::from_value(input).unwrap();
    for family in ["electrostatic", "magnetostatic"] {
        let mut samples = Vec::new();
        for _ in 0..7 {
            let started = Instant::now();
            for _ in 0..100_000 {
                if family == "electrostatic" {
                    black_box(
                        precompute_electrostatic_plane_quad_element(
                            black_box(&electric),
                            black_box(&electric.elements[0]),
                        )
                        .unwrap(),
                    );
                } else {
                    black_box(
                        precompute_quad_element(
                            black_box(&magnetic),
                            black_box(&magnetic.elements[0]),
                        )
                        .unwrap(),
                    );
                }
            }
            samples.push(started.elapsed().as_secs_f64() * 1.0e9 / 100_000.0);
        }
        samples.sort_by(f64::total_cmp);
        eprintln!(
            "scalar-plane-precompute family={family} samples=7 elements_per_sample=100000 median_ns_per_element={:.3}",
            samples[3]
        );
    }
}
