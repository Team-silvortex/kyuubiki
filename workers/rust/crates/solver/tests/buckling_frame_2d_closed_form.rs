use kyuubiki_protocol::{
    Frame2dElementInput, Frame2dNodeInput, SolveBucklingFrame2dRequest, SolveFrame2dRequest,
};
use kyuubiki_solver::solve_buckling_frame_2d;

const YOUNGS_MODULUS: f64 = 205.0e9;
const INERTIA: f64 = 7.4e-6;
const REFERENCE_FORCE: f64 = 100_000.0;

#[test]
fn static_preload_column_converges_to_euler_critical_load() {
    let length: f64 = 3.2;
    let expected = std::f64::consts::PI.powi(2) * YOUNGS_MODULUS * INERTIA / length.powi(2);
    let mut errors = Vec::new();

    for elements in [1, 2, 4, 8, 16] {
        let result = solve_buckling_frame_2d(&column_request(elements, length, true, 1))
            .expect("preloaded frame column should solve");
        let critical = result.minimum_load_factor * REFERENCE_FORCE;
        errors.push((critical - expected).abs());
        assert_eq!(result.modes[0].shape.len(), (elements + 1) * 3);
        assert!(
            result
                .element_preloads
                .iter()
                .all(|preload| preload.active_in_geometric_stiffness)
        );
        for preload in result.element_preloads {
            assert_relative(preload.reference_compressive_force, REFERENCE_FORCE, 1.0e-8);
        }
    }

    assert!(errors.windows(2).all(|pair| pair[1] < pair[0]));
    assert!(errors[4] / expected < 1.0e-5);
}

#[test]
fn critical_factor_is_invariant_to_global_column_orientation() {
    let vertical = solve_buckling_frame_2d(&column_request(8, 3.2, true, 1))
        .expect("vertical column should solve");
    let horizontal = solve_buckling_frame_2d(&column_request(8, 3.2, false, 1))
        .expect("horizontal column should solve");

    assert_relative(
        horizontal.minimum_load_factor,
        vertical.minimum_load_factor,
        1.0e-8,
    );
}

#[test]
fn semidefinite_geometric_stiffness_supports_sorted_multiple_modes() {
    let result = solve_buckling_frame_2d(&column_request(8, 3.2, true, 3))
        .expect("multi-mode frame column should solve");

    assert_eq!(result.modes.len(), 3);
    assert!(
        result
            .modes
            .windows(2)
            .all(|pair| pair[0].load_factor < pair[1].load_factor)
    );
    for mode in result.modes {
        let norm = mode
            .shape
            .iter()
            .map(|value| value * value)
            .sum::<f64>()
            .sqrt();
        assert_relative(norm, 1.0, 1.0e-10);
        assert!(mode.residual_norm.is_finite());
    }
}

fn column_request(
    element_count: usize,
    length: f64,
    vertical: bool,
    mode_count: usize,
) -> SolveBucklingFrame2dRequest {
    let segment = length / element_count as f64;
    let nodes = (0..=element_count)
        .map(|index| {
            let endpoint = index == 0 || index == element_count;
            let loaded = index == element_count;
            Frame2dNodeInput {
                id: format!("n{index}"),
                x: if vertical {
                    0.0
                } else {
                    index as f64 * segment
                },
                y: if vertical {
                    index as f64 * segment
                } else {
                    0.0
                },
                fix_x: if vertical { endpoint } else { index == 0 },
                fix_y: if vertical { index == 0 } else { endpoint },
                fix_rz: false,
                load_x: if !vertical && loaded {
                    -REFERENCE_FORCE
                } else {
                    0.0
                },
                load_y: if vertical && loaded {
                    -REFERENCE_FORCE
                } else {
                    0.0
                },
                moment_z: 0.0,
            }
        })
        .collect();
    let elements = (0..element_count)
        .map(|index| Frame2dElementInput {
            id: format!("e{index}"),
            node_i: index,
            node_j: index + 1,
            area: 0.01,
            youngs_modulus: YOUNGS_MODULUS,
            moment_of_inertia: INERTIA,
            section_modulus: 1.0e-4,
        })
        .collect();
    SolveBucklingFrame2dRequest {
        frame: SolveFrame2dRequest { nodes, elements },
        mode_count: Some(mode_count),
    }
}

fn assert_relative(actual: f64, expected: f64, tolerance: f64) {
    let relative = (actual - expected).abs() / expected.abs().max(1.0);
    assert!(
        relative <= tolerance,
        "actual={actual}, expected={expected}, relative={relative}"
    );
}
