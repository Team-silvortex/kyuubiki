use kyuubiki_protocol::{
    SolveThermalPlaneQuad2dRequest, ThermalPlaneNodeInput, ThermalPlaneQuadElementInput,
};
use serde::{Deserialize, Serialize};

use crate::{
    CompositeThermalAlgebraicValidation, CompositeThermalConvergenceRegimeAssessment,
    composite_thermal_convergence_regime, missing_thermal_algebraic_validation,
    missing_thermal_convergence_regime,
};

pub const COMPOSITE_THERMAL_MESH_CONVERGENCE_SCHEMA_VERSION: &str =
    "kyuubiki.composite-thermal-mesh-convergence/v1";
pub const COMPOSITE_THERMAL_CONSTRAINT_SENSITIVITY_SCHEMA_VERSION: &str =
    "kyuubiki.composite-thermal-constraint-sensitivity/v1";
pub const COMPOSITE_THERMAL_REFINEMENT_LEVELS: [usize; 4] = [1, 2, 4, 8];

const CONVERGENCE_TOLERANCE: f64 = 2.0e-2;
const COORDINATE_TOLERANCE: f64 = 1.0e-12;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompositeThermalMeshSample {
    pub elements_per_original_element: usize,
    pub node_count: usize,
    pub element_count: usize,
    pub max_displacement_m: f64,
    pub total_strain_energy_j: f64,
    pub max_von_mises_stress_pa: f64,
    pub displacement_relative_change: Option<f64>,
    pub strain_energy_relative_change: Option<f64>,
    pub peak_stress_relative_change: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompositeThermalMeshConvergence {
    pub schema_version: String,
    pub method: String,
    pub refinement_levels: Vec<usize>,
    pub samples: Vec<CompositeThermalMeshSample>,
    pub finest_pair_displacement_relative_change: Option<f64>,
    pub finest_pair_strain_energy_relative_change: Option<f64>,
    pub finest_pair_peak_stress_relative_change: Option<f64>,
    pub convergence_tolerance: f64,
    pub pass_metrics: Vec<String>,
    pub diagnostic_metrics: Vec<String>,
    #[serde(default = "missing_thermal_convergence_regime")]
    pub regime_assessment: CompositeThermalConvergenceRegimeAssessment,
    #[serde(default = "missing_thermal_algebraic_validation")]
    pub algebraic_validation: CompositeThermalAlgebraicValidation,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompositeThermalConstraintSensitivity {
    pub schema_version: String,
    pub primary_restraint: String,
    pub alternative_restraint: String,
    pub primary_status: String,
    pub alternative_status: String,
    pub displacement_change_ratio_alternative_to_primary: Option<f64>,
    pub strain_energy_change_ratio_alternative_to_primary: Option<f64>,
    pub peak_stress_change_ratio_alternative_to_primary: Option<f64>,
    pub diagnosis: String,
    pub qualification_effect: String,
}

pub fn composite_thermal_refinement_requests(
    request: &SolveThermalPlaneQuad2dRequest,
) -> Result<Vec<(usize, SolveThermalPlaneQuad2dRequest)>, String> {
    refinement_requests(
        request,
        ThermalRestraint::FullEdgeClamp,
        MeshSpacing::Uniform,
    )
}

pub fn composite_thermal_regularized_refinement_requests(
    request: &SolveThermalPlaneQuad2dRequest,
) -> Result<Vec<(usize, SolveThermalPlaneQuad2dRequest)>, String> {
    refinement_requests(
        request,
        ThermalRestraint::RollerEdgeWithVerticalAnchor,
        MeshSpacing::Uniform,
    )
}

pub fn composite_thermal_interface_graded_refinement_requests(
    request: &SolveThermalPlaneQuad2dRequest,
) -> Result<Vec<(usize, SolveThermalPlaneQuad2dRequest)>, String> {
    refinement_requests(
        request,
        ThermalRestraint::FullEdgeClamp,
        MeshSpacing::CosineGraded,
    )
}

fn refinement_requests(
    request: &SolveThermalPlaneQuad2dRequest,
    restraint: ThermalRestraint,
    spacing: MeshSpacing,
) -> Result<Vec<(usize, SolveThermalPlaneQuad2dRequest)>, String> {
    let model = StructuredThermalModel::from_request(request)?;
    Ok(COMPOSITE_THERMAL_REFINEMENT_LEVELS
        .iter()
        .map(|level| (*level, model.refined_request(*level, restraint, spacing)))
        .collect())
}

pub fn composite_thermal_mesh_convergence(
    samples_by_level: &[(usize, f64, f64, f64)],
) -> CompositeThermalMeshConvergence {
    build_mesh_convergence(
        samples_by_level,
        "structured_quad_h_refinement_preserving_piecewise_linear_temperature_full_edge_clamp",
    )
}

pub fn composite_thermal_regularized_mesh_convergence(
    samples_by_level: &[(usize, f64, f64, f64)],
) -> CompositeThermalMeshConvergence {
    build_mesh_convergence(
        samples_by_level,
        "structured_quad_h_refinement_preserving_piecewise_linear_temperature_roller_edge_with_vertical_anchor",
    )
}

pub fn composite_thermal_interface_graded_mesh_convergence(
    samples_by_level: &[(usize, f64, f64, f64)],
) -> CompositeThermalMeshConvergence {
    build_mesh_convergence(
        samples_by_level,
        "cosine_graded_quad_h_refinement_at_clamp_interfaces_and_free_edges",
    )
}

pub fn composite_thermal_constraint_sensitivity(
    primary: &CompositeThermalMeshConvergence,
    alternative: &CompositeThermalMeshConvergence,
) -> CompositeThermalConstraintSensitivity {
    let displacement_ratio = ratio(
        alternative.finest_pair_displacement_relative_change,
        primary.finest_pair_displacement_relative_change,
    );
    let energy_ratio = ratio(
        alternative.finest_pair_strain_energy_relative_change,
        primary.finest_pair_strain_energy_relative_change,
    );
    let stress_ratio = ratio(
        alternative.finest_pair_peak_stress_relative_change,
        primary.finest_pair_peak_stress_relative_change,
    );
    let alternative_displacement_passes = alternative
        .finest_pair_displacement_relative_change
        .is_some_and(|value| value <= CONVERGENCE_TOLERANCE);
    let alternative_energy_passes = alternative
        .finest_pair_strain_energy_relative_change
        .is_some_and(|value| value <= CONVERGENCE_TOLERANCE);
    let diagnosis = if alternative.status == "missing" || primary.status == "missing" {
        "insufficient_evidence"
    } else if alternative_energy_passes {
        "restraint_dominated_nonconvergence"
    } else if alternative_displacement_passes {
        "mixed_restraint_sensitivity_and_persistent_energy_nonconvergence"
    } else {
        "persistent_nonconvergence_across_restraint_models"
    };
    CompositeThermalConstraintSensitivity {
        schema_version: COMPOSITE_THERMAL_CONSTRAINT_SENSITIVITY_SCHEMA_VERSION.to_string(),
        primary_restraint: "full_edge_clamp".to_string(),
        alternative_restraint: "roller_edge_with_vertical_anchor".to_string(),
        primary_status: primary.status.clone(),
        alternative_status: alternative.status.clone(),
        displacement_change_ratio_alternative_to_primary: displacement_ratio,
        strain_energy_change_ratio_alternative_to_primary: energy_ratio,
        peak_stress_change_ratio_alternative_to_primary: stress_ratio,
        diagnosis: diagnosis.to_string(),
        qualification_effect: "diagnostic_only_does_not_override_primary_quality_gates".to_string(),
    }
}

fn build_mesh_convergence(
    samples_by_level: &[(usize, f64, f64, f64)],
    method: &str,
) -> CompositeThermalMeshConvergence {
    let mut previous: Option<(f64, f64, f64)> = None;
    let samples = samples_by_level
        .iter()
        .map(|(level, displacement, energy, stress)| {
            let changes = previous.map(|(old_displacement, old_energy, old_stress)| {
                (
                    relative_change(*displacement, old_displacement),
                    relative_change(*energy, old_energy),
                    relative_change(*stress, old_stress),
                )
            });
            previous = Some((*displacement, *energy, *stress));
            CompositeThermalMeshSample {
                elements_per_original_element: *level,
                node_count: (3 * level + 1) * (level + 1),
                element_count: 3 * level * level,
                max_displacement_m: *displacement,
                total_strain_energy_j: *energy,
                max_von_mises_stress_pa: *stress,
                displacement_relative_change: changes.map(|values| values.0),
                strain_energy_relative_change: changes.map(|values| values.1),
                peak_stress_relative_change: changes.map(|values| values.2),
            }
        })
        .collect::<Vec<_>>();
    let complete = samples.len() == COMPOSITE_THERMAL_REFINEMENT_LEVELS.len()
        && samples
            .iter()
            .zip(COMPOSITE_THERMAL_REFINEMENT_LEVELS)
            .all(|(sample, level)| {
                sample.elements_per_original_element == level
                    && sample.max_displacement_m.is_finite()
                    && sample.total_strain_energy_j.is_finite()
                    && sample.total_strain_energy_j >= 0.0
                    && sample.max_von_mises_stress_pa.is_finite()
            });
    let finest_pair_displacement_relative_change = complete
        .then(|| samples.last()?.displacement_relative_change)
        .flatten();
    let finest_pair_strain_energy_relative_change = complete
        .then(|| samples.last()?.strain_energy_relative_change)
        .flatten();
    let finest_pair_peak_stress_relative_change = complete
        .then(|| samples.last()?.peak_stress_relative_change)
        .flatten();
    let status = match (
        finest_pair_displacement_relative_change,
        finest_pair_strain_energy_relative_change,
    ) {
        (Some(displacement), Some(energy))
            if displacement <= CONVERGENCE_TOLERANCE && energy <= CONVERGENCE_TOLERANCE =>
        {
            "pass"
        }
        (Some(_), Some(_)) => "fail",
        _ => "missing",
    };
    CompositeThermalMeshConvergence {
        schema_version: COMPOSITE_THERMAL_MESH_CONVERGENCE_SCHEMA_VERSION.to_string(),
        method: method.to_string(),
        refinement_levels: COMPOSITE_THERMAL_REFINEMENT_LEVELS.to_vec(),
        samples,
        finest_pair_displacement_relative_change,
        finest_pair_strain_energy_relative_change,
        finest_pair_peak_stress_relative_change,
        convergence_tolerance: CONVERGENCE_TOLERANCE,
        pass_metrics: vec![
            "max_displacement_m".to_string(),
            "total_strain_energy_j".to_string(),
        ],
        diagnostic_metrics: vec!["max_von_mises_stress_pa".to_string()],
        regime_assessment: composite_thermal_convergence_regime(samples_by_level),
        algebraic_validation: missing_thermal_algebraic_validation(),
        status: status.to_string(),
    }
}

fn relative_change(current: f64, previous: f64) -> f64 {
    (current - previous).abs() / current.abs().max(previous.abs()).max(f64::EPSILON)
}

fn ratio(numerator: Option<f64>, denominator: Option<f64>) -> Option<f64> {
    match (numerator, denominator) {
        (Some(numerator), Some(denominator)) if denominator > f64::EPSILON => {
            Some(numerator / denominator)
        }
        _ => None,
    }
}

#[derive(Debug, Clone)]
struct Layer {
    x_min: f64,
    x_max: f64,
    input: ThermalPlaneQuadElementInput,
}

#[derive(Debug, Clone)]
struct StructuredThermalModel {
    y_min: f64,
    y_max: f64,
    layers: Vec<Layer>,
    temperatures: Vec<(f64, f64)>,
}

#[derive(Debug, Clone, Copy)]
enum ThermalRestraint {
    FullEdgeClamp,
    RollerEdgeWithVerticalAnchor,
}

#[derive(Debug, Clone, Copy)]
enum MeshSpacing {
    Uniform,
    CosineGraded,
}

impl StructuredThermalModel {
    fn from_request(request: &SolveThermalPlaneQuad2dRequest) -> Result<Self, String> {
        if request.elements.len() != 3 {
            return Err("composite thermal convergence requires exactly three layers".to_string());
        }
        if request.nodes.iter().any(|node| {
            node.load_x != 0.0
                || node.load_y != 0.0
                || !node.x.is_finite()
                || !node.y.is_finite()
                || !node.temperature_delta.is_finite()
        }) {
            return Err(
                "composite thermal convergence requires finite nodes without mechanical loads"
                    .to_string(),
            );
        }
        let y_min = request
            .nodes
            .iter()
            .map(|node| node.y)
            .reduce(f64::min)
            .ok_or_else(|| "composite thermal convergence requires nodes".to_string())?;
        let y_max = request
            .nodes
            .iter()
            .map(|node| node.y)
            .reduce(f64::max)
            .unwrap();
        if y_max - y_min <= COORDINATE_TOLERANCE {
            return Err("composite thermal convergence requires positive height".to_string());
        }
        let mut layers = request
            .elements
            .iter()
            .cloned()
            .map(|input| {
                let xs = [
                    node(request, input.node_i)?.x,
                    node(request, input.node_j)?.x,
                    node(request, input.node_k)?.x,
                    node(request, input.node_l)?.x,
                ];
                let x_min = xs.into_iter().reduce(f64::min).unwrap();
                let x_max = xs.into_iter().reduce(f64::max).unwrap();
                if x_max - x_min <= COORDINATE_TOLERANCE {
                    return Err(format!("thermal element {} has zero width", input.id));
                }
                Ok(Layer {
                    x_min,
                    x_max,
                    input,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        layers.sort_by(|left, right| left.x_min.total_cmp(&right.x_min));
        for pair in layers.windows(2) {
            if (pair[0].x_max - pair[1].x_min).abs() > COORDINATE_TOLERANCE {
                return Err("composite thermal layers must be contiguous".to_string());
            }
        }
        let temperatures = temperature_profile(&request.nodes);
        if temperatures.len() < 2 {
            return Err("composite thermal convergence requires a temperature profile".to_string());
        }
        Ok(Self {
            y_min,
            y_max,
            layers,
            temperatures,
        })
    }

    fn refined_request(
        &self,
        level: usize,
        restraint: ThermalRestraint,
        spacing: MeshSpacing,
    ) -> SolveThermalPlaneQuad2dRequest {
        let columns = self.layers.len() * level;
        let mut xs = Vec::with_capacity(columns + 1);
        for layer in &self.layers {
            for subdivision in 0..level {
                let position = spacing.position(subdivision, level);
                xs.push(layer.x_min + (layer.x_max - layer.x_min) * position);
            }
        }
        xs.push(self.layers.last().unwrap().x_max);
        let x_min = xs[0];
        let nodes = (0..=level)
            .flat_map(|row| {
                let y = self.y_min + (self.y_max - self.y_min) * spacing.position(row, level);
                xs.iter()
                    .enumerate()
                    .map(move |(column, x)| ThermalPlaneNodeInput {
                        id: format!("n_{column}_{row}"),
                        x: *x,
                        y,
                        fix_x: (*x - x_min).abs() <= COORDINATE_TOLERANCE,
                        fix_y: match restraint {
                            ThermalRestraint::FullEdgeClamp => {
                                (*x - x_min).abs() <= COORDINATE_TOLERANCE
                            }
                            ThermalRestraint::RollerEdgeWithVerticalAnchor => {
                                (*x - x_min).abs() <= COORDINATE_TOLERANCE
                                    && (y - self.y_min).abs() <= COORDINATE_TOLERANCE
                            }
                        },
                        load_x: 0.0,
                        load_y: 0.0,
                        temperature_delta: interpolate_temperature(&self.temperatures, *x),
                    })
            })
            .collect::<Vec<_>>();
        let row_width = columns + 1;
        let elements = (0..level)
            .flat_map(|row| (0..columns).map(move |column| (row, column)))
            .map(|(row, column)| {
                let source = &self.layers[column / level].input;
                let bottom = row * row_width;
                let top = (row + 1) * row_width;
                ThermalPlaneQuadElementInput {
                    id: format!("layer_{}_element_{column}_{row}", column / level),
                    node_i: bottom + column,
                    node_j: bottom + column + 1,
                    node_k: top + column + 1,
                    node_l: top + column,
                    thickness: source.thickness,
                    youngs_modulus: source.youngs_modulus,
                    poisson_ratio: source.poisson_ratio,
                    thermal_expansion: source.thermal_expansion,
                }
            })
            .collect();
        SolveThermalPlaneQuad2dRequest { nodes, elements }
    }
}

impl MeshSpacing {
    fn position(self, index: usize, divisions: usize) -> f64 {
        let uniform = index as f64 / divisions as f64;
        match self {
            Self::Uniform => uniform,
            Self::CosineGraded => 0.5 * (1.0 - (std::f64::consts::PI * uniform).cos()),
        }
    }
}

fn node(
    request: &SolveThermalPlaneQuad2dRequest,
    index: usize,
) -> Result<&ThermalPlaneNodeInput, String> {
    request
        .nodes
        .get(index)
        .ok_or_else(|| format!("thermal element references unknown node {index}"))
}

fn temperature_profile(nodes: &[ThermalPlaneNodeInput]) -> Vec<(f64, f64)> {
    let mut sorted = nodes
        .iter()
        .map(|node| (node.x, node.temperature_delta))
        .collect::<Vec<_>>();
    sorted.sort_by(|left, right| left.0.total_cmp(&right.0));
    let mut profile: Vec<(f64, f64)> = Vec::new();
    for (x, temperature) in sorted {
        if let Some(last) = profile.last_mut()
            && (last.0 - x).abs() <= COORDINATE_TOLERANCE
        {
            last.1 = (last.1 + temperature) * 0.5;
            continue;
        }
        profile.push((x, temperature));
    }
    profile
}

fn interpolate_temperature(profile: &[(f64, f64)], x: f64) -> f64 {
    if x <= profile[0].0 {
        return profile[0].1;
    }
    for pair in profile.windows(2) {
        if x <= pair[1].0 {
            let ratio = (x - pair[0].0) / (pair[1].0 - pair[0].0);
            return pair[0].1 + ratio * (pair[1].1 - pair[0].1);
        }
    }
    profile.last().unwrap().1
}

#[cfg(test)]
mod tests {
    use super::{
        composite_thermal_constraint_sensitivity,
        composite_thermal_interface_graded_refinement_requests, composite_thermal_mesh_convergence,
        composite_thermal_refinement_requests, composite_thermal_regularized_mesh_convergence,
        composite_thermal_regularized_refinement_requests,
    };
    use kyuubiki_protocol::SolveThermalPlaneQuad2dRequest;

    #[test]
    fn refinement_preserves_material_layers_and_temperature_profile() {
        let request: SolveThermalPlaneQuad2dRequest =
            serde_json::from_value(test_model()).expect("fixture should decode");
        let refinements =
            composite_thermal_refinement_requests(&request).expect("fixture should refine");
        let finest = &refinements[3].1;

        assert_eq!(finest.nodes.len(), 225);
        assert_eq!(finest.elements.len(), 192);
        assert_eq!(finest.nodes[8].temperature_delta, 95.0);
        assert_eq!(finest.elements[8].youngs_modulus, 2.5e9);
        assert_eq!(finest.elements[16].youngs_modulus, 70.0e9);
    }

    #[test]
    fn convergence_requires_all_levels_and_uses_energy_and_displacement() {
        let samples = [
            (1, 2.0e-4, 0.040, 8.0e7),
            (2, 2.1e-4, 0.039, 8.2e7),
            (4, 2.11e-4, 0.0388, 8.4e7),
            (8, 2.111e-4, 0.03875, 9.5e7),
        ];
        let passed = composite_thermal_mesh_convergence(&samples);
        let missing = composite_thermal_mesh_convergence(&samples[..2]);

        assert_eq!(passed.status, "pass");
        assert_eq!(passed.diagnostic_metrics, ["max_von_mises_stress_pa"]);
        assert_eq!(passed.regime_assessment.metrics.len(), 3);
        assert_eq!(missing.status, "missing");
        assert_eq!(missing.regime_assessment.status, "missing");
    }

    #[test]
    fn regularized_refinement_keeps_horizontal_edge_support_and_one_vertical_anchor() {
        let request: SolveThermalPlaneQuad2dRequest =
            serde_json::from_value(test_model()).expect("fixture should decode");
        let refinements =
            composite_thermal_regularized_refinement_requests(&request).expect("should refine");
        let finest = &refinements[3].1;
        let fixed_x = finest.nodes.iter().filter(|node| node.fix_x).count();
        let fixed_y = finest.nodes.iter().filter(|node| node.fix_y).count();

        assert_eq!(fixed_x, 9);
        assert_eq!(fixed_y, 1);
        assert!(finest.nodes.iter().all(|node| !node.fix_y || node.fix_x));
    }

    #[test]
    fn graded_refinement_clusters_nodes_at_layer_and_free_edge_boundaries() {
        let request: SolveThermalPlaneQuad2dRequest =
            serde_json::from_value(test_model()).expect("fixture should decode");
        let refinements = composite_thermal_interface_graded_refinement_requests(&request)
            .expect("should refine");
        let level_four = &refinements[2].1;
        let first_width = level_four.nodes[1].x - level_four.nodes[0].x;
        let middle_width = level_four.nodes[2].x - level_four.nodes[1].x;
        let row_width = 13;
        let first_height = level_four.nodes[row_width].y - level_four.nodes[0].y;
        let middle_height = level_four.nodes[2 * row_width].y - level_four.nodes[row_width].y;

        assert!(first_width < middle_width);
        assert!(first_height < middle_height);
        assert!((level_four.nodes[4].x - 0.03).abs() < 1.0e-12);
    }

    #[test]
    fn sensitivity_does_not_promote_regularized_displacement_over_failed_energy() {
        let primary = composite_thermal_mesh_convergence(&[
            (1, 2.0e-4, 0.040, 8.0e7),
            (2, 1.9e-4, 0.025, 9.0e7),
            (4, 1.8e-4, 0.018, 1.0e8),
            (8, 1.75e-4, 0.013, 1.2e8),
        ]);
        let regularized = composite_thermal_regularized_mesh_convergence(&[
            (1, 2.0e-4, 0.040, 8.0e7),
            (2, 1.9e-4, 0.025, 9.0e7),
            (4, 1.8e-4, 0.018, 1.0e8),
            (8, 1.79e-4, 0.012, 1.1e8),
        ]);
        let sensitivity = composite_thermal_constraint_sensitivity(&primary, &regularized);

        assert_eq!(
            sensitivity.diagnosis,
            "mixed_restraint_sensitivity_and_persistent_energy_nonconvergence"
        );
        assert_eq!(
            sensitivity.qualification_effect,
            "diagnostic_only_does_not_override_primary_quality_gates"
        );
    }

    fn test_model() -> serde_json::Value {
        let mut nodes = Vec::new();
        for (index, (x, y)) in [
            (0.0, 0.0),
            (0.03, 0.0),
            (0.06, 0.0),
            (0.09, 0.0),
            (0.0, 0.03),
            (0.03, 0.03),
            (0.06, 0.03),
            (0.09, 0.03),
        ]
        .into_iter()
        .enumerate()
        {
            nodes.push(serde_json::json!({
                "id": format!("n{index}"), "x": x, "y": y,
                "fix_x": matches!(index, 0 | 4), "fix_y": matches!(index, 0 | 4),
                "load_x": 0.0, "load_y": 0.0,
                "temperature_delta": if matches!(index, 1 | 5) { 95.0 } else { 45.0 }
            }));
        }
        serde_json::json!({
            "nodes": nodes,
            "elements": [
                element("left", 0, 1, 5, 4, 110.0e9, 17.0e-6),
                element("core", 1, 2, 6, 5, 2.5e9, 45.0e-6),
                element("right", 2, 3, 7, 6, 70.0e9, 23.0e-6)
            ]
        })
    }

    fn element(
        id: &str,
        node_i: usize,
        node_j: usize,
        node_k: usize,
        node_l: usize,
        youngs_modulus: f64,
        thermal_expansion: f64,
    ) -> serde_json::Value {
        serde_json::json!({
            "id": id, "node_i": node_i, "node_j": node_j, "node_k": node_k, "node_l": node_l,
            "thickness": 0.001, "youngs_modulus": youngs_modulus,
            "poisson_ratio": 0.32, "thermal_expansion": thermal_expansion
        })
    }
}
