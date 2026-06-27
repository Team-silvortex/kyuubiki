use kyuubiki_engine::{EngineSolveRequest, solve};
use kyuubiki_protocol::{
    AnalysisResult, Frame2dNodeInput, ModalFrame2dElementInput, SolveModalFrame2dRequest,
};

#[test]
fn engine_solves_modal_frame_2d() {
    let result = solve(EngineSolveRequest::ModalFrame2d(SolveModalFrame2dRequest {
        nodes: vec![
            node("n0", 0.0, 0.0, true, true, true),
            node("n1", 2.0, 0.0, false, false, false),
        ],
        elements: vec![ModalFrame2dElementInput {
            id: "e0".to_string(),
            node_i: 0,
            node_j: 1,
            area: 0.01,
            youngs_modulus: 210.0e9,
            moment_of_inertia: 8.333e-6,
            section_modulus: 1.667e-4,
            density: 7850.0,
        }],
        mode_count: Some(2),
    }))
    .expect("engine modal solve should succeed");

    match result {
        AnalysisResult::ModalFrame2d(result) => {
            assert_eq!(result.modes.len(), 2);
            assert!(result.min_frequency_hz > 0.0);
        }
        other => panic!("unexpected result: {other:?}"),
    }
}

fn node(id: &str, x: f64, y: f64, fix_x: bool, fix_y: bool, fix_rz: bool) -> Frame2dNodeInput {
    Frame2dNodeInput {
        id: id.to_string(),
        x,
        y,
        fix_x,
        fix_y,
        fix_rz,
        load_x: 0.0,
        load_y: 0.0,
        moment_z: 0.0,
    }
}
