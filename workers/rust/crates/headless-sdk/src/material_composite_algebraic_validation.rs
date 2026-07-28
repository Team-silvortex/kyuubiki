use serde::{Deserialize, Serialize};

pub const COMPOSITE_THERMAL_ALGEBRAIC_VALIDATION_SCHEMA_VERSION: &str =
    "kyuubiki.composite-thermal-algebraic-validation/v1";

const RELATIVE_RESIDUAL_TOLERANCE: f64 = 1.0e-10;
const REFINEMENT_LEVELS: [usize; 4] = [1, 2, 4, 8];
const REQUIRED_SERIES: [&str; 3] = [
    "uniform_full_edge_clamp",
    "uniform_regularized_restraint",
    "interface_graded_full_edge_clamp",
];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompositeThermalAlgebraicSample {
    pub elements_per_original_element: usize,
    pub node_count: usize,
    pub element_count: usize,
    pub solver_iterations: usize,
    pub matrix_non_zero_count: usize,
    pub residual_norm: f64,
    pub rhs_norm: f64,
    pub relative_residual: f64,
}

impl CompositeThermalAlgebraicSample {
    pub fn new(
        level: usize,
        node_count: usize,
        element_count: usize,
        solver_iterations: usize,
        matrix_non_zero_count: usize,
        residual_norm: f64,
        rhs_norm: f64,
    ) -> Self {
        Self {
            elements_per_original_element: level,
            node_count,
            element_count,
            solver_iterations,
            matrix_non_zero_count,
            residual_norm,
            rhs_norm,
            relative_residual: residual_norm / rhs_norm.max(1.0),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompositeThermalAlgebraicSeries {
    pub label: String,
    pub solver_path: String,
    pub samples: Vec<CompositeThermalAlgebraicSample>,
    pub max_relative_residual: Option<f64>,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompositeThermalAlgebraicValidation {
    pub schema_version: String,
    pub method: String,
    pub required_series: Vec<String>,
    pub relative_residual_tolerance: f64,
    pub series: Vec<CompositeThermalAlgebraicSeries>,
    pub max_relative_residual: Option<f64>,
    pub status: String,
    pub qualification_effect: String,
}

pub fn composite_thermal_algebraic_series(
    label: &str,
    samples: Vec<CompositeThermalAlgebraicSample>,
) -> CompositeThermalAlgebraicSeries {
    let complete = samples.len() == REFINEMENT_LEVELS.len()
        && samples
            .iter()
            .zip(REFINEMENT_LEVELS)
            .all(|(sample, level)| {
                sample.elements_per_original_element == level
                    && sample.node_count > 0
                    && sample.element_count > 0
                    && sample.matrix_non_zero_count > 0
                    && sample.residual_norm.is_finite()
                    && sample.residual_norm >= 0.0
                    && sample.rhs_norm.is_finite()
                    && sample.rhs_norm >= 0.0
                    && sample.relative_residual.is_finite()
            });
    let max_relative_residual = complete.then(|| {
        samples
            .iter()
            .map(|sample| sample.relative_residual)
            .fold(0.0_f64, f64::max)
    });
    let status = match max_relative_residual {
        Some(value) if value <= RELATIVE_RESIDUAL_TOLERANCE => "pass",
        Some(_) => "fail",
        None => "missing",
    };
    CompositeThermalAlgebraicSeries {
        label: label.to_string(),
        solver_path: if samples.iter().all(|sample| sample.solver_iterations == 0) {
            "dense_direct".to_string()
        } else {
            "mixed_or_iterative".to_string()
        },
        samples,
        max_relative_residual,
        status: status.to_string(),
    }
}

pub fn composite_thermal_algebraic_validation(
    series: Vec<CompositeThermalAlgebraicSeries>,
) -> CompositeThermalAlgebraicValidation {
    let complete = REQUIRED_SERIES.iter().all(|required| {
        series
            .iter()
            .any(|candidate| candidate.label == *required && candidate.status != "missing")
    });
    let max_relative_residual = complete.then(|| {
        series
            .iter()
            .filter_map(|candidate| candidate.max_relative_residual)
            .fold(0.0_f64, f64::max)
    });
    let status = if !complete {
        "missing"
    } else if series.iter().all(|candidate| candidate.status == "pass") {
        "pass"
    } else {
        "fail"
    };
    CompositeThermalAlgebraicValidation {
        schema_version: COMPOSITE_THERMAL_ALGEBRAIC_VALIDATION_SCHEMA_VERSION.to_string(),
        method: "original_system_recomputed_l2_residual_over_max_rhs_l2_or_one".to_string(),
        required_series: REQUIRED_SERIES
            .iter()
            .map(|value| value.to_string())
            .collect(),
        relative_residual_tolerance: RELATIVE_RESIDUAL_TOLERANCE,
        series,
        max_relative_residual,
        status: status.to_string(),
        qualification_effect: "algebraic_gate_does_not_override_discretization_gates".to_string(),
    }
}

pub fn missing_thermal_algebraic_validation() -> CompositeThermalAlgebraicValidation {
    composite_thermal_algebraic_validation(Vec::new())
}

#[cfg(test)]
mod tests {
    use super::{
        CompositeThermalAlgebraicSample, composite_thermal_algebraic_series,
        composite_thermal_algebraic_validation,
    };

    #[test]
    fn requires_every_mesh_family_and_bounds_relative_residual() {
        let series = [
            "uniform_full_edge_clamp",
            "uniform_regularized_restraint",
            "interface_graded_full_edge_clamp",
        ]
        .map(|label| composite_thermal_algebraic_series(label, samples(1.0e-13)))
        .to_vec();

        let validation = composite_thermal_algebraic_validation(series);

        assert_eq!(validation.status, "pass");
        assert!(validation.max_relative_residual.unwrap() <= 1.0e-10);
        assert_eq!(
            validation.qualification_effect,
            "algebraic_gate_does_not_override_discretization_gates"
        );
    }

    #[test]
    fn rejects_incomplete_or_excessive_residual_series() {
        let missing =
            composite_thermal_algebraic_validation(vec![composite_thermal_algebraic_series(
                "uniform_full_edge_clamp",
                samples(1.0e-13),
            )]);
        let failed = composite_thermal_algebraic_validation(
            [
                "uniform_full_edge_clamp",
                "uniform_regularized_restraint",
                "interface_graded_full_edge_clamp",
            ]
            .map(|label| composite_thermal_algebraic_series(label, samples(1.0e-6)))
            .to_vec(),
        );

        assert_eq!(missing.status, "missing");
        assert_eq!(failed.status, "fail");
    }

    fn samples(relative_residual: f64) -> Vec<CompositeThermalAlgebraicSample> {
        REFINEMENT_LEVELS
            .iter()
            .map(|level| {
                CompositeThermalAlgebraicSample::new(
                    *level,
                    4,
                    1,
                    0,
                    16,
                    relative_residual * 10.0,
                    10.0,
                )
            })
            .collect()
    }

    use super::REFINEMENT_LEVELS;
}
