use kyuubiki_protocol::{SolveSpring1dRequest, Spring1dElementInput, Spring1dNodeInput};
use kyuubiki_solver::solve_spring_1d;

const TOL: f64 = 1.0e-10;

#[test]
fn spring_1d_matches_series_equivalent_stiffness_closed_form() {
    let load = 1200.0;
    let stiffnesses = [35_000.0, 20_000.0, 50_000.0];
    let result = solve_spring_1d(&series_request(load, &stiffnesses))
        .expect("series spring closed-form fixture should solve");

    let expected_tip = load
        * stiffnesses
            .iter()
            .map(|stiffness| 1.0 / stiffness)
            .sum::<f64>();
    let mut expected_displacement = 0.0;
    assert_close(result.nodes[0].ux, 0.0);
    for (index, stiffness) in stiffnesses.iter().enumerate() {
        let expected_extension = load / stiffness;
        expected_displacement += expected_extension;
        let element = &result.elements[index];
        assert_close(element.extension, expected_extension);
        assert_close(element.force, load);
        assert_close(element.strain_energy, 0.5 * load * expected_extension);
        assert_close(result.nodes[index + 1].ux, expected_displacement);
    }
    let expected_energy = 0.5 * load * expected_tip;
    assert_close(result.max_displacement, expected_tip);
    assert_close(result.max_force, load);
    assert_close(result.total_strain_energy, expected_energy);
}

#[test]
fn spring_1d_reports_zero_response_for_zero_load() {
    let result = solve_spring_1d(&series_request(0.0, &[100.0, 200.0]))
        .expect("zero-load spring fixture should solve");

    assert_close(result.max_displacement, 0.0);
    assert_close(result.max_force, 0.0);
    assert_close(result.total_strain_energy, 0.0);
    for node in &result.nodes {
        assert_close(node.ux, 0.0);
    }
    for element in &result.elements {
        assert_close(element.extension, 0.0);
        assert_close(element.force, 0.0);
        assert_close(element.strain_energy, 0.0);
    }
}

fn series_request(load: f64, stiffnesses: &[f64]) -> SolveSpring1dRequest {
    let mut nodes = Vec::with_capacity(stiffnesses.len() + 1);
    for index in 0..=stiffnesses.len() {
        nodes.push(Spring1dNodeInput {
            id: format!("n{index}"),
            x: index as f64,
            fix_x: index == 0,
            load_x: if index == stiffnesses.len() {
                load
            } else {
                0.0
            },
        });
    }
    let elements = stiffnesses
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

fn assert_close(actual: f64, expected: f64) {
    let scale = expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= TOL * scale,
        "expected {actual} to be close to {expected}",
    );
}
