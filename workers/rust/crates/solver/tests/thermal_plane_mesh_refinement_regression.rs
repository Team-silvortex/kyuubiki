use kyuubiki_protocol::{
    HeatPlaneNodeInput, HeatPlaneQuadElementInput, HeatPlaneTriangleElementInput,
    SolveHeatPlaneQuad2dRequest, SolveHeatPlaneQuad2dResult, SolveHeatPlaneTriangle2dRequest,
    SolveHeatPlaneTriangle2dResult, SolveThermalPlaneQuad2dRequest, SolveThermalPlaneQuad2dResult,
    SolveThermalPlaneTriangle2dRequest, SolveThermalPlaneTriangle2dResult, ThermalPlaneNodeInput,
    ThermalPlaneQuadElementInput, ThermalPlaneTriangleElementInput,
};
use kyuubiki_solver::{
    solve_heat_plane_quad_2d, solve_heat_plane_triangle_2d, solve_thermal_plane_quad_2d,
    solve_thermal_plane_triangle_2d,
};

const TOL: f64 = 1.0e-10;
const THICKNESS: f64 = 0.02;
const CONDUCTIVITY: f64 = 45.0;
const YOUNGS_MODULUS: f64 = 70.0e9;
const POISSON_RATIO: f64 = 0.33;
const THERMAL_EXPANSION: f64 = 11.0e-6;
const TEMPERATURE_DELTA: f64 = 30.0;

#[test]
fn heat_plane_triangle_refinement_matches_quad_patch_temperatures_and_heat_flow() {
    let triangle = solve_heat_plane_triangle_2d(&heat_triangle_patch())
        .expect("two-triangle heat patch should solve");
    let quad = solve_heat_plane_quad_2d(&heat_quad_patch()).expect("quad heat patch should solve");

    assert_eq!(triangle.nodes.len(), quad.nodes.len());
    for (triangle_node, quad_node) in triangle.nodes.iter().zip(quad.nodes.iter()) {
        assert_close(triangle_node.temperature, quad_node.temperature);
    }

    assert_close(triangle.max_temperature, quad.max_temperature);
    assert_close(triangle.max_heat_flux, quad.max_heat_flux);
    assert_close(
        triangle.total_abs_heat_flow_rate,
        quad.total_abs_heat_flow_rate,
    );
    assert_heat_triangle_summary(&triangle);
    assert_heat_quad_summary(&quad);
}

#[test]
fn heat_plane_triangle_linear_field_is_diagonal_invariant_and_conductivity_scaled() {
    let diagonal_a = solve_heat_plane_triangle_2d(&heat_triangle_patch())
        .expect("first diagonal heat patch should solve");
    let diagonal_b =
        solve_heat_plane_triangle_2d(&heat_triangle_cross_diagonal_patch(CONDUCTIVITY))
            .expect("second diagonal heat patch should solve");
    let perturbed =
        solve_heat_plane_triangle_2d(&heat_triangle_cross_diagonal_patch(CONDUCTIVITY * 1.07))
            .expect("conductivity-perturbed heat patch should solve");
    let thick = solve_heat_plane_triangle_2d(&heat_triangle_cross_diagonal_patch_with_material(
        CONDUCTIVITY,
        THICKNESS * 1.8,
    ))
    .expect("thickness-perturbed heat patch should solve");

    assert_eq!(diagonal_a.nodes.len(), diagonal_b.nodes.len());
    for (left, right) in diagonal_a.nodes.iter().zip(diagonal_b.nodes.iter()) {
        assert_close(left.temperature, right.temperature);
    }

    for element in diagonal_b.elements.iter() {
        assert_close(element.temperature_gradient_x, 0.0);
        assert_close(element.temperature_gradient_y, -80.0);
        assert_close(element.heat_flux_x, 0.0);
        assert_close(element.heat_flux_y, 3600.0);
    }

    assert_close(diagonal_a.max_heat_flux, diagonal_b.max_heat_flux);
    assert_close(
        diagonal_a.total_abs_heat_flow_rate,
        diagonal_b.total_abs_heat_flow_rate,
    );
    assert_close(perturbed.max_heat_flux / diagonal_b.max_heat_flux, 1.07);
    assert_close(
        perturbed.total_abs_heat_flow_rate / diagonal_b.total_abs_heat_flow_rate,
        1.07,
    );
    assert_close(
        perturbed.elements[0].temperature_gradient_y,
        diagonal_b.elements[0].temperature_gradient_y,
    );
    assert_close(
        thick.elements[0].temperature_gradient_y,
        diagonal_b.elements[0].temperature_gradient_y,
    );
    assert_close(
        thick.elements[0].heat_flux_y,
        diagonal_b.elements[0].heat_flux_y,
    );
    assert_close(
        thick.total_abs_heat_flow_rate / diagonal_b.total_abs_heat_flow_rate,
        1.8,
    );
    assert_heat_triangle_summary(&diagonal_a);
    assert_heat_triangle_summary(&diagonal_b);
    assert_heat_triangle_summary(&perturbed);
    assert_heat_triangle_summary(&thick);
}

#[test]
fn heat_plane_quad_manufactured_linear_field_is_refinement_invariant() {
    for subdivisions in [1_usize, 2, 4, 8] {
        let result = solve_heat_plane_quad_2d(&heat_quad_mesh(subdivisions))
            .expect("refined quad heat manufactured field should solve");

        assert_close(result.max_temperature, 100.0);
        assert_close(result.max_heat_flux, 3600.0);
        assert_close(result.total_abs_heat_flow_rate, 72.0);
        assert_eq!(result.elements.len(), subdivisions * subdivisions);

        for (index, node) in result.nodes.iter().enumerate() {
            assert_eq!(node.index, index);
            assert_close(node.temperature, manufactured_temperature(node.y));
        }
        for (index, element) in result.elements.iter().enumerate() {
            assert_eq!(element.index, index);
            assert_close(element.temperature_gradient_x, 0.0);
            assert_close(element.temperature_gradient_y, -80.0);
            assert_close(element.heat_flux_x, 0.0);
            assert_close(element.heat_flux_y, 3600.0);
        }
        assert_heat_quad_summary(&result);
    }
}

#[test]
fn heat_plane_triangle_manufactured_linear_field_is_refinement_invariant() {
    for subdivisions in [1_usize, 2, 4, 8] {
        let result = solve_heat_plane_triangle_2d(&heat_triangle_mesh(subdivisions))
            .expect("refined triangle heat manufactured field should solve");

        assert_close(result.max_temperature, 100.0);
        assert_close(result.max_heat_flux, 3600.0);
        assert_close(result.total_abs_heat_flow_rate, 72.0);
        assert_eq!(result.elements.len(), subdivisions * subdivisions * 2);

        for (index, node) in result.nodes.iter().enumerate() {
            assert_eq!(node.index, index);
            assert_close(node.temperature, manufactured_temperature(node.y));
        }
        for (index, element) in result.elements.iter().enumerate() {
            assert_eq!(element.index, index);
            assert_close(element.temperature_gradient_x, 0.0);
            assert_close(element.temperature_gradient_y, -80.0);
            assert_close(element.heat_flux_x, 0.0);
            assert_close(element.heat_flux_y, 3600.0);
        }
        assert_heat_triangle_summary(&result);
    }
}

#[test]
fn thermal_plane_triangle_refinement_matches_quad_patch_stress_and_energy() {
    let triangle = solve_thermal_plane_triangle_2d(&thermal_triangle_patch())
        .expect("two-triangle thermal patch should solve");
    let quad = solve_thermal_plane_quad_2d(&thermal_quad_patch())
        .expect("quad thermal patch should solve");
    let hotter = solve_thermal_plane_triangle_2d(&thermal_triangle_patch_with_material(
        THICKNESS,
        TEMPERATURE_DELTA * 1.4,
    ))
    .expect("temperature-scaled thermal patch should solve");
    let thick = solve_thermal_plane_triangle_2d(&thermal_triangle_patch_with_material(
        THICKNESS * 1.6,
        TEMPERATURE_DELTA,
    ))
    .expect("thickness-scaled thermal patch should solve");

    assert_eq!(triangle.nodes.len(), quad.nodes.len());
    for (triangle_node, quad_node) in triangle.nodes.iter().zip(quad.nodes.iter()) {
        assert_close(triangle_node.ux, quad_node.ux);
        assert_close(triangle_node.uy, quad_node.uy);
        assert_close(triangle_node.temperature_delta, quad_node.temperature_delta);
    }

    assert_close(triangle.max_displacement, quad.max_displacement);
    assert_close(triangle.max_temperature_delta, quad.max_temperature_delta);
    assert_close(triangle.max_stress, quad.max_stress);
    assert_close(triangle.total_strain_energy, quad.total_strain_energy);
    assert_close(
        triangle.max_strain_energy_density,
        quad.max_strain_energy_density,
    );

    assert_close(hotter.max_stress / triangle.max_stress, 1.4);
    assert_close(
        hotter.max_strain_energy_density / triangle.max_strain_energy_density,
        1.4 * 1.4,
    );
    assert_close(
        hotter.total_strain_energy / triangle.total_strain_energy,
        1.4 * 1.4,
    );
    assert_close(thick.max_stress, triangle.max_stress);
    assert_close(
        thick.max_strain_energy_density,
        triangle.max_strain_energy_density,
    );
    assert_close(
        thick.total_strain_energy / triangle.total_strain_energy,
        1.6,
    );
    assert_thermal_triangle_summary(&triangle);
    assert_thermal_quad_summary(&quad);
    assert_thermal_triangle_summary(&hotter);
    assert_thermal_triangle_summary(&thick);
}

fn assert_heat_triangle_summary(result: &SolveHeatPlaneTriangle2dResult) {
    let mut max_heat_flux = 0.0_f64;
    let mut total_abs_heat_flow_rate = 0.0_f64;

    for (index, element) in result.elements.iter().enumerate() {
        assert_eq!(element.index, index);
        let input = &result.input.elements[element.index];
        let node_i = &result.nodes[element.node_i];
        let node_j = &result.nodes[element.node_j];
        let node_k = &result.nodes[element.node_k];
        assert_close(
            element.area,
            triangle_area(
                (node_i.x, node_i.y),
                (node_j.x, node_j.y),
                (node_k.x, node_k.y),
            ),
        );
        let average_temperature = (result.nodes[element.node_i].temperature
            + result.nodes[element.node_j].temperature
            + result.nodes[element.node_k].temperature)
            / 3.0;
        assert_close(element.average_temperature, average_temperature);
        assert_close(
            element.heat_flux_x,
            -input.conductivity * element.temperature_gradient_x,
        );
        assert_close(
            element.heat_flux_y,
            -input.conductivity * element.temperature_gradient_y,
        );
        assert_close(
            element.heat_flux_magnitude,
            magnitude(element.heat_flux_x, element.heat_flux_y),
        );
        assert_close(
            element.heat_flow_rate,
            element.heat_flux_magnitude * element.area * input.thickness,
        );
        max_heat_flux = max_heat_flux.max(element.heat_flux_magnitude);
        total_abs_heat_flow_rate += element.heat_flow_rate.abs();
    }

    assert_close(result.max_temperature, max_heat_temperature(result));
    assert_close(result.max_heat_flux, max_heat_flux);
    assert_close(result.total_abs_heat_flow_rate, total_abs_heat_flow_rate);
}

fn assert_heat_quad_summary(result: &SolveHeatPlaneQuad2dResult) {
    let mut max_heat_flux = 0.0_f64;
    let mut total_abs_heat_flow_rate = 0.0_f64;

    for (index, element) in result.elements.iter().enumerate() {
        assert_eq!(element.index, index);
        let input = &result.input.elements[element.index];
        let node_i = &result.nodes[element.node_i];
        let node_j = &result.nodes[element.node_j];
        let node_k = &result.nodes[element.node_k];
        let node_l = &result.nodes[element.node_l];
        assert_close(
            element.area,
            quad_area(
                (node_i.x, node_i.y),
                (node_j.x, node_j.y),
                (node_k.x, node_k.y),
                (node_l.x, node_l.y),
            ),
        );
        let average_temperature = (result.nodes[element.node_i].temperature
            + result.nodes[element.node_j].temperature
            + result.nodes[element.node_k].temperature
            + result.nodes[element.node_l].temperature)
            / 4.0;
        assert_close(element.average_temperature, average_temperature);
        assert_close(
            element.heat_flux_x,
            -input.conductivity * element.temperature_gradient_x,
        );
        assert_close(
            element.heat_flux_y,
            -input.conductivity * element.temperature_gradient_y,
        );
        assert_close(
            element.heat_flux_magnitude,
            magnitude(element.heat_flux_x, element.heat_flux_y),
        );
        assert_close(
            element.heat_flow_rate,
            element.heat_flux_magnitude * element.area * input.thickness,
        );
        max_heat_flux = max_heat_flux.max(element.heat_flux_magnitude);
        total_abs_heat_flow_rate += element.heat_flow_rate.abs();
    }

    assert_close(result.max_temperature, max_heat_temperature(result));
    assert_close(result.max_heat_flux, max_heat_flux);
    assert_close(result.total_abs_heat_flow_rate, total_abs_heat_flow_rate);
}

fn assert_thermal_triangle_summary(result: &SolveThermalPlaneTriangle2dResult) {
    let mut max_stress = 0.0_f64;
    let mut max_energy_density = 0.0_f64;
    let mut total_strain_energy = 0.0_f64;

    for (index, element) in result.elements.iter().enumerate() {
        assert_eq!(element.index, index);
        let input = &result.input.elements[element.index];
        let node_i = &result.nodes[element.node_i];
        let node_j = &result.nodes[element.node_j];
        let node_k = &result.nodes[element.node_k];
        assert_close(
            element.area,
            triangle_area(
                (node_i.x, node_i.y),
                (node_j.x, node_j.y),
                (node_k.x, node_k.y),
            ),
        );
        let average_temperature_delta = (result.nodes[element.node_i].temperature_delta
            + result.nodes[element.node_j].temperature_delta
            + result.nodes[element.node_k].temperature_delta)
            / 3.0;
        assert_close(element.average_temperature_delta, average_temperature_delta);
        assert_close(
            element.thermal_strain,
            input.thermal_expansion * average_temperature_delta,
        );
        assert_close(
            element.von_mises,
            von_mises(element.stress_x, element.stress_y, element.tau_xy),
        );
        assert_close(
            element.max_in_plane_shear,
            in_plane_shear(element.stress_x, element.stress_y, element.tau_xy),
        );
        max_stress = max_stress.max(element.von_mises);
        max_energy_density = max_energy_density.max(element.strain_energy_density);
        total_strain_energy += element.strain_energy_density * element.area * input.thickness;
    }

    assert_close(result.max_displacement, max_thermal_displacement(result));
    assert_close(
        result.max_temperature_delta,
        max_thermal_temperature(result),
    );
    assert_close(result.max_stress, max_stress);
    assert_close(result.max_strain_energy_density, max_energy_density);
    assert_close(result.total_strain_energy, total_strain_energy);
}

fn assert_thermal_quad_summary(result: &SolveThermalPlaneQuad2dResult) {
    let mut max_stress = 0.0_f64;
    let mut max_energy_density = 0.0_f64;
    let mut total_strain_energy = 0.0_f64;

    for (index, element) in result.elements.iter().enumerate() {
        assert_eq!(element.index, index);
        let input = &result.input.elements[element.index];
        let node_i = &result.nodes[element.node_i];
        let node_j = &result.nodes[element.node_j];
        let node_k = &result.nodes[element.node_k];
        let node_l = &result.nodes[element.node_l];
        assert_close(
            element.area,
            quad_area(
                (node_i.x, node_i.y),
                (node_j.x, node_j.y),
                (node_k.x, node_k.y),
                (node_l.x, node_l.y),
            ),
        );
        let average_temperature_delta = (result.nodes[element.node_i].temperature_delta
            + result.nodes[element.node_j].temperature_delta
            + result.nodes[element.node_k].temperature_delta
            + result.nodes[element.node_l].temperature_delta)
            / 4.0;
        assert_close(element.average_temperature_delta, average_temperature_delta);
        assert_close(
            element.thermal_strain,
            input.thermal_expansion * average_temperature_delta,
        );
        assert_close(
            element.von_mises,
            von_mises(element.stress_x, element.stress_y, element.tau_xy),
        );
        assert_close(
            element.max_in_plane_shear,
            in_plane_shear(element.stress_x, element.stress_y, element.tau_xy),
        );
        max_stress = max_stress.max(element.von_mises);
        max_energy_density = max_energy_density.max(element.strain_energy_density);
        total_strain_energy += element.strain_energy_density * element.area * input.thickness;
    }

    assert_close(result.max_displacement, max_thermal_displacement(result));
    assert_close(
        result.max_temperature_delta,
        max_thermal_temperature(result),
    );
    assert_close(result.max_stress, max_stress);
    assert_close(result.max_strain_energy_density, max_energy_density);
    assert_close(result.total_strain_energy, total_strain_energy);
}

fn heat_triangle_patch() -> SolveHeatPlaneTriangle2dRequest {
    SolveHeatPlaneTriangle2dRequest {
        nodes: heat_nodes(),
        elements: vec![heat_tri("lower", 0, 1, 2), heat_tri("upper", 0, 2, 3)],
    }
}

fn heat_quad_patch() -> SolveHeatPlaneQuad2dRequest {
    SolveHeatPlaneQuad2dRequest {
        nodes: heat_nodes(),
        elements: vec![HeatPlaneQuadElementInput {
            id: "quad".to_string(),
            node_i: 0,
            node_j: 1,
            node_k: 2,
            node_l: 3,
            thickness: THICKNESS,
            conductivity: CONDUCTIVITY,
        }],
    }
}

fn heat_quad_mesh(subdivisions: usize) -> SolveHeatPlaneQuad2dRequest {
    let mut elements = Vec::new();
    for y_index in 0..subdivisions {
        for x_index in 0..subdivisions {
            elements.push(HeatPlaneQuadElementInput {
                id: format!("quad-{x_index}-{y_index}"),
                node_i: heat_grid_index(x_index, y_index, subdivisions),
                node_j: heat_grid_index(x_index + 1, y_index, subdivisions),
                node_k: heat_grid_index(x_index + 1, y_index + 1, subdivisions),
                node_l: heat_grid_index(x_index, y_index + 1, subdivisions),
                thickness: THICKNESS,
                conductivity: CONDUCTIVITY,
            });
        }
    }

    SolveHeatPlaneQuad2dRequest {
        nodes: heat_grid_nodes(subdivisions),
        elements,
    }
}

fn heat_triangle_mesh(subdivisions: usize) -> SolveHeatPlaneTriangle2dRequest {
    let mut elements = Vec::new();
    for y_index in 0..subdivisions {
        for x_index in 0..subdivisions {
            let lower_left = heat_grid_index(x_index, y_index, subdivisions);
            let lower_right = heat_grid_index(x_index + 1, y_index, subdivisions);
            let upper_right = heat_grid_index(x_index + 1, y_index + 1, subdivisions);
            let upper_left = heat_grid_index(x_index, y_index + 1, subdivisions);
            elements.push(heat_tri(
                &format!("tri-a-{x_index}-{y_index}"),
                lower_left,
                lower_right,
                upper_right,
            ));
            elements.push(heat_tri(
                &format!("tri-b-{x_index}-{y_index}"),
                lower_left,
                upper_right,
                upper_left,
            ));
        }
    }

    SolveHeatPlaneTriangle2dRequest {
        nodes: heat_grid_nodes(subdivisions),
        elements,
    }
}

fn heat_grid_nodes(subdivisions: usize) -> Vec<HeatPlaneNodeInput> {
    let mut nodes = Vec::new();
    for y_index in 0..=subdivisions {
        for x_index in 0..=subdivisions {
            let x = x_index as f64 / subdivisions as f64;
            let y = y_index as f64 / subdivisions as f64;
            nodes.push(heat_node(
                &format!("n-{x_index}-{y_index}"),
                x,
                y,
                true,
                manufactured_temperature(y),
            ));
        }
    }
    nodes
}

fn heat_grid_index(x_index: usize, y_index: usize, subdivisions: usize) -> usize {
    y_index * (subdivisions + 1) + x_index
}

fn manufactured_temperature(y: f64) -> f64 {
    100.0 - 80.0 * y
}

fn heat_triangle_cross_diagonal_patch(conductivity: f64) -> SolveHeatPlaneTriangle2dRequest {
    heat_triangle_cross_diagonal_patch_with_material(conductivity, THICKNESS)
}

fn heat_triangle_cross_diagonal_patch_with_material(
    conductivity: f64,
    thickness: f64,
) -> SolveHeatPlaneTriangle2dRequest {
    SolveHeatPlaneTriangle2dRequest {
        nodes: heat_nodes(),
        elements: vec![
            heat_tri_with_material("left", 0, 1, 3, conductivity, thickness),
            heat_tri_with_material("right", 1, 2, 3, conductivity, thickness),
        ],
    }
}

fn thermal_triangle_patch() -> SolveThermalPlaneTriangle2dRequest {
    thermal_triangle_patch_with_material(THICKNESS, TEMPERATURE_DELTA)
}

fn thermal_triangle_patch_with_material(
    thickness: f64,
    temperature_delta: f64,
) -> SolveThermalPlaneTriangle2dRequest {
    let elements = vec![thermal_tri("lower", 0, 1, 2), thermal_tri("upper", 0, 2, 3)]
        .into_iter()
        .map(|mut element| {
            element.thickness = thickness;
            element
        })
        .collect();

    SolveThermalPlaneTriangle2dRequest {
        nodes: thermal_nodes_with_delta(temperature_delta),
        elements,
    }
}

fn thermal_quad_patch() -> SolveThermalPlaneQuad2dRequest {
    SolveThermalPlaneQuad2dRequest {
        nodes: thermal_nodes(),
        elements: vec![ThermalPlaneQuadElementInput {
            id: "quad".to_string(),
            node_i: 0,
            node_j: 1,
            node_k: 2,
            node_l: 3,
            thickness: THICKNESS,
            youngs_modulus: YOUNGS_MODULUS,
            poisson_ratio: POISSON_RATIO,
            thermal_expansion: THERMAL_EXPANSION,
        }],
    }
}

fn heat_nodes() -> Vec<HeatPlaneNodeInput> {
    vec![
        heat_node("hot-left", 0.0, 0.0, true, 100.0),
        heat_node("hot-right", 1.0, 0.0, true, 100.0),
        heat_node("cold-right", 1.0, 1.0, true, 20.0),
        heat_node("cold-left", 0.0, 1.0, true, 20.0),
    ]
}

fn thermal_nodes() -> Vec<ThermalPlaneNodeInput> {
    thermal_nodes_with_delta(TEMPERATURE_DELTA)
}

fn thermal_nodes_with_delta(temperature_delta: f64) -> Vec<ThermalPlaneNodeInput> {
    vec![
        thermal_node("n0", 0.0, 0.0, temperature_delta),
        thermal_node("n1", 1.0, 0.0, temperature_delta),
        thermal_node("n2", 1.0, 1.0, temperature_delta),
        thermal_node("n3", 0.0, 1.0, temperature_delta),
    ]
}

fn heat_node(id: &str, x: f64, y: f64, fixed: bool, temperature: f64) -> HeatPlaneNodeInput {
    HeatPlaneNodeInput {
        id: id.to_string(),
        x,
        y,
        fix_temperature: fixed,
        temperature,
        heat_load: 0.0,
    }
}

fn thermal_node(id: &str, x: f64, y: f64, temperature_delta: f64) -> ThermalPlaneNodeInput {
    ThermalPlaneNodeInput {
        id: id.to_string(),
        x,
        y,
        fix_x: true,
        fix_y: true,
        load_x: 0.0,
        load_y: 0.0,
        temperature_delta,
    }
}

fn heat_tri(
    id: &str,
    node_i: usize,
    node_j: usize,
    node_k: usize,
) -> HeatPlaneTriangleElementInput {
    HeatPlaneTriangleElementInput {
        id: id.to_string(),
        node_i,
        node_j,
        node_k,
        thickness: THICKNESS,
        conductivity: CONDUCTIVITY,
    }
}

fn heat_tri_with_material(
    id: &str,
    node_i: usize,
    node_j: usize,
    node_k: usize,
    conductivity: f64,
    thickness: f64,
) -> HeatPlaneTriangleElementInput {
    HeatPlaneTriangleElementInput {
        id: id.to_string(),
        node_i,
        node_j,
        node_k,
        thickness,
        conductivity,
    }
}

fn thermal_tri(
    id: &str,
    node_i: usize,
    node_j: usize,
    node_k: usize,
) -> ThermalPlaneTriangleElementInput {
    ThermalPlaneTriangleElementInput {
        id: id.to_string(),
        node_i,
        node_j,
        node_k,
        thickness: THICKNESS,
        youngs_modulus: YOUNGS_MODULUS,
        poisson_ratio: POISSON_RATIO,
        thermal_expansion: THERMAL_EXPANSION,
    }
}

fn assert_close(actual: f64, expected: f64) {
    let scale = expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= TOL * scale,
        "expected {actual} to be close to {expected}",
    );
}

fn max_heat_temperature(result: &impl HeatPlaneSummary) -> f64 {
    result
        .node_temperatures()
        .into_iter()
        .map(f64::abs)
        .fold(0.0_f64, f64::max)
}

fn max_thermal_displacement(result: &impl ThermalPlaneSummary) -> f64 {
    result
        .node_displacements()
        .into_iter()
        .map(|(ux, uy)| magnitude(ux, uy))
        .fold(0.0_f64, f64::max)
}

fn max_thermal_temperature(result: &impl ThermalPlaneSummary) -> f64 {
    result
        .node_temperature_deltas()
        .into_iter()
        .map(f64::abs)
        .fold(0.0_f64, f64::max)
}

trait HeatPlaneSummary {
    fn node_temperatures(&self) -> Vec<f64>;
}

impl HeatPlaneSummary for SolveHeatPlaneTriangle2dResult {
    fn node_temperatures(&self) -> Vec<f64> {
        self.nodes.iter().map(|node| node.temperature).collect()
    }
}

impl HeatPlaneSummary for SolveHeatPlaneQuad2dResult {
    fn node_temperatures(&self) -> Vec<f64> {
        self.nodes.iter().map(|node| node.temperature).collect()
    }
}

trait ThermalPlaneSummary {
    fn node_displacements(&self) -> Vec<(f64, f64)>;
    fn node_temperature_deltas(&self) -> Vec<f64>;
}

impl ThermalPlaneSummary for SolveThermalPlaneTriangle2dResult {
    fn node_displacements(&self) -> Vec<(f64, f64)> {
        self.nodes.iter().map(|node| (node.ux, node.uy)).collect()
    }

    fn node_temperature_deltas(&self) -> Vec<f64> {
        self.nodes
            .iter()
            .map(|node| node.temperature_delta)
            .collect()
    }
}

impl ThermalPlaneSummary for SolveThermalPlaneQuad2dResult {
    fn node_displacements(&self) -> Vec<(f64, f64)> {
        self.nodes.iter().map(|node| (node.ux, node.uy)).collect()
    }

    fn node_temperature_deltas(&self) -> Vec<f64> {
        self.nodes
            .iter()
            .map(|node| node.temperature_delta)
            .collect()
    }
}

fn von_mises(stress_x: f64, stress_y: f64, tau_xy: f64) -> f64 {
    (stress_x * stress_x - stress_x * stress_y + stress_y * stress_y + 3.0 * tau_xy * tau_xy)
        .max(0.0)
        .sqrt()
}

fn in_plane_shear(stress_x: f64, stress_y: f64, tau_xy: f64) -> f64 {
    let half_delta = 0.5 * (stress_x - stress_y);
    (half_delta * half_delta + tau_xy * tau_xy).sqrt()
}

fn triangle_area(a: (f64, f64), b: (f64, f64), c: (f64, f64)) -> f64 {
    0.5 * ((b.0 - a.0) * (c.1 - a.1) - (c.0 - a.0) * (b.1 - a.1)).abs()
}

fn quad_area(a: (f64, f64), b: (f64, f64), c: (f64, f64), d: (f64, f64)) -> f64 {
    triangle_area(a, b, c) + triangle_area(a, c, d)
}

fn magnitude(x: f64, y: f64) -> f64 {
    (x * x + y * y).sqrt()
}
