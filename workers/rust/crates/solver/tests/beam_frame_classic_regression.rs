use kyuubiki_protocol::{
    Beam1dElementInput, Beam1dNodeInput, Frame2dElementInput, Frame2dNodeInput, SolveBeam1dRequest,
    SolveFrame2dRequest, SolveTorsion1dRequest, Torsion1dElementInput, Torsion1dNodeInput,
};
use kyuubiki_solver::{solve_beam_1d, solve_frame_2d, solve_torsion_1d};

const TOL: f64 = 1.0e-10;

#[test]
fn beam_frame_classic_regression_matches_closed_form_cantilever_and_torsion_cases() {
    let length: f64 = 2.0;
    let load: f64 = 1000.0;
    let youngs_modulus: f64 = 210.0e9;
    let moment_of_inertia: f64 = 8.0e-6;
    let section_modulus: f64 = 1.6e-4;
    let expected_tip_uy = -load * length.powi(3) / (3.0 * youngs_modulus * moment_of_inertia);
    let expected_tip_rz = -load * length.powi(2) / (2.0 * youngs_modulus * moment_of_inertia);
    let expected_moment = load * length;
    let expected_bending_stress = expected_moment / section_modulus;

    let beam = solve_beam_1d(&SolveBeam1dRequest {
        nodes: vec![
            beam_node("fixed", 0.0, true, true, 0.0, 0.0),
            beam_node("tip", length, false, false, -load, 0.0),
        ],
        elements: vec![Beam1dElementInput {
            id: "cantilever".to_string(),
            node_i: 0,
            node_j: 1,
            youngs_modulus,
            moment_of_inertia,
            section_modulus,
            distributed_load_y: 0.0,
        }],
    })
    .expect("classic cantilever beam should solve");
    assert_close(beam.nodes[1].uy, expected_tip_uy);
    assert_close(beam.nodes[1].rz, expected_tip_rz);
    assert_close(beam.max_moment, expected_moment);
    assert_close(beam.max_stress, expected_bending_stress);

    let frame = solve_frame_2d(&SolveFrame2dRequest {
        nodes: vec![
            frame_node("fixed", 0.0, 0.0, true, true, true, 0.0, 0.0, 0.0),
            frame_node("tip", length, 0.0, false, false, false, 0.0, -load, 0.0),
        ],
        elements: vec![Frame2dElementInput {
            id: "cantilever-frame".to_string(),
            node_i: 0,
            node_j: 1,
            area: 0.02,
            youngs_modulus,
            moment_of_inertia,
            section_modulus,
        }],
    })
    .expect("classic frame cantilever should solve");
    assert_close(frame.nodes[1].uy, expected_tip_uy);
    assert_close(frame.nodes[1].rz, expected_tip_rz);
    assert_close(frame.nodes[1].ux, 0.0);
    assert_close(frame.max_moment, expected_moment);
    assert_close(frame.max_stress, expected_bending_stress);

    let torsion_length: f64 = 1.5;
    let torque: f64 = 2500.0;
    let shear_modulus: f64 = 79.0e9;
    let polar_moment: f64 = 1.8e-6;
    let torsion_section_modulus: f64 = 1.2e-4;
    let expected_twist = torque * torsion_length / (shear_modulus * polar_moment);
    let expected_shear_stress = torque / torsion_section_modulus;
    let torsion = solve_torsion_1d(&SolveTorsion1dRequest {
        nodes: vec![
            torsion_node("fixed", 0.0, true, 0.0),
            torsion_node("tip", torsion_length, false, torque),
        ],
        elements: vec![Torsion1dElementInput {
            id: "shaft".to_string(),
            node_i: 0,
            node_j: 1,
            shear_modulus,
            polar_moment,
            section_modulus: torsion_section_modulus,
        }],
    })
    .expect("classic torsion shaft should solve");
    assert_close(torsion.nodes[1].rz, expected_twist);
    assert_close(torsion.max_torque, torque);
    assert_close(torsion.max_stress, expected_shear_stress);
}

fn beam_node(
    id: &str,
    x: f64,
    fix_y: bool,
    fix_rz: bool,
    load_y: f64,
    moment_z: f64,
) -> Beam1dNodeInput {
    Beam1dNodeInput {
        id: id.to_string(),
        x,
        fix_y,
        fix_rz,
        load_y,
        moment_z,
    }
}

#[allow(clippy::too_many_arguments)]
fn frame_node(
    id: &str,
    x: f64,
    y: f64,
    fix_x: bool,
    fix_y: bool,
    fix_rz: bool,
    load_x: f64,
    load_y: f64,
    moment_z: f64,
) -> Frame2dNodeInput {
    Frame2dNodeInput {
        id: id.to_string(),
        x,
        y,
        fix_x,
        fix_y,
        fix_rz,
        load_x,
        load_y,
        moment_z,
    }
}

fn torsion_node(id: &str, x: f64, fix_rz: bool, torque_z: f64) -> Torsion1dNodeInput {
    Torsion1dNodeInput {
        id: id.to_string(),
        x,
        fix_rz,
        torque_z,
    }
}

fn assert_close(actual: f64, expected: f64) {
    let scale = expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= TOL * scale,
        "expected {actual} to be close to {expected}",
    );
}
