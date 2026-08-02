use crate::composite_heat_element_mean_temperature;
use kyuubiki_protocol::{SolveHeatPlaneQuad2dResult, SolveThermalPlaneQuad2dRequest};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub const COMPOSITE_THERMAL_EXPANSION_PROJECTION_SCHEMA_VERSION: &str =
    "kyuubiki.composite-thermal-expansion-projection/v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompositeThermalExpansionFeedbackSpec {
    pub regions: Vec<CompositeThermalExpansionRegionSpec>,
    pub parameter_source: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompositeThermalExpansionRegionSpec {
    pub element_id: String,
    pub reference_temperature_c: f64,
    pub temperature_coefficient_1_k: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompositeThermalExpansionRegionUpdate {
    pub element_id: String,
    pub mean_temperature_c: f64,
    pub reference_thermal_expansion_1_k: f64,
    pub adjusted_thermal_expansion_1_k: f64,
    pub relative_change: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompositeThermalExpansionProjection {
    pub schema_version: String,
    pub model: String,
    pub status: String,
    pub parameter_source: String,
    pub declared_region_count: usize,
    pub updated_region_count: usize,
    pub coverage_fraction: f64,
    pub max_relative_change: f64,
    pub updates: Vec<CompositeThermalExpansionRegionUpdate>,
}

pub fn project_composite_temperature_dependent_expansion(
    heat: &SolveHeatPlaneQuad2dResult,
    thermal_seed: &SolveThermalPlaneQuad2dRequest,
    spec: &CompositeThermalExpansionFeedbackSpec,
) -> Result<
    (
        SolveThermalPlaneQuad2dRequest,
        CompositeThermalExpansionProjection,
    ),
    String,
> {
    validate_spec(spec)?;
    let mut request = thermal_seed.clone();
    let mut updates = Vec::with_capacity(spec.regions.len());
    for region in &spec.regions {
        let heat_element = heat
            .input
            .elements
            .iter()
            .find(|element| element.id == region.element_id)
            .ok_or_else(|| {
                format!(
                    "thermal expansion projection is missing heat element {}",
                    region.element_id
                )
            })?;
        let thermal_element = request
            .elements
            .iter_mut()
            .find(|element| element.id == region.element_id)
            .ok_or_else(|| {
                format!(
                    "thermal expansion projection is missing structural element {}",
                    region.element_id
                )
            })?;
        if [
            heat_element.node_i,
            heat_element.node_j,
            heat_element.node_k,
            heat_element.node_l,
        ] != [
            thermal_element.node_i,
            thermal_element.node_j,
            thermal_element.node_k,
            thermal_element.node_l,
        ] {
            return Err(format!(
                "thermal expansion projection element {} topology does not match",
                region.element_id
            ));
        }
        let mean_temperature_c = composite_heat_element_mean_temperature(heat, &region.element_id)?;
        let reference = thermal_element.thermal_expansion;
        if !reference.is_finite() {
            return Err(format!(
                "thermal expansion projection element {} has invalid reference coefficient",
                region.element_id
            ));
        }
        let scale = 1.0
            + region.temperature_coefficient_1_k
                * (mean_temperature_c - region.reference_temperature_c);
        let adjusted = reference * scale;
        if !adjusted.is_finite() {
            return Err(format!(
                "thermal expansion projection element {} produced an invalid coefficient",
                region.element_id
            ));
        }
        thermal_element.thermal_expansion = adjusted;
        updates.push(CompositeThermalExpansionRegionUpdate {
            element_id: region.element_id.clone(),
            mean_temperature_c,
            reference_thermal_expansion_1_k: reference,
            adjusted_thermal_expansion_1_k: adjusted,
            relative_change: relative_change(adjusted, reference),
        });
    }
    let max_relative_change = updates
        .iter()
        .map(|update| update.relative_change)
        .fold(0.0_f64, f64::max);
    let updated_region_count = updates.len();
    let declared_region_count = spec.regions.len();
    Ok((
        request,
        CompositeThermalExpansionProjection {
            schema_version: COMPOSITE_THERMAL_EXPANSION_PROJECTION_SCHEMA_VERSION.to_string(),
            model: "regional_linear_temperature_dependent_thermal_expansion".to_string(),
            status: "pass".to_string(),
            parameter_source: spec.parameter_source.clone(),
            declared_region_count,
            updated_region_count,
            coverage_fraction: updated_region_count as f64 / declared_region_count as f64,
            max_relative_change,
            updates,
        },
    ))
}

fn validate_spec(spec: &CompositeThermalExpansionFeedbackSpec) -> Result<(), String> {
    let mut ids = HashSet::new();
    if spec.parameter_source.trim().is_empty()
        || spec.regions.is_empty()
        || spec.regions.iter().any(|region| {
            region.element_id.trim().is_empty()
                || !ids.insert(region.element_id.as_str())
                || !region.reference_temperature_c.is_finite()
                || !region.temperature_coefficient_1_k.is_finite()
        })
    {
        return Err("thermal expansion feedback specification is invalid".to_string());
    }
    Ok(())
}

fn relative_change(current: f64, reference: f64) -> f64 {
    if reference.abs() <= f64::EPSILON {
        (current - reference).abs()
    } else {
        (current - reference).abs() / reference.abs()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CompositeThermalExpansionFeedbackSpec, CompositeThermalExpansionRegionSpec,
        project_composite_temperature_dependent_expansion,
    };
    use kyuubiki_protocol::{SolveHeatPlaneQuad2dResult, SolveThermalPlaneQuad2dRequest};
    use serde_json::json;

    #[test]
    fn projects_local_temperature_into_structural_expansion() {
        let heat: SolveHeatPlaneQuad2dResult = serde_json::from_value(json!({
            "input": {
                "nodes": [],
                "elements": [{"id": "core", "node_i": 0, "node_j": 1, "node_k": 2, "node_l": 3, "thickness": 1.0, "conductivity": 1.0}]
            },
            "nodes": [
                {"index": 0, "id": "n0", "x": 0.0, "y": 0.0, "temperature": 45.0, "heat_load": 0.0},
                {"index": 1, "id": "n1", "x": 1.0, "y": 0.0, "temperature": 45.0, "heat_load": 0.0},
                {"index": 2, "id": "n2", "x": 1.0, "y": 1.0, "temperature": 45.0, "heat_load": 0.0},
                {"index": 3, "id": "n3", "x": 0.0, "y": 1.0, "temperature": 45.0, "heat_load": 0.0}
            ],
            "elements": [],
            "max_temperature": 45.0,
            "max_heat_flux": 0.0,
            "total_abs_heat_flow_rate": 0.0
        }))
        .expect("heat result");
        let thermal: SolveThermalPlaneQuad2dRequest = serde_json::from_value(json!({
            "nodes": [],
            "elements": [{"id": "core", "node_i": 0, "node_j": 1, "node_k": 2, "node_l": 3, "thickness": 1.0, "youngs_modulus": 1.0, "poisson_ratio": 0.3, "thermal_expansion": 10.0e-6}]
        }))
        .expect("thermal request");
        let spec = CompositeThermalExpansionFeedbackSpec {
            regions: vec![CompositeThermalExpansionRegionSpec {
                element_id: "core".to_string(),
                reference_temperature_c: 35.0,
                temperature_coefficient_1_k: 1.0e-3,
            }],
            parameter_source: "screening_sensitivity_not_material_card".to_string(),
        };

        let (adjusted, projection) =
            project_composite_temperature_dependent_expansion(&heat, &thermal, &spec)
                .expect("projection");

        assert!((adjusted.elements[0].thermal_expansion - 10.1e-6).abs() < 1.0e-18);
        assert_eq!(projection.coverage_fraction, 1.0);
        assert!((projection.max_relative_change - 0.01).abs() < 1.0e-12);
    }
}
