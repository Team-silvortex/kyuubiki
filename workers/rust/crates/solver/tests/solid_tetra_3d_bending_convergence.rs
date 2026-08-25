use kyuubiki_protocol::{
    SolidTetra3dElementInput, SolidTetra3dNodeInput, SolveSolidTetra3dRequest,
    SolveSolidTetra3dResult,
};
use kyuubiki_solver::solve_solid_tetra_3d;

const LENGTH: f64 = 2.0;
const WIDTH: f64 = 1.0;
const HEIGHT: f64 = 1.0;
const YOUNGS_MODULUS: f64 = 210.0e9;
const POISSON_RATIO: f64 = 0.27;
const CURVATURE: f64 = 2.0e-4;
const BALANCE_TOLERANCE: f64 = 2.0e-8;

#[test]
fn solid_tetra_3d_pure_bending_converges_on_regular_and_warped_meshes() {
    verify_regular_mesh_convergence();
    verify_warped_mesh_convergence();
}

fn verify_regular_mesh_convergence() {
    let mut displacement_errors = Vec::new();
    let mut stress_errors = Vec::new();
    let mut energy_errors = Vec::new();

    for divisions in [2, 4, 8, 16] {
        let request = bending_request(divisions, 0.0);
        assert_end_traction_resultants(&request, divisions);
        let result = solve_solid_tetra_3d(&request)
            .expect("self-equilibrated pure-bending solid should solve");
        let displacement_error = displacement_l2_error(&result);
        let stress_error = stress_l2_error(&result);
        let energy_error = relative_error(result.total_strain_energy, analytic_energy());

        assert!(
            result.equilibrium.free_residual_relative_error <= BALANCE_TOLERANCE,
            "mesh {divisions} free residual was {}",
            result.equilibrium.free_residual_relative_error,
        );
        assert!(
            result.equilibrium.force_balance_relative_error <= BALANCE_TOLERANCE,
            "mesh {divisions} force balance error was {}",
            result.equilibrium.force_balance_relative_error,
        );
        assert!(
            vector_norm(result.equilibrium.reaction_force)
                <= BALANCE_TOLERANCE * result.equilibrium.applied_force_scale,
            "mesh {divisions} nullspace anchors introduced reaction {:?}",
            result.equilibrium.reaction_force,
        );

        displacement_errors.push(displacement_error);
        stress_errors.push(stress_error);
        energy_errors.push(energy_error);
    }

    assert_strictly_contracting("displacement", &displacement_errors);
    assert_strictly_contracting("stress", &stress_errors);
    assert_strictly_contracting("energy", &energy_errors);
    println!(
        "solid tetra pure bending relative errors: displacement={displacement_errors:?}, stress={stress_errors:?}, energy={energy_errors:?}",
    );
    let finest = displacement_errors.len() - 1;
    assert!(
        displacement_errors[finest] < 0.04,
        "displacement errors: {displacement_errors:?}",
    );
    assert!(
        stress_errors[finest] < 0.17,
        "stress errors: {stress_errors:?}",
    );
    assert!(
        energy_errors[finest] < 0.04,
        "energy errors: {energy_errors:?}",
    );
}

fn verify_warped_mesh_convergence() {
    let mut displacement_errors = Vec::new();
    let mut stress_errors = Vec::new();
    let mut energy_errors = Vec::new();
    let mut minimum_qualities = Vec::new();

    for divisions in [4, 8, 16] {
        let request = bending_request(divisions, 0.22);
        assert_end_traction_resultants(&request, divisions);
        let result = solve_solid_tetra_3d(&request)
            .expect("warped pure-bending solid should solve without inverted elements");
        displacement_errors.push(displacement_l2_error(&result));
        stress_errors.push(stress_l2_error(&result));
        energy_errors.push(relative_error(
            result.total_strain_energy,
            analytic_energy(),
        ));
        minimum_qualities.push(result.quality.minimum_mean_ratio_quality);
        assert!(
            result.quality.minimum_mean_ratio_quality > result.quality.severe_distortion_threshold,
            "mesh {divisions} quality summary: {:?}",
            result.quality,
        );
        assert!(result.equilibrium.free_residual_relative_error <= BALANCE_TOLERANCE);
        assert!(result.equilibrium.force_balance_relative_error <= BALANCE_TOLERANCE);
    }

    assert_strictly_contracting("warped displacement", &displacement_errors);
    assert_strictly_contracting("warped stress", &stress_errors);
    assert_strictly_contracting("warped energy", &energy_errors);
    println!(
        "warped solid tetra pure bending relative errors: displacement={displacement_errors:?}, stress={stress_errors:?}, energy={energy_errors:?}, minimum_quality={minimum_qualities:?}",
    );
    assert!(displacement_errors[2] < 0.05, "{displacement_errors:?}");
    assert!(stress_errors[2] < 0.21, "{stress_errors:?}");
    assert!(energy_errors[2] < 0.05, "{energy_errors:?}");
}

fn bending_request(divisions: usize, warp_fraction: f64) -> SolveSolidTetra3dRequest {
    let mut nodes = bending_nodes(divisions, warp_fraction);
    let elements = bending_elements(divisions);
    apply_end_traction(&mut nodes, divisions, 0, -1.0);
    apply_end_traction(&mut nodes, divisions, divisions, 1.0);
    SolveSolidTetra3dRequest { nodes, elements }
}

fn bending_nodes(divisions: usize, warp_fraction: f64) -> Vec<SolidTetra3dNodeInput> {
    let mut nodes = Vec::with_capacity((divisions + 1).pow(3));
    let center = divisions / 2;
    for i in 0..=divisions {
        for j in 0..=divisions {
            for k in 0..=divisions {
                let origin = i == center && j == center && k == center;
                let y_anchor = i == center && j == divisions && k == center;
                let z_anchor = i == center && j == center && k == divisions;
                let mut coordinates = [
                    -0.5 * LENGTH + LENGTH * i as f64 / divisions as f64,
                    -0.5 * WIDTH + WIDTH * j as f64 / divisions as f64,
                    -0.5 * HEIGHT + HEIGHT * k as f64 / divisions as f64,
                ];
                let boundary = [i, j, k]
                    .into_iter()
                    .any(|index| index == 0 || index == divisions);
                if !(boundary || origin || y_anchor || z_anchor) {
                    let steps = [LENGTH, WIDTH, HEIGHT].map(|length| length / divisions as f64);
                    for axis in 0..3 {
                        coordinates[axis] +=
                            warp_fraction * steps[axis] * deterministic_offset(i, j, k, axis);
                    }
                }
                nodes.push(SolidTetra3dNodeInput {
                    id: format!("n-{i}-{j}-{k}"),
                    x: coordinates[0],
                    y: coordinates[1],
                    z: coordinates[2],
                    fix_x: origin || y_anchor || z_anchor,
                    fix_y: origin || z_anchor,
                    fix_z: origin,
                    load_x: 0.0,
                    load_y: 0.0,
                    load_z: 0.0,
                });
            }
        }
    }
    nodes
}

fn deterministic_offset(i: usize, j: usize, k: usize, axis: usize) -> f64 {
    let phase = (i * 17 + j * 31 + k * 43 + axis * 59) as f64;
    (phase * 0.618_033_988_749_894_8).sin()
}

fn bending_elements(divisions: usize) -> Vec<SolidTetra3dElementInput> {
    let mut elements = Vec::with_capacity(6 * divisions.pow(3));
    for i in 0..divisions {
        for j in 0..divisions {
            for k in 0..divisions {
                let v000 = node_index(i, j, k, divisions);
                let v100 = node_index(i + 1, j, k, divisions);
                let v010 = node_index(i, j + 1, k, divisions);
                let v110 = node_index(i + 1, j + 1, k, divisions);
                let v001 = node_index(i, j, k + 1, divisions);
                let v101 = node_index(i + 1, j, k + 1, divisions);
                let v011 = node_index(i, j + 1, k + 1, divisions);
                let v111 = node_index(i + 1, j + 1, k + 1, divisions);
                for (local, [node_a, node_b, node_c, node_d]) in [
                    [v000, v100, v110, v111],
                    [v000, v110, v010, v111],
                    [v000, v010, v011, v111],
                    [v000, v011, v001, v111],
                    [v000, v001, v101, v111],
                    [v000, v101, v100, v111],
                ]
                .into_iter()
                .enumerate()
                {
                    elements.push(SolidTetra3dElementInput {
                        id: format!("t-{i}-{j}-{k}-{local}"),
                        node_a,
                        node_b,
                        node_c,
                        node_d,
                        youngs_modulus: YOUNGS_MODULUS,
                        poisson_ratio: POISSON_RATIO,
                    });
                }
            }
        }
    }
    elements
}

fn apply_end_traction(
    nodes: &mut [SolidTetra3dNodeInput],
    divisions: usize,
    i: usize,
    outward_normal_x: f64,
) {
    let triangle_area = WIDTH * HEIGHT / (2.0 * divisions.pow(2) as f64);
    for j in 0..divisions {
        for k in 0..divisions {
            let v00 = node_index(i, j, k, divisions);
            let v10 = node_index(i, j + 1, k, divisions);
            let v01 = node_index(i, j, k + 1, divisions);
            let v11 = node_index(i, j + 1, k + 1, divisions);
            apply_linear_triangle_traction(nodes, [v00, v10, v11], outward_normal_x, triangle_area);
            apply_linear_triangle_traction(nodes, [v00, v11, v01], outward_normal_x, triangle_area);
        }
    }
}

fn apply_linear_triangle_traction(
    nodes: &mut [SolidTetra3dNodeInput],
    triangle: [usize; 3],
    outward_normal_x: f64,
    area: f64,
) {
    let traction = triangle.map(|node| outward_normal_x * analytic_stress_x(nodes[node].z));
    for local in 0..3 {
        let other_a = (local + 1) % 3;
        let other_b = (local + 2) % 3;
        nodes[triangle[local]].load_x +=
            area * (2.0 * traction[local] + traction[other_a] + traction[other_b]) / 12.0;
    }
}

fn assert_end_traction_resultants(request: &SolveSolidTetra3dRequest, divisions: usize) {
    let expected_moment = YOUNGS_MODULUS * CURVATURE * WIDTH * HEIGHT.powi(3) / 12.0;
    for (i, normal) in [(0, -1.0), (divisions, 1.0)] {
        let end_nodes = request.nodes.iter().filter(|node| {
            (node.x - (-0.5 * LENGTH + LENGTH * i as f64 / divisions as f64)).abs()
                <= f64::EPSILON * LENGTH
        });
        let (force, moment) = end_nodes.fold((0.0, 0.0), |(force, moment), node| {
            (force + node.load_x, moment + node.z * node.load_x)
        });
        assert_scaled_close(force, 0.0, expected_moment / HEIGHT);
        assert_scaled_close(moment, -normal * expected_moment, expected_moment);
    }
}

fn displacement_l2_error(result: &SolveSolidTetra3dResult) -> f64 {
    let (error, reference) = result.nodes.iter().fold((0.0, 0.0), |sum, node| {
        let expected = analytic_displacement(node.x, node.y, node.z);
        let actual = [node.ux, node.uy, node.uz];
        (
            sum.0
                + (0..3)
                    .map(|axis| (actual[axis] - expected[axis]).powi(2))
                    .sum::<f64>(),
            sum.1 + expected.iter().map(|value| value * value).sum::<f64>(),
        )
    });
    (error / reference).sqrt()
}

fn stress_l2_error(result: &SolveSolidTetra3dResult) -> f64 {
    let (error, reference) = result.elements.iter().fold((0.0, 0.0), |sum, element| {
        let z = [
            element.node_a,
            element.node_b,
            element.node_c,
            element.node_d,
        ]
        .iter()
        .map(|&node| result.nodes[node].z)
        .sum::<f64>()
            / 4.0;
        let expected = analytic_stress_x(z);
        let stress_error = (element.stress_x - expected).powi(2)
            + element.stress_y.powi(2)
            + element.stress_z.powi(2)
            + element.shear_xy.powi(2)
            + element.shear_yz.powi(2)
            + element.shear_zx.powi(2);
        (
            sum.0 + element.volume * stress_error,
            sum.1 + element.volume * expected.powi(2),
        )
    });
    (error / reference).sqrt()
}

fn analytic_displacement(x: f64, y: f64, z: f64) -> [f64; 3] {
    [
        -CURVATURE * x * z,
        POISSON_RATIO * CURVATURE * y * z,
        0.5 * CURVATURE * (x * x - POISSON_RATIO * y * y + POISSON_RATIO * z * z),
    ]
}

fn analytic_stress_x(z: f64) -> f64 {
    -YOUNGS_MODULUS * CURVATURE * z
}

fn analytic_energy() -> f64 {
    0.5 * YOUNGS_MODULUS * CURVATURE.powi(2) * LENGTH * WIDTH * HEIGHT.powi(3) / 12.0
}

fn node_index(i: usize, j: usize, k: usize, divisions: usize) -> usize {
    let width = divisions + 1;
    (i * width + j) * width + k
}

fn assert_strictly_contracting(label: &str, errors: &[f64]) {
    for pair in errors.windows(2) {
        assert!(
            pair[1] < 0.8 * pair[0],
            "{label} error did not contract sufficiently: {errors:?}",
        );
    }
}

fn relative_error(actual: f64, expected: f64) -> f64 {
    (actual - expected).abs() / expected.abs().max(1.0e-30)
}

fn vector_norm(vector: [f64; 3]) -> f64 {
    vector.iter().map(|value| value * value).sum::<f64>().sqrt()
}

fn assert_scaled_close(actual: f64, expected: f64, scale: f64) {
    assert!(
        (actual - expected).abs() <= 1.0e-10 * scale.abs().max(1.0e-30),
        "expected {actual:.12e} to be close to {expected:.12e}",
    );
}
