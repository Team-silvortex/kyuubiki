use super::common::assert_close;
use kyuubiki_protocol::{SolveTorsion1dRequest, Torsion1dElementInput, Torsion1dNodeInput};
use kyuubiki_solver::solve_torsion_1d;

#[test]
fn torsion_shaft_refinement_and_perturbations_match_closed_form() {
    let length: f64 = 1.8;
    let torque: f64 = 2_750.0;
    let shear_modulus: f64 = 79.0e9;
    let polar_moment: f64 = 2.1e-6;
    let section_modulus: f64 = 1.35e-4;
    let expected_twist = torque * length / (shear_modulus * polar_moment);
    let expected_stress = torque / section_modulus;
    let expected_energy = 0.5 * torque * expected_twist;

    for elements in [1_usize, 2, 4, 8, 16] {
        let request = torsion_shaft_request(
            elements,
            length,
            torque,
            shear_modulus,
            polar_moment,
            section_modulus,
        );
        let result = solve_torsion_1d(&request).expect("refined torsion shaft should solve");
        let tip = result
            .nodes
            .last()
            .expect("torsion shaft should have a tip node");

        assert_eq!(result.nodes.len(), elements + 1);
        assert_eq!(result.elements.len(), elements);
        assert_close(tip.rz, expected_twist, "torsion tip rotation");
        assert_close(result.max_rotation, expected_twist, "torsion max rotation");
        assert_close(result.max_torque, torque, "torsion max torque");
        assert_close(result.max_stress, expected_stress, "torsion max stress");
        assert_close(
            result.total_strain_energy,
            expected_energy,
            "torsion strain energy",
        );
    }

    let baseline = solve_torsion_1d(&torsion_shaft_request(
        8,
        length,
        torque,
        shear_modulus,
        polar_moment,
        section_modulus,
    ))
    .expect("baseline torsion shaft should solve");
    let doubled_torque = solve_torsion_1d(&torsion_shaft_request(
        8,
        length,
        torque * 2.0,
        shear_modulus,
        polar_moment,
        section_modulus,
    ))
    .expect("torque perturbation should solve");
    assert_close(
        doubled_torque.max_rotation / baseline.max_rotation,
        2.0,
        "torque to rotation scaling",
    );
    assert_close(
        doubled_torque.max_stress / baseline.max_stress,
        2.0,
        "torque to stress scaling",
    );
    assert_close(
        doubled_torque.total_strain_energy / baseline.total_strain_energy,
        4.0,
        "torque to energy scaling",
    );

    let doubled_polar_moment = solve_torsion_1d(&torsion_shaft_request(
        8,
        length,
        torque,
        shear_modulus,
        polar_moment * 2.0,
        section_modulus * 2.0,
    ))
    .expect("polar moment perturbation should solve");
    assert_close(
        doubled_polar_moment.max_rotation / baseline.max_rotation,
        0.5,
        "polar moment to rotation scaling",
    );
    assert_close(
        doubled_polar_moment.max_stress / baseline.max_stress,
        0.5,
        "section modulus to stress scaling",
    );
    assert_close(
        doubled_polar_moment.total_strain_energy / baseline.total_strain_energy,
        0.5,
        "polar moment to energy scaling",
    );
}

fn torsion_shaft_request(
    elements: usize,
    length: f64,
    torque: f64,
    shear_modulus: f64,
    polar_moment: f64,
    section_modulus: f64,
) -> SolveTorsion1dRequest {
    let nodes = (0..=elements)
        .map(|index| {
            let is_fixed = index == 0;
            let is_tip = index == elements;
            Torsion1dNodeInput {
                id: format!("tn{index}"),
                x: length * index as f64 / elements as f64,
                fix_rz: is_fixed,
                torque_z: if is_tip { torque } else { 0.0 },
            }
        })
        .collect::<Vec<_>>();

    let elements = (0..elements)
        .map(|index| Torsion1dElementInput {
            id: format!("ts{index}"),
            node_i: index,
            node_j: index + 1,
            shear_modulus,
            polar_moment,
            section_modulus,
        })
        .collect::<Vec<_>>();

    SolveTorsion1dRequest { nodes, elements }
}
