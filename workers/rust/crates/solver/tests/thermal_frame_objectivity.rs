use kyuubiki_protocol::{
    SolveThermalFrame2dRequest, SolveThermalFrame3dRequest, ThermalFrame2dElementInput,
    ThermalFrame2dNodeInput, ThermalFrame3dDirectionalSpringInput, ThermalFrame3dElementInput,
    ThermalFrame3dNodeInput,
};
use kyuubiki_solver::{solve_thermal_frame_2d, solve_thermal_frame_3d};

const REL_TOL: f64 = 5.0e-9;
const ABS_TOL: f64 = 1.0e-11;

#[test]
fn thermal_frame_2d_preserves_coupled_response_under_rigid_rotation() {
    let baseline_request = thermal_frame_2d_request();
    let baseline =
        solve_thermal_frame_2d(&baseline_request).expect("baseline thermal frame 2d should solve");

    for angle in [std::f64::consts::FRAC_PI_6, -std::f64::consts::FRAC_PI_4] {
        let rotated_request = rotate_frame_2d(baseline_request.clone(), angle);
        let rotated = solve_thermal_frame_2d(&rotated_request)
            .expect("rotated thermal frame 2d should solve");
        let (expected_ux, expected_uy) =
            rotate_pair(baseline.nodes[1].ux, baseline.nodes[1].uy, angle);

        assert_close(rotated.nodes[1].ux, expected_ux, "rotated 2d tip ux");
        assert_close(rotated.nodes[1].uy, expected_uy, "rotated 2d tip uy");
        assert_close(
            rotated.nodes[1].rz,
            baseline.nodes[1].rz,
            "rotation-invariant 2d tip rz",
        );
        compare_frame_2d_summaries(&baseline, &rotated);
        compare_frame_2d_elements(&baseline.elements[0], &rotated.elements[0]);
    }
}

#[test]
fn thermal_frame_3d_preserves_coupled_response_under_arbitrary_rotation() {
    let baseline_request = thermal_frame_3d_request();
    let baseline =
        solve_thermal_frame_3d(&baseline_request).expect("baseline thermal frame 3d should solve");

    for (axis, angle) in [([1.0, 2.0, -1.0], 0.61), ([-0.4, 0.9, 1.3], -0.77)] {
        let rotated_request = rotate_frame_3d(baseline_request.clone(), axis, angle);
        let rotated = solve_thermal_frame_3d(&rotated_request)
            .expect("arbitrarily rotated thermal frame 3d should solve");
        assert_frame_3d_covariance(&baseline, &rotated, axis, angle);
    }
}

#[test]
fn thermal_frame_3d_non_collinear_assembly_preserves_response_under_rotation() {
    let baseline_request = thermal_frame_3d_assembly_request();
    let baseline = solve_thermal_frame_3d(&baseline_request)
        .expect("non-collinear thermal frame assembly should solve");

    for (axis, angle) in [([0.7, -1.1, 0.4], 0.52), ([-0.3, 0.6, 1.4], -0.83)] {
        let rotated_request = rotate_frame_3d(baseline_request.clone(), axis, angle);
        let rotated = solve_thermal_frame_3d(&rotated_request)
            .expect("rotated non-collinear thermal frame assembly should solve");
        assert_frame_3d_covariance(&baseline, &rotated, axis, angle);
    }
}

#[test]
fn thermal_frame_3d_branched_multi_support_assembly_is_objective() {
    let baseline_request = thermal_frame_3d_branched_request();
    let baseline = solve_thermal_frame_3d(&baseline_request)
        .expect("branched multi-support thermal frame should solve");

    for (axis, angle) in [([1.2, -0.5, 0.8], 0.47), ([-0.6, 1.0, 0.3], -0.71)] {
        let rotated_request = rotate_frame_3d(baseline_request.clone(), axis, angle);
        let rotated = solve_thermal_frame_3d(&rotated_request)
            .expect("rotated branched multi-support thermal frame should solve");
        assert_frame_3d_covariance(&baseline, &rotated, axis, angle);
    }
}

fn thermal_frame_2d_request() -> SolveThermalFrame2dRequest {
    SolveThermalFrame2dRequest {
        nodes: vec![
            ThermalFrame2dNodeInput {
                id: "root".to_string(),
                x: 0.0,
                y: 0.0,
                fix_x: true,
                fix_y: true,
                fix_rz: true,
                load_x: 0.0,
                load_y: 0.0,
                moment_z: 0.0,
                temperature_delta: 18.0,
            },
            ThermalFrame2dNodeInput {
                id: "tip".to_string(),
                x: 1.45,
                y: 0.0,
                fix_x: false,
                fix_y: false,
                fix_rz: false,
                load_x: 180.0,
                load_y: -920.0,
                moment_z: 135.0,
                temperature_delta: 47.0,
            },
        ],
        elements: vec![ThermalFrame2dElementInput {
            id: "member".to_string(),
            node_i: 0,
            node_j: 1,
            area: 0.016,
            youngs_modulus: 205.0e9,
            moment_of_inertia: 7.2e-6,
            section_modulus: 1.35e-4,
            thermal_expansion: 11.2e-6,
            section_depth: 0.24,
            temperature_gradient_y: 22.0,
        }],
    }
}

fn thermal_frame_3d_request() -> SolveThermalFrame3dRequest {
    SolveThermalFrame3dRequest {
        nodes: vec![
            thermal_frame_3d_node("root", 0.0, true, 16.0, [0.0; 3], [0.0; 3]),
            thermal_frame_3d_node(
                "tip",
                1.35,
                false,
                44.0,
                [210.0, -760.0, 330.0],
                [52.0, 86.0, -41.0],
            ),
        ],
        elements: vec![ThermalFrame3dElementInput {
            id: "member".to_string(),
            node_i: 0,
            node_j: 1,
            local_y_axis: Some([0.0, 1.0, 0.0]),
            area: 0.018,
            youngs_modulus: 208.0e9,
            shear_modulus: 80.0e9,
            torsion_constant: 4.8e-6,
            moment_of_inertia_y: 7.0e-6,
            moment_of_inertia_z: 5.3e-6,
            section_modulus_y: 1.4e-4,
            section_modulus_z: 1.1e-4,
            thermal_expansion: 11.4e-6,
            section_depth_y: 0.22,
            section_depth_z: 0.18,
            temperature_gradient_y: 21.0,
            temperature_gradient_z: 15.0,
        }],
        directional_springs: Vec::new(),
    }
}

fn thermal_frame_3d_assembly_request() -> SolveThermalFrame3dRequest {
    SolveThermalFrame3dRequest {
        nodes: vec![
            thermal_frame_3d_node_at("root", [0.0, 0.0, 0.0], true, 14.0, [0.0; 3], [0.0; 3]),
            thermal_frame_3d_node_at("elbow-a", [0.9, 0.2, 0.15], false, 27.0, [0.0; 3], [0.0; 3]),
            thermal_frame_3d_node_at(
                "elbow-b",
                [1.45, 0.95, 0.4],
                false,
                39.0,
                [0.0; 3],
                [0.0; 3],
            ),
            thermal_frame_3d_node_at(
                "tip",
                [1.7, 1.25, 1.15],
                false,
                53.0,
                [260.0, -680.0, 410.0],
                [64.0, 91.0, -48.0],
            ),
        ],
        elements: vec![
            thermal_frame_3d_element("member-a", 0, 1, [0.0, 0.0, 1.0], 13.0, 8.0),
            thermal_frame_3d_element("member-b", 1, 2, [0.2, -0.3, 1.0], 19.0, 12.0),
            thermal_frame_3d_element("member-c", 2, 3, [1.0, 0.1, -0.2], 24.0, 17.0),
        ],
        directional_springs: Vec::new(),
    }
}

fn thermal_frame_3d_branched_request() -> SolveThermalFrame3dRequest {
    SolveThermalFrame3dRequest {
        nodes: vec![
            thermal_frame_3d_node_at("support-a", [0.0, 0.0, 0.0], true, 12.0, [0.0; 3], [0.0; 3]),
            thermal_frame_3d_node_at(
                "support-b",
                [0.1, 1.0, 0.35],
                true,
                31.0,
                [0.0; 3],
                [0.0; 3],
            ),
            thermal_frame_3d_node_at(
                "junction",
                [1.05, 0.55, 0.45],
                false,
                43.0,
                [0.0; 3],
                [0.0; 3],
            ),
            thermal_frame_3d_node_at(
                "tip",
                [1.65, 0.85, 1.2],
                false,
                58.0,
                [310.0, -540.0, 370.0],
                [72.0, 55.0, -63.0],
            ),
        ],
        elements: vec![
            thermal_frame_3d_element("branch-a", 0, 2, [0.0, 0.2, 1.0], 14.0, 9.0),
            thermal_frame_3d_element("branch-b", 1, 2, [0.3, -0.1, 1.0], 20.0, 13.0),
            thermal_frame_3d_element("stem", 2, 3, [1.0, 0.0, -0.1], 27.0, 18.0),
        ],
        directional_springs: vec![ThermalFrame3dDirectionalSpringInput {
            id: "tip-directional-support".to_string(),
            node: 3,
            direction: [0.4, -0.2, 1.0],
            stiffness: 2.8e6,
        }],
    }
}

fn thermal_frame_3d_element(
    id: &str,
    node_i: usize,
    node_j: usize,
    local_y_axis: [f64; 3],
    temperature_gradient_y: f64,
    temperature_gradient_z: f64,
) -> ThermalFrame3dElementInput {
    ThermalFrame3dElementInput {
        id: id.to_string(),
        node_i,
        node_j,
        local_y_axis: Some(local_y_axis),
        area: 0.018,
        youngs_modulus: 208.0e9,
        shear_modulus: 80.0e9,
        torsion_constant: 4.8e-6,
        moment_of_inertia_y: 7.0e-6,
        moment_of_inertia_z: 5.3e-6,
        section_modulus_y: 1.4e-4,
        section_modulus_z: 1.1e-4,
        thermal_expansion: 11.4e-6,
        section_depth_y: 0.22,
        section_depth_z: 0.18,
        temperature_gradient_y,
        temperature_gradient_z,
    }
}

fn thermal_frame_3d_node(
    id: &str,
    x: f64,
    fixed: bool,
    temperature_delta: f64,
    load: [f64; 3],
    moment: [f64; 3],
) -> ThermalFrame3dNodeInput {
    thermal_frame_3d_node_at(id, [x, 0.0, 0.0], fixed, temperature_delta, load, moment)
}

fn thermal_frame_3d_node_at(
    id: &str,
    position: [f64; 3],
    fixed: bool,
    temperature_delta: f64,
    load: [f64; 3],
    moment: [f64; 3],
) -> ThermalFrame3dNodeInput {
    ThermalFrame3dNodeInput {
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
        temperature_delta,
    }
}

fn rotate_frame_2d(
    mut request: SolveThermalFrame2dRequest,
    angle: f64,
) -> SolveThermalFrame2dRequest {
    for node in &mut request.nodes {
        (node.x, node.y) = rotate_pair(node.x, node.y, angle);
        (node.load_x, node.load_y) = rotate_pair(node.load_x, node.load_y, angle);
    }
    request
}

fn rotate_frame_3d(
    mut request: SolveThermalFrame3dRequest,
    axis: [f64; 3],
    angle: f64,
) -> SolveThermalFrame3dRequest {
    for node in &mut request.nodes {
        [node.x, node.y, node.z] = rotate_vector_3d([node.x, node.y, node.z], axis, angle);
        [node.load_x, node.load_y, node.load_z] =
            rotate_vector_3d([node.load_x, node.load_y, node.load_z], axis, angle);
        [node.moment_x, node.moment_y, node.moment_z] =
            rotate_vector_3d([node.moment_x, node.moment_y, node.moment_z], axis, angle);
    }
    for element in &mut request.elements {
        element.local_y_axis = element
            .local_y_axis
            .map(|local_y| rotate_vector_3d(local_y, axis, angle));
    }
    for spring in &mut request.directional_springs {
        spring.direction = rotate_vector_3d(spring.direction, axis, angle);
    }
    request
}

fn rotate_vector_3d(vector: [f64; 3], axis: [f64; 3], angle: f64) -> [f64; 3] {
    let axis_norm = (axis[0] * axis[0] + axis[1] * axis[1] + axis[2] * axis[2]).sqrt();
    let unit = [
        axis[0] / axis_norm,
        axis[1] / axis_norm,
        axis[2] / axis_norm,
    ];
    let cosine = angle.cos();
    let sine = angle.sin();
    let dot = unit[0] * vector[0] + unit[1] * vector[1] + unit[2] * vector[2];
    let cross = [
        unit[1] * vector[2] - unit[2] * vector[1],
        unit[2] * vector[0] - unit[0] * vector[2],
        unit[0] * vector[1] - unit[1] * vector[0],
    ];
    std::array::from_fn(|index| {
        vector[index] * cosine + cross[index] * sine + unit[index] * dot * (1.0 - cosine)
    })
}

fn compare_frame_2d_summaries(
    baseline: &kyuubiki_protocol::SolveThermalFrame2dResult,
    rotated: &kyuubiki_protocol::SolveThermalFrame2dResult,
) {
    for (label, actual, expected) in [
        (
            "2d max displacement",
            rotated.max_displacement,
            baseline.max_displacement,
        ),
        (
            "2d max rotation",
            rotated.max_rotation,
            baseline.max_rotation,
        ),
        ("2d max moment", rotated.max_moment, baseline.max_moment),
        ("2d max stress", rotated.max_stress, baseline.max_stress),
        (
            "2d max axial force",
            rotated.max_axial_force,
            baseline.max_axial_force,
        ),
        (
            "2d max temperature",
            rotated.max_temperature_delta,
            baseline.max_temperature_delta,
        ),
        (
            "2d max gradient",
            rotated.max_temperature_gradient,
            baseline.max_temperature_gradient,
        ),
        (
            "2d total energy",
            rotated.total_strain_energy,
            baseline.total_strain_energy,
        ),
    ] {
        assert_close(actual, expected, label);
    }
}

fn compare_frame_2d_elements(
    baseline: &kyuubiki_protocol::ThermalFrame2dElementResult,
    rotated: &kyuubiki_protocol::ThermalFrame2dElementResult,
) {
    for (label, actual, expected) in [
        ("2d length", rotated.length, baseline.length),
        (
            "2d average temperature",
            rotated.average_temperature_delta,
            baseline.average_temperature_delta,
        ),
        (
            "2d thermal strain",
            rotated.thermal_strain,
            baseline.thermal_strain,
        ),
        (
            "2d mechanical strain",
            rotated.mechanical_strain,
            baseline.mechanical_strain,
        ),
        (
            "2d total strain",
            rotated.total_strain,
            baseline.total_strain,
        ),
        (
            "2d gradient",
            rotated.temperature_gradient_y,
            baseline.temperature_gradient_y,
        ),
        (
            "2d thermal curvature",
            rotated.thermal_curvature,
            baseline.thermal_curvature,
        ),
        (
            "2d axial force i",
            rotated.axial_force_i,
            baseline.axial_force_i,
        ),
        (
            "2d shear force i",
            rotated.shear_force_i,
            baseline.shear_force_i,
        ),
        ("2d moment i", rotated.moment_i, baseline.moment_i),
        (
            "2d axial force j",
            rotated.axial_force_j,
            baseline.axial_force_j,
        ),
        (
            "2d shear force j",
            rotated.shear_force_j,
            baseline.shear_force_j,
        ),
        ("2d moment j", rotated.moment_j, baseline.moment_j),
        (
            "2d axial stress",
            rotated.axial_stress,
            baseline.axial_stress,
        ),
        (
            "2d bending stress",
            rotated.max_bending_stress,
            baseline.max_bending_stress,
        ),
        (
            "2d combined stress",
            rotated.max_combined_stress,
            baseline.max_combined_stress,
        ),
        (
            "2d strain energy",
            rotated.strain_energy,
            baseline.strain_energy,
        ),
    ] {
        assert_close(actual, expected, label);
    }
}

fn compare_frame_3d_summaries(
    baseline: &kyuubiki_protocol::SolveThermalFrame3dResult,
    rotated: &kyuubiki_protocol::SolveThermalFrame3dResult,
) {
    for (label, actual, expected) in [
        (
            "3d max displacement",
            rotated.max_displacement,
            baseline.max_displacement,
        ),
        (
            "3d max rotation",
            rotated.max_rotation,
            baseline.max_rotation,
        ),
        ("3d max moment", rotated.max_moment, baseline.max_moment),
        ("3d max stress", rotated.max_stress, baseline.max_stress),
        (
            "3d max axial force",
            rotated.max_axial_force,
            baseline.max_axial_force,
        ),
        (
            "3d max temperature",
            rotated.max_temperature_delta,
            baseline.max_temperature_delta,
        ),
        (
            "3d max gradient",
            rotated.max_temperature_gradient,
            baseline.max_temperature_gradient,
        ),
        (
            "3d total energy",
            rotated.total_strain_energy,
            baseline.total_strain_energy,
        ),
    ] {
        assert_close(actual, expected, label);
    }
}

fn assert_frame_3d_covariance(
    baseline: &kyuubiki_protocol::SolveThermalFrame3dResult,
    rotated: &kyuubiki_protocol::SolveThermalFrame3dResult,
    axis: [f64; 3],
    angle: f64,
) {
    assert_eq!(baseline.nodes.len(), rotated.nodes.len());
    assert_eq!(baseline.elements.len(), rotated.elements.len());
    assert_eq!(
        baseline.directional_springs.len(),
        rotated.directional_springs.len()
    );
    for (index, (baseline_node, rotated_node)) in
        baseline.nodes.iter().zip(&rotated.nodes).enumerate()
    {
        assert_vector_close(
            [rotated_node.ux, rotated_node.uy, rotated_node.uz],
            rotate_vector_3d(
                [baseline_node.ux, baseline_node.uy, baseline_node.uz],
                axis,
                angle,
            ),
            &format!("rotated 3d node {index} displacement"),
        );
        assert_vector_close(
            [rotated_node.rx, rotated_node.ry, rotated_node.rz],
            rotate_vector_3d(
                [baseline_node.rx, baseline_node.ry, baseline_node.rz],
                axis,
                angle,
            ),
            &format!("rotated 3d node {index} rotation"),
        );
        assert_close(
            rotated_node.temperature_delta,
            baseline_node.temperature_delta,
            &format!("rotated 3d node {index} temperature"),
        );
    }
    compare_frame_3d_summaries(baseline, rotated);
    for (baseline_element, rotated_element) in baseline.elements.iter().zip(&rotated.elements) {
        compare_frame_3d_elements(baseline_element, rotated_element);
    }
    for (index, (baseline_spring, rotated_spring)) in baseline
        .directional_springs
        .iter()
        .zip(&rotated.directional_springs)
        .enumerate()
    {
        assert_vector_close(
            rotated_spring.direction,
            rotate_vector_3d(baseline_spring.direction, axis, angle),
            &format!("rotated directional spring {index} direction"),
        );
        for (label, actual, expected) in [
            (
                "displacement",
                rotated_spring.displacement,
                baseline_spring.displacement,
            ),
            (
                "reaction force",
                rotated_spring.reaction_force,
                baseline_spring.reaction_force,
            ),
            (
                "stiffness",
                rotated_spring.stiffness,
                baseline_spring.stiffness,
            ),
            (
                "strain energy",
                rotated_spring.strain_energy,
                baseline_spring.strain_energy,
            ),
        ] {
            assert_close(
                actual,
                expected,
                &format!("rotated directional spring {index} {label}"),
            );
        }
    }
}

fn compare_frame_3d_elements(
    baseline: &kyuubiki_protocol::ThermalFrame3dElementResult,
    rotated: &kyuubiki_protocol::ThermalFrame3dElementResult,
) {
    let baseline_values = frame_3d_element_values(baseline);
    let rotated_values = frame_3d_element_values(rotated);
    for (index, (actual, expected)) in rotated_values.into_iter().zip(baseline_values).enumerate() {
        assert_close(actual, expected, &format!("3d element invariant {index}"));
    }
}

fn frame_3d_element_values(element: &kyuubiki_protocol::ThermalFrame3dElementResult) -> [f64; 24] {
    [
        element.length,
        element.average_temperature_delta,
        element.thermal_strain,
        element.mechanical_strain,
        element.total_strain,
        element.temperature_gradient_y,
        element.temperature_gradient_z,
        element.thermal_curvature_y,
        element.thermal_curvature_z,
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
        element.strain_energy,
    ]
}

fn rotate_pair(x: f64, y: f64, angle: f64) -> (f64, f64) {
    let sine = angle.sin();
    let cosine = angle.cos();
    (cosine * x - sine * y, sine * x + cosine * y)
}

fn assert_close(actual: f64, expected: f64, label: &str) {
    let tolerance = ABS_TOL.max(REL_TOL * expected.abs().max(1.0));
    assert!(
        (actual - expected).abs() <= tolerance,
        "{label}: expected {actual} to be close to {expected} within {tolerance}",
    );
}

fn assert_vector_close(actual: [f64; 3], expected: [f64; 3], label: &str) {
    for index in 0..3 {
        assert_close(actual[index], expected[index], &format!("{label}[{index}]"));
    }
}
