use kyuubiki_protocol::{
    Frame2dNodeInput, Frame3dNodeInput, ModalFrame2dElementInput, ModalFrame3dElementInput,
    SolveModalFrame2dRequest, SolveModalFrame3dRequest,
};
use kyuubiki_solver::{solve_modal_frame_2d, solve_modal_frame_3d};

const TOL: f64 = 1.0e-10;
const DENSITY: f64 = 7850.0;
const AREA: f64 = 0.01;
const YOUNGS_MODULUS: f64 = 210.0e9;
const MOMENT_OF_INERTIA: f64 = 8.333e-6;
const SECTION_MODULUS: f64 = 1.667e-4;
const SHEAR_MODULUS: f64 = 80.0e9;
const TORSION_CONSTANT: f64 = 1.0e-5;

#[test]
fn modal_frame_2d_sanity_retains_ordering_normalization_and_length_scaling() {
    let short = solve_modal_frame_2d(&modal_2d_request(1.0)).expect("short 2d modal frame");
    let long = solve_modal_frame_2d(&modal_2d_request(2.0)).expect("long 2d modal frame");

    assert_eq!(short.modes.len(), 3);
    assert_eq!(long.modes.len(), 3);
    assert_strictly_increasing(short.modes.iter().map(|mode| mode.natural_frequency_hz));
    assert_strictly_increasing(long.modes.iter().map(|mode| mode.natural_frequency_hz));
    assert_close(short.total_mass, DENSITY * AREA);
    assert_close(long.total_mass, DENSITY * AREA * 2.0);

    for (short_mode, long_mode) in short.modes.iter().zip(long.modes.iter()) {
        assert!(short_mode.natural_frequency_hz > long_mode.natural_frequency_hz);
        assert_modal_shape(&short_mode.shape, &short.free_dofs, 6);
        assert_modal_shape(&long_mode.shape, &long.free_dofs, 6);
        assert_close(short_mode.participation_norm, 1.0);
        assert_close(long_mode.participation_norm, 1.0);
    }
}

#[test]
fn modal_frame_3d_sanity_retains_ordering_normalization_and_length_scaling() {
    let short = solve_modal_frame_3d(&modal_3d_request(1.0)).expect("short 3d modal frame");
    let long = solve_modal_frame_3d(&modal_3d_request(2.0)).expect("long 3d modal frame");

    assert_eq!(short.modes.len(), 3);
    assert_eq!(long.modes.len(), 3);
    assert_non_decreasing(short.modes.iter().map(|mode| mode.natural_frequency_hz));
    assert_non_decreasing(long.modes.iter().map(|mode| mode.natural_frequency_hz));
    assert_close(short.total_mass, DENSITY * AREA);
    assert_close(long.total_mass, DENSITY * AREA * 2.0);

    for (short_mode, long_mode) in short.modes.iter().zip(long.modes.iter()) {
        assert!(short_mode.natural_frequency_hz > long_mode.natural_frequency_hz);
        assert_modal_shape(&short_mode.shape, &short.free_dofs, 12);
        assert_modal_shape(&long_mode.shape, &long.free_dofs, 12);
        assert_close(short_mode.participation_norm, 1.0);
        assert_close(long_mode.participation_norm, 1.0);
    }
}

fn modal_2d_request(length: f64) -> SolveModalFrame2dRequest {
    SolveModalFrame2dRequest {
        nodes: vec![node_2d("fixed", 0.0, true), node_2d("tip", length, false)],
        elements: vec![ModalFrame2dElementInput {
            id: "beam".to_string(),
            node_i: 0,
            node_j: 1,
            area: AREA,
            youngs_modulus: YOUNGS_MODULUS,
            moment_of_inertia: MOMENT_OF_INERTIA,
            section_modulus: SECTION_MODULUS,
            density: DENSITY,
        }],
        mode_count: Some(3),
    }
}

fn modal_3d_request(length: f64) -> SolveModalFrame3dRequest {
    SolveModalFrame3dRequest {
        nodes: vec![node_3d("fixed", 0.0, true), node_3d("tip", length, false)],
        elements: vec![ModalFrame3dElementInput {
            id: "beam".to_string(),
            node_i: 0,
            node_j: 1,
            area: AREA,
            youngs_modulus: YOUNGS_MODULUS,
            shear_modulus: SHEAR_MODULUS,
            torsion_constant: TORSION_CONSTANT,
            moment_of_inertia_y: MOMENT_OF_INERTIA,
            moment_of_inertia_z: MOMENT_OF_INERTIA,
            density: DENSITY,
        }],
        mode_count: Some(3),
    }
}

fn node_2d(id: &str, x: f64, fixed: bool) -> Frame2dNodeInput {
    Frame2dNodeInput {
        id: id.to_string(),
        x,
        y: 0.0,
        fix_x: fixed,
        fix_y: fixed,
        fix_rz: fixed,
        load_x: 0.0,
        load_y: 0.0,
        moment_z: 0.0,
    }
}

fn node_3d(id: &str, x: f64, fixed: bool) -> Frame3dNodeInput {
    Frame3dNodeInput {
        id: id.to_string(),
        x,
        y: 0.0,
        z: 0.0,
        fix_x: fixed,
        fix_y: fixed,
        fix_z: fixed,
        fix_rx: fixed,
        fix_ry: fixed,
        fix_rz: fixed,
        load_x: 0.0,
        load_y: 0.0,
        load_z: 0.0,
        moment_x: 0.0,
        moment_y: 0.0,
        moment_z: 0.0,
    }
}

fn assert_modal_shape(shape: &[f64], free_dofs: &[usize], expected_len: usize) {
    assert_eq!(shape.len(), expected_len);
    for (index, value) in shape.iter().enumerate() {
        assert!(value.is_finite());
        if !free_dofs.contains(&index) {
            assert_close(*value, 0.0);
        }
    }
    assert!(shape.iter().any(|value| value.abs() > 1.0e-6));
}

fn assert_strictly_increasing(values: impl Iterator<Item = f64>) {
    let mut previous = 0.0;
    for value in values {
        assert!(
            value > previous,
            "expected {value} to be greater than {previous}"
        );
        previous = value;
    }
}

fn assert_non_decreasing(values: impl Iterator<Item = f64>) {
    let mut previous = 0.0;
    for value in values {
        assert!(
            value >= previous - TOL,
            "expected {value} to be >= {previous}"
        );
        previous = value;
    }
}

fn assert_close(actual: f64, expected: f64) {
    let scale = expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= TOL * scale,
        "expected {actual} to be close to {expected}",
    );
}
