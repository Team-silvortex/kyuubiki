use kyuubiki_protocol::{
    ElectrostaticPlaneQuadElementResult, HeatPlaneQuadElementResult,
    SolveElectrostaticPlaneQuad2dResult, SolveHeatPlaneQuad2dResult, SolveThermalPlaneQuad2dResult,
    ThermalPlaneNodeResult, ThermalPlaneQuadElementResult,
};
use serde_json::Value;

pub(super) fn electrostatic_peak_summary(
    result: &SolveElectrostaticPlaneQuad2dResult,
    peak_element: &ElectrostaticPlaneQuadElementResult,
) -> Value {
    serde_json::json!({
        "peak_element_id": peak_element.id,
        "peak_electric_field": peak_element.electric_field_magnitude,
        "peak_flux_density": peak_element.electric_flux_density_magnitude,
        "peak_average_potential": peak_element.average_potential,
        "peak_electric_field_x": peak_element.electric_field_x,
        "peak_electric_field_y": peak_element.electric_field_y,
        "peak_flux_density_x": peak_element.electric_flux_density_x,
        "peak_flux_density_y": peak_element.electric_flux_density_y,
        "peak_stored_energy": peak_element.stored_energy,
        "peak_potential_gradient_x": peak_element.potential_gradient_x,
        "peak_potential_gradient_y": peak_element.potential_gradient_y,
        "peak_potential_gradient_magnitude": magnitude2(
            peak_element.potential_gradient_x,
            peak_element.potential_gradient_y,
        ),
        "electrostatic_peak_field": peak_element.electric_field_magnitude,
        "electrostatic_peak_average_potential": peak_element.average_potential,
        "electrostatic_peak_field_x": peak_element.electric_field_x,
        "electrostatic_peak_field_y": peak_element.electric_field_y,
        "electrostatic_peak_potential_gradient_magnitude": magnitude2(
            peak_element.potential_gradient_x,
            peak_element.potential_gradient_y,
        ),
        "electrostatic_field_peak_magnitude": peak_element.electric_field_magnitude,
        "electrostatic_field_peak_x": peak_element.electric_field_x,
        "electrostatic_field_peak_y": peak_element.electric_field_y,
        "electrostatic_field_peak_element_id": peak_element.id,
        "electrostatic_peak_flux_density": peak_element.electric_flux_density_magnitude,
        "electrostatic_peak_flux_density_x": peak_element.electric_flux_density_x,
        "electrostatic_peak_flux_density_y": peak_element.electric_flux_density_y,
        "electrostatic_peak_stored_energy": peak_element.stored_energy,
        "electrostatic_peak_field_id": peak_element.id,
        "electrostatic_potential_max": result.max_potential,
        "max_potential": result.max_potential,
        "max_electric_field": result.max_electric_field,
        "max_flux_density": result.max_flux_density,
        "total_stored_energy": result.total_stored_energy,
    })
}

pub(super) fn thermal_peak_summary(
    result: &SolveHeatPlaneQuad2dResult,
    peak_element: &HeatPlaneQuadElementResult,
) -> Value {
    serde_json::json!({
        "peak_element_id": peak_element.id,
        "peak_heat_flux": peak_element.heat_flux_magnitude,
        "peak_average_temperature": peak_element.average_temperature,
        "peak_heat_flux_x": peak_element.heat_flux_x,
        "peak_heat_flux_y": peak_element.heat_flux_y,
        "peak_temperature_gradient_x": peak_element.temperature_gradient_x,
        "peak_temperature_gradient_y": peak_element.temperature_gradient_y,
        "peak_temperature_gradient_magnitude": magnitude2(
            peak_element.temperature_gradient_x,
            peak_element.temperature_gradient_y,
        ),
        "thermal_peak_flux": peak_element.heat_flux_magnitude,
        "thermal_peak_average_temperature": peak_element.average_temperature,
        "thermal_peak_flux_x": peak_element.heat_flux_x,
        "thermal_peak_flux_y": peak_element.heat_flux_y,
        "thermal_peak_temperature_gradient_magnitude": magnitude2(
            peak_element.temperature_gradient_x,
            peak_element.temperature_gradient_y,
        ),
        "thermal_flux_peak_magnitude": peak_element.heat_flux_magnitude,
        "thermal_flux_peak_x": peak_element.heat_flux_x,
        "thermal_flux_peak_y": peak_element.heat_flux_y,
        "thermal_flux_peak_element_id": peak_element.id,
        "thermal_peak_flux_id": peak_element.id,
        "thermal_temperature_max": result.max_temperature,
        "max_temperature": result.max_temperature,
        "max_heat_flux": result.max_heat_flux,
    })
}

pub(super) fn thermo_peak_summary(
    result: &SolveThermalPlaneQuad2dResult,
    peak_node: &ThermalPlaneNodeResult,
    peak_element: &ThermalPlaneQuadElementResult,
) -> Value {
    let scalar_summary = serde_json::json!({
        "peak_node_id": peak_node.id,
        "peak_displacement": peak_node.displacement_magnitude,
        "peak_displacement_x": peak_node.ux,
        "peak_displacement_y": peak_node.uy,
        "peak_node_temperature_delta": peak_node.temperature_delta,
        "peak_element_id": peak_element.id,
        "peak_von_mises": peak_element.von_mises,
        "peak_stress_x": peak_element.stress_x,
        "peak_stress_y": peak_element.stress_y,
        "peak_tau_xy": peak_element.tau_xy,
        "peak_element_temperature_delta": peak_element.average_temperature_delta,
        "peak_thermal_strain": peak_element.thermal_strain,
        "peak_mechanical_strain_x": peak_element.mechanical_strain_x,
        "peak_mechanical_strain_y": peak_element.mechanical_strain_y,
        "peak_total_strain_x": peak_element.total_strain_x,
        "peak_total_strain_y": peak_element.total_strain_y,
        "peak_gamma_xy": peak_element.gamma_xy,
        "peak_principal_stress_1": peak_element.principal_stress_1,
        "peak_principal_stress_2": peak_element.principal_stress_2,
        "peak_max_in_plane_shear": peak_element.max_in_plane_shear,
        "thermo_peak_displacement": peak_node.displacement_magnitude,
        "thermo_peak_displacement_x": peak_node.ux,
        "thermo_peak_displacement_y": peak_node.uy,
        "thermo_displacement_peak_magnitude": peak_node.displacement_magnitude,
        "thermo_displacement_peak_x": peak_node.ux,
        "thermo_displacement_peak_y": peak_node.uy,
        "thermo_displacement_peak_element_id": peak_node.id,
        "thermo_peak_displacement_id": peak_node.id,
    });
    let stress_summary = serde_json::json!({
        "thermo_peak_stress": peak_element.von_mises,
        "thermo_stress_peak": peak_element.von_mises,
        "thermo_peak_stress_id": peak_element.id,
        "thermo_stress_peak_element_id": peak_element.id,
        "thermo_peak_thermal_strain": peak_element.thermal_strain,
        "thermo_peak_mechanical_strain_x": peak_element.mechanical_strain_x,
        "thermo_peak_mechanical_strain_y": peak_element.mechanical_strain_y,
        "thermo_peak_total_strain_x": peak_element.total_strain_x,
        "thermo_peak_total_strain_y": peak_element.total_strain_y,
        "thermo_peak_gamma_xy": peak_element.gamma_xy,
        "thermo_peak_principal_stress_1": peak_element.principal_stress_1,
        "thermo_peak_principal_stress_2": peak_element.principal_stress_2,
        "thermo_peak_max_in_plane_shear": peak_element.max_in_plane_shear,
        "thermo_temperature_delta_max": result.max_temperature_delta,
        "max_displacement": result.max_displacement,
        "max_stress": result.max_stress,
        "max_temperature_delta": result.max_temperature_delta,
    });

    merge_summary_objects(scalar_summary, stress_summary)
}

fn merge_summary_objects(base: Value, overlay: Value) -> Value {
    let mut merged = match base {
        Value::Object(object) => object,
        _ => serde_json::Map::new(),
    };
    if let Value::Object(overlay) = overlay {
        merged.extend(overlay);
    }
    Value::Object(merged)
}

fn magnitude2(x: f64, y: f64) -> f64 {
    (x.powi(2) + y.powi(2)).sqrt()
}
