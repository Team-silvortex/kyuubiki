use super::common::assert_close;
use kyuubiki_protocol::{Frame3dElementInput, Frame3dNodeInput, SolveFrame3dRequest};
use kyuubiki_solver::solve_frame_3d;

#[test]
fn frame_3d_cantilever_refinement_matches_closed_form() {
    for element_count in [1_usize, 2, 4, 8, 16] {
        let case = Frame3dCase {
            length: 2.0,
            load_y: -1000.0,
            youngs_modulus: 210.0e9,
            moment_of_inertia_z: 8.0e-6,
            section_modulus_z: 1.6e-4,
        };
        let result = solve_frame_3d(&frame_3d_cantilever_request(case, element_count))
            .expect("refined 3d frame cantilever should solve");
        let expected = frame_3d_closed_form(case);
        let fixed = &result.nodes[0];
        let tip = result.nodes.last().expect("tip node should exist");
        let root = &result.elements[0];

        assert_close(fixed.ux, 0.0, "3d frame fixed ux");
        assert_close(fixed.uy, 0.0, "3d frame fixed uy");
        assert_close(fixed.uz, 0.0, "3d frame fixed uz");
        assert_close(fixed.rx, 0.0, "3d frame fixed rx");
        assert_close(fixed.ry, 0.0, "3d frame fixed ry");
        assert_close(fixed.rz, 0.0, "3d frame fixed rz");
        assert_close(tip.ux, 0.0, "3d frame tip ux");
        assert_close(tip.uy, expected.tip_uy, "3d frame tip uy");
        assert_close(tip.uz, 0.0, "3d frame tip uz");
        assert_close(tip.rx, 0.0, "3d frame tip rx");
        assert_close(tip.ry, 0.0, "3d frame tip ry");
        assert_close(tip.rz, expected.tip_rz, "3d frame tip rz");
        assert_close(
            result.max_displacement,
            expected.tip_uy.abs(),
            "3d frame max displacement",
        );
        assert_close(
            result.max_rotation,
            expected.tip_rz.abs(),
            "3d frame max rotation",
        );
        assert_close(
            result.max_moment,
            expected.root_moment,
            "3d frame max moment",
        );
        assert_close(result.max_stress, expected.stress, "3d frame max stress");
        assert_close(
            result.total_strain_energy,
            expected.energy,
            "3d frame total energy",
        );
        assert_close(
            root.length,
            case.length / element_count as f64,
            "3d frame root element length",
        );
        assert_frame_force_close(
            root.shear_force_y_i,
            case.load_y.abs(),
            "3d frame root shear y i",
        );
        assert_frame_force_close(
            root.moment_z_i,
            expected.root_moment,
            "3d frame root moment z",
        );
        assert_close(root.axial_stress, 0.0, "3d frame axial stress");
        assert_close(
            root.max_bending_stress,
            expected.stress,
            "3d frame bending stress",
        );
    }
}

#[test]
fn frame_3d_cantilever_perturbations_preserve_expected_scaling() {
    let base_case = Frame3dCase {
        length: 2.0,
        load_y: -1000.0,
        youngs_modulus: 210.0e9,
        moment_of_inertia_z: 8.0e-6,
        section_modulus_z: 1.6e-4,
    };
    let base = solve_frame_3d(&frame_3d_cantilever_request(base_case, 4))
        .expect("base 3d frame cantilever should solve");
    let base_expected = frame_3d_closed_form(base_case);

    for case in [
        Frame3dCase {
            load_y: -2400.0,
            ..base_case
        },
        Frame3dCase {
            moment_of_inertia_z: 1.6e-5,
            ..base_case
        },
        Frame3dCase {
            section_modulus_z: 3.2e-4,
            ..base_case
        },
    ] {
        let result = solve_frame_3d(&frame_3d_cantilever_request(case, 4))
            .expect("perturbed 3d frame cantilever should solve");
        let expected = frame_3d_closed_form(case);
        let tip = result.nodes.last().expect("tip node should exist");

        assert_close(tip.uy, expected.tip_uy, "3d frame perturbed tip uy");
        assert_close(tip.rz, expected.tip_rz, "3d frame perturbed tip rz");
        assert_close(
            result.max_moment,
            expected.root_moment,
            "3d frame perturbed max moment",
        );
        assert_close(
            result.max_stress,
            expected.stress,
            "3d frame perturbed max stress",
        );
        assert_close(
            result.total_strain_energy,
            expected.energy,
            "3d frame perturbed energy",
        );
        assert_close(
            result.max_displacement / base.max_displacement,
            expected.tip_uy.abs() / base_expected.tip_uy.abs(),
            "3d frame displacement ratio",
        );
        assert_close(
            result.max_stress / base.max_stress,
            expected.stress / base_expected.stress,
            "3d frame stress ratio",
        );
    }
}

#[derive(Clone, Copy)]
struct Frame3dCase {
    length: f64,
    load_y: f64,
    youngs_modulus: f64,
    moment_of_inertia_z: f64,
    section_modulus_z: f64,
}

struct Frame3dExpected {
    tip_uy: f64,
    tip_rz: f64,
    root_moment: f64,
    stress: f64,
    energy: f64,
}

fn frame_3d_closed_form(case: Frame3dCase) -> Frame3dExpected {
    let tip_uy =
        case.load_y * case.length.powi(3) / (3.0 * case.youngs_modulus * case.moment_of_inertia_z);
    let tip_rz =
        case.load_y * case.length.powi(2) / (2.0 * case.youngs_modulus * case.moment_of_inertia_z);
    let root_moment = case.load_y.abs() * case.length;
    let stress = root_moment / case.section_modulus_z;
    let energy = 0.5 * case.load_y.abs() * tip_uy.abs();

    Frame3dExpected {
        tip_uy,
        tip_rz,
        root_moment,
        stress,
        energy,
    }
}

fn frame_3d_cantilever_request(case: Frame3dCase, element_count: usize) -> SolveFrame3dRequest {
    let dx = case.length / element_count as f64;
    let nodes = (0..=element_count)
        .map(|index| {
            let is_fixed = index == 0;
            let is_tip = index == element_count;
            frame_3d_node(
                &format!("n{index}"),
                index as f64 * dx,
                is_fixed,
                if is_tip { case.load_y } else { 0.0 },
            )
        })
        .collect::<Vec<_>>();
    let elements = (0..element_count)
        .map(|index| Frame3dElementInput {
            id: format!("e{index}"),
            node_i: index,
            node_j: index + 1,
            area: 0.02,
            youngs_modulus: case.youngs_modulus,
            shear_modulus: 80.0e9,
            torsion_constant: 5.0e-6,
            moment_of_inertia_y: case.moment_of_inertia_z,
            moment_of_inertia_z: case.moment_of_inertia_z,
            section_modulus_y: case.section_modulus_z,
            section_modulus_z: case.section_modulus_z,
        })
        .collect::<Vec<_>>();

    SolveFrame3dRequest { nodes, elements }
}

fn frame_3d_node(id: &str, x: f64, fixed: bool, load_y: f64) -> Frame3dNodeInput {
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
        load_y,
        load_z: 0.0,
        moment_x: 0.0,
        moment_y: 0.0,
        moment_z: 0.0,
    }
}

fn assert_frame_force_close(actual: f64, expected: f64, label: &str) {
    let scale = expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= 1.0e-9 * scale,
        "{label}: expected {actual} to be close to {expected}",
    );
}
