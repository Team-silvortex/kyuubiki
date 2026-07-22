use kyuubiki_protocol::{
    SolveThermalFrame3dRequest, ThermalFrame3dDirectionalConstraintInput,
    ThermalFrame3dDirectionalRotationalConstraintInput, ThermalFrame3dElementInput,
    ThermalFrame3dNodeInput,
};
use kyuubiki_solver::solve_thermal_frame_3d;

const REL_TOL: f64 = 2.0e-9;
const ABS_TOL: f64 = 1.0e-11;

#[test]
fn directional_translation_constraint_matches_coupled_closed_form() {
    let length = 1.7;
    let youngs_modulus = 205.0e9;
    let area = 0.014;
    let inertia_z = 6.2e-6;
    let load = [360_000.0, -1_400.0];
    let result = solve_thermal_frame_3d(&translation_constraint_request(
        length,
        youngs_modulus,
        area,
        inertia_z,
        load,
    ))
    .expect("directionally constrained translation fixture should solve");

    let axial_stiffness = youngs_modulus * area / length;
    let transverse_stiffness = 12.0 * youngs_modulus * inertia_z / length.powi(3);
    let ux = (load[0] - load[1]) / (axial_stiffness + transverse_stiffness);
    let uy = -ux;
    let direction_scale = 2.0_f64.sqrt().recip();
    let expected_reaction = direction_scale
        * ((axial_stiffness * ux - load[0]) + (transverse_stiffness * uy - load[1]));
    let tip = &result.nodes[1];
    let constraint = &result.directional_constraints[0];

    assert_close(tip.ux, ux, "tip ux");
    assert_close(tip.uy, uy, "tip uy");
    assert_close(
        constraint.displacement,
        0.0,
        "exact constrained displacement",
    );
    assert_close(
        constraint.reaction_force,
        expected_reaction,
        "constraint reaction",
    );
    assert_close(
        result.total_strain_energy,
        0.5 * (load[0] * ux + load[1] * uy),
        "translation fixture energy",
    );
}

#[test]
fn directional_rotation_constraint_matches_coupled_closed_form() {
    let length = 2.2;
    let youngs_modulus = 205.0e9;
    let shear_modulus = 79.0e9;
    let inertia_y = 7.1e-6;
    let torsion_constant = 4.9e-6;
    let moment = [68_000.0, -24_000.0];
    let result = solve_thermal_frame_3d(&rotation_constraint_request(
        length,
        youngs_modulus,
        shear_modulus,
        inertia_y,
        torsion_constant,
        moment,
    ))
    .expect("directionally constrained rotation fixture should solve");

    let torsion_stiffness = shear_modulus * torsion_constant / length;
    let bending_stiffness = 4.0 * youngs_modulus * inertia_y / length;
    let rx = (moment[0] - moment[1]) / (torsion_stiffness + bending_stiffness);
    let ry = -rx;
    let direction_scale = 2.0_f64.sqrt().recip();
    let expected_reaction = direction_scale
        * ((torsion_stiffness * rx - moment[0]) + (bending_stiffness * ry - moment[1]));
    let tip = &result.nodes[1];
    let constraint = &result.directional_rotational_constraints[0];

    assert_close(tip.rx, rx, "tip rx");
    assert_close(tip.ry, ry, "tip ry");
    assert_close(constraint.rotation, 0.0, "exact constrained rotation");
    assert_close(
        constraint.reaction_moment,
        expected_reaction,
        "constraint reaction moment",
    );
    assert_close(
        result.total_strain_energy,
        0.5 * (moment[0] * rx + moment[1] * ry),
        "rotation fixture energy",
    );
}

fn translation_constraint_request(
    length: f64,
    youngs_modulus: f64,
    area: f64,
    inertia_z: f64,
    load: [f64; 2],
) -> SolveThermalFrame3dRequest {
    let mut tip = node("tip", length, false);
    tip.fix_z = true;
    tip.fix_rx = true;
    tip.fix_ry = true;
    tip.fix_rz = true;
    tip.load_x = load[0];
    tip.load_y = load[1];
    SolveThermalFrame3dRequest {
        nodes: vec![node("root", 0.0, true), tip],
        elements: vec![element(
            youngs_modulus,
            79.0e9,
            area,
            7.1e-6,
            inertia_z,
            4.9e-6,
        )],
        directional_springs: Vec::new(),
        directional_rotational_springs: Vec::new(),
        directional_constraints: vec![ThermalFrame3dDirectionalConstraintInput {
            id: "diagonal-guide".to_string(),
            node: 1,
            direction: [1.0, 1.0, 0.0],
        }],
        directional_rotational_constraints: Vec::new(),
    }
}

fn rotation_constraint_request(
    length: f64,
    youngs_modulus: f64,
    shear_modulus: f64,
    inertia_y: f64,
    torsion_constant: f64,
    moment: [f64; 2],
) -> SolveThermalFrame3dRequest {
    let mut tip = node("tip", length, false);
    tip.fix_x = true;
    tip.fix_y = true;
    tip.fix_z = true;
    tip.fix_rz = true;
    tip.moment_x = moment[0];
    tip.moment_y = moment[1];
    SolveThermalFrame3dRequest {
        nodes: vec![node("root", 0.0, true), tip],
        elements: vec![element(
            youngs_modulus,
            shear_modulus,
            0.014,
            inertia_y,
            6.2e-6,
            torsion_constant,
        )],
        directional_springs: Vec::new(),
        directional_rotational_springs: Vec::new(),
        directional_constraints: Vec::new(),
        directional_rotational_constraints: vec![
            ThermalFrame3dDirectionalRotationalConstraintInput {
                id: "diagonal-rotational-guide".to_string(),
                node: 1,
                direction: [1.0, 1.0, 0.0],
            },
        ],
    }
}

fn element(
    youngs_modulus: f64,
    shear_modulus: f64,
    area: f64,
    inertia_y: f64,
    inertia_z: f64,
    torsion_constant: f64,
) -> ThermalFrame3dElementInput {
    ThermalFrame3dElementInput {
        id: "member".to_string(),
        node_i: 0,
        node_j: 1,
        local_y_axis: Some([0.0, 1.0, 0.0]),
        area,
        youngs_modulus,
        shear_modulus,
        torsion_constant,
        moment_of_inertia_y: inertia_y,
        moment_of_inertia_z: inertia_z,
        section_modulus_y: 1.4e-4,
        section_modulus_z: 1.1e-4,
        thermal_expansion: 11.4e-6,
        section_depth_y: 0.22,
        section_depth_z: 0.18,
        temperature_gradient_y: 0.0,
        temperature_gradient_z: 0.0,
    }
}

fn node(id: &str, x: f64, fixed: bool) -> ThermalFrame3dNodeInput {
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
        load_x: 0.0,
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
