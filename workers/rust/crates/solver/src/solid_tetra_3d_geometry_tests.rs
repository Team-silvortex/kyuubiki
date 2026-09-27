use super::{SolidTetra3dElementKernel, element_dof_map, geometry, reference};
use kyuubiki_protocol::SolidTetra3dElementInput;

pub(super) const POINTS: [[f64; 3]; 4] = [
    [0.0, 0.0, 0.0],
    [1.0, 0.25, 0.0],
    [0.25, 1.25, 0.25],
    [0.0, 0.25, 1.5],
];

pub(super) fn element() -> SolidTetra3dElementInput {
    SolidTetra3dElementInput {
        id: "geometry-probe".into(),
        node_a: 0,
        node_b: 1,
        node_c: 2,
        node_d: 3,
        youngs_modulus: 1200.0,
        poisson_ratio: 0.25,
    }
}

fn close(actual: f64, expected: f64, scale: f64) {
    assert!(
        actual.is_finite() && (actual - expected).abs() <= 2e-13 * scale,
        "actual={actual:e} expected={expected:e} scale={scale:e}"
    );
}

#[test]
fn tetra_geometry_is_invariant_under_exact_large_coordinate_translations() {
    let expected = geometry(POINTS, "origin").unwrap();
    for offset in [2f64.powi(20), -2f64.powi(20), 2f64.powi(40), -2f64.powi(40)] {
        let points = POINTS.map(|point| [point[0] + offset, point[1] - offset, point[2] + offset]);
        let actual = geometry(points, "translated").expect("translation must not damage shape");
        close(actual.0, expected.0, expected.0);
        close(actual.1, expected.1, 1.0);
        for (row, reference) in actual.2.iter().zip(expected.2) {
            for (&value, target) in row.iter().zip(reference) {
                close(value, target, 1.0);
            }
        }
    }
}

#[test]
fn tetra_geometry_retains_representable_volume_when_the_raw_determinant_overflows() {
    let reference = [[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    let base = geometry(reference, "unit").unwrap();
    for scale in [1e-100_f64, 1e-50, 1.0, 1e50, 1e100, 1e103] {
        let points = reference.map(|point| point.map(|value| value * scale));
        let actual = geometry(points, "scaled").expect("volume and gradients are representable");
        let volume = ((base.0 * scale) * scale) * scale;
        close(actual.0, volume, volume);
        close(actual.1, base.1, 1.0);
        for (row, expected) in actual.2.iter().zip(base.2) {
            for (&value, target) in row.iter().zip(expected) {
                close(value * scale, target, 1.0);
            }
        }
    }
}

#[test]
fn tetra_affine_strain_and_stress_hold_for_every_node_permutation() {
    let element = element();
    let gradient = [
        [0.001, 0.002, -0.001],
        [0.004, -0.002, 0.003],
        [-0.003, 0.001, 0.004],
    ];
    let strain = [0.001, -0.002, 0.004, 0.006, 0.004, -0.004];
    let lambda = 480.0;
    let mu = 480.0;
    let trace = strain[0] + strain[1] + strain[2];
    for a in 0..4 {
        for b in 0..4 {
            for c in 0..4 {
                for d in 0..4 {
                    let indices = [a, b, c, d];
                    if indices
                        .iter()
                        .enumerate()
                        .any(|(i, x)| indices[..i].contains(x))
                    {
                        continue;
                    }
                    let points = indices.map(|i| POINTS[i]);
                    let shifted = points.map(|point| point.map(|x| x + 2f64.powi(40)));
                    let kernel = SolidTetra3dElementKernel::new(shifted, &element)
                        .expect("every orientation must preserve the affine patch");
                    let displacements: Vec<_> = points
                        .iter()
                        .flat_map(|point| {
                            gradient.map(|row| (0..3).map(|i| row[i] * point[i]).sum::<f64>())
                        })
                        .collect();
                    let result =
                        kernel.result(0, &element, &element_dof_map(&element), &displacements);
                    let actual_strain = [
                        result.strain_x,
                        result.strain_y,
                        result.strain_z,
                        result.gamma_xy,
                        result.gamma_yz,
                        result.gamma_zx,
                    ];
                    let actual_stress = [
                        result.stress_x,
                        result.stress_y,
                        result.stress_z,
                        result.shear_xy,
                        result.shear_yz,
                        result.shear_zx,
                    ];
                    for i in 0..6 {
                        close(actual_strain[i], strain[i], 0.01);
                        let stress = if i < 3 {
                            lambda * trace + 2.0 * mu * strain[i]
                        } else {
                            mu * strain[i]
                        };
                        close(actual_stress[i], stress, 10.0);
                    }
                }
            }
        }
    }
}

#[test]
fn tetra_kernel_rejects_unrepresentable_constitutive_and_stiffness_coefficients() {
    let mut material = element();
    material.youngs_modulus = f64::MAX;
    material.poisson_ratio = 0.49;
    assert!(SolidTetra3dElementKernel::new(POINTS, &material).is_err());
    material.poisson_ratio = 0.0;
    let huge = POINTS.map(|point| point.map(|value| value * 1e100));
    assert!(SolidTetra3dElementKernel::new(huge, &material).is_err());
    assert!(SolidTetra3dElementKernel::new(POINTS, &element()).is_ok());
}

pub(super) fn compare_kernels(
    actual: &SolidTetra3dElementKernel,
    expected: &SolidTetra3dElementKernel,
) {
    close(actual.volume, expected.volume, expected.volume);
    close(actual.mean_ratio_quality, expected.mean_ratio_quality, 1.0);
    for (a, b) in actual.b.iter().flatten().zip(expected.b.iter().flatten()) {
        close(*a, *b, 1.0);
    }
    let stiffness_scale = expected
        .stiffness
        .iter()
        .flatten()
        .map(|x| x.abs())
        .fold(0.0, f64::max);
    for row in 0..12 {
        for column in 0..12 {
            close(
                actual.stiffness[row][column],
                expected.stiffness[row][column],
                stiffness_scale,
            );
            close(
                actual.stiffness[row][column],
                actual.stiffness[column][row],
                stiffness_scale,
            );
        }
    }
}

#[test]
fn tetra_kernel_matches_independent_cofactors_on_skewed_oriented_material_cases() {
    let mut element = element();
    for shear in [-0.5, 0.0, 0.75] {
        for poisson_ratio in [-0.5, 0.0, 0.25, 0.49] {
            element.poisson_ratio = poisson_ratio;
            for order in [[0, 1, 2, 3], [1, 3, 0, 2], [3, 2, 1, 0], [0, 2, 1, 3]] {
                let points = order.map(|i| {
                    let [x, y, z] = POINTS[i];
                    [x + shear * y + 0.25, y + 0.5 * z - 0.5, z + 0.75]
                });
                let actual = SolidTetra3dElementKernel::new(points, &element).unwrap();
                compare_kernels(&actual, &reference::kernel(points, &element).unwrap());
                // Rigid translations and infinitesimal rotations carry no strain energy.
                for mode in 0..6 {
                    let displacement: Vec<_> = points
                        .iter()
                        .flat_map(|&[x, y, z]| match mode {
                            0 => [1.0, 0.0, 0.0],
                            1 => [0.0, 1.0, 0.0],
                            2 => [0.0, 0.0, 1.0],
                            3 => [0.0, -z, y],
                            4 => [z, 0.0, -x],
                            _ => [-y, x, 0.0],
                        })
                        .collect();
                    for row in actual.stiffness {
                        let force = row
                            .iter()
                            .zip(&displacement)
                            .map(|(k, u)| k * u)
                            .sum::<f64>();
                        let scale = row.iter().map(|x| x.abs()).sum::<f64>();
                        close(force, 0.0, scale);
                    }
                }
            }
        }
    }
}

#[test]
fn tetra_geometry_rejects_collapsed_or_unrepresentable_cells_before_replay() {
    let unit = [[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    for (points, expected) in [
        ([[0.0; 3]; 4], "zero volume"),
        (
            unit.map(|p| p.map(|x| x * 1e-110)),
            "volume is not representable",
        ),
        (
            unit.map(|p| p.map(|x| x * 1e104)),
            "volume is not representable",
        ),
        (
            [[0.0; 3], unit[1], unit[2], [0.0, 0.0, 1e-20]],
            "near-degenerate",
        ),
        (
            [
                [-f64::MAX, 0.0, 0.0],
                [f64::MAX, 0.0, 0.0],
                unit[2],
                unit[3],
            ],
            "edge differences",
        ),
    ] {
        let error = geometry(points, "bad-cell").unwrap_err();
        assert!(
            error.contains(expected) && error.contains("bad-cell"),
            "{error}"
        );
    }
    assert!(geometry(unit, "replay").is_ok());
}
