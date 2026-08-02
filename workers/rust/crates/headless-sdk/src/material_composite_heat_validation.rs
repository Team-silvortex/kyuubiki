use kyuubiki_protocol::{
    HeatPlaneNodeInput, HeatPlaneQuadElementInput, SolveHeatPlaneQuad2dRequest,
};
use serde::{Deserialize, Serialize};

pub const COMPOSITE_HEAT_CROSS_VALIDATION_SCHEMA_VERSION: &str =
    "kyuubiki.composite-heat-cross-validation/v1";
pub const COMPOSITE_HEAT_MESH_CONVERGENCE_SCHEMA_VERSION: &str =
    "kyuubiki.composite-heat-mesh-convergence/v1";
pub const COMPOSITE_HEAT_REFINEMENT_LEVELS: [usize; 4] = [1, 2, 4, 8];

const LAYER_WIDTH_M: f64 = 0.03;
const PANEL_HEIGHT_M: f64 = 0.03;
const PANEL_THICKNESS_M: f64 = 0.001;
const FIXED_TEMPERATURE_C: f64 = 35.0;
const HEAT_LOAD_PER_INTERFACE_NODE_W: f64 = 0.01;
const RELATIVE_ERROR_TOLERANCE: f64 = 1.0e-9;
const MESH_CONVERGENCE_TOLERANCE: f64 = 1.0e-8;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompositeHeatCrossValidation {
    pub schema_version: String,
    pub method: String,
    pub conductivities_w_mk: Vec<f64>,
    pub fixed_temperature_c: f64,
    pub total_heat_load_w: f64,
    pub expected_max_temperature_c: f64,
    pub fem_max_temperature_c: Option<f64>,
    pub absolute_error_c: Option<f64>,
    pub relative_error: Option<f64>,
    pub relative_error_tolerance: f64,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompositeHeatMeshConvergenceSample {
    pub elements_per_layer: usize,
    pub node_count: usize,
    pub element_count: usize,
    pub max_temperature_c: f64,
    pub analytic_relative_error: f64,
    pub relative_change_from_previous: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompositeHeatMeshConvergence {
    pub schema_version: String,
    pub method: String,
    pub refinement_levels: Vec<usize>,
    pub samples: Vec<CompositeHeatMeshConvergenceSample>,
    pub max_analytic_relative_error: Option<f64>,
    pub finest_pair_relative_change: Option<f64>,
    pub relative_error_tolerance: f64,
    pub status: String,
}

pub fn composite_heat_cross_validation(
    conductivities_w_mk: [f64; 3],
    fem_max_temperature_c: Option<f64>,
) -> CompositeHeatCrossValidation {
    build_cross_validation(
        conductivities_w_mk,
        2.0 * HEAT_LOAD_PER_INTERFACE_NODE_W,
        fem_max_temperature_c,
        false,
    )
}

pub fn composite_heat_cross_validation_for_distributed_load(
    conductivities_w_mk: [f64; 3],
    total_heat_load_w: f64,
    fem_max_temperature_c: Option<f64>,
) -> CompositeHeatCrossValidation {
    build_cross_validation(
        conductivities_w_mk,
        total_heat_load_w,
        fem_max_temperature_c,
        true,
    )
}

fn build_cross_validation(
    conductivities_w_mk: [f64; 3],
    total_heat_load_w: f64,
    fem_max_temperature_c: Option<f64>,
    distributed_dielectric_load: bool,
) -> CompositeHeatCrossValidation {
    let expected_max_temperature_c = if distributed_dielectric_load {
        expected_max_temperature_for_distributed_load(conductivities_w_mk, total_heat_load_w)
    } else {
        expected_max_temperature(conductivities_w_mk)
    };
    let absolute_error_c =
        fem_max_temperature_c.map(|value| (value - expected_max_temperature_c).abs());
    let relative_error = absolute_error_c.map(|error| error / expected_max_temperature_c.abs());
    let status = match relative_error {
        Some(error) if error <= RELATIVE_ERROR_TOLERANCE => "pass",
        Some(_) => "fail",
        None => "missing",
    };
    CompositeHeatCrossValidation {
        schema_version: COMPOSITE_HEAT_CROSS_VALIDATION_SCHEMA_VERSION.to_string(),
        method: if distributed_dielectric_load {
            "layered_thermal_resistance_with_uniform_dielectric_generation_closed_form"
        } else {
            "layered_thermal_resistance_closed_form"
        }
        .to_string(),
        conductivities_w_mk: conductivities_w_mk.to_vec(),
        fixed_temperature_c: FIXED_TEMPERATURE_C,
        total_heat_load_w,
        expected_max_temperature_c,
        fem_max_temperature_c,
        absolute_error_c,
        relative_error,
        relative_error_tolerance: RELATIVE_ERROR_TOLERANCE,
        status: status.to_string(),
    }
}

pub fn composite_heat_refinement_requests(
    conductivities_w_mk: [f64; 3],
) -> Vec<(usize, SolveHeatPlaneQuad2dRequest)> {
    COMPOSITE_HEAT_REFINEMENT_LEVELS
        .iter()
        .map(|level| (*level, refined_heat_model(conductivities_w_mk, *level)))
        .collect()
}

pub fn composite_heat_refinement_requests_for_distributed_load(
    conductivities_w_mk: [f64; 3],
    total_heat_load_w: f64,
) -> Result<Vec<(usize, SolveHeatPlaneQuad2dRequest)>, String> {
    COMPOSITE_HEAT_REFINEMENT_LEVELS
        .iter()
        .map(|level| {
            let request = refined_heat_model_without_load(conductivities_w_mk, *level);
            crate::distribute_composite_dielectric_heat_load(&request, total_heat_load_w)
                .map(|request| (*level, request))
        })
        .collect()
}

pub fn composite_heat_mesh_convergence(
    conductivities_w_mk: [f64; 3],
    max_temperatures_by_level: &[(usize, f64)],
) -> CompositeHeatMeshConvergence {
    build_mesh_convergence(
        expected_max_temperature(conductivities_w_mk),
        max_temperatures_by_level,
    )
}

fn build_mesh_convergence(
    expected: f64,
    max_temperatures_by_level: &[(usize, f64)],
) -> CompositeHeatMeshConvergence {
    let mut previous = None;
    let samples = max_temperatures_by_level
        .iter()
        .map(|(level, temperature)| {
            let columns = 3 * level;
            let relative_change_from_previous =
                previous.map(|value: f64| (temperature - value).abs() / expected.abs());
            previous = Some(*temperature);
            CompositeHeatMeshConvergenceSample {
                elements_per_layer: *level,
                node_count: 2 * (columns + 1),
                element_count: columns,
                max_temperature_c: *temperature,
                analytic_relative_error: (temperature - expected).abs() / expected.abs(),
                relative_change_from_previous,
            }
        })
        .collect::<Vec<_>>();
    let complete = samples.len() == COMPOSITE_HEAT_REFINEMENT_LEVELS.len()
        && samples
            .iter()
            .zip(COMPOSITE_HEAT_REFINEMENT_LEVELS)
            .all(|(sample, level)| {
                sample.elements_per_layer == level
                    && sample.max_temperature_c.is_finite()
                    && sample.analytic_relative_error.is_finite()
            });
    let max_analytic_relative_error = complete.then(|| {
        samples
            .iter()
            .map(|sample| sample.analytic_relative_error)
            .fold(0.0_f64, f64::max)
    });
    let finest_pair_relative_change = complete
        .then(|| {
            samples
                .last()
                .and_then(|sample| sample.relative_change_from_previous)
        })
        .flatten();
    let status = match (max_analytic_relative_error, finest_pair_relative_change) {
        (Some(error), Some(change))
            if error <= MESH_CONVERGENCE_TOLERANCE && change <= MESH_CONVERGENCE_TOLERANCE =>
        {
            "pass"
        }
        (Some(_), Some(_)) => "fail",
        _ => "missing",
    };
    CompositeHeatMeshConvergence {
        schema_version: COMPOSITE_HEAT_MESH_CONVERGENCE_SCHEMA_VERSION.to_string(),
        method: "structured_quad_h_refinement".to_string(),
        refinement_levels: COMPOSITE_HEAT_REFINEMENT_LEVELS.to_vec(),
        samples,
        max_analytic_relative_error,
        finest_pair_relative_change,
        relative_error_tolerance: MESH_CONVERGENCE_TOLERANCE,
        status: status.to_string(),
    }
}

pub fn composite_heat_mesh_convergence_for_distributed_load(
    conductivities_w_mk: [f64; 3],
    total_heat_load_w: f64,
    max_temperatures_by_level: &[(usize, f64)],
) -> CompositeHeatMeshConvergence {
    build_mesh_convergence(
        expected_max_temperature_for_distributed_load(conductivities_w_mk, total_heat_load_w),
        max_temperatures_by_level,
    )
}

fn expected_max_temperature(conductivities_w_mk: [f64; 3]) -> f64 {
    let cross_section_m2 = PANEL_HEIGHT_M * PANEL_THICKNESS_M;
    let downstream_thermal_resistance_k_w = (LAYER_WIDTH_M / conductivities_w_mk[1]
        + LAYER_WIDTH_M / conductivities_w_mk[2])
        / cross_section_m2;
    FIXED_TEMPERATURE_C + 2.0 * HEAT_LOAD_PER_INTERFACE_NODE_W * downstream_thermal_resistance_k_w
}

fn expected_max_temperature_for_distributed_load(
    conductivities_w_mk: [f64; 3],
    total_heat_load_w: f64,
) -> f64 {
    let cross_section_m2 = PANEL_HEIGHT_M * PANEL_THICKNESS_M;
    let dielectric_resistance = LAYER_WIDTH_M / (conductivities_w_mk[1] * cross_section_m2);
    let substrate_resistance = LAYER_WIDTH_M / (conductivities_w_mk[2] * cross_section_m2);
    FIXED_TEMPERATURE_C + total_heat_load_w * (0.5 * dielectric_resistance + substrate_resistance)
}

fn refined_heat_model(
    conductivities_w_mk: [f64; 3],
    elements_per_layer: usize,
) -> SolveHeatPlaneQuad2dRequest {
    build_refined_heat_model(conductivities_w_mk, elements_per_layer, true)
}

fn refined_heat_model_without_load(
    conductivities_w_mk: [f64; 3],
    elements_per_layer: usize,
) -> SolveHeatPlaneQuad2dRequest {
    build_refined_heat_model(conductivities_w_mk, elements_per_layer, false)
}

fn build_refined_heat_model(
    conductivities_w_mk: [f64; 3],
    elements_per_layer: usize,
    include_interface_load: bool,
) -> SolveHeatPlaneQuad2dRequest {
    let columns = 3 * elements_per_layer;
    let column_width = LAYER_WIDTH_M / elements_per_layer as f64;
    let nodes = [0.0, PANEL_HEIGHT_M]
        .into_iter()
        .flat_map(|y| {
            (0..=columns).map(move |column| {
                let at_right = column == columns;
                HeatPlaneNodeInput {
                    id: format!("n_{column}_{}", if y == 0.0 { "bottom" } else { "top" }),
                    x: column as f64 * column_width,
                    y,
                    fix_temperature: at_right,
                    temperature: FIXED_TEMPERATURE_C,
                    heat_load: if include_interface_load && column == elements_per_layer {
                        HEAT_LOAD_PER_INTERFACE_NODE_W
                    } else {
                        0.0
                    },
                }
            })
        })
        .collect::<Vec<_>>();
    let top_offset = columns + 1;
    let elements = (0..columns)
        .map(|column| HeatPlaneQuadElementInput {
            id: format!("layer_{}_element_{column}", column / elements_per_layer),
            node_i: column,
            node_j: column + 1,
            node_k: top_offset + column + 1,
            node_l: top_offset + column,
            thickness: PANEL_THICKNESS_M,
            conductivity: conductivities_w_mk[column / elements_per_layer],
        })
        .collect();
    SolveHeatPlaneQuad2dRequest { nodes, elements }
}

#[cfg(test)]
mod tests {
    use super::{
        composite_heat_cross_validation, composite_heat_cross_validation_for_distributed_load,
        composite_heat_mesh_convergence, composite_heat_refinement_requests,
        composite_heat_refinement_requests_for_distributed_load,
    };

    const CONDUCTIVITIES: [f64; 3] = [390.0, 0.25, 160.0];

    #[test]
    fn layered_thermal_resistance_matches_fixture_temperature() {
        let validation = composite_heat_cross_validation(CONDUCTIVITIES, Some(115.125));

        assert_eq!(validation.status, "pass");
        assert!((validation.expected_max_temperature_c - 115.125).abs() < 1.0e-12);
    }

    #[test]
    fn refinement_requests_preserve_interface_load_and_materials() {
        let requests = composite_heat_refinement_requests(CONDUCTIVITIES);
        let finest = &requests[3].1;

        assert_eq!(finest.nodes.len(), 50);
        assert_eq!(finest.elements.len(), 24);
        assert_eq!(
            finest
                .nodes
                .iter()
                .filter(|node| node.heat_load > 0.0)
                .count(),
            2
        );
        assert_eq!(finest.elements[8].conductivity, 0.25);
        assert_eq!(finest.elements[16].conductivity, 160.0);
    }

    #[test]
    fn distributed_dielectric_generation_matches_closed_form_and_conserves_load() {
        let validation = composite_heat_cross_validation_for_distributed_load(
            CONDUCTIVITIES,
            0.02,
            Some(75.125),
        );
        let requests =
            composite_heat_refinement_requests_for_distributed_load(CONDUCTIVITIES, 0.02)
                .expect("refinements");

        assert_eq!(validation.status, "pass");
        assert!((validation.expected_max_temperature_c - 75.125).abs() < 1.0e-12);
        assert!(requests.iter().all(|(_, request)| {
            (request.nodes.iter().map(|node| node.heat_load).sum::<f64>() - 0.02).abs() < 1.0e-15
        }));
    }

    #[test]
    fn mesh_convergence_requires_complete_level_sequence() {
        let fields = [(1, 115.125), (2, 115.125), (4, 115.125), (8, 115.125)];
        let passed = composite_heat_mesh_convergence(CONDUCTIVITIES, &fields);
        let missing = composite_heat_mesh_convergence(CONDUCTIVITIES, &fields[..2]);

        assert_eq!(passed.status, "pass");
        assert_eq!(passed.finest_pair_relative_change, Some(0.0));
        assert_eq!(missing.status, "missing");
    }
}
