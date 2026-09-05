use kyuubiki_protocol::{
    ElectricConductionPlaneNodeInput, ElectricConductionPlaneQuadElementInput,
    SolveElectricConductionPlaneQuad2dRequest,
};
use kyuubiki_solver::solve_electric_conduction_plane_quad_2d;

const TOL: f64 = 2.0e-10;

#[test]
fn electric_conduction_review_recovers_rotated_ohmic_field_current_and_power() {
    for (angle, conductivity) in [(0.0_f64, 5.0), (0.73, 7.5)] {
        let length = 2.0;
        let width = 0.75;
        let thickness = 0.2;
        let voltage = 4.0;
        let axis = [angle.cos(), angle.sin()];
        let request = rotated_rectangle(angle, length, width, thickness, conductivity, voltage);
        let result = solve_electric_conduction_plane_quad_2d(&request)
            .expect("rotated ohmic rectangle should solve");

        let field = voltage / length;
        let current_density = conductivity * field;
        let current = current_density * width * thickness;
        let power = current * voltage;
        assert_eq!(result.nodes.len(), 4);
        assert_eq!(result.elements.len(), 1);
        assert_close(result.max_electric_potential_v, voltage);
        assert_close(result.max_electric_field_v_m, field);
        assert_close(result.max_current_density_a_m2, current_density);
        assert_close(result.total_injected_current_a, current);
        assert_close(result.total_extracted_current_a, current);
        assert_close(result.total_bulk_joule_power_w, power);
        assert_close(result.total_joule_power_w, power);
        assert_close(result.total_electrical_input_power_w, power);
        assert_close(result.total_source_power_w, power);
        assert_close(result.total_dissipated_power_w, power);
        assert!(result.current_balance_relative_error < TOL);
        assert!(result.power_balance_relative_error < TOL);
        assert!(result.source_power_balance_relative_error < TOL);

        let element = &result.elements[0];
        assert_close(element.area_m2, length * width);
        assert_close(element.average_electric_potential_v, voltage / 2.0);
        assert_close(element.electric_potential_gradient_x_v_m, field * axis[0]);
        assert_close(element.electric_potential_gradient_y_v_m, field * axis[1]);
        assert_close(element.electric_field_x_v_m, -field * axis[0]);
        assert_close(element.electric_field_y_v_m, -field * axis[1]);
        assert_close(element.electric_field_magnitude_v_m, field);
        assert_close(element.current_density_x_a_m2, -current_density * axis[0]);
        assert_close(element.current_density_y_a_m2, -current_density * axis[1]);
        assert_close(element.current_density_magnitude_a_m2, current_density);
        assert_close(
            element.volumetric_joule_heating_w_m3,
            conductivity * field.powi(2),
        );
        assert_close(element.joule_power_w, power);

        let mut clockwise = request.clone();
        clockwise.elements[0].node_j = 3;
        clockwise.elements[0].node_l = 1;
        let clockwise_result = solve_electric_conduction_plane_quad_2d(&clockwise)
            .expect("clockwise rectangle should preserve the physical response");
        assert_close(clockwise_result.max_electric_field_v_m, field);
        assert_close(clockwise_result.total_injected_current_a, current);
        assert_close(clockwise_result.total_joule_power_w, power);
    }
}

fn rotated_rectangle(
    angle: f64,
    length: f64,
    width: f64,
    thickness: f64,
    conductivity: f64,
    voltage: f64,
) -> SolveElectricConductionPlaneQuad2dRequest {
    let axis = [angle.cos(), angle.sin()];
    let normal = [-axis[1], axis[0]];
    let origin = [0.4, -0.2];
    let left_top = add(origin, scale(normal, width));
    let right_bottom = add(origin, scale(axis, length));
    let right_top = add(right_bottom, scale(normal, width));
    SolveElectricConductionPlaneQuad2dRequest {
        nodes: vec![
            node("left-bottom", origin, 0.0),
            node("right-bottom", right_bottom, voltage),
            node("right-top", right_top, voltage),
            node("left-top", left_top, 0.0),
        ],
        elements: vec![ElectricConductionPlaneQuadElementInput {
            id: "conductor".to_string(),
            node_i: 0,
            node_j: 1,
            node_k: 2,
            node_l: 3,
            thickness,
            electrical_conductivity_s_m: conductivity,
        }],
        contact_interfaces: vec![],
        terminals: vec![],
    }
}

fn node(id: &str, point: [f64; 2], potential: f64) -> ElectricConductionPlaneNodeInput {
    ElectricConductionPlaneNodeInput {
        id: id.to_string(),
        x: point[0],
        y: point[1],
        fix_electric_potential: true,
        electric_potential_v: potential,
        current_source_a: 0.0,
    }
}

fn add(left: [f64; 2], right: [f64; 2]) -> [f64; 2] {
    [left[0] + right[0], left[1] + right[1]]
}

fn scale(vector: [f64; 2], factor: f64) -> [f64; 2] {
    [vector[0] * factor, vector[1] * factor]
}

fn assert_close(actual: f64, expected: f64) {
    let scale = expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= TOL * scale,
        "expected {actual} to be close to {expected}",
    );
}
