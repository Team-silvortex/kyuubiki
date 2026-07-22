use kyuubiki_protocol::{
    SolveThermalFrame3dRequest, ThermalFrame3dDirectionalRotationalSpringInput,
    ThermalFrame3dDirectionalSpringInput, ThermalFrame3dElementInput, ThermalFrame3dNodeInput,
};
use kyuubiki_solver::solve_thermal_frame_3d;

const REL_TOL: f64 = 1.0e-10;
const ABS_TOL: f64 = 1.0e-12;

#[test]
fn thermal_frame_3d_directional_spring_matches_axial_closed_form() {
    let length = 1.8;
    let area = 0.016;
    let youngs_modulus = 205.0e9;
    let stiffness = 2.4e8;
    let load = 720_000.0;
    let result = solve_thermal_frame_3d(&axial_spring_request(
        length,
        area,
        youngs_modulus,
        stiffness,
        load,
    ))
    .expect("axially spring-supported thermal frame should solve");

    let member_stiffness = youngs_modulus * area / length;
    let expected_displacement = load / (member_stiffness + stiffness);
    let expected_member_energy = 0.5 * member_stiffness * expected_displacement.powi(2);
    let expected_spring_energy = 0.5 * stiffness * expected_displacement.powi(2);
    let tip = &result.nodes[1];
    let member = &result.elements[0];
    let spring = &result.directional_springs[0];

    assert_close(tip.ux, expected_displacement, "tip axial displacement");
    assert_close(tip.uy, 0.0, "tip transverse y displacement");
    assert_close(tip.uz, 0.0, "tip transverse z displacement");
    assert_close(
        member.axial_force_j,
        member_stiffness * expected_displacement,
        "member tip axial force",
    );
    assert_close(
        member.strain_energy,
        expected_member_energy,
        "member energy",
    );
    assert_eq!(spring.direction, [1.0, 0.0, 0.0]);
    assert_close(
        spring.displacement,
        expected_displacement,
        "spring displacement",
    );
    assert_close(
        spring.reaction_force,
        -stiffness * expected_displacement,
        "spring reaction",
    );
    assert_close(
        spring.strain_energy,
        expected_spring_energy,
        "spring energy",
    );
    assert_close(
        result.total_strain_energy,
        0.5 * load * expected_displacement,
        "total energy",
    );
}

#[test]
fn thermal_frame_3d_directional_rotational_spring_matches_torsion_closed_form() {
    let length = 2.1;
    let shear_modulus = 79.0e9;
    let torsion_constant = 5.2e-6;
    let stiffness = 3.6e5;
    let moment = 84_000.0;
    let result = solve_thermal_frame_3d(&torsion_spring_request(
        length,
        shear_modulus,
        torsion_constant,
        stiffness,
        moment,
    ))
    .expect("torsionally spring-supported thermal frame should solve");

    let member_stiffness = shear_modulus * torsion_constant / length;
    let expected_rotation = moment / (member_stiffness + stiffness);
    let expected_member_energy = 0.5 * member_stiffness * expected_rotation.powi(2);
    let expected_spring_energy = 0.5 * stiffness * expected_rotation.powi(2);
    let tip = &result.nodes[1];
    let member = &result.elements[0];
    let spring = &result.directional_rotational_springs[0];

    assert_close(tip.rx, expected_rotation, "tip torsional rotation");
    assert_close(tip.ry, 0.0, "tip rotation y");
    assert_close(tip.rz, 0.0, "tip rotation z");
    assert_close(
        member.torsion_j,
        member_stiffness * expected_rotation,
        "member tip torsion",
    );
    assert_close(
        member.strain_energy,
        expected_member_energy,
        "member energy",
    );
    assert_eq!(spring.direction, [1.0, 0.0, 0.0]);
    assert_close(spring.rotation, expected_rotation, "spring rotation");
    assert_close(
        spring.reaction_moment,
        -stiffness * expected_rotation,
        "spring reaction moment",
    );
    assert_close(
        spring.strain_energy,
        expected_spring_energy,
        "spring energy",
    );
    assert_close(
        result.total_strain_energy,
        0.5 * moment * expected_rotation,
        "total energy",
    );
}

fn axial_spring_request(
    length: f64,
    area: f64,
    youngs_modulus: f64,
    stiffness: f64,
    load: f64,
) -> SolveThermalFrame3dRequest {
    SolveThermalFrame3dRequest {
        nodes: vec![
            node("root", 0.0, true, 0.0),
            node("tip", length, false, load),
        ],
        elements: vec![ThermalFrame3dElementInput {
            id: "member".to_string(),
            node_i: 0,
            node_j: 1,
            local_y_axis: Some([0.0, 1.0, 0.0]),
            area,
            youngs_modulus,
            shear_modulus: 80.0e9,
            torsion_constant: 4.8e-6,
            moment_of_inertia_y: 7.0e-6,
            moment_of_inertia_z: 5.3e-6,
            section_modulus_y: 1.4e-4,
            section_modulus_z: 1.1e-4,
            thermal_expansion: 11.4e-6,
            section_depth_y: 0.22,
            section_depth_z: 0.18,
            temperature_gradient_y: 0.0,
            temperature_gradient_z: 0.0,
        }],
        directional_springs: vec![ThermalFrame3dDirectionalSpringInput {
            id: "axial-support".to_string(),
            node: 1,
            direction: [2.0, 0.0, 0.0],
            stiffness,
        }],
        directional_rotational_springs: Vec::new(),
        directional_constraints: Vec::new(),
        directional_rotational_constraints: Vec::new(),
    }
}

fn torsion_spring_request(
    length: f64,
    shear_modulus: f64,
    torsion_constant: f64,
    stiffness: f64,
    moment: f64,
) -> SolveThermalFrame3dRequest {
    SolveThermalFrame3dRequest {
        nodes: vec![
            torsion_node("root", 0.0, true, 0.0),
            torsion_node("tip", length, false, moment),
        ],
        elements: vec![ThermalFrame3dElementInput {
            id: "torsion-member".to_string(),
            node_i: 0,
            node_j: 1,
            local_y_axis: Some([0.0, 1.0, 0.0]),
            area: 0.016,
            youngs_modulus: 205.0e9,
            shear_modulus,
            torsion_constant,
            moment_of_inertia_y: 7.0e-6,
            moment_of_inertia_z: 5.3e-6,
            section_modulus_y: 1.4e-4,
            section_modulus_z: 1.1e-4,
            thermal_expansion: 11.4e-6,
            section_depth_y: 0.22,
            section_depth_z: 0.18,
            temperature_gradient_y: 0.0,
            temperature_gradient_z: 0.0,
        }],
        directional_springs: Vec::new(),
        directional_rotational_springs: vec![ThermalFrame3dDirectionalRotationalSpringInput {
            id: "torsional-support".to_string(),
            node: 1,
            direction: [3.0, 0.0, 0.0],
            stiffness,
        }],
        directional_constraints: Vec::new(),
        directional_rotational_constraints: Vec::new(),
    }
}

fn torsion_node(id: &str, x: f64, root: bool, moment_x: f64) -> ThermalFrame3dNodeInput {
    ThermalFrame3dNodeInput {
        id: id.to_string(),
        x,
        y: 0.0,
        z: 0.0,
        fix_x: true,
        fix_y: true,
        fix_z: true,
        fix_rx: root,
        fix_ry: true,
        fix_rz: true,
        load_x: 0.0,
        load_y: 0.0,
        load_z: 0.0,
        moment_x,
        moment_y: 0.0,
        moment_z: 0.0,
        temperature_delta: 0.0,
    }
}

fn node(id: &str, x: f64, fixed: bool, load_x: f64) -> ThermalFrame3dNodeInput {
    ThermalFrame3dNodeInput {
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
        load_x,
        load_y: 0.0,
        load_z: 0.0,
        moment_x: 0.0,
        moment_y: 0.0,
        moment_z: 0.0,
        temperature_delta: 0.0,
    }
}

fn assert_close(actual: f64, expected: f64, label: &str) {
    let tolerance = ABS_TOL.max(REL_TOL * expected.abs().max(1.0));
    assert!(
        (actual - expected).abs() <= tolerance,
        "{label}: expected {actual} to be close to {expected} within {tolerance}",
    );
}
