use kyuubiki_protocol::{
    Frame2dElementInput, Frame2dNodeInput, SolveBucklingFrame2dRequest, SolveFrame2dRequest,
};
use kyuubiki_solver::solve_buckling_frame_2d;

#[test]
fn portal_frame_extracts_column_compression_and_global_modes() {
    let result = solve_buckling_frame_2d(&portal_request(0.0))
        .expect("symmetric preloaded portal should solve");

    assert_eq!(result.modes.len(), 3);
    assert_eq!(result.free_dofs.len(), 6);
    assert!(result.minimum_load_factor > 0.0);
    assert!(
        result.modes[0]
            .shape
            .iter()
            .any(|value| value.abs() > 1.0e-4)
    );

    let left = preload(&result, "left-column");
    let beam = preload(&result, "beam");
    let right = preload(&result, "right-column");
    assert_relative(left.reference_compressive_force, 80_000.0, 1.0e-9);
    assert_relative(right.reference_compressive_force, 80_000.0, 1.0e-9);
    assert!(left.active_in_geometric_stiffness);
    assert!(right.active_in_geometric_stiffness);
    assert!(beam.reference_compressive_force < 1.0e-6);
    assert!(!beam.active_in_geometric_stiffness);
}

#[test]
fn portal_buckling_is_objective_under_rigid_rotation() {
    let baseline =
        solve_buckling_frame_2d(&portal_request(0.0)).expect("baseline portal should solve");
    let rotated =
        solve_buckling_frame_2d(&portal_request(0.731)).expect("rotated portal should solve");

    assert_relative(
        rotated.minimum_load_factor,
        baseline.minimum_load_factor,
        2.0e-8,
    );
    for id in ["left-column", "beam", "right-column"] {
        assert_relative(
            preload(&rotated, id).reference_compressive_force,
            preload(&baseline, id).reference_compressive_force,
            2.0e-8,
        );
    }
}

fn portal_request(angle: f64) -> SolveBucklingFrame2dRequest {
    let points = [(0.0, 0.0), (4.0, 0.0), (0.0, 3.0), (4.0, 3.0)];
    let nodes = points
        .into_iter()
        .enumerate()
        .map(|(index, (x, y))| {
            let (x, y) = rotate(x, y, angle);
            let loaded = index >= 2;
            let (load_x, load_y) = if loaded {
                rotate(0.0, -80_000.0, angle)
            } else {
                (0.0, 0.0)
            };
            Frame2dNodeInput {
                id: format!("n{index}"),
                x,
                y,
                fix_x: index < 2,
                fix_y: index < 2,
                fix_rz: index < 2,
                load_x,
                load_y,
                moment_z: 0.0,
            }
        })
        .collect();
    let elements = [
        element("left-column", 0, 2),
        element("beam", 2, 3),
        element("right-column", 1, 3),
    ];
    SolveBucklingFrame2dRequest {
        frame: SolveFrame2dRequest {
            nodes,
            elements: elements.into(),
        },
        mode_count: Some(3),
    }
}

fn element(id: &str, node_i: usize, node_j: usize) -> Frame2dElementInput {
    Frame2dElementInput {
        id: id.to_string(),
        node_i,
        node_j,
        area: 0.012,
        youngs_modulus: 210.0e9,
        moment_of_inertia: 9.0e-5,
        section_modulus: 6.0e-4,
    }
}

fn rotate(x: f64, y: f64, angle: f64) -> (f64, f64) {
    let (sine, cosine) = angle.sin_cos();
    (cosine * x - sine * y, sine * x + cosine * y)
}

fn preload<'a>(
    result: &'a kyuubiki_protocol::SolveBucklingFrame2dResult,
    id: &str,
) -> &'a kyuubiki_protocol::BucklingFrame2dElementPreloadResult {
    result
        .element_preloads
        .iter()
        .find(|preload| preload.id == id)
        .expect("portal element preload should exist")
}

fn assert_relative(actual: f64, expected: f64, tolerance: f64) {
    let relative = (actual - expected).abs() / expected.abs().max(1.0);
    assert!(
        relative <= tolerance,
        "actual={actual}, expected={expected}, relative={relative}"
    );
}
