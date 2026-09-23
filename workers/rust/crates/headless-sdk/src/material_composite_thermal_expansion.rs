use crate::composite_feedback_relative_change;
use crate::material_composite_temperature_map::map_composite_temperatures;
use kyuubiki_protocol::{SolveHeatPlaneQuad2dResult, SolveThermalPlaneQuad2dRequest};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

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
    let mapping = map_composite_temperatures(heat, thermal_seed)?;
    let heat_elements = heat
        .input
        .elements
        .iter()
        .map(|element| (element.id.as_str(), element))
        .collect::<HashMap<_, _>>();
    let thermal_elements = thermal_seed
        .elements
        .iter()
        .enumerate()
        .map(|(index, element)| (element.id.as_str(), index))
        .collect::<HashMap<_, _>>();
    if heat_elements.len() != heat.input.elements.len()
        || thermal_elements.len() != thermal_seed.elements.len()
        || heat_elements
            .keys()
            .chain(thermal_elements.keys())
            .any(|id| id.trim().is_empty())
    {
        return Err("thermal expansion element IDs must be nonempty and unique".into());
    }
    let mut request = thermal_seed.clone();
    let mut updates = Vec::with_capacity(spec.regions.len());
    for region in &spec.regions {
        let heat_element = heat_elements
            .get(region.element_id.as_str())
            .ok_or_else(|| {
                format!(
                    "thermal expansion projection is missing heat element {}",
                    region.element_id
                )
            })?;
        let target_index = thermal_elements
            .get(region.element_id.as_str())
            .ok_or_else(|| {
                format!(
                    "thermal expansion projection is missing structural element {}",
                    region.element_id
                )
            })?;
        let thermal_element = &mut request.elements[*target_index];
        let source_indices = [
            heat_element.node_i,
            heat_element.node_j,
            heat_element.node_k,
            heat_element.node_l,
        ];
        let target_indices = [
            thermal_element.node_i,
            thermal_element.node_j,
            thermal_element.node_k,
            thermal_element.node_l,
        ];
        let mut mapped_indices = [0; 4];
        let mut temperatures = [0.0; 4];
        for (position, index) in target_indices.into_iter().enumerate() {
            let source = mapping.nodes.get(index).ok_or_else(|| {
                format!(
                    "thermal expansion element {} references an unknown node",
                    region.element_id
                )
            })?;
            mapped_indices[position] = source.index;
            temperatures[position] = source.temperature;
        }
        if !matching_quad_cycle(source_indices, mapped_indices) {
            return Err(format!(
                "thermal expansion projection element {} topology does not match",
                region.element_id
            ));
        }
        let mean_temperature_c = temperatures[0]
            .midpoint(temperatures[1])
            .midpoint(temperatures[2].midpoint(temperatures[3]));
        let reference = thermal_element.thermal_expansion;
        if !reference.is_finite() || reference < 0.0 {
            return Err(format!(
                "thermal expansion projection element {} requires a finite non-negative reference coefficient for the thermal plane solver",
                region.element_id
            ));
        }
        let temperature_delta = mean_temperature_c - region.reference_temperature_c;
        let scale = 1.0 + region.temperature_coefficient_1_k * temperature_delta;
        let adjusted = reference * scale;
        if !temperature_delta.is_finite()
            || !scale.is_finite()
            || !adjusted.is_finite()
            || (reference != 0.0 && scale != 0.0 && adjusted == 0.0)
        {
            return Err(format!(
                "thermal expansion projection element {} produced an invalid coefficient",
                region.element_id
            ));
        }
        if adjusted < 0.0 {
            return Err(format!(
                "thermal expansion projection element {} produced a negative coefficient unsupported by the thermal plane solver",
                region.element_id
            ));
        }
        let relative_change = composite_feedback_relative_change(adjusted, reference);
        if !relative_change.is_finite() {
            return Err(format!(
                "thermal expansion element {} relative change is not representable",
                region.element_id
            ));
        }
        thermal_element.thermal_expansion = adjusted;
        updates.push(CompositeThermalExpansionRegionUpdate {
            element_id: region.element_id.clone(),
            mean_temperature_c,
            reference_thermal_expansion_1_k: reference,
            adjusted_thermal_expansion_1_k: adjusted,
            relative_change,
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

fn matching_quad_cycle(source: [usize; 4], mapped: [usize; 4]) -> bool {
    if source
        .iter()
        .enumerate()
        .any(|(index, node)| source[..index].contains(node))
    {
        return false;
    }
    (0..4).any(|offset| {
        (0..4).all(|index| source[index] == mapped[(offset + index) % 4])
            || (0..4).all(|index| source[index] == mapped[(offset + 4 - index) % 4])
    })
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
                "nodes": [
                    {"id": "n0", "x": 0.0, "y": 0.0, "fix_temperature": true, "temperature": 45.0},
                    {"id": "n1", "x": 1.0, "y": 0.0, "fix_temperature": true, "temperature": 45.0},
                    {"id": "n2", "x": 1.0, "y": 1.0, "fix_temperature": true, "temperature": 45.0},
                    {"id": "n3", "x": 0.0, "y": 1.0, "fix_temperature": true, "temperature": 45.0}
                ],
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
            "nodes": heat.nodes.iter().map(|node| json!({
                "id": node.id, "x": node.x, "y": node.y, "fix_x": true, "fix_y": true,
                "load_x": 0.0, "load_y": 0.0
            })).collect::<Vec<_>>(),
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
