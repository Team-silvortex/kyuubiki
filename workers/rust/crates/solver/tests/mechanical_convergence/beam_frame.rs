use super::common::assert_close;
use kyuubiki_protocol::{
    Beam1dElementInput, Beam1dNodeInput, Frame2dElementInput, Frame2dNodeInput, SolveBeam1dRequest,
    SolveFrame2dRequest,
};
use kyuubiki_solver::{solve_beam_1d, solve_frame_2d};

#[test]
fn tip_loaded_beam_refinement_is_closed_form_invariant() {
    let length: f64 = 2.4;
    let load: f64 = 1_350.0;
    let youngs_modulus: f64 = 210.0e9;
    let moment_of_inertia: f64 = 9.5e-6;
    let section_modulus: f64 = 1.9e-4;
    let expected_tip_uy = -load * length.powi(3) / (3.0 * youngs_modulus * moment_of_inertia);
    let expected_tip_rz = -load * length.powi(2) / (2.0 * youngs_modulus * moment_of_inertia);
    let expected_root_moment = load * length;
    let expected_stress = expected_root_moment / section_modulus;
    let expected_energy = 0.5 * load * expected_tip_uy.abs();

    for elements in [1_usize, 2, 4, 8, 16] {
        let request = beam_cantilever_request(
            elements,
            length,
            load,
            youngs_modulus,
            moment_of_inertia,
            section_modulus,
        );
        let result = solve_beam_1d(&request).expect("refined cantilever beam should solve");
        let tip = result.nodes.last().expect("beam should have a tip node");

        assert_eq!(result.nodes.len(), elements + 1);
        assert_eq!(result.elements.len(), elements);
        assert_close(tip.uy, expected_tip_uy, "beam tip displacement");
        assert_close(tip.rz, expected_tip_rz, "beam tip rotation");
        assert_close(
            result.max_displacement,
            expected_tip_uy.abs(),
            "beam max displacement",
        );
        assert_close(
            result.max_rotation,
            expected_tip_rz.abs(),
            "beam max rotation",
        );
        assert_close(result.max_moment, expected_root_moment, "beam root moment");
        assert_close(result.max_stress, expected_stress, "beam max stress");
        assert_close(
            result.total_strain_energy,
            expected_energy,
            "beam strain energy",
        );
    }
}

#[test]
fn frame_2d_cantilever_refinement_matches_beam_closed_form() {
    let length: f64 = 2.1;
    let load: f64 = 1_150.0;
    let area: f64 = 0.022;
    let youngs_modulus: f64 = 205.0e9;
    let moment_of_inertia: f64 = 8.4e-6;
    let section_modulus: f64 = 1.7e-4;
    let expected_tip_uy = -load * length.powi(3) / (3.0 * youngs_modulus * moment_of_inertia);
    let expected_tip_rz = -load * length.powi(2) / (2.0 * youngs_modulus * moment_of_inertia);
    let expected_root_moment = load * length;
    let expected_stress = expected_root_moment / section_modulus;
    let expected_energy = 0.5 * load * expected_tip_uy.abs();

    for elements in [1_usize, 2, 4, 8, 16] {
        let request = frame_cantilever_request(
            elements,
            length,
            load,
            area,
            youngs_modulus,
            moment_of_inertia,
            section_modulus,
        );
        let result = solve_frame_2d(&request).expect("refined 2D frame should solve");
        let tip = result.nodes.last().expect("frame should have a tip node");

        assert_eq!(result.nodes.len(), elements + 1);
        assert_eq!(result.elements.len(), elements);
        assert_close(tip.ux, 0.0, "frame tip axial displacement");
        assert_close(tip.uy, expected_tip_uy, "frame tip transverse displacement");
        assert_close(tip.rz, expected_tip_rz, "frame tip rotation");
        assert_close(
            result.max_displacement,
            expected_tip_uy.abs(),
            "frame max displacement",
        );
        assert_close(
            result.max_rotation,
            expected_tip_rz.abs(),
            "frame max rotation",
        );
        assert_close(result.max_moment, expected_root_moment, "frame root moment");
        assert_close(result.max_stress, expected_stress, "frame max stress");
        assert_close(
            result.total_strain_energy,
            expected_energy,
            "frame strain energy",
        );
    }
}

fn beam_cantilever_request(
    elements: usize,
    length: f64,
    load: f64,
    youngs_modulus: f64,
    moment_of_inertia: f64,
    section_modulus: f64,
) -> SolveBeam1dRequest {
    let nodes = (0..=elements)
        .map(|index| {
            let is_fixed = index == 0;
            let is_tip = index == elements;
            Beam1dNodeInput {
                id: format!("n{index}"),
                x: length * index as f64 / elements as f64,
                fix_y: is_fixed,
                fix_rz: is_fixed,
                load_y: if is_tip { -load } else { 0.0 },
                moment_z: 0.0,
            }
        })
        .collect::<Vec<_>>();

    let elements = (0..elements)
        .map(|index| Beam1dElementInput {
            id: format!("b{index}"),
            node_i: index,
            node_j: index + 1,
            youngs_modulus,
            moment_of_inertia,
            section_modulus,
            distributed_load_y: 0.0,
        })
        .collect::<Vec<_>>();

    SolveBeam1dRequest { nodes, elements }
}

fn frame_cantilever_request(
    elements: usize,
    length: f64,
    load: f64,
    area: f64,
    youngs_modulus: f64,
    moment_of_inertia: f64,
    section_modulus: f64,
) -> SolveFrame2dRequest {
    let nodes = (0..=elements)
        .map(|index| {
            let is_fixed = index == 0;
            let is_tip = index == elements;
            Frame2dNodeInput {
                id: format!("fn{index}"),
                x: length * index as f64 / elements as f64,
                y: 0.0,
                fix_x: is_fixed,
                fix_y: is_fixed,
                fix_rz: is_fixed,
                load_x: 0.0,
                load_y: if is_tip { -load } else { 0.0 },
                moment_z: 0.0,
            }
        })
        .collect::<Vec<_>>();

    let elements = (0..elements)
        .map(|index| Frame2dElementInput {
            id: format!("fb{index}"),
            node_i: index,
            node_j: index + 1,
            area,
            youngs_modulus,
            moment_of_inertia,
            section_modulus,
        })
        .collect::<Vec<_>>();

    SolveFrame2dRequest { nodes, elements }
}
