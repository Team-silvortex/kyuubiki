use kyuubiki_protocol::{
    SolveThermalTruss2dRequest, ThermalTruss2dElementInput, ThermalTruss2dElementResult,
    ThermalTruss2dNodeInput,
};
use kyuubiki_solver::solve_thermal_truss_2d;

const TOL: f64 = 1.0e-7;

#[test]
fn thermal_truss_2d_review_bundle_checks_mixed_temperature_load_and_node_balance() {
    let request = SolveThermalTruss2dRequest {
        nodes: vec![
            node("left_support", 0.0, 0.0, true, true, 0.0, 0.0, 40.0),
            node("right_roller", 1.0, 0.0, false, true, 0.0, 0.0, 40.0),
            node("loaded_apex", 0.5, 0.8, false, false, 0.0, -400.0, 25.0),
        ],
        elements: vec![
            element("left_web", 0, 2),
            element("right_web", 1, 2),
            element("bottom_chord", 0, 1),
        ],
    };

    let result = solve_thermal_truss_2d(&request).expect("review 2d thermal truss should solve");

    assert_eq!(result.nodes.len(), 3);
    assert_eq!(result.elements.len(), 3);
    assert_close(result.nodes[0].ux, 0.0, 1.0e-12);
    assert_close(result.nodes[0].uy, 0.0, 1.0e-12);
    assert_close(result.nodes[1].uy, 0.0, 1.0e-12);
    assert_close(result.nodes[1].ux, 4.801_785_714_285_713e-4, 1.0e-12);
    assert_close(result.nodes[2].uy, 2.834_443_641_425_211e-4, 1.0e-12);
    assert_close(result.max_displacement, 4.801_785_714_285_713e-4, 1.0e-12);
    assert_close(result.max_axial_force, 235.849_528_301_435_58, 1.0e-9);
    assert_close(result.max_stress, 23_584.952_830_143_557, 1.0e-9);
    assert_close(result.max_temperature_delta, 40.0, 1.0e-12);
    assert!(result.total_strain_energy > 0.0);
    assert!(result.max_strain_energy_density > 0.0);

    assert_close(result.elements[0].average_temperature_delta, 32.5, 1.0e-12);
    assert_close(result.elements[1].average_temperature_delta, 32.5, 1.0e-12);
    assert_close(result.elements[2].average_temperature_delta, 40.0, 1.0e-12);
    for (index, element) in result.elements.iter().enumerate() {
        assert_eq!(element.index, index);
        assert!(element.length > 0.0);
        assert!(element.thermal_strain.is_finite());
        assert!(element.mechanical_strain.is_finite());
        assert!(element.total_strain.is_finite());
        assert!(element.stress.is_finite());
        assert!(element.axial_force.is_finite());
        assert_close(
            element.thermal_strain,
            12.0e-6 * element.average_temperature_delta,
            1.0e-12,
        );
        assert_close(element.stress, 70.0e9 * element.mechanical_strain, 1.0e-12);
        assert_close(
            element.strain_energy_density,
            0.5 * element.stress * element.mechanical_strain,
            1.0e-12,
        );
    }
    assert_close(
        result.total_strain_energy,
        total_strain_energy(&request, &result.elements),
        TOL,
    );

    let (internal_x, internal_y) = loaded_node_internal_force(&request, &result.elements, 2);
    assert_close(internal_x + request.nodes[2].load_x, 0.0, TOL);
    assert_close(internal_y + request.nodes[2].load_y, 0.0, TOL);
}

fn total_strain_energy(
    request: &SolveThermalTruss2dRequest,
    elements: &[ThermalTruss2dElementResult],
) -> f64 {
    elements
        .iter()
        .zip(request.elements.iter())
        .map(|(element, input)| element.strain_energy_density * input.area * element.length)
        .sum()
}

fn loaded_node_internal_force(
    request: &SolveThermalTruss2dRequest,
    elements: &[ThermalTruss2dElementResult],
    node_index: usize,
) -> (f64, f64) {
    let mut force_x = 0.0;
    let mut force_y = 0.0;
    for element in elements {
        if element.node_i != node_index && element.node_j != node_index {
            continue;
        }
        let node_i = &request.nodes[element.node_i];
        let node_j = &request.nodes[element.node_j];
        let dx = node_j.x - node_i.x;
        let dy = node_j.y - node_i.y;
        let length = (dx * dx + dy * dy).sqrt();
        let c = dx / length;
        let s = dy / length;
        let sign = if element.node_i == node_index {
            1.0
        } else {
            -1.0
        };
        force_x += sign * element.axial_force * c;
        force_y += sign * element.axial_force * s;
    }
    (force_x, force_y)
}

#[allow(clippy::too_many_arguments)]
fn node(
    id: &str,
    x: f64,
    y: f64,
    fix_x: bool,
    fix_y: bool,
    load_x: f64,
    load_y: f64,
    temperature_delta: f64,
) -> ThermalTruss2dNodeInput {
    ThermalTruss2dNodeInput {
        id: id.to_string(),
        x,
        y,
        fix_x,
        fix_y,
        load_x,
        load_y,
        temperature_delta,
    }
}

fn element(id: &str, node_i: usize, node_j: usize) -> ThermalTruss2dElementInput {
    ThermalTruss2dElementInput {
        id: id.to_string(),
        node_i,
        node_j,
        area: 0.01,
        youngs_modulus: 70.0e9,
        thermal_expansion: 12.0e-6,
    }
}

fn assert_close(actual: f64, expected: f64, tolerance: f64) {
    let scale = expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= tolerance * scale,
        "expected {actual} to be close to {expected}",
    );
}
