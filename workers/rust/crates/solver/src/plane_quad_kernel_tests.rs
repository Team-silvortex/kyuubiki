use super::{
    PlaneQuadComputed, multiply_matrix_vector_3x8, plane_quad_state,
    precompute_plane_quad_element_from_coordinates as compute, reference,
};

pub(super) const POINTS: [[f64; 2]; 4] = [[-0.5, 0.0], [1.5, 0.25], [1.75, 1.25], [0.0, 1.0]];

pub(super) fn close(actual: f64, expected: f64, scale: f64) {
    assert!(
        actual.is_finite() && (actual - expected).abs() <= 2e-12 * scale,
        "actual={actual:e} expected={expected:e} scale={scale:e}"
    );
}

pub(super) fn compare(actual: &PlaneQuadComputed, expected: &PlaneQuadComputed) {
    close(actual.area, expected.area, expected.area);
    for (a, b) in actual.gauss_points.iter().zip(&expected.gauss_points) {
        close(a.det_jacobian, b.det_jacobian, b.det_jacobian);
        for (&x, &y) in a.b_matrix.iter().flatten().zip(b.b_matrix.iter().flatten()) {
            close(x, y, 1.0);
        }
        for (&x, &y) in a.shape_functions.iter().zip(&b.shape_functions) {
            close(x, y, 1.0);
        }
    }
    let scale = expected
        .stiffness
        .iter()
        .flatten()
        .map(|x| x.abs())
        .fold(0.0, f64::max);
    for row in 0..8 {
        for column in 0..8 {
            close(
                actual.stiffness[row][column],
                expected.stiffness[row][column],
                scale,
            );
            close(
                actual.stiffness[row][column],
                actual.stiffness[column][row],
                scale,
            );
        }
    }
}

#[test]
fn q4_kernel_is_invariant_under_exact_large_coordinate_translations() {
    let reference = compute(POINTS, 0.5, 1200.0, 0.25).unwrap();
    for offset in [2f64.powi(20), -2f64.powi(20), 2f64.powi(40), -2f64.powi(40)] {
        let translated = POINTS.map(|[x, y]| [x + offset, y - offset]);
        compare(&compute(translated, 0.5, 1200.0, 0.25).unwrap(), &reference);
    }
}

#[test]
fn q4_translated_affine_strain_is_correct_at_every_gauss_point_and_cyclic_order() {
    let expected = [0.002, 0.004, 0.002];
    for shift in 0..4 {
        let points: [_; 4] = std::array::from_fn(|i| POINTS[(i + shift) % 4]);
        let coordinates = points.map(|[x, y]| [x + 2f64.powi(40), y - 2f64.powi(40)]);
        let computed = compute(coordinates, 0.5, 1200.0, 0.25).unwrap();
        let displacements = std::array::from_fn(|i| {
            let [x, y] = points[i / 2];
            if i % 2 == 0 {
                0.002 * x + 0.003 * y
            } else {
                -0.001 * x + 0.004 * y
            }
        });
        for point in &computed.gauss_points {
            let strain = multiply_matrix_vector_3x8(&point.b_matrix, &displacements);
            for i in 0..3 {
                close(strain[i], expected[i], 0.01);
            }
        }
        let result = plane_quad_state(&computed, &displacements);
        let stress = [3.84, 5.76, 0.96];
        for i in 0..3 {
            close(result.strain[i], expected[i], 0.01);
            close(result.stress[i], stress[i], 10.0);
        }
        close(
            result.strain_energy_density,
            0.5 * (0..3).map(|i| expected[i] * stress[i]).sum::<f64>(),
            0.02,
        );
    }
}

#[test]
fn q4_kernel_rejects_nonfinite_coefficients_and_integrated_area_then_replays() {
    let square = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    for (points, thickness, modulus, poisson) in [
        (square, 1.0, f64::MAX, 0.49),
        (square, f64::MAX, 1200.0, 0.25),
        (square.map(|p| p.map(|x| x * 1.7e154)), 1.0, 1.0, 0.0),
    ] {
        let error = compute(points, thickness, modulus, poisson).unwrap_err();
        assert!(error.contains("representable"), "{error}");
    }
    assert!(compute(square, 1.0, 1200.0, 0.25).is_ok());
}

#[test]
fn q4_factored_stiffness_matches_nine_term_reference_and_rigid_modes() {
    for shear in [-0.5, 0.0, 0.75] {
        for poisson in [-0.5, 0.0, 0.25, 0.49] {
            for shift in 0..4 {
                let points = std::array::from_fn(|i| {
                    let [x, y] = POINTS[(i + shift) % 4];
                    [x + shear * y + 0.25, y - 0.5]
                });
                let computed = compute(points, 0.5, 1200.0, poisson).unwrap();
                compare(
                    &computed,
                    &reference::compute(points, 0.5, 1200.0, poisson).unwrap(),
                );
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
                        let force = row
                            .iter()
                            .zip(&displacements)
                            .map(|(k, u)| k * u)
                            .sum::<f64>();
                        close(force, 0.0, row.iter().map(|x| x.abs()).sum());
                    }
                }
            }
        }
    }
}
