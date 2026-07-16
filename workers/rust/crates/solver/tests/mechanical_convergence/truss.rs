use super::common::assert_close;
use kyuubiki_protocol::{
    SolveTruss2dRequest, SolveTruss3dRequest, Truss3dElementInput, Truss3dNodeInput,
    TrussElementInput, TrussNodeInput,
};
use kyuubiki_solver::{solve_truss_2d, solve_truss_3d};

#[test]
fn symmetric_truss_geometry_and_load_perturbations_match_closed_form() {
    let area: f64 = 0.014;
    let youngs_modulus: f64 = 72.0e9;

    for (half_span, height, load) in [(0.45, 0.65, 900.0), (0.7, 0.95, 1_300.0)] {
        let request = symmetric_truss_request(half_span, height, -load, area, youngs_modulus);
        let result = solve_truss_2d(&request).expect("symmetric truss should solve");
        let expected = symmetric_truss_closed_form(half_span, height, -load, area, youngs_modulus);

        assert_close(
            result.nodes[2].ux,
            0.0,
            "truss apex horizontal displacement",
        );
        assert_close(
            result.nodes[2].uy,
            expected.apex_uy,
            "truss apex vertical displacement",
        );
        assert_close(
            result.max_displacement,
            expected.apex_uy.abs(),
            "truss max displacement",
        );
        assert_close(result.max_stress, expected.stress.abs(), "truss max stress");
        assert_close(
            result.total_strain_energy,
            expected.total_energy,
            "truss strain energy",
        );

        for element in &result.elements {
            assert_close(
                element.length,
                expected.member_length,
                "truss member length",
            );
            assert_close(
                element.axial_force,
                expected.axial_force,
                "truss axial force",
            );
            assert_close(element.stress, expected.stress, "truss stress");
            assert_close(element.strain, expected.strain, "truss strain");
        }
    }
}

#[test]
fn tripod_truss_3d_geometry_and_load_perturbations_match_closed_form() {
    let area: f64 = 0.013;
    let youngs_modulus: f64 = 71.0e9;

    for (radius, height, load) in [(0.55, 0.82, 1_250.0), (0.72, 1.05, 1_700.0)] {
        let request = tripod_truss_request(radius, height, -load, area, youngs_modulus);
        let result = solve_truss_3d(&request).expect("symmetric 3D tripod truss should solve");
        let expected = tripod_truss_closed_form(radius, height, -load, area, youngs_modulus);

        let apex = &result.nodes[3];
        assert_close(apex.ux, 0.0, "tripod apex x displacement");
        assert_close(apex.uy, 0.0, "tripod apex y displacement");
        assert_close(apex.uz, expected.apex_uz, "tripod apex z displacement");
        assert_close(
            result.max_displacement,
            expected.apex_uz.abs(),
            "tripod max displacement",
        );
        assert_close(
            result.max_stress,
            expected.stress.abs(),
            "tripod max stress",
        );
        assert_close(
            result.total_strain_energy,
            expected.total_energy,
            "tripod strain energy",
        );

        for element in &result.elements {
            assert_close(
                element.length,
                expected.member_length,
                "tripod member length",
            );
            assert_close(
                element.axial_force,
                expected.axial_force,
                "tripod axial force",
            );
            assert_close(element.stress, expected.stress, "tripod stress");
            assert_close(element.strain, expected.strain, "tripod strain");
        }
    }
}

struct SymmetricTrussExpected {
    member_length: f64,
    axial_force: f64,
    apex_uy: f64,
    stress: f64,
    strain: f64,
    total_energy: f64,
}

struct TripodTrussExpected {
    member_length: f64,
    axial_force: f64,
    apex_uz: f64,
    stress: f64,
    strain: f64,
    total_energy: f64,
}

fn symmetric_truss_closed_form(
    half_span: f64,
    height: f64,
    load_y: f64,
    area: f64,
    youngs_modulus: f64,
) -> SymmetricTrussExpected {
    let member_length = (half_span * half_span + height * height).sqrt();
    let sin_theta = height / member_length;
    let axial_force = load_y / (2.0 * sin_theta);
    let apex_uy = load_y * member_length / (2.0 * youngs_modulus * area * sin_theta * sin_theta);
    let stress = axial_force / area;
    let strain = stress / youngs_modulus;
    let total_energy = 2.0 * 0.5 * stress * strain * area * member_length;

    SymmetricTrussExpected {
        member_length,
        axial_force,
        apex_uy,
        stress,
        strain,
        total_energy,
    }
}

fn tripod_truss_closed_form(
    radius: f64,
    height: f64,
    load_z: f64,
    area: f64,
    youngs_modulus: f64,
) -> TripodTrussExpected {
    let member_length = (radius * radius + height * height).sqrt();
    let vertical_direction = height / member_length;
    let axial_force = load_z / (3.0 * vertical_direction);
    let apex_uz =
        load_z * member_length / (3.0 * youngs_modulus * area * vertical_direction.powi(2));
    let stress = axial_force / area;
    let strain = stress / youngs_modulus;
    let total_energy = 3.0 * 0.5 * stress * strain * area * member_length;

    TripodTrussExpected {
        member_length,
        axial_force,
        apex_uz,
        stress,
        strain,
        total_energy,
    }
}

fn symmetric_truss_request(
    half_span: f64,
    height: f64,
    load_y: f64,
    area: f64,
    youngs_modulus: f64,
) -> SolveTruss2dRequest {
    SolveTruss2dRequest {
        nodes: vec![
            truss_node("left", -half_span, 0.0, true, true, 0.0),
            truss_node("right", half_span, 0.0, true, true, 0.0),
            truss_node("apex", 0.0, height, false, false, load_y),
        ],
        elements: vec![
            truss_element("left-web", 0, 2, area, youngs_modulus),
            truss_element("right-web", 1, 2, area, youngs_modulus),
        ],
    }
}

fn tripod_truss_request(
    radius: f64,
    height: f64,
    load_z: f64,
    area: f64,
    youngs_modulus: f64,
) -> SolveTruss3dRequest {
    let root_three_over_two = 3.0_f64.sqrt() * 0.5;

    SolveTruss3dRequest {
        nodes: vec![
            truss_3d_node("base-a", radius, 0.0, 0.0, true, 0.0),
            truss_3d_node(
                "base-b",
                -0.5 * radius,
                root_three_over_two * radius,
                0.0,
                true,
                0.0,
            ),
            truss_3d_node(
                "base-c",
                -0.5 * radius,
                -root_three_over_two * radius,
                0.0,
                true,
                0.0,
            ),
            truss_3d_node("apex", 0.0, 0.0, height, false, load_z),
        ],
        elements: vec![
            truss_3d_element("leg-a", 0, 3, area, youngs_modulus),
            truss_3d_element("leg-b", 1, 3, area, youngs_modulus),
            truss_3d_element("leg-c", 2, 3, area, youngs_modulus),
        ],
    }
}

fn truss_node(id: &str, x: f64, y: f64, fix_x: bool, fix_y: bool, load_y: f64) -> TrussNodeInput {
    TrussNodeInput {
        id: id.to_string(),
        x,
        y,
        fix_x,
        fix_y,
        load_x: 0.0,
        load_y,
    }
}

fn truss_element(
    id: &str,
    node_i: usize,
    node_j: usize,
    area: f64,
    youngs_modulus: f64,
) -> TrussElementInput {
    TrussElementInput {
        id: id.to_string(),
        node_i,
        node_j,
        area,
        youngs_modulus,
    }
}

fn truss_3d_node(id: &str, x: f64, y: f64, z: f64, fixed: bool, load_z: f64) -> Truss3dNodeInput {
    Truss3dNodeInput {
        id: id.to_string(),
        x,
        y,
        z,
        fix_x: fixed,
        fix_y: fixed,
        fix_z: fixed,
        load_x: 0.0,
        load_y: 0.0,
        load_z,
    }
}

fn truss_3d_element(
    id: &str,
    node_i: usize,
    node_j: usize,
    area: f64,
    youngs_modulus: f64,
) -> Truss3dElementInput {
    Truss3dElementInput {
        id: id.to_string(),
        node_i,
        node_j,
        area,
        youngs_modulus,
    }
}
