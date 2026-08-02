use crate::CompositePanelCandidate;
use kyuubiki_protocol::{
    ElectrostaticPlaneNodeInput, ElectrostaticPlaneQuadElementInput,
    SolveElectrostaticPlaneQuad2dRequest,
};
use serde::{Deserialize, Serialize};

pub const COMPOSITE_ELECTROSTATIC_CROSS_VALIDATION_SCHEMA_VERSION: &str =
    "kyuubiki.composite-electrostatic-cross-validation/v1";
pub const COMPOSITE_ELECTROSTATIC_MESH_CONVERGENCE_SCHEMA_VERSION: &str =
    "kyuubiki.composite-electrostatic-mesh-convergence/v1";
pub const COMPOSITE_ELECTROSTATIC_REFINEMENT_LEVELS: [usize; 4] = [1, 2, 4, 8];

const APPLIED_POTENTIAL_V: f64 = 900.0;
const LAYER_WIDTH_M: f64 = 0.03;
const RELATIVE_ERROR_TOLERANCE: f64 = 1.0e-9;
const MESH_CONVERGENCE_TOLERANCE: f64 = 1.0e-8;
const PANEL_HEIGHT_M: f64 = 0.03;
const PANEL_THICKNESS_M: f64 = 0.001;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompositeElectrostaticCrossValidation {
    pub schema_version: String,
    pub method: String,
    pub applied_potential_v: f64,
    pub layer_widths_m: Vec<f64>,
    pub relative_permittivities: Vec<f64>,
    pub expected_layer_fields_v_m: Vec<f64>,
    pub expected_max_electric_field_v_m: f64,
    pub fem_max_electric_field_v_m: Option<f64>,
    pub absolute_error_v_m: Option<f64>,
    pub relative_error: Option<f64>,
    pub relative_error_tolerance: f64,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompositeElectrostaticMeshConvergenceSample {
    pub elements_per_layer: usize,
    pub node_count: usize,
    pub element_count: usize,
    pub max_electric_field_v_m: f64,
    pub analytic_relative_error: f64,
    pub relative_change_from_previous: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompositeElectrostaticMeshConvergence {
    pub schema_version: String,
    pub method: String,
    pub refinement_levels: Vec<usize>,
    pub samples: Vec<CompositeElectrostaticMeshConvergenceSample>,
    pub max_analytic_relative_error: Option<f64>,
    pub finest_pair_relative_change: Option<f64>,
    pub relative_error_tolerance: f64,
    pub status: String,
}

pub fn composite_electrostatic_cross_validation(
    candidate: &CompositePanelCandidate,
    fem_max_electric_field_v_m: Option<f64>,
) -> CompositeElectrostaticCrossValidation {
    composite_electrostatic_cross_validation_for_dielectric(
        candidate.dielectric_relative_permittivity,
        fem_max_electric_field_v_m,
    )
}

pub fn composite_electrostatic_cross_validation_for_dielectric(
    dielectric_relative_permittivity: f64,
    fem_max_electric_field_v_m: Option<f64>,
) -> CompositeElectrostaticCrossValidation {
    let relative_permittivities =
        composite_relative_permittivities(dielectric_relative_permittivity);
    let layer_widths_m = vec![LAYER_WIDTH_M; relative_permittivities.len()];
    let expected_layer_fields_v_m = expected_layer_fields(&relative_permittivities);
    let expected_max_electric_field_v_m = expected_layer_fields_v_m
        .iter()
        .copied()
        .fold(0.0, f64::max);
    let absolute_error_v_m =
        fem_max_electric_field_v_m.map(|value| (value - expected_max_electric_field_v_m).abs());
    let relative_error =
        absolute_error_v_m.map(|error| error / expected_max_electric_field_v_m.abs());
    let status = match relative_error {
        Some(error) if error <= RELATIVE_ERROR_TOLERANCE => "pass",
        Some(_) => "fail",
        None => "missing",
    };
    CompositeElectrostaticCrossValidation {
        schema_version: COMPOSITE_ELECTROSTATIC_CROSS_VALIDATION_SCHEMA_VERSION.to_string(),
        method: "layered_dielectric_series_closed_form".to_string(),
        applied_potential_v: APPLIED_POTENTIAL_V,
        layer_widths_m,
        relative_permittivities,
        expected_layer_fields_v_m,
        expected_max_electric_field_v_m,
        fem_max_electric_field_v_m,
        absolute_error_v_m,
        relative_error,
        relative_error_tolerance: RELATIVE_ERROR_TOLERANCE,
        status: status.to_string(),
    }
}

pub fn composite_electrostatic_refinement_requests(
    candidate: &CompositePanelCandidate,
) -> Vec<(usize, SolveElectrostaticPlaneQuad2dRequest)> {
    composite_electrostatic_refinement_requests_for_dielectric(
        candidate.dielectric_relative_permittivity,
    )
}

pub fn composite_electrostatic_refinement_requests_for_dielectric(
    dielectric_relative_permittivity: f64,
) -> Vec<(usize, SolveElectrostaticPlaneQuad2dRequest)> {
    COMPOSITE_ELECTROSTATIC_REFINEMENT_LEVELS
        .iter()
        .map(|level| {
            (
                *level,
                refined_electrostatic_model(dielectric_relative_permittivity, *level),
            )
        })
        .collect()
}

pub fn composite_electrostatic_mesh_convergence(
    candidate: &CompositePanelCandidate,
    max_fields_by_level: &[(usize, f64)],
) -> CompositeElectrostaticMeshConvergence {
    composite_electrostatic_mesh_convergence_for_dielectric(
        candidate.dielectric_relative_permittivity,
        max_fields_by_level,
    )
}

pub fn composite_electrostatic_mesh_convergence_for_dielectric(
    dielectric_relative_permittivity: f64,
    max_fields_by_level: &[(usize, f64)],
) -> CompositeElectrostaticMeshConvergence {
    let expected = expected_layer_fields(&composite_relative_permittivities(
        dielectric_relative_permittivity,
    ))
    .into_iter()
    .fold(0.0_f64, f64::max);
    let mut previous = None;
    let samples = max_fields_by_level
        .iter()
        .map(|(level, field)| {
            let columns = 3 * level;
            let relative_change_from_previous =
                previous.map(|value: f64| (field - value).abs() / expected.abs());
            previous = Some(*field);
            CompositeElectrostaticMeshConvergenceSample {
                elements_per_layer: *level,
                node_count: 2 * (columns + 1),
                element_count: columns,
                max_electric_field_v_m: *field,
                analytic_relative_error: (field - expected).abs() / expected.abs(),
                relative_change_from_previous,
            }
        })
        .collect::<Vec<_>>();
    let complete = samples.len() == COMPOSITE_ELECTROSTATIC_REFINEMENT_LEVELS.len()
        && samples
            .iter()
            .zip(COMPOSITE_ELECTROSTATIC_REFINEMENT_LEVELS)
            .all(|(sample, level)| {
                sample.elements_per_layer == level
                    && sample.max_electric_field_v_m.is_finite()
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
    CompositeElectrostaticMeshConvergence {
        schema_version: COMPOSITE_ELECTROSTATIC_MESH_CONVERGENCE_SCHEMA_VERSION.to_string(),
        method: "structured_quad_h_refinement".to_string(),
        refinement_levels: COMPOSITE_ELECTROSTATIC_REFINEMENT_LEVELS.to_vec(),
        samples,
        max_analytic_relative_error,
        finest_pair_relative_change,
        relative_error_tolerance: MESH_CONVERGENCE_TOLERANCE,
        status: status.to_string(),
    }
}

fn refined_electrostatic_model(
    dielectric_relative_permittivity: f64,
    elements_per_layer: usize,
) -> SolveElectrostaticPlaneQuad2dRequest {
    let columns = 3 * elements_per_layer;
    let column_width = LAYER_WIDTH_M / elements_per_layer as f64;
    let nodes = [0.0, PANEL_HEIGHT_M]
        .into_iter()
        .flat_map(|y| {
            (0..=columns).map(move |column| {
                let x = column as f64 * column_width;
                let at_left = column == 0;
                let at_right = column == columns;
                ElectrostaticPlaneNodeInput {
                    id: format!("n_{column}_{}", if y == 0.0 { "bottom" } else { "top" }),
                    x,
                    y,
                    fix_potential: at_left || at_right,
                    potential: if at_right { APPLIED_POTENTIAL_V } else { 0.0 },
                    charge_density: 0.0,
                }
            })
        })
        .collect::<Vec<_>>();
    let top_offset = columns + 1;
    let permittivities = composite_relative_permittivities(dielectric_relative_permittivity);
    let elements = (0..columns)
        .map(|column| ElectrostaticPlaneQuadElementInput {
            id: format!("layer_{}_element_{column}", column / elements_per_layer),
            node_i: column,
            node_j: column + 1,
            node_k: top_offset + column + 1,
            node_l: top_offset + column,
            thickness: PANEL_THICKNESS_M,
            permittivity: permittivities[column / elements_per_layer],
        })
        .collect();
    SolveElectrostaticPlaneQuad2dRequest { nodes, elements }
}

fn composite_relative_permittivities(dielectric_relative_permittivity: f64) -> Vec<f64> {
    vec![1.0, dielectric_relative_permittivity, 4.2]
}

fn expected_layer_fields(relative_permittivities: &[f64]) -> Vec<f64> {
    let electric_displacement_scale = APPLIED_POTENTIAL_V
        / relative_permittivities
            .iter()
            .map(|permittivity| LAYER_WIDTH_M / permittivity)
            .sum::<f64>();
    let expected_layer_fields_v_m = relative_permittivities
        .iter()
        .map(|permittivity| electric_displacement_scale / permittivity)
        .collect::<Vec<_>>();
    expected_layer_fields_v_m
}

#[cfg(test)]
mod tests {
    use super::{
        composite_electrostatic_cross_validation, composite_electrostatic_mesh_convergence,
        composite_electrostatic_refinement_requests,
    };
    use crate::composite_panel_candidates;

    #[test]
    fn layered_dielectric_closed_form_matches_known_polyimide_value() {
        let candidate = &composite_panel_candidates()[0];
        let expected = 19_579.524_680_073_12;
        let validation = composite_electrostatic_cross_validation(candidate, Some(expected));

        assert!((validation.expected_max_electric_field_v_m - expected).abs() < 1.0e-9);
        assert_eq!(validation.status, "pass");
        assert!(
            validation
                .relative_error
                .is_some_and(|error| error < 1.0e-15)
        );
    }

    #[test]
    fn cross_validation_reports_failed_and_missing_results() {
        let candidate = &composite_panel_candidates()[0];
        let failed = composite_electrostatic_cross_validation(candidate, Some(1.0));
        let missing = composite_electrostatic_cross_validation(candidate, None);

        assert_eq!(failed.status, "fail");
        assert_eq!(missing.status, "missing");
        assert!(failed.relative_error.is_some_and(|error| error > 0.9));
    }

    #[test]
    fn refinement_requests_preserve_layer_boundaries_and_materials() {
        let candidate = &composite_panel_candidates()[0];
        let requests = composite_electrostatic_refinement_requests(candidate);

        assert_eq!(requests.len(), 4);
        assert_eq!(requests[3].0, 8);
        assert_eq!(requests[3].1.nodes.len(), 50);
        assert_eq!(requests[3].1.elements.len(), 24);
        assert_eq!(requests[3].1.elements[7].permittivity, 1.0);
        assert_eq!(
            requests[3].1.elements[8].permittivity,
            candidate.dielectric_relative_permittivity
        );
        assert_eq!(requests[3].1.elements[16].permittivity, 4.2);
    }

    #[test]
    fn mesh_convergence_requires_all_refinement_levels() {
        let candidate = &composite_panel_candidates()[0];
        let expected = composite_electrostatic_cross_validation(candidate, None)
            .expected_max_electric_field_v_m;
        let passed = composite_electrostatic_mesh_convergence(
            candidate,
            &[(1, expected), (2, expected), (4, expected), (8, expected)],
        );
        let missing = composite_electrostatic_mesh_convergence(candidate, &[(1, expected)]);

        assert_eq!(passed.status, "pass");
        assert_eq!(passed.samples.len(), 4);
        assert_eq!(passed.finest_pair_relative_change, Some(0.0));
        assert_eq!(missing.status, "missing");
    }
}
