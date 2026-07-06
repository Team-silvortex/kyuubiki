use super::*;

#[test]
fn plane_quad_profile_exposes_solver_stage_breakdown() {
    let request = SolvePlaneQuad2dRequest {
        nodes: vec![
            PlaneNodeInput {
                id: "n0".to_string(),
                x: 0.0,
                y: 0.0,
                fix_x: true,
                fix_y: true,
                load_x: 0.0,
                load_y: 0.0,
            },
            PlaneNodeInput {
                id: "n1".to_string(),
                x: 1.0,
                y: 0.0,
                fix_x: false,
                fix_y: true,
                load_x: 0.0,
                load_y: 0.0,
            },
            PlaneNodeInput {
                id: "n2".to_string(),
                x: 1.0,
                y: 1.0,
                fix_x: false,
                fix_y: false,
                load_x: 0.0,
                load_y: -1000.0,
            },
            PlaneNodeInput {
                id: "n3".to_string(),
                x: 0.0,
                y: 1.0,
                fix_x: true,
                fix_y: false,
                load_x: 0.0,
                load_y: -1000.0,
            },
        ],
        elements: vec![PlaneQuadElementInput {
            id: "q0".to_string(),
            node_i: 0,
            node_j: 1,
            node_k: 2,
            node_l: 3,
            thickness: 0.02,
            youngs_modulus: 70.0e9,
            poisson_ratio: 0.33,
        }],
    };

    let profile = profile_plane_quad_2d(&request).expect("plane quad profile should solve");
    let labels = profile
        .stages
        .iter()
        .map(|stage| stage.label)
        .collect::<Vec<_>>();

    for expected in [
        "precompute",
        "assemble_global",
        "solve_system",
        "reduce_system",
        "solve_spd_system",
        "scatter_solution",
        "assemble",
    ] {
        assert!(labels.contains(&expected), "missing profile stage {expected}");
    }
}
