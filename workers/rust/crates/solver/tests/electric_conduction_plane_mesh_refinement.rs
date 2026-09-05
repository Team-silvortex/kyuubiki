use kyuubiki_protocol::{
    ElectricConductionPlaneNodeInput, ElectricConductionPlaneQuadElementInput,
    SolveElectricConductionPlaneQuad2dRequest,
};
use kyuubiki_solver::solve_electric_conduction_plane_quad_2d;

const TOL: f64 = 2.0e-9;
const LENGTH: f64 = 2.0;
const WIDTH: f64 = 1.0;
const THICKNESS: f64 = 0.25;
const CONDUCTIVITY: f64 = 4.0;
const VOLTAGE: f64 = 3.0;

#[test]
fn manufactured_linear_potential_is_invariant_under_quad_mesh_refinement() {
    let expected_field = VOLTAGE / LENGTH;
    let expected_current_density = CONDUCTIVITY * expected_field;
    let expected_current = expected_current_density * WIDTH * THICKNESS;
    let expected_power = expected_current * VOLTAGE;

    for divisions in [1, 2, 4, 8] {
        let result = solve_electric_conduction_plane_quad_2d(&grid_request(divisions))
            .unwrap_or_else(|error| panic!("{divisions}x{divisions} grid failed: {error}"));

        assert_eq!(result.nodes.len(), (divisions + 1).pow(2));
        assert_eq!(result.elements.len(), divisions.pow(2));
        for node in &result.nodes {
            assert_close(node.electric_potential_v, VOLTAGE * node.x / LENGTH);
        }
        for element in &result.elements {
            assert_close(element.electric_field_x_v_m, -expected_field);
            assert_close(element.electric_field_y_v_m, 0.0);
            assert_close(element.current_density_x_a_m2, -expected_current_density);
            assert_close(element.current_density_y_a_m2, 0.0);
        }
        assert_close(result.max_electric_field_v_m, expected_field);
        assert_close(result.max_current_density_a_m2, expected_current_density);
        assert_close(result.total_injected_current_a, expected_current);
        assert_close(result.total_extracted_current_a, expected_current);
        assert_close(result.total_joule_power_w, expected_power);
        assert!(result.current_balance_relative_error < TOL);
        assert!(result.free_current_residual_relative_error < TOL);
        assert!(result.power_balance_relative_error < TOL);
    }
}

fn grid_request(divisions: usize) -> SolveElectricConductionPlaneQuad2dRequest {
    let row_width = divisions + 1;
    let mut nodes = Vec::with_capacity(row_width * row_width);
    for row in 0..=divisions {
        let y = WIDTH * row as f64 / divisions as f64;
        for column in 0..=divisions {
            let x = LENGTH * column as f64 / divisions as f64;
            let fixed = column == 0 || column == divisions;
            nodes.push(ElectricConductionPlaneNodeInput {
                id: format!("n-{row}-{column}"),
                x,
                y,
                fix_electric_potential: fixed,
                electric_potential_v: if column == divisions { VOLTAGE } else { 0.0 },
                current_source_a: 0.0,
            });
        }
    }

    let mut elements = Vec::with_capacity(divisions * divisions);
    for row in 0..divisions {
        for column in 0..divisions {
            let lower_left = row * row_width + column;
            let lower_right = lower_left + 1;
            let upper_left = lower_left + row_width;
            let upper_right = upper_left + 1;
            elements.push(ElectricConductionPlaneQuadElementInput {
                id: format!("e-{row}-{column}"),
                node_i: lower_left,
                node_j: lower_right,
                node_k: upper_right,
                node_l: upper_left,
                thickness: THICKNESS,
                electrical_conductivity_s_m: CONDUCTIVITY,
            });
        }
    }

    SolveElectricConductionPlaneQuad2dRequest {
        nodes,
        elements,
        contact_interfaces: vec![],
        terminals: vec![],
    }
}

fn assert_close(actual: f64, expected: f64) {
    let scale = expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= TOL * scale,
        "expected {actual} to be close to {expected}",
    );
}
