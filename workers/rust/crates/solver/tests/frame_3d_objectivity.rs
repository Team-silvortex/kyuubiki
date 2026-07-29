use kyuubiki_protocol::{
    Frame3dElementInput, Frame3dElementResult, Frame3dNodeInput, Frame3dNodeResult,
    SolveFrame3dRequest, SolveFrame3dResult,
};
use kyuubiki_solver::solve_frame_3d;

#[test]
fn asymmetric_frame_3d_section_is_objective_under_arbitrary_rigid_rotation() {
    let baseline_request = cantilever_request();
    let baseline =
        solve_frame_3d(&baseline_request).expect("baseline asymmetric 3D frame should solve");

    for (axis, angle) in [
        ([1.0, 2.0, 3.0], 0.73),
        ([-2.0, 0.5, 1.0], -1.11),
        ([0.0, 1.0, 0.0], std::f64::consts::FRAC_PI_2),
    ] {
        let rotated_request = rotate_request(baseline_request.clone(), axis, angle);
        let rotated =
            solve_frame_3d(&rotated_request).expect("rotated asymmetric 3D frame should solve");
        assert_objective(&baseline, &rotated, axis, angle);
    }
}

fn cantilever_request() -> SolveFrame3dRequest {
    SolveFrame3dRequest {
        nodes: vec![
            node("root", [0.0, 0.0, 0.0], true, [0.0; 3], [0.0; 3]),
            node(
                "tip",
                [2.3, 0.0, 0.0],
                false,
                [300.0, -1_000.0, 650.0],
                [120.0, -90.0, 75.0],
            ),
        ],
        elements: vec![Frame3dElementInput {
            id: "asymmetric".to_string(),
            node_i: 0,
            node_j: 1,
            local_y_axis: Some([0.0, 1.0, 0.0]),
            area: 0.015,
            youngs_modulus: 205.0e9,
            shear_modulus: 79.0e9,
            torsion_constant: 3.0e-6,
            moment_of_inertia_y: 4.0e-6,
            moment_of_inertia_z: 9.0e-6,
            section_modulus_y: 1.1e-4,
            section_modulus_z: 1.8e-4,
        }],
    }
}

fn node(
    id: &str,
    position: [f64; 3],
    fixed: bool,
    load: [f64; 3],
    moment: [f64; 3],
) -> Frame3dNodeInput {
    Frame3dNodeInput {
        id: id.to_string(),
        x: position[0],
        y: position[1],
        z: position[2],
        fix_x: fixed,
        fix_y: fixed,
        fix_z: fixed,
        fix_rx: fixed,
        fix_ry: fixed,
        fix_rz: fixed,
        load_x: load[0],
        load_y: load[1],
        load_z: load[2],
        moment_x: moment[0],
        moment_y: moment[1],
        moment_z: moment[2],
    }
}

fn rotate_request(
    mut request: SolveFrame3dRequest,
    axis: [f64; 3],
    angle: f64,
) -> SolveFrame3dRequest {
    for node in &mut request.nodes {
        let position = rotate([node.x, node.y, node.z], axis, angle);
        let load = rotate([node.load_x, node.load_y, node.load_z], axis, angle);
        let moment = rotate([node.moment_x, node.moment_y, node.moment_z], axis, angle);
        [node.x, node.y, node.z] = position;
        [node.load_x, node.load_y, node.load_z] = load;
        [node.moment_x, node.moment_y, node.moment_z] = moment;
    }
    for element in &mut request.elements {
        element.local_y_axis = element.local_y_axis.map(|value| rotate(value, axis, angle));
    }
    request
}

fn assert_objective(
    baseline: &SolveFrame3dResult,
    rotated: &SolveFrame3dResult,
    axis: [f64; 3],
    angle: f64,
) {
    assert_close(
        rotated.max_displacement,
        baseline.max_displacement,
        "max displacement",
    );
    assert_close(rotated.max_rotation, baseline.max_rotation, "max rotation");
    assert_close(rotated.max_moment, baseline.max_moment, "max moment");
    assert_close(rotated.max_stress, baseline.max_stress, "max stress");
    assert_close(
        rotated.total_strain_energy,
        baseline.total_strain_energy,
        "total strain energy",
    );

    for (baseline_node, rotated_node) in baseline.nodes.iter().zip(&rotated.nodes) {
        assert_rotated_node(baseline_node, rotated_node, axis, angle);
    }
    for (baseline_element, rotated_element) in baseline.elements.iter().zip(&rotated.elements) {
        assert_element_close(baseline_element, rotated_element);
    }
}

fn assert_rotated_node(
    baseline: &Frame3dNodeResult,
    rotated: &Frame3dNodeResult,
    axis: [f64; 3],
    angle: f64,
) {
    let expected_displacement = rotate([baseline.ux, baseline.uy, baseline.uz], axis, angle);
    let expected_rotation = rotate([baseline.rx, baseline.ry, baseline.rz], axis, angle);
    for component in 0..3 {
        assert_close(
            [rotated.ux, rotated.uy, rotated.uz][component],
            expected_displacement[component],
            "displacement covariance",
        );
        assert_close(
            [rotated.rx, rotated.ry, rotated.rz][component],
            expected_rotation[component],
            "rotation covariance",
        );
    }
    assert_close(
        rotated.displacement_magnitude,
        baseline.displacement_magnitude,
        "displacement magnitude",
    );
    assert_close(
        rotated.rotation_magnitude,
        baseline.rotation_magnitude,
        "rotation magnitude",
    );
}

fn assert_element_close(baseline: &Frame3dElementResult, rotated: &Frame3dElementResult) {
    let baseline_values = element_values(baseline);
    let rotated_values = element_values(rotated);
    for (left, right) in baseline_values.into_iter().zip(rotated_values) {
        assert_close(right, left, "local element response");
    }
}

fn element_values(element: &Frame3dElementResult) -> [f64; 20] {
    [
        element.length,
        element.axial_force_i,
        element.shear_force_y_i,
        element.shear_force_z_i,
        element.torsion_i,
        element.moment_y_i,
        element.moment_z_i,
        element.axial_force_j,
        element.shear_force_y_j,
        element.shear_force_z_j,
        element.torsion_j,
        element.moment_y_j,
        element.moment_z_j,
        element.axial_stress,
        element.max_bending_stress,
        element.max_combined_stress,
        element.strain_energy,
        element.moment_y_i.abs().max(element.moment_y_j.abs()),
        element.moment_z_i.abs().max(element.moment_z_j.abs()),
        element.torsion_i.abs().max(element.torsion_j.abs()),
    ]
}

fn rotate(vector: [f64; 3], axis: [f64; 3], angle: f64) -> [f64; 3] {
    let norm = dot(axis, axis).sqrt();
    let unit = [axis[0] / norm, axis[1] / norm, axis[2] / norm];
    let cosine = angle.cos();
    let sine = angle.sin();
    let cross = [
        unit[1] * vector[2] - unit[2] * vector[1],
        unit[2] * vector[0] - unit[0] * vector[2],
        unit[0] * vector[1] - unit[1] * vector[0],
    ];
    let projection = dot(unit, vector) * (1.0 - cosine);
    [
        vector[0] * cosine + cross[0] * sine + unit[0] * projection,
        vector[1] * cosine + cross[1] * sine + unit[1] * projection,
        vector[2] * cosine + cross[2] * sine + unit[2] * projection,
    ]
}

fn dot(left: [f64; 3], right: [f64; 3]) -> f64 {
    left[0] * right[0] + left[1] * right[1] + left[2] * right[2]
}

fn assert_close(actual: f64, expected: f64, label: &str) {
    let scale = expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= 1.0e-8 * scale,
        "{label}: expected {actual} to be close to {expected}",
    );
}
