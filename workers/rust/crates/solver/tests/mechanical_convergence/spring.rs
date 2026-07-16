use super::common::assert_close;
use kyuubiki_protocol::{
    SolveSpring1dRequest, SolveSpring2dRequest, SolveSpring3dRequest, Spring1dElementInput,
    Spring1dNodeInput, Spring2dElementInput, Spring2dNodeInput, Spring3dElementInput,
    Spring3dNodeInput,
};
use kyuubiki_solver::{solve_spring_1d, solve_spring_2d, solve_spring_3d};

#[test]
fn spring_1d_series_chain_matches_equivalent_stiffness_under_perturbations() {
    for case in [
        Spring1dCase {
            load: 1200.0,
            stiffnesses: [35_000.0, 20_000.0, 50_000.0],
        },
        Spring1dCase {
            load: 2400.0,
            stiffnesses: [35_000.0, 20_000.0, 50_000.0],
        },
        Spring1dCase {
            load: 1200.0,
            stiffnesses: [70_000.0, 40_000.0, 100_000.0],
        },
    ] {
        let result =
            solve_spring_1d(&spring_1d_series_request(case)).expect("series spring should solve");
        let expected = spring_1d_closed_form(case);

        assert_close(result.nodes[0].ux, 0.0, "spring 1d fixed ux");
        for (index, expected_node) in expected.node_displacements.iter().enumerate() {
            assert_close(
                result.nodes[index].ux,
                *expected_node,
                "spring 1d node displacement",
            );
        }
        for (index, expected_extension) in expected.element_extensions.iter().enumerate() {
            let element = &result.elements[index];
            assert_close(
                element.extension,
                *expected_extension,
                "spring 1d extension",
            );
            assert_close(element.force, case.load, "spring 1d force");
            assert_close(
                element.strain_energy,
                0.5 * case.load * expected_extension,
                "spring 1d element energy",
            );
        }
        assert_close(
            result.max_displacement,
            expected.tip_displacement,
            "spring 1d max displacement",
        );
        assert_close(result.max_force, case.load.abs(), "spring 1d max force");
        assert_close(
            result.total_strain_energy,
            expected.total_energy,
            "spring 1d total energy",
        );
    }
}

#[test]
fn spring_2d_orthogonal_vector_stiffness_matches_closed_form_under_perturbations() {
    for case in [
        Spring2dCase {
            load_x: 800.0,
            load_y: -600.0,
            stiffness_x: 40_000.0,
            stiffness_y: 30_000.0,
        },
        Spring2dCase {
            load_x: 1600.0,
            load_y: -1200.0,
            stiffness_x: 40_000.0,
            stiffness_y: 30_000.0,
        },
        Spring2dCase {
            load_x: 800.0,
            load_y: -600.0,
            stiffness_x: 80_000.0,
            stiffness_y: 60_000.0,
        },
    ] {
        let result = solve_spring_2d(&spring_2d_orthogonal_request(case))
            .expect("2d orthogonal spring should solve");
        let expected = spring_2d_closed_form(case);
        let free = &result.nodes[0];

        assert_close(free.ux, expected.ux, "spring 2d free ux");
        assert_close(free.uy, expected.uy, "spring 2d free uy");
        for fixed in &result.nodes[1..] {
            assert_close(fixed.ux, 0.0, "spring 2d fixed ux");
            assert_close(fixed.uy, 0.0, "spring 2d fixed uy");
        }
        assert_close(result.elements[0].force, -case.load_x, "spring 2d x force");
        assert_close(result.elements[1].force, -case.load_y, "spring 2d y force");
        assert_close(
            result.elements[0].extension,
            -expected.ux,
            "spring 2d x extension",
        );
        assert_close(
            result.elements[1].extension,
            -expected.uy,
            "spring 2d y extension",
        );
        assert_close(
            result.total_strain_energy,
            expected.total_energy,
            "spring 2d total energy",
        );
        assert_close(
            result.max_displacement,
            (expected.ux.powi(2) + expected.uy.powi(2)).sqrt(),
            "spring 2d max displacement",
        );
    }
}

#[test]
fn spring_3d_orthogonal_vector_stiffness_matches_closed_form_under_perturbations() {
    for case in [
        Spring3dCase {
            load_x: 450.0,
            load_y: -300.0,
            load_z: 900.0,
            stiffness_x: 45_000.0,
            stiffness_y: 30_000.0,
            stiffness_z: 60_000.0,
        },
        Spring3dCase {
            load_x: 900.0,
            load_y: -600.0,
            load_z: 1800.0,
            stiffness_x: 45_000.0,
            stiffness_y: 30_000.0,
            stiffness_z: 60_000.0,
        },
        Spring3dCase {
            load_x: 450.0,
            load_y: -300.0,
            load_z: 900.0,
            stiffness_x: 90_000.0,
            stiffness_y: 60_000.0,
            stiffness_z: 120_000.0,
        },
    ] {
        let result = solve_spring_3d(&spring_3d_orthogonal_request(case))
            .expect("3d orthogonal spring should solve");
        let expected = spring_3d_closed_form(case);
        let free = &result.nodes[0];

        assert_close(free.ux, expected.ux, "spring 3d free ux");
        assert_close(free.uy, expected.uy, "spring 3d free uy");
        assert_close(free.uz, expected.uz, "spring 3d free uz");
        for fixed in &result.nodes[1..] {
            assert_close(fixed.ux, 0.0, "spring 3d fixed ux");
            assert_close(fixed.uy, 0.0, "spring 3d fixed uy");
            assert_close(fixed.uz, 0.0, "spring 3d fixed uz");
        }
        assert_close(result.elements[0].force, -case.load_x, "spring 3d x force");
        assert_close(result.elements[1].force, -case.load_y, "spring 3d y force");
        assert_close(result.elements[2].force, -case.load_z, "spring 3d z force");
        assert_close(
            result.elements[0].extension,
            -expected.ux,
            "spring 3d x extension",
        );
        assert_close(
            result.elements[1].extension,
            -expected.uy,
            "spring 3d y extension",
        );
        assert_close(
            result.elements[2].extension,
            -expected.uz,
            "spring 3d z extension",
        );
        assert_close(
            result.total_strain_energy,
            expected.total_energy,
            "spring 3d total energy",
        );
        assert_close(
            result.max_displacement,
            (expected.ux.powi(2) + expected.uy.powi(2) + expected.uz.powi(2)).sqrt(),
            "spring 3d max displacement",
        );
    }
}

#[derive(Clone, Copy)]
struct Spring1dCase {
    load: f64,
    stiffnesses: [f64; 3],
}

struct Spring1dExpected {
    node_displacements: Vec<f64>,
    element_extensions: Vec<f64>,
    tip_displacement: f64,
    total_energy: f64,
}

#[derive(Clone, Copy)]
struct Spring2dCase {
    load_x: f64,
    load_y: f64,
    stiffness_x: f64,
    stiffness_y: f64,
}

struct Spring2dExpected {
    ux: f64,
    uy: f64,
    total_energy: f64,
}

#[derive(Clone, Copy)]
struct Spring3dCase {
    load_x: f64,
    load_y: f64,
    load_z: f64,
    stiffness_x: f64,
    stiffness_y: f64,
    stiffness_z: f64,
}

struct Spring3dExpected {
    ux: f64,
    uy: f64,
    uz: f64,
    total_energy: f64,
}

fn spring_1d_closed_form(case: Spring1dCase) -> Spring1dExpected {
    let mut node_displacements = vec![0.0];
    let mut element_extensions = Vec::new();
    let mut displacement = 0.0;
    let mut total_energy = 0.0;

    for stiffness in case.stiffnesses {
        let extension = case.load / stiffness;
        displacement += extension;
        total_energy += 0.5 * case.load * extension;
        element_extensions.push(extension);
        node_displacements.push(displacement);
    }

    Spring1dExpected {
        node_displacements,
        element_extensions,
        tip_displacement: displacement,
        total_energy,
    }
}

fn spring_2d_closed_form(case: Spring2dCase) -> Spring2dExpected {
    let ux = case.load_x / case.stiffness_x;
    let uy = case.load_y / case.stiffness_y;
    Spring2dExpected {
        ux,
        uy,
        total_energy: 0.5 * (case.load_x * ux + case.load_y * uy),
    }
}

fn spring_3d_closed_form(case: Spring3dCase) -> Spring3dExpected {
    let ux = case.load_x / case.stiffness_x;
    let uy = case.load_y / case.stiffness_y;
    let uz = case.load_z / case.stiffness_z;
    Spring3dExpected {
        ux,
        uy,
        uz,
        total_energy: 0.5 * (case.load_x * ux + case.load_y * uy + case.load_z * uz),
    }
}

fn spring_1d_series_request(case: Spring1dCase) -> SolveSpring1dRequest {
    let nodes = (0..=case.stiffnesses.len())
        .map(|index| Spring1dNodeInput {
            id: format!("n{index}"),
            x: index as f64,
            fix_x: index == 0,
            load_x: if index == case.stiffnesses.len() {
                case.load
            } else {
                0.0
            },
        })
        .collect::<Vec<_>>();
    let elements = case
        .stiffnesses
        .iter()
        .enumerate()
        .map(|(index, stiffness)| Spring1dElementInput {
            id: format!("k{index}"),
            node_i: index,
            node_j: index + 1,
            stiffness: *stiffness,
        })
        .collect();

    SolveSpring1dRequest { nodes, elements }
}

fn spring_2d_orthogonal_request(case: Spring2dCase) -> SolveSpring2dRequest {
    SolveSpring2dRequest {
        nodes: vec![
            spring_2d_node("free", 0.0, 0.0, false, false, case.load_x, case.load_y),
            spring_2d_node("fixed-x", 1.0, 0.0, true, true, 0.0, 0.0),
            spring_2d_node("fixed-y", 0.0, 1.0, true, true, 0.0, 0.0),
        ],
        elements: vec![
            Spring2dElementInput {
                id: "kx".to_string(),
                node_i: 0,
                node_j: 1,
                stiffness: case.stiffness_x,
            },
            Spring2dElementInput {
                id: "ky".to_string(),
                node_i: 0,
                node_j: 2,
                stiffness: case.stiffness_y,
            },
        ],
    }
}

fn spring_3d_orthogonal_request(case: Spring3dCase) -> SolveSpring3dRequest {
    SolveSpring3dRequest {
        nodes: vec![
            spring_3d_node(
                "free",
                0.0,
                0.0,
                0.0,
                false,
                false,
                false,
                case.load_x,
                case.load_y,
                case.load_z,
            ),
            spring_3d_node("fixed-x", 1.0, 0.0, 0.0, true, true, true, 0.0, 0.0, 0.0),
            spring_3d_node("fixed-y", 0.0, 1.0, 0.0, true, true, true, 0.0, 0.0, 0.0),
            spring_3d_node("fixed-z", 0.0, 0.0, 1.0, true, true, true, 0.0, 0.0, 0.0),
        ],
        elements: vec![
            Spring3dElementInput {
                id: "kx".to_string(),
                node_i: 0,
                node_j: 1,
                stiffness: case.stiffness_x,
            },
            Spring3dElementInput {
                id: "ky".to_string(),
                node_i: 0,
                node_j: 2,
                stiffness: case.stiffness_y,
            },
            Spring3dElementInput {
                id: "kz".to_string(),
                node_i: 0,
                node_j: 3,
                stiffness: case.stiffness_z,
            },
        ],
    }
}

fn spring_2d_node(
    id: &str,
    x: f64,
    y: f64,
    fix_x: bool,
    fix_y: bool,
    load_x: f64,
    load_y: f64,
) -> Spring2dNodeInput {
    Spring2dNodeInput {
        id: id.to_string(),
        x,
        y,
        fix_x,
        fix_y,
        load_x,
        load_y,
    }
}

#[allow(clippy::too_many_arguments)]
fn spring_3d_node(
    id: &str,
    x: f64,
    y: f64,
    z: f64,
    fix_x: bool,
    fix_y: bool,
    fix_z: bool,
    load_x: f64,
    load_y: f64,
    load_z: f64,
) -> Spring3dNodeInput {
    Spring3dNodeInput {
        id: id.to_string(),
        x,
        y,
        z,
        fix_x,
        fix_y,
        fix_z,
        load_x,
        load_y,
        load_z,
    }
}
