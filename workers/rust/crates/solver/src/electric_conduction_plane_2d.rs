use crate::solve_heat_plane_quad_2d;
use kyuubiki_protocol::{
    ElectricConductionPlaneNodeResult, ElectricConductionPlaneQuadElementResult,
    HeatPlaneNodeInput, HeatPlaneQuadElementInput, SolveElectricConductionPlaneQuad2dRequest,
    SolveElectricConductionPlaneQuad2dResult, SolveHeatPlaneQuad2dRequest,
};

pub fn solve_electric_conduction_plane_quad_2d(
    request: &SolveElectricConductionPlaneQuad2dRequest,
) -> Result<SolveElectricConductionPlaneQuad2dResult, String> {
    validate_request(request)?;
    let heat_request = SolveHeatPlaneQuad2dRequest {
        nodes: request
            .nodes
            .iter()
            .map(|node| HeatPlaneNodeInput {
                id: node.id.clone(),
                x: node.x,
                y: node.y,
                fix_temperature: node.fix_electric_potential,
                temperature: node.electric_potential_v,
                heat_load: node.current_source_a,
            })
            .collect(),
        elements: request
            .elements
            .iter()
            .map(|element| HeatPlaneQuadElementInput {
                id: element.id.clone(),
                node_i: element.node_i,
                node_j: element.node_j,
                node_k: element.node_k,
                node_l: element.node_l,
                thickness: element.thickness,
                conductivity: element.electrical_conductivity_s_m,
            })
            .collect(),
    };
    let solved = solve_heat_plane_quad_2d(&heat_request)
        .map_err(|error| format!("electric conduction solve failed: {error}"))?;
    let nodes = solved
        .nodes
        .iter()
        .map(|node| ElectricConductionPlaneNodeResult {
            index: node.index,
            id: node.id.clone(),
            x: node.x,
            y: node.y,
            electric_potential_v: node.temperature,
            current_source_a: node.heat_load,
        })
        .collect::<Vec<_>>();
    let elements = solved
        .elements
        .iter()
        .zip(request.elements.iter())
        .map(|(solved, input)| {
            let electric_field_x_v_m = -solved.temperature_gradient_x;
            let electric_field_y_v_m = -solved.temperature_gradient_y;
            let electric_field_magnitude_v_m = electric_field_x_v_m.hypot(electric_field_y_v_m);
            let current_density_x_a_m2 = solved.heat_flux_x;
            let current_density_y_a_m2 = solved.heat_flux_y;
            let current_density_magnitude_a_m2 =
                current_density_x_a_m2.hypot(current_density_y_a_m2);
            let volumetric_joule_heating_w_m3 =
                input.electrical_conductivity_s_m * electric_field_magnitude_v_m.powi(2);
            let joule_power_w = volumetric_joule_heating_w_m3 * solved.area * input.thickness;
            ElectricConductionPlaneQuadElementResult {
                index: solved.index,
                id: solved.id.clone(),
                node_i: solved.node_i,
                node_j: solved.node_j,
                node_k: solved.node_k,
                node_l: solved.node_l,
                area_m2: solved.area,
                average_electric_potential_v: solved.average_temperature,
                electric_potential_gradient_x_v_m: solved.temperature_gradient_x,
                electric_potential_gradient_y_v_m: solved.temperature_gradient_y,
                electric_field_x_v_m,
                electric_field_y_v_m,
                electric_field_magnitude_v_m,
                current_density_x_a_m2,
                current_density_y_a_m2,
                current_density_magnitude_a_m2,
                volumetric_joule_heating_w_m3,
                joule_power_w,
            }
        })
        .collect::<Vec<_>>();
    let max_electric_potential_v = nodes
        .iter()
        .map(|node| node.electric_potential_v.abs())
        .fold(0.0_f64, f64::max);
    let max_electric_field_v_m = elements
        .iter()
        .map(|element| element.electric_field_magnitude_v_m)
        .fold(0.0_f64, f64::max);
    let max_current_density_a_m2 = elements
        .iter()
        .map(|element| element.current_density_magnitude_a_m2)
        .fold(0.0_f64, f64::max);
    let total_joule_power_w = elements.iter().map(|element| element.joule_power_w).sum();
    Ok(SolveElectricConductionPlaneQuad2dResult {
        input: request.clone(),
        nodes,
        elements,
        max_electric_potential_v,
        max_electric_field_v_m,
        max_current_density_a_m2,
        total_joule_power_w,
    })
}

fn validate_request(request: &SolveElectricConductionPlaneQuad2dRequest) -> Result<(), String> {
    if request.nodes.is_empty() || request.elements.is_empty() {
        return Err("electric conduction model requires nodes and elements".to_string());
    }
    if !request.nodes.iter().any(|node| node.fix_electric_potential) {
        return Err("electric conduction model requires a fixed electric potential".to_string());
    }
    if request.nodes.iter().any(|node| {
        node.id.trim().is_empty()
            || !node.x.is_finite()
            || !node.y.is_finite()
            || !node.electric_potential_v.is_finite()
            || !node.current_source_a.is_finite()
    }) {
        return Err("electric conduction node parameters are invalid".to_string());
    }
    if request.elements.iter().any(|element| {
        element.id.trim().is_empty()
            || !element.thickness.is_finite()
            || element.thickness <= 0.0
            || !element.electrical_conductivity_s_m.is_finite()
            || element.electrical_conductivity_s_m <= 0.0
    }) {
        return Err("electric conduction element parameters are invalid".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::solve_electric_conduction_plane_quad_2d;
    use kyuubiki_protocol::{
        ElectricConductionPlaneNodeInput, ElectricConductionPlaneQuadElementInput,
        SolveElectricConductionPlaneQuad2dRequest,
    };

    #[test]
    fn voltage_driven_conductor_matches_uniform_current_and_joule_power() {
        let resistivity_ohm_m = 1.68e-8;
        let conductivity_s_m = 1.0 / resistivity_ohm_m;
        let voltage_v = 2.0 * resistivity_ohm_m * 0.03 / 3.0e-5;
        let result = solve_electric_conduction_plane_quad_2d(&request(conductivity_s_m, voltage_v))
            .expect("electric conduction solve");

        assert!((result.max_current_density_a_m2 - 2.0 / 3.0e-5).abs() < 1.0e-8);
        assert!((result.total_joule_power_w - 2.0_f64.powi(2) * 1.68e-5).abs() < 1.0e-15);
        assert_eq!(result.elements[0].joule_power_w, result.total_joule_power_w);
    }

    fn request(conductivity_s_m: f64, voltage_v: f64) -> SolveElectricConductionPlaneQuad2dRequest {
        let points = [(0.0, 0.0), (0.03, 0.0), (0.03, 0.03), (0.0, 0.03)];
        SolveElectricConductionPlaneQuad2dRequest {
            nodes: points
                .iter()
                .enumerate()
                .map(|(index, (x, y))| ElectricConductionPlaneNodeInput {
                    id: format!("n{index}"),
                    x: *x,
                    y: *y,
                    fix_electric_potential: true,
                    electric_potential_v: if matches!(index, 1 | 2) {
                        voltage_v
                    } else {
                        0.0
                    },
                    current_source_a: 0.0,
                })
                .collect(),
            elements: vec![ElectricConductionPlaneQuadElementInput {
                id: "conductor".to_string(),
                node_i: 0,
                node_j: 1,
                node_k: 2,
                node_l: 3,
                thickness: 0.001,
                electrical_conductivity_s_m: conductivity_s_m,
            }],
        }
    }
}
