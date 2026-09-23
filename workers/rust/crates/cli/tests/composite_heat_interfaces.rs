use kyuubiki_headless_sdk::{
    composite_heat_cross_validation_for_regional_loads,
    composite_heat_mesh_convergence_for_regional_loads, composite_heat_refinement_requests,
    composite_heat_refinement_requests_for_regional_loads,
};
use kyuubiki_solver::solve_heat_plane_quad_2d;

const WIDTH: f64 = 0.03;
const AREA: f64 = 0.03 * 0.001;

fn close(actual: f64, expected: f64, scale: f64, label: &str) {
    assert!(actual.is_finite());
    assert!(
        (actual - expected).abs() <= 2.0e-8 * scale,
        "{label}: actual={actual}, expected={expected}, scale={scale}"
    );
}

// Integrate the one-dimensional Fourier flux from this point to the right sink.
// This oracle is independent of the SDK's maximum-temperature assessment.
fn expected_rise(x: f64, k: [f64; 3], powers: [f64; 3]) -> f64 {
    let mut upstream = 0.0;
    let mut rise = 0.0;
    for layer in 0..3 {
        let fraction = ((x - layer as f64 * WIDTH) / WIDTH).clamp(0.0, 1.0);
        rise += WIDTH / (k[layer] * AREA)
            * (upstream * (1.0 - fraction) + 0.5 * powers[layer] * (1.0 - fraction * fraction));
        upstream += powers[layer];
    }
    rise
}

#[test]
fn regional_generation_matches_nodal_temperatures_interface_flux_and_outlet_balance() {
    let conductivities = [[390.0, 0.25, 160.0], [0.5, 20.0, 2.0], [10.0; 3]];
    let load_cases = [[0.01, 0.02, 0.03], [0.02, 0.0, 0.0], [0.0, 0.0, 0.02]];
    for k in conductivities {
        for powers in load_cases {
            let total: f64 = powers.iter().sum();
            let mut temperatures = Vec::new();
            for (level, request) in
                composite_heat_refinement_requests_for_regional_loads(k, powers).unwrap()
            {
                let result = solve_heat_plane_quad_2d(&request).unwrap();
                let maximum_rise = expected_rise(0.0, k, powers);
                for node in &result.nodes {
                    close(
                        node.temperature - 35.0,
                        expected_rise(node.x, k, powers),
                        maximum_rise,
                        "nodal rise",
                    );
                }
                let mut upstream = 0.0;
                let mut previous_right_face = 0.0;
                for (column, element) in result.elements.iter().enumerate() {
                    let layer = column / level;
                    if column > 0 && column % level == 0 {
                        upstream += powers[layer - 1];
                    }
                    let local_fraction = (column % level) as f64 + 0.5;
                    let expected_power = upstream + powers[layer] * local_fraction / level as f64;
                    let half_cell_power = powers[layer] / (2 * level) as f64;
                    let center_flow = element.heat_flux_x * AREA;
                    close(
                        center_flow,
                        expected_power,
                        total,
                        "cell-center Fourier flow",
                    );
                    close(
                        element.heat_flux_y * AREA,
                        0.0,
                        total,
                        "insulated transverse flow",
                    );
                    close(
                        center_flow - half_cell_power,
                        previous_right_face,
                        total,
                        "shared-face power continuity",
                    );
                    previous_right_face = center_flow + half_cell_power;
                }
                close(
                    previous_right_face,
                    total,
                    total,
                    "outlet power equals all source power",
                );
                close(
                    request.nodes.iter().map(|node| node.heat_load).sum(),
                    total,
                    total,
                    "assembled source power",
                );
                assert_eq!(
                    composite_heat_cross_validation_for_regional_loads(
                        k,
                        powers,
                        Some(result.max_temperature),
                    )
                    .status,
                    "pass"
                );
                temperatures.push((level, result.max_temperature));
            }
            assert_eq!(
                composite_heat_mesh_convergence_for_regional_loads(k, powers, &temperatures).status,
                "pass"
            );
        }
    }
}

#[test]
fn interface_point_source_has_temperature_continuity_and_the_correct_flux_jump() {
    let k = [390.0, 0.25, 160.0];
    let power = 0.02;
    for (level, request) in composite_heat_refinement_requests(k) {
        let result = solve_heat_plane_quad_2d(&request).unwrap();
        for node in &result.nodes {
            let remaining_dielectric = (2.0 * WIDTH - node.x).clamp(0.0, WIDTH);
            let remaining_substrate = (3.0 * WIDTH - node.x).clamp(0.0, WIDTH);
            let rise = power / AREA * (remaining_dielectric / k[1] + remaining_substrate / k[2]);
            close(
                node.temperature - 35.0,
                rise,
                80.125,
                "piecewise-linear rise",
            );
        }
        for (column, element) in result.elements.iter().enumerate() {
            let expected = if column < level { 0.0 } else { power };
            close(
                element.heat_flux_x * AREA,
                expected,
                power,
                "interface-source flow",
            );
        }
        let left = result.elements[level - 1].heat_flux_x * AREA;
        let right = result.elements[level].heat_flux_x * AREA;
        close(right - left, power, power, "source jump");
    }
}

#[test]
fn scaling_sources_and_conductivities_together_preserves_temperature_and_scales_flow() {
    let base_k = [390.0, 0.25, 160.0];
    let base_powers = [0.01, 0.02, 0.03];
    let baseline_request =
        composite_heat_refinement_requests_for_regional_loads(base_k, base_powers)
            .unwrap()
            .pop()
            .unwrap()
            .1;
    let baseline = solve_heat_plane_quad_2d(&baseline_request).unwrap();
    for scale in [1.0e-6, 1.0e6] {
        let k = base_k.map(|value| value * scale);
        let powers = base_powers.map(|value| value * scale);
        let request = composite_heat_refinement_requests_for_regional_loads(k, powers)
            .unwrap()
            .pop()
            .unwrap()
            .1;
        let result = solve_heat_plane_quad_2d(&request).unwrap();
        for (actual, expected) in result.nodes.iter().zip(&baseline.nodes) {
            close(
                actual.temperature,
                expected.temperature,
                baseline.max_temperature,
                "scaled T",
            );
        }
        for (actual, expected) in result.elements.iter().zip(&baseline.elements) {
            close(
                actual.heat_flux_x / scale,
                expected.heat_flux_x,
                0.06 / AREA,
                "scaled flux",
            );
        }
        assert_eq!(
            composite_heat_cross_validation_for_regional_loads(
                k,
                powers,
                Some(result.max_temperature)
            )
            .status,
            "pass"
        );
    }
}

#[test]
fn rejected_source_distribution_does_not_poison_a_following_physical_solve() {
    let k = [390.0, 0.25, 160.0];
    assert!(composite_heat_refinement_requests_for_regional_loads(k, [1.0, 1.0e-20, 0.0]).is_err());
    let powers = [0.01, 0.02, 0.03];
    let (_, request) = composite_heat_refinement_requests_for_regional_loads(k, powers)
        .unwrap()
        .remove(0);
    let result = solve_heat_plane_quad_2d(&request).unwrap();
    close(
        result.max_temperature - 35.0,
        expected_rise(0.0, k, powers),
        100.0,
        "clean replay",
    );
}
