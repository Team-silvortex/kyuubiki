use super::{longitudinal_quadrature::ADAPTIVE_POINT_COUNT, section_response};
use crate::frame_2d_material_p_delta::{
    CompiledFrame2dFiber, CompiledFrame2dMaterial, CompiledFrame2dPointMaterial,
    Frame2dMaterialHistory, Frame2dMaterialPointHistory,
};

const YOUNGS_MODULUS: f64 = 1_000.0;
const YIELD_STRENGTH: f64 = 100.0;
const HARDENING_RATIO: f64 = 0.05;
const FIBER_COUNT: usize = 32;
const DENSE_STATION_COUNT: usize = 20_000;

#[test]
fn adaptive_cyclic_path_tracks_an_independent_dense_history_reference() {
    let material = rectangular_material();
    let mut adaptive_history = Frame2dMaterialHistory::default();
    let mut dense_history =
        vec![Frame2dMaterialPointHistory::default(); DENSE_STATION_COUNT * FIBER_COUNT];
    let mut selected_orders = Vec::new();
    let mut previous_accumulated_plasticity = 0.0_f64;
    let mut maximum_force_error = 0.0_f64;
    let path = [
        (0.005, 0.01, -0.005),
        (0.03, -0.25, 0.05),
        (0.0, 0.0, 0.0),
        (-0.02, 0.2, -0.04),
        (0.015, -0.15, 0.03),
    ];

    for (step, (extension, phi_i, phi_j)) in path.into_iter().enumerate() {
        let response = section_response(
            Some(&material),
            YOUNGS_MODULUS,
            2.0,
            2.0 / 3.0,
            1.0,
            extension,
            phi_i,
            phi_j,
            &adaptive_history,
        );
        let reference =
            dense_reference_response(&material, extension, phi_i, phi_j, &mut dense_history);
        let actual = [response.axial_force, response.moment_i, response.moment_j];
        let error = relative_vector_error(actual, reference);
        let accumulated_plasticity = response
            .history
            .fiber_points
            .iter()
            .map(|point| point.equivalent_plastic_strain)
            .sum::<f64>();

        eprintln!(
            "adaptive cyclic step={step}, order={}, force_error={error:.6e}",
            response.active_longitudinal_integration_points
        );
        assert!(error.is_finite(), "step={step}, error={error}");
        assert_eq!(
            response.evaluated_fiber_point_count,
            ADAPTIVE_POINT_COUNT * FIBER_COUNT
        );
        assert_eq!(
            response.history.fiber_points.len(),
            ADAPTIVE_POINT_COUNT * FIBER_COUNT
        );
        assert!(response.longitudinal_integration_error.is_some());
        assert!(accumulated_plasticity + 1.0e-12 >= previous_accumulated_plasticity);

        selected_orders.push(response.active_longitudinal_integration_points);
        maximum_force_error = maximum_force_error.max(error);
        previous_accumulated_plasticity = accumulated_plasticity;
        adaptive_history = response.history;
    }

    assert!(
        maximum_force_error < 0.01,
        "maximum_force_error={maximum_force_error:.6e}"
    );
    assert_eq!(selected_orders[0], 2, "orders={selected_orders:?}");
    assert!(selected_orders.contains(&12), "orders={selected_orders:?}");
    assert!(
        selected_orders
            .windows(2)
            .any(|orders| orders[0] != orders[1]),
        "orders={selected_orders:?}"
    );
}

fn rectangular_material() -> CompiledFrame2dMaterial {
    let fiber_depth = 2.0 / FIBER_COUNT as f64;
    CompiledFrame2dMaterial {
        yield_strength: YIELD_STRENGTH,
        hardening_ratio: HARDENING_RATIO,
        initial_axial_stress: 0.0,
        section_fibers: (0..FIBER_COUNT)
            .map(|index| CompiledFrame2dFiber {
                y: -1.0 + (index as f64 + 0.5) * fiber_depth,
                area: fiber_depth,
                initial_axial_stress: 0.0,
                material: CompiledFrame2dPointMaterial {
                    youngs_modulus: YOUNGS_MODULUS,
                    yield_strength: YIELD_STRENGTH,
                    hardening_ratio: HARDENING_RATIO,
                    damage: None,
                },
                uses_material_override: false,
            })
            .collect(),
        fiber_material_ids: Vec::new(),
        longitudinal_integration_points: 4,
        adaptive_longitudinal_integration: true,
        longitudinal_integration_tolerance: 1.0e-3,
    }
}

fn dense_reference_response(
    material: &CompiledFrame2dMaterial,
    extension: f64,
    phi_i: f64,
    phi_j: f64,
    history: &mut [Frame2dMaterialPointHistory],
) -> [f64; 3] {
    let mut forces = [0.0; 3];
    for station_index in 0..DENSE_STATION_COUNT {
        let xi = (station_index as f64 + 0.5) / DENSE_STATION_COUNT as f64;
        let curvature_i = -4.0 + 6.0 * xi;
        let curvature_j = -2.0 + 6.0 * xi;
        for (fiber_index, fiber) in material.section_fibers.iter().enumerate() {
            let point_index = station_index * FIBER_COUNT + fiber_index;
            let strain = extension + fiber.y * (curvature_i * phi_i + curvature_j * phi_j);
            let (stress, next_history) =
                independent_bilinear_response(strain, history[point_index]);
            history[point_index] = next_history;
            let gradient = [1.0, fiber.y * curvature_i, fiber.y * curvature_j];
            for row in 0..3 {
                forces[row] += fiber.area * stress * gradient[row] / DENSE_STATION_COUNT as f64;
            }
        }
    }
    forces
}

fn independent_bilinear_response(
    strain: f64,
    committed: Frame2dMaterialPointHistory,
) -> (f64, Frame2dMaterialPointHistory) {
    let trial_stress = YOUNGS_MODULUS * (strain - committed.plastic_strain);
    let relative_trial = trial_stress - committed.backstress;
    let yield_excess = relative_trial.abs() - YIELD_STRENGTH;
    if yield_excess <= YIELD_STRENGTH * 1.0e-12 {
        return (
            trial_stress,
            Frame2dMaterialPointHistory {
                tangent_modulus: YOUNGS_MODULUS,
                ..committed
            },
        );
    }
    let plastic_modulus = YOUNGS_MODULUS * HARDENING_RATIO / (1.0 - HARDENING_RATIO);
    let plastic_increment = yield_excess / (YOUNGS_MODULUS + plastic_modulus);
    let direction = relative_trial.signum();
    let plastic_strain = committed.plastic_strain + plastic_increment * direction;
    let backstress = committed.backstress + plastic_modulus * plastic_increment * direction;
    let stress = trial_stress - YOUNGS_MODULUS * plastic_increment * direction;
    (
        stress,
        Frame2dMaterialPointHistory {
            plastic_strain,
            backstress,
            equivalent_plastic_strain: committed.equivalent_plastic_strain + plastic_increment,
            damage: committed.damage,
            tangent_modulus: YOUNGS_MODULUS * HARDENING_RATIO,
        },
    )
}

fn relative_vector_error(actual: [f64; 3], expected: [f64; 3]) -> f64 {
    let difference = actual
        .iter()
        .zip(expected)
        .map(|(actual, expected)| (actual - expected).powi(2))
        .sum::<f64>()
        .sqrt();
    let expected_norm = expected
        .iter()
        .map(|value| value.powi(2))
        .sum::<f64>()
        .sqrt();
    difference / expected_norm.max(YIELD_STRENGTH * 2.0 * 1.0e-12)
}
