use kyuubiki_protocol::{
    ElectricConductionPlaneQuadElementInput, SolveElectricConductionPlaneQuad2dRequest,
    SolveElectricConductionPlaneQuad2dResult, SolveHeatPlaneQuad2dRequest,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub const COMPOSITE_CURRENT_TO_HEAT_PROJECTION_SCHEMA_VERSION: &str =
    "kyuubiki.composite-current-to-heat-projection/v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompositeCurrentConductionFeedbackSpec {
    pub regions: Vec<CompositeCurrentConductionRegionSpec>,
    pub parameter_source: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompositeCurrentConductionRegionSpec {
    pub element_id: String,
    pub reference_resistivity_ohm_m: f64,
    pub reference_temperature_c: f64,
    pub resistivity_temperature_coefficient_1_k: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompositeCurrentRegionProjection {
    pub element_id: String,
    pub coupling_temperature_c: f64,
    pub electrical_conductivity_s_m: f64,
    pub electric_field_magnitude_v_m: f64,
    pub current_density_magnitude_a_m2: f64,
    pub volumetric_joule_heating_w_m3: f64,
    pub source_volume_m3: f64,
    pub joule_power_w: f64,
    pub distributed_heat_load_w: f64,
    pub energy_balance_relative_error: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompositeCurrentToHeatProjection {
    pub schema_version: String,
    pub model: String,
    pub status: String,
    pub parameter_source: String,
    pub boundary_voltage_span_v: f64,
    pub max_current_density_a_m2: f64,
    pub total_joule_loss_w: f64,
    pub distributed_total_heat_load_w: f64,
    pub energy_balance_relative_error: f64,
    pub regions: Vec<CompositeCurrentRegionProjection>,
    pub assumptions: Vec<String>,
}

pub fn temperature_adjusted_composite_current_request(
    base: &SolveElectricConductionPlaneQuad2dRequest,
    spec: &CompositeCurrentConductionFeedbackSpec,
    coupling_temperatures_c: &[(String, f64)],
) -> Result<SolveElectricConductionPlaneQuad2dRequest, String> {
    validate_spec(spec)?;
    let mut request = base.clone();
    for region in &spec.regions {
        let temperature = coupling_temperature(coupling_temperatures_c, &region.element_id)?;
        let scale = 1.0
            + region.resistivity_temperature_coefficient_1_k
                * (temperature - region.reference_temperature_c);
        let resistivity = region.reference_resistivity_ohm_m * scale;
        if !resistivity.is_finite() || resistivity <= 0.0 {
            return Err(format!(
                "current feedback produced invalid resistivity for {}",
                region.element_id
            ));
        }
        let element = request
            .elements
            .iter_mut()
            .find(|element| element.id == region.element_id)
            .ok_or_else(|| {
                format!(
                    "current conduction model is missing element {}",
                    region.element_id
                )
            })?;
        element.electrical_conductivity_s_m = 1.0 / resistivity;
    }
    Ok(request)
}

pub fn project_composite_solved_current_to_heat(
    current: &SolveElectricConductionPlaneQuad2dResult,
    heat_seed: &SolveHeatPlaneQuad2dRequest,
    spec: &CompositeCurrentConductionFeedbackSpec,
    coupling_temperatures_c: &[(String, f64)],
) -> Result<
    (
        SolveHeatPlaneQuad2dRequest,
        CompositeCurrentToHeatProjection,
    ),
    String,
> {
    validate_spec(spec)?;
    let before = heat_seed
        .nodes
        .iter()
        .map(|node| node.heat_load)
        .sum::<f64>();
    let mut request = heat_seed.clone();
    let mut regions = Vec::with_capacity(spec.regions.len());
    for region in &spec.regions {
        let source = current
            .elements
            .iter()
            .find(|element| element.id == region.element_id)
            .ok_or_else(|| format!("current result is missing element {}", region.element_id))?;
        let input = current
            .input
            .elements
            .iter()
            .find(|element| element.id == region.element_id)
            .ok_or_else(|| format!("current input is missing element {}", region.element_id))?;
        let target = request
            .elements
            .iter()
            .find(|element| element.id == region.element_id)
            .ok_or_else(|| {
                format!(
                    "heat model is missing current element {}",
                    region.element_id
                )
            })?;
        validate_geometry(current, heat_seed, input, target)?;
        let target_nodes = [target.node_i, target.node_j, target.node_k, target.node_l];
        for index in target_nodes {
            request
                .nodes
                .get_mut(index)
                .ok_or_else(|| format!("heat element {} has an unknown node", target.id))?
                .heat_load += source.joule_power_w / 4.0;
        }
        regions.push(CompositeCurrentRegionProjection {
            element_id: region.element_id.clone(),
            coupling_temperature_c: coupling_temperature(
                coupling_temperatures_c,
                &region.element_id,
            )?,
            electrical_conductivity_s_m: input.electrical_conductivity_s_m,
            electric_field_magnitude_v_m: source.electric_field_magnitude_v_m,
            current_density_magnitude_a_m2: source.current_density_magnitude_a_m2,
            volumetric_joule_heating_w_m3: source.volumetric_joule_heating_w_m3,
            source_volume_m3: source.area_m2 * input.thickness,
            joule_power_w: source.joule_power_w,
            distributed_heat_load_w: source.joule_power_w,
            energy_balance_relative_error: 0.0,
        });
    }
    let total_joule_loss_w = regions
        .iter()
        .map(|region| region.joule_power_w)
        .sum::<f64>();
    let after = request.nodes.iter().map(|node| node.heat_load).sum::<f64>();
    let distributed_total_heat_load_w = after - before;
    let energy_balance_relative_error =
        relative_error(distributed_total_heat_load_w, total_joule_loss_w);
    if energy_balance_relative_error > 1.0e-12 {
        return Err("solved current heat projection lost energy".to_string());
    }
    let potentials = current
        .nodes
        .iter()
        .map(|node| node.electric_potential_v)
        .collect::<Vec<_>>();
    let boundary_voltage_span_v = potentials.iter().copied().fold(f64::NEG_INFINITY, f64::max)
        - potentials.iter().copied().fold(f64::INFINITY, f64::min);
    Ok((
        request,
        CompositeCurrentToHeatProjection {
            schema_version: COMPOSITE_CURRENT_TO_HEAT_PROJECTION_SCHEMA_VERSION.to_string(),
            model: "solved_steady_current_density_sigma_e_squared".to_string(),
            status: "pass".to_string(),
            parameter_source: spec.parameter_source.clone(),
            boundary_voltage_span_v,
            max_current_density_a_m2: current.max_current_density_a_m2,
            total_joule_loss_w,
            distributed_total_heat_load_w,
            energy_balance_relative_error,
            regions,
            assumptions: vec![
                "Steady scalar electrical conductivity is solved on a two-dimensional quad mesh."
                    .to_string(),
                "Joule heating uses the solved local sigma times electric-field magnitude squared."
                    .to_string(),
                "Contact resistance and terminal impedance remain separate interface models."
                    .to_string(),
            ],
        },
    ))
}

fn validate_spec(spec: &CompositeCurrentConductionFeedbackSpec) -> Result<(), String> {
    if spec.regions.is_empty() || spec.parameter_source.trim().is_empty() {
        return Err("current feedback requires regions and a parameter source".to_string());
    }
    let mut ids = HashSet::new();
    if spec.regions.iter().any(|region| {
        region.element_id.trim().is_empty()
            || !ids.insert(region.element_id.as_str())
            || !region.reference_resistivity_ohm_m.is_finite()
            || region.reference_resistivity_ohm_m <= 0.0
            || !region.reference_temperature_c.is_finite()
            || !region.resistivity_temperature_coefficient_1_k.is_finite()
    }) {
        return Err("current feedback region parameters are invalid".to_string());
    }
    Ok(())
}

fn coupling_temperature(values: &[(String, f64)], element_id: &str) -> Result<f64, String> {
    values
        .iter()
        .find(|(id, _)| id == element_id)
        .map(|(_, value)| *value)
        .filter(|value| value.is_finite())
        .ok_or_else(|| format!("missing current coupling temperature for {element_id}"))
}

fn validate_geometry(
    current: &SolveElectricConductionPlaneQuad2dResult,
    heat: &SolveHeatPlaneQuad2dRequest,
    source: &ElectricConductionPlaneQuadElementInput,
    target: &kyuubiki_protocol::HeatPlaneQuadElementInput,
) -> Result<(), String> {
    let source_indices = [source.node_i, source.node_j, source.node_k, source.node_l];
    let target_indices = [target.node_i, target.node_j, target.node_k, target.node_l];
    for source_index in source_indices {
        let source_node = current
            .input
            .nodes
            .get(source_index)
            .ok_or_else(|| format!("current element {} has an unknown node", source.id))?;
        let matched = target_indices.iter().any(|index| {
            heat.nodes.get(*index).is_some_and(|target_node| {
                (target_node.x - source_node.x).hypot(target_node.y - source_node.y) <= 1.0e-12
            })
        });
        if !matched {
            return Err(format!(
                "current and heat element {} geometry does not match",
                source.id
            ));
        }
    }
    Ok(())
}

fn relative_error(actual: f64, expected: f64) -> f64 {
    if expected.abs() <= f64::EPSILON {
        actual.abs()
    } else {
        (actual - expected).abs() / expected.abs()
    }
}
