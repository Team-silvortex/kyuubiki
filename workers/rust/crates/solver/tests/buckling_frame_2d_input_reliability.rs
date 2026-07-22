use kyuubiki_protocol::{
    Frame2dElementInput, Frame2dNodeInput, SolveBucklingFrame2dRequest, SolveFrame2dRequest,
};
use kyuubiki_solver::solve_buckling_frame_2d;

#[test]
fn tension_only_reference_state_is_rejected() {
    let error = solve_buckling_frame_2d(&axial_request(10_000.0))
        .expect_err("tension reference state must not create compressive geometric stiffness");
    assert!(error.contains("no compressive member force"));
}

#[test]
fn zero_reference_load_is_rejected() {
    let error = solve_buckling_frame_2d(&axial_request(0.0))
        .expect_err("zero reference state must not produce a buckling factor");
    assert!(error.contains("no compressive member force"));
}

#[test]
fn invalid_frame_input_is_rejected_before_modal_assembly() {
    let mut request = axial_request(-10_000.0);
    request.frame.elements[0].moment_of_inertia = 0.0;
    let error = solve_buckling_frame_2d(&request).expect_err("invalid section must fail");
    assert!(error.contains("moment_of_inertia must be positive"));
}

fn axial_request(load_x: f64) -> SolveBucklingFrame2dRequest {
    SolveBucklingFrame2dRequest {
        frame: SolveFrame2dRequest {
            nodes: vec![
                Frame2dNodeInput {
                    id: "fixed".into(),
                    x: 0.0,
                    y: 0.0,
                    fix_x: true,
                    fix_y: true,
                    fix_rz: false,
                    load_x: 0.0,
                    load_y: 0.0,
                    moment_z: 0.0,
                },
                Frame2dNodeInput {
                    id: "loaded".into(),
                    x: 2.0,
                    y: 0.0,
                    fix_x: false,
                    fix_y: true,
                    fix_rz: false,
                    load_x,
                    load_y: 0.0,
                    moment_z: 0.0,
                },
            ],
            elements: vec![Frame2dElementInput {
                id: "column".into(),
                node_i: 0,
                node_j: 1,
                area: 0.01,
                youngs_modulus: 200.0e9,
                moment_of_inertia: 5.0e-6,
                section_modulus: 1.0e-4,
            }],
        },
        mode_count: Some(1),
    }
}
