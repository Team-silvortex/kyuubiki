use kyuubiki_protocol::SolveHeatPlaneQuad2dRequest;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub const COMPOSITE_JOULE_HEATING_PROJECTION_SCHEMA_VERSION: &str =
    "kyuubiki.composite-joule-heating-projection/v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompositeJouleHeatingSpec {
    pub regions: Vec<CompositeJouleHeatingRegionSpec>,
    pub parameter_source: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompositeJouleHeatingRegionSpec {
    pub element_id: String,
    pub current_a: f64,
    pub path_length_m: f64,
    pub cross_section_area_m2: f64,
    pub reference_resistivity_ohm_m: f64,
    pub reference_temperature_c: f64,
    pub resistivity_temperature_coefficient_1_k: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompositeJouleHeatingRegionProjection {
    pub element_id: String,
    pub coupling_temperature_c: f64,
    pub current_a: f64,
    pub resistivity_ohm_m: f64,
    pub resistance_ohm: f64,
    pub power_w: f64,
    pub distributed_heat_load_w: f64,
    pub energy_balance_relative_error: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompositeJouleHeatingProjection {
    pub schema_version: String,
    pub model: String,
    pub status: String,
    pub parameter_source: String,
    pub total_joule_loss_w: f64,
    pub distributed_total_heat_load_w: f64,
    pub energy_balance_relative_error: f64,
    pub regions: Vec<CompositeJouleHeatingRegionProjection>,
    pub assumptions: Vec<String>,
}

pub fn project_composite_joule_heating_to_heat(
    heat_seed: &SolveHeatPlaneQuad2dRequest,
    spec: &CompositeJouleHeatingSpec,
    coupling_temperatures_c: &[(String, f64)],
) -> Result<(SolveHeatPlaneQuad2dRequest, CompositeJouleHeatingProjection), String> {
    validate_spec(spec, coupling_temperatures_c)?;
    let before = total_heat_load(heat_seed)?;
    let mut request = heat_seed.clone();
    let mut regions = Vec::with_capacity(spec.regions.len());

    for region in &spec.regions {
        let coupling_temperature_c = coupling_temperatures_c
            .iter()
            .find(|(element_id, _)| element_id == &region.element_id)
            .map(|(_, temperature)| *temperature)
            .ok_or_else(|| format!("missing Joule coupling state for {}", region.element_id))?;
        let temperature_scale = 1.0
            + region.resistivity_temperature_coefficient_1_k
                * (coupling_temperature_c - region.reference_temperature_c);
        let resistivity_ohm_m = region.reference_resistivity_ohm_m * temperature_scale;
        if !resistivity_ohm_m.is_finite() || resistivity_ohm_m <= 0.0 {
            return Err(format!(
                "Joule heating produced invalid resistivity for {}",
                region.element_id
            ));
        }
        let resistance_ohm =
            resistivity_ohm_m * region.path_length_m / region.cross_section_area_m2;
        let power_w = region.current_a.powi(2) * resistance_ohm;
        if !resistance_ohm.is_finite()
            || resistance_ohm <= 0.0
            || !power_w.is_finite()
            || power_w < 0.0
        {
            return Err(format!(
                "Joule heating produced invalid power for {}",
                region.element_id
            ));
        }
        let node_indices = request
            .elements
            .iter()
            .find(|element| element.id == region.element_id)
            .map(|element| {
                [
                    element.node_i,
                    element.node_j,
                    element.node_k,
                    element.node_l,
                ]
            })
            .ok_or_else(|| format!("heat model is missing Joule element {}", region.element_id))?;
        for index in node_indices {
            let node = request.nodes.get_mut(index).ok_or_else(|| {
                format!(
                    "Joule element {} references unknown node {index}",
                    region.element_id
                )
            })?;
            node.heat_load += power_w / 4.0;
        }
        regions.push(CompositeJouleHeatingRegionProjection {
            element_id: region.element_id.clone(),
            coupling_temperature_c,
            current_a: region.current_a,
            resistivity_ohm_m,
            resistance_ohm,
            power_w,
            distributed_heat_load_w: power_w,
            energy_balance_relative_error: 0.0,
        });
    }

    let total_joule_loss_w = regions.iter().map(|region| region.power_w).sum::<f64>();
    let distributed_total_heat_load_w = total_heat_load(&request)? - before;
    let energy_balance_relative_error =
        relative_error(distributed_total_heat_load_w, total_joule_loss_w);
    if energy_balance_relative_error > 1.0e-12 {
        return Err("composite Joule heat-load distribution lost energy".to_string());
    }
    Ok((
        request,
        CompositeJouleHeatingProjection {
            schema_version: COMPOSITE_JOULE_HEATING_PROJECTION_SCHEMA_VERSION.to_string(),
            model: "temperature_dependent_prescribed_current_i_squared_r".to_string(),
            status: "pass".to_string(),
            parameter_source: spec.parameter_source.clone(),
            total_joule_loss_w,
            distributed_total_heat_load_w,
            energy_balance_relative_error,
            regions,
            assumptions: vec![
                "Each declared conductor follows a uniform prescribed-current path.".to_string(),
                "Temperature-dependent scalar resistivity is linearized about its reference temperature."
                    .to_string(),
                "Joule power is lumped consistently to the four nodes of each conductor element."
                    .to_string(),
            ],
        },
    ))
}

fn validate_spec(
    spec: &CompositeJouleHeatingSpec,
    coupling_temperatures_c: &[(String, f64)],
) -> Result<(), String> {
    if spec.regions.is_empty() || spec.parameter_source.trim().is_empty() {
        return Err("Joule heating requires regions and a parameter source".to_string());
    }
    let mut ids = HashSet::new();
    for region in &spec.regions {
        if region.element_id.trim().is_empty()
            || !ids.insert(region.element_id.as_str())
            || !region.current_a.is_finite()
            || region.current_a < 0.0
            || !region.path_length_m.is_finite()
            || region.path_length_m <= 0.0
            || !region.cross_section_area_m2.is_finite()
            || region.cross_section_area_m2 <= 0.0
            || !region.reference_resistivity_ohm_m.is_finite()
            || region.reference_resistivity_ohm_m <= 0.0
            || !region.reference_temperature_c.is_finite()
            || !region.resistivity_temperature_coefficient_1_k.is_finite()
        {
            return Err("Joule heating region parameters are invalid".to_string());
        }
    }
    if coupling_temperatures_c
        .iter()
        .any(|(id, temperature)| id.trim().is_empty() || !temperature.is_finite())
    {
        return Err("Joule heating coupling temperatures are invalid".to_string());
    }
    Ok(())
}

fn total_heat_load(request: &SolveHeatPlaneQuad2dRequest) -> Result<f64, String> {
    let total = request.nodes.iter().map(|node| node.heat_load).sum::<f64>();
    if !total.is_finite() {
        return Err("composite heat request contains non-finite nodal load".to_string());
    }
    Ok(total)
}

fn relative_error(actual: f64, expected: f64) -> f64 {
    if expected.abs() <= f64::EPSILON {
        actual.abs()
    } else {
        (actual - expected).abs() / expected.abs()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CompositeJouleHeatingRegionSpec, CompositeJouleHeatingSpec,
        project_composite_joule_heating_to_heat,
    };
    use kyuubiki_protocol::SolveHeatPlaneQuad2dRequest;
    use serde_json::json;

    #[test]
    fn prescribed_current_projection_tracks_temperature_and_conserves_power() {
        let request: SolveHeatPlaneQuad2dRequest = serde_json::from_value(json!({
            "nodes": [
                {"id":"n0","x":0.0,"y":0.0,"fix_temperature":false,"temperature":35.0,"heat_load":0.01},
                {"id":"n1","x":0.03,"y":0.0,"fix_temperature":false,"temperature":35.0,"heat_load":0.01},
                {"id":"n2","x":0.03,"y":0.03,"fix_temperature":false,"temperature":35.0,"heat_load":0.01},
                {"id":"n3","x":0.0,"y":0.03,"fix_temperature":false,"temperature":35.0,"heat_load":0.01}
            ],
            "elements": [{"id":"conductor_left","node_i":0,"node_j":1,"node_k":2,"node_l":3,"thickness":0.001,"conductivity":390.0}]
        }))
        .expect("heat request");
        let spec = CompositeJouleHeatingSpec {
            regions: vec![CompositeJouleHeatingRegionSpec {
                element_id: "conductor_left".to_string(),
                current_a: 2.0,
                path_length_m: 0.03,
                cross_section_area_m2: 3.0e-5,
                reference_resistivity_ohm_m: 1.68e-8,
                reference_temperature_c: 35.0,
                resistivity_temperature_coefficient_1_k: 3.93e-3,
            }],
            parameter_source: "test_fixture".to_string(),
        };

        let (projected, evidence) = project_composite_joule_heating_to_heat(
            &request,
            &spec,
            &[("conductor_left".to_string(), 45.0)],
        )
        .expect("Joule projection");

        let expected = 4.0 * 1.68e-8 * 1.0393 * 0.03 / 3.0e-5;
        assert!((evidence.total_joule_loss_w - expected).abs() < 1.0e-15);
        assert!(evidence.energy_balance_relative_error <= 1.0e-12);
        assert!(
            (projected
                .nodes
                .iter()
                .map(|node| node.heat_load)
                .sum::<f64>()
                - 0.04
                - expected)
                .abs()
                < 1.0e-15
        );
    }
}
