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
    assert_energy_balance(&result);
}

#[test]
fn spring_1d_reports_zero_response_for_zero_load() {
    let result = solve_spring_1d(&series_request(0.0, &[100.0, 200.0]))
        .expect("zero-load spring fixture should solve");

    assert_close(result.max_displacement, 0.0);
    assert_close(result.max_force, 0.0);
    assert_close(result.total_strain_energy, 0.0);
    assert_energy_balance(&result);
    for node in &result.nodes {
        assert_close(node.ux, 0.0);
    }
    for element in &result.elements {
        assert_close(element.extension, 0.0);
        assert_close(element.force, 0.0);
        assert_close(element.strain_energy, 0.0);
    }
}

#[test]
fn spring_1d_tracks_load_and_stiffness_scaling() {
    let load = 900.0;
    let stiffnesses = [18_000.0, 27_000.0, 36_000.0];
    let baseline =
        solve_spring_1d(&series_request(load, &stiffnesses)).expect("baseline spring chain");
    assert_energy_balance(&baseline);

    let load_scale = 1.6;
    let load_scaled = solve_spring_1d(&series_request(load * load_scale, &stiffnesses))
        .expect("load-scaled spring chain");
    assert_close(
        load_scaled.max_displacement / baseline.max_displacement,
        load_scale,
    );
    assert_close(load_scaled.max_force / baseline.max_force, load_scale);
    assert_close(
        load_scaled.total_strain_energy / baseline.total_strain_energy,
        load_scale * load_scale,
    );
    assert_energy_balance(&load_scaled);
    for (scaled, base) in load_scaled.elements.iter().zip(&baseline.elements) {
        assert_close(scaled.extension / base.extension, load_scale);
        assert_close(scaled.force / base.force, load_scale);
        assert_close(
            scaled.strain_energy / base.strain_energy,
            load_scale * load_scale,
        );
    }

    let stiffness_scale = 1.8;
    let stiffened = stiffnesses
        .iter()
        .map(|stiffness| stiffness * stiffness_scale)
        .collect::<Vec<_>>();
    let stiffness_scaled =
        solve_spring_1d(&series_request(load, &stiffened)).expect("stiffness-scaled spring chain");
    assert_close(
        stiffness_scaled.max_displacement / baseline.max_displacement,
        1.0 / stiffness_scale,
    );
    assert_close(stiffness_scaled.max_force, baseline.max_force);
    assert_close(
        stiffness_scaled.total_strain_energy / baseline.total_strain_energy,
        1.0 / stiffness_scale,
    );
    assert_energy_balance(&stiffness_scaled);
    for (scaled, base) in stiffness_scaled.elements.iter().zip(&baseline.elements) {
        assert_close(scaled.extension / base.extension, 1.0 / stiffness_scale);
        assert_close(scaled.force, base.force);
        assert_close(
            scaled.strain_energy / base.strain_energy,
            1.0 / stiffness_scale,
        );
    }

    let spacing_scale = 2.25;
    let longer = solve_spring_1d(&series_request_with_spacing(
        load,
        &stiffnesses,
        spacing_scale,
    ))
    .expect("geometry-scaled spring chain");
    assert_close(longer.max_displacement, baseline.max_displacement);
    assert_close(longer.max_force, baseline.max_force);
    assert_close(longer.total_strain_energy, baseline.total_strain_energy);
    assert_energy_balance(&longer);
    for (scaled, base) in longer.elements.iter().zip(&baseline.elements) {
        assert_close(scaled.length / base.length, spacing_scale);
        assert_close(scaled.extension, base.extension);
        assert_close(scaled.force, base.force);
        assert_close(scaled.strain_energy, base.strain_energy);
    }
}

fn series_request(load: f64, stiffnesses: &[f64]) -> SolveSpring1dRequest {
    series_request_with_spacing(load, stiffnesses, 1.0)
}

fn series_request_with_spacing(
    load: f64,
    stiffnesses: &[f64],
    spacing: f64,
) -> SolveSpring1dRequest {
    let mut nodes = Vec::with_capacity(stiffnesses.len() + 1);
    for index in 0..=stiffnesses.len() {
        nodes.push(Spring1dNodeInput {
            id: format!("n{index}"),
            x: index as f64 * spacing,
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

fn assert_energy_balance(result: &kyuubiki_protocol::SolveSpring1dResult) {
    assert_close(
        result.total_strain_energy,
        result
            .elements
            .iter()
            .map(|element| element.strain_energy)
            .sum(),
    );
    let external_work = result
        .input
        .nodes
        .iter()
        .zip(result.nodes.iter())
        .map(|(input, result)| input.load_x * result.ux)
        .sum::<f64>();
    assert_close(result.total_strain_energy, 0.5 * external_work);
}

fn assert_close(actual: f64, expected: f64) {
    let scale = expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= TOL * scale,
        "expected {actual} to be close to {expected}",
    );
}
