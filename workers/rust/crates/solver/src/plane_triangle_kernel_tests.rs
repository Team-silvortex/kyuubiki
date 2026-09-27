use super::{
    PlaneTriangleComputed, plane_triangle_state,
    precompute_plane_triangle_element_from_coordinates as compute, reference,
    thermal_plane_triangle_equivalent_load,
};

pub(super) const POINTS: [[f64; 2]; 3] = [[-0.5, 0.0], [1.5, 0.25], [0.25, 1.5]];
const ORDERS: [[usize; 3]; 6] = [
    [0, 1, 2],
    [1, 2, 0],
    [2, 0, 1],
    [0, 2, 1],
    [2, 1, 0],
    [1, 0, 2],
];

pub(super) fn close(actual: f64, expected: f64, scale: f64) {
    assert!(
        actual.is_finite() && (actual - expected).abs() <= 2e-12 * scale,
        "actual={actual:e} expected={expected:e} scale={scale:e}"
    );
}

#[test]
fn triangle_affine_strain_and_signed_stress_survive_all_orders_and_translations() {
    for order in ORDERS {
        let points = order.map(|i| POINTS[i]);
        for offset in [0.0, 2f64.powi(40), -2f64.powi(40)] {
            let coordinates = points.map(|[x, y]| [x + offset, y - offset]);
            let computed = compute(coordinates, 0.5, 1200.0, 0.25).unwrap();
            let displacements = std::array::from_fn(|i| {
                let [x, y] = points[i / 2];
                if i % 2 == 0 {
                    0.002 * x + 0.003 * y
                } else {
                    -0.001 * x + 0.004 * y
                }
            });
            let state = plane_triangle_state(&computed, &displacements);
            for (actual, expected) in state.strain.into_iter().zip([0.002, 0.004, 0.002]) {
                close(actual, expected, 0.01);
            }
            for (actual, expected) in state.stress.into_iter().zip([3.84, 5.76, 0.96]) {
                close(actual, expected, 10.0);
            }
            close(computed.area, 1.40625, 2.0);
            close(state.strain_energy_density, 0.01632, 0.02);
        }
    }
}

#[test]
fn triangle_thermal_load_obeys_affine_virtual_work_and_node_permutation() {
    let reference = compute(POINTS, 0.5, 1200.0, 0.25).unwrap();
    let load = |c: &super::PlaneTriangleComputed| {
        thermal_plane_triangle_equivalent_load(&c.b_matrix, &c.d_matrix, c.area, 0.5, 1e-5, 50.0)
    };
    let reference_load = load(&reference);
    for order in ORDERS {
        let points = order.map(|i| POINTS[i]);
        let computed = compute(points, 0.5, 1200.0, 0.25).unwrap();
        let forces = load(&computed);
        let work: f64 = forces
            .iter()
            .enumerate()
            .map(|(i, force)| {
                let [x, y] = points[i / 2];
                force
                    * if i % 2 == 0 {
                        0.002 * x + 0.003 * y
                    } else {
                        -0.001 * x + 0.004 * y
                    }
            })
            .sum();
        close(work, 0.8 * (0.002 + 0.004) * 0.5 * 1.40625, 0.01);
        for i in 0..6 {
            close(forces[i], reference_load[2 * order[i / 2] + i % 2], 1.0);
        }
    }
}

#[test]
fn triangle_nonfinite_geometry_and_coefficients_are_rejected_before_assembly() {
    let regular = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]];
    for (points, thickness, modulus, poisson) in [
        (regular, 1.0, f64::MAX, 0.49),
        (regular, f64::MAX, 1200.0, 0.25),
        (regular.map(|p| p.map(|x| x * 1.7e154)), 1.0, 1.0, 0.0),
        ([[f64::NAN, 0.0], [1.0, 0.0], [0.0, 1.0]], 1.0, 1.0, 0.0),
    ] {
        let error = compute(points, thickness, modulus, poisson).unwrap_err();
        assert!(error.contains("representable"), "{error}");
    }
    assert!(compute(regular, 1.0, 1200.0, 0.25).is_ok());
}

pub(super) fn compare(actual: &PlaneTriangleComputed, expected: &PlaneTriangleComputed) {
    close(actual.area, expected.area, expected.area);
    let scale = expected
        .stiffness
        .iter()
        .flatten()
        .map(|x| x.abs())
        .fold(0.0, f64::max);
    for row in 0..6 {
        for column in 0..6 {
            close(
                actual.stiffness[row][column],
                expected.stiffness[row][column],
                scale,
            );
            assert_eq!(actual.stiffness[row][column], actual.stiffness[column][row]);
        }
    }
    for (&actual, &expected) in actual
        .b_matrix
        .iter()
        .flatten()
        .zip(expected.b_matrix.iter().flatten())
    {
        close(actual, expected, 1.0);
    }
}

#[test]
fn triangle_factored_stiffness_matches_dense_reference_and_all_order_rigid_modes() {
    for shear in [-0.5, 0.0, 0.75] {
        for poisson in [-0.5, 0.0, 0.25, 0.49] {
            let original = POINTS.map(|[x, y]| [x + shear * y, y]);
            let reference = reference::compute(original, 0.5, 1200.0, poisson).unwrap();
            for order in ORDERS {
                let points = order.map(|i| original[i]);
                let computed = compute(points, 0.5, 1200.0, poisson).unwrap();
                let mut reordered = reference.clone();
                reordered.stiffness = std::array::from_fn(|row| {
                    std::array::from_fn(|column| {
                        reference.stiffness[2 * order[row / 2] + row % 2]
                            [2 * order[column / 2] + column % 2]
                    })
                });
                reordered.b_matrix = std::array::from_fn(|row| {
                    std::array::from_fn(|column| {
                        reference.b_matrix[row][2 * order[column / 2] + column % 2]
                    })
                });
                compare(&computed, &reordered);
                for mode in 0..3 {
                    let displacements: Vec<_> = points
                        .iter()
                        .flat_map(|&[x, y]| match mode {
                            0 => [1.0, 0.0],
                            1 => [0.0, 1.0],
                            _ => [-y, x],
                        })
                        .collect();
                    for row in computed.stiffness {
                        let force: f64 = row.iter().zip(&displacements).map(|(k, u)| k * u).sum();
                        close(force, 0.0, row.iter().map(|x| x.abs()).sum());
                    }
                }
            }
        }
    }
}
