use kyuubiki_protocol::SolveThermalPlaneQuad2dResult;
use serde::{Deserialize, Serialize};

use crate::{COMPOSITE_THERMAL_REFINEMENT_LEVELS, CompositeThermalMeshConvergence};

pub const COMPOSITE_THERMAL_STRESS_RECOVERY_SCHEMA_VERSION: &str =
    "kyuubiki.composite-thermal-stress-recovery/v1";
pub const COMPOSITE_THERMAL_INTERFACE_GRADING_SCHEMA_VERSION: &str =
    "kyuubiki.composite-thermal-interface-grading-assessment/v1";

const RECOVERED_STRESS_CONVERGENCE_TOLERANCE: f64 = 2.0e-2;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompositeThermalRecoveredStressStatistics {
    pub area_weighted_mean_stress_pa: f64,
    pub area_weighted_rms_stress_pa: f64,
    pub area_weighted_p95_stress_pa: f64,
    pub max_stress_pa: f64,
    pub max_to_p95_ratio: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompositeThermalStressRecoverySample {
    pub elements_per_original_element: usize,
    pub element_count: usize,
    pub statistics: CompositeThermalRecoveredStressStatistics,
    pub rms_relative_change: Option<f64>,
    pub p95_relative_change: Option<f64>,
    pub max_relative_change: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompositeThermalStressRecovery {
    pub schema_version: String,
    pub method: String,
    pub refinement_levels: Vec<usize>,
    pub samples: Vec<CompositeThermalStressRecoverySample>,
    pub finest_pair_rms_relative_change: Option<f64>,
    pub finest_pair_p95_relative_change: Option<f64>,
    pub finest_pair_max_relative_change: Option<f64>,
    pub convergence_tolerance: f64,
    pub pass_metrics: Vec<String>,
    pub diagnostic_metrics: Vec<String>,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompositeThermalInterfaceGradingAssessment {
    pub schema_version: String,
    pub graded_mesh_convergence: CompositeThermalMeshConvergence,
    pub graded_stress_recovery: CompositeThermalStressRecovery,
    pub displacement_change_ratio_graded_to_uniform: Option<f64>,
    pub strain_energy_change_ratio_graded_to_uniform: Option<f64>,
    pub rms_change_ratio_graded_to_uniform: Option<f64>,
    pub p95_change_ratio_graded_to_uniform: Option<f64>,
    pub max_change_ratio_graded_to_uniform: Option<f64>,
    pub diagnosis: String,
    pub qualification_effect: String,
}

pub fn composite_thermal_recovered_stress_statistics(
    result: &SolveThermalPlaneQuad2dResult,
) -> Result<CompositeThermalRecoveredStressStatistics, String> {
    if result.elements.len() != result.input.elements.len() || result.elements.is_empty() {
        return Err(
            "thermal stress recovery requires aligned non-empty element results".to_string(),
        );
    }
    let mut weighted_stresses = result
        .elements
        .iter()
        .zip(result.input.elements.iter())
        .map(|(element, input)| {
            let stress = element.von_mises.abs();
            let weight = element.area * input.thickness;
            if !stress.is_finite() || !weight.is_finite() || weight <= 0.0 {
                return Err(format!(
                    "thermal stress recovery received invalid element {}",
                    element.id
                ));
            }
            Ok((stress, weight))
        })
        .collect::<Result<Vec<_>, String>>()?;
    weighted_stresses.sort_by(|left, right| left.0.total_cmp(&right.0));
    let total_weight = weighted_stresses
        .iter()
        .map(|(_, weight)| weight)
        .sum::<f64>();
    let mean = weighted_stresses
        .iter()
        .map(|(stress, weight)| stress * weight)
        .sum::<f64>()
        / total_weight;
    let rms = (weighted_stresses
        .iter()
        .map(|(stress, weight)| stress * stress * weight)
        .sum::<f64>()
        / total_weight)
        .sqrt();
    let p95 = weighted_percentile(&weighted_stresses, total_weight, 0.95);
    let max = weighted_stresses.last().map(|item| item.0).unwrap_or(0.0);
    Ok(CompositeThermalRecoveredStressStatistics {
        area_weighted_mean_stress_pa: mean,
        area_weighted_rms_stress_pa: rms,
        area_weighted_p95_stress_pa: p95,
        max_stress_pa: max,
        max_to_p95_ratio: if p95 > f64::EPSILON { max / p95 } else { 1.0 },
    })
}

pub fn composite_thermal_stress_recovery(
    statistics_by_level: &[(usize, CompositeThermalRecoveredStressStatistics)],
) -> CompositeThermalStressRecovery {
    build_stress_recovery(
        statistics_by_level,
        "uniform_mesh_element_area_weighted_von_mises_p95_and_rms",
    )
}

pub fn composite_thermal_interface_graded_stress_recovery(
    statistics_by_level: &[(usize, CompositeThermalRecoveredStressStatistics)],
) -> CompositeThermalStressRecovery {
    build_stress_recovery(
        statistics_by_level,
        "interface_graded_mesh_element_area_weighted_von_mises_p95_and_rms",
    )
}

pub fn composite_thermal_interface_grading_assessment(
    uniform_mesh: &CompositeThermalMeshConvergence,
    uniform_stress: &CompositeThermalStressRecovery,
    graded_mesh: CompositeThermalMeshConvergence,
    graded_stress: CompositeThermalStressRecovery,
) -> CompositeThermalInterfaceGradingAssessment {
    let p95_ratio = ratio(
        graded_stress.finest_pair_p95_relative_change,
        uniform_stress.finest_pair_p95_relative_change,
    );
    let p95_improved = p95_ratio.is_some_and(|value| value < 0.5);
    let energy_failed = graded_mesh
        .finest_pair_strain_energy_relative_change
        .is_some_and(|value| value > RECOVERED_STRESS_CONVERGENCE_TOLERANCE);
    let rms_failed = graded_stress
        .finest_pair_rms_relative_change
        .is_some_and(|value| value > RECOVERED_STRESS_CONVERGENCE_TOLERANCE);
    let peak_failed = graded_stress
        .finest_pair_max_relative_change
        .is_some_and(|value| value > RECOVERED_STRESS_CONVERGENCE_TOLERANCE);
    let diagnosis = if graded_mesh.status == "missing" || graded_stress.status == "missing" {
        "insufficient_evidence"
    } else if p95_improved && energy_failed && rms_failed && peak_failed {
        "localized_tail_resolution_improved_but_global_energy_and_peak_unstable"
    } else if graded_mesh.status == "pass" && graded_stress.status == "pass" {
        "graded_mesh_converged"
    } else {
        "graded_mesh_did_not_resolve_nonconvergence"
    };
    CompositeThermalInterfaceGradingAssessment {
        schema_version: COMPOSITE_THERMAL_INTERFACE_GRADING_SCHEMA_VERSION.to_string(),
        displacement_change_ratio_graded_to_uniform: ratio(
            graded_mesh.finest_pair_displacement_relative_change,
            uniform_mesh.finest_pair_displacement_relative_change,
        ),
        strain_energy_change_ratio_graded_to_uniform: ratio(
            graded_mesh.finest_pair_strain_energy_relative_change,
            uniform_mesh.finest_pair_strain_energy_relative_change,
        ),
        rms_change_ratio_graded_to_uniform: ratio(
            graded_stress.finest_pair_rms_relative_change,
            uniform_stress.finest_pair_rms_relative_change,
        ),
        p95_change_ratio_graded_to_uniform: p95_ratio,
        max_change_ratio_graded_to_uniform: ratio(
            graded_stress.finest_pair_max_relative_change,
            uniform_stress.finest_pair_max_relative_change,
        ),
        graded_mesh_convergence: graded_mesh,
        graded_stress_recovery: graded_stress,
        diagnosis: diagnosis.to_string(),
        qualification_effect: "diagnostic_only_does_not_override_uniform_mesh_gates".to_string(),
    }
}

fn build_stress_recovery(
    statistics_by_level: &[(usize, CompositeThermalRecoveredStressStatistics)],
    method: &str,
) -> CompositeThermalStressRecovery {
    let mut previous: Option<&CompositeThermalRecoveredStressStatistics> = None;
    let samples = statistics_by_level
        .iter()
        .map(|(level, statistics)| {
            let changes = previous.map(|old| {
                (
                    relative_change(
                        statistics.area_weighted_rms_stress_pa,
                        old.area_weighted_rms_stress_pa,
                    ),
                    relative_change(
                        statistics.area_weighted_p95_stress_pa,
                        old.area_weighted_p95_stress_pa,
                    ),
                    relative_change(statistics.max_stress_pa, old.max_stress_pa),
                )
            });
            previous = Some(statistics);
            CompositeThermalStressRecoverySample {
                elements_per_original_element: *level,
                element_count: 3 * level * level,
                statistics: statistics.clone(),
                rms_relative_change: changes.map(|value| value.0),
                p95_relative_change: changes.map(|value| value.1),
                max_relative_change: changes.map(|value| value.2),
            }
        })
        .collect::<Vec<_>>();
    let complete = samples.len() == COMPOSITE_THERMAL_REFINEMENT_LEVELS.len()
        && samples
            .iter()
            .zip(COMPOSITE_THERMAL_REFINEMENT_LEVELS)
            .all(|(sample, level)| {
                sample.elements_per_original_element == level
                    && valid_statistics(&sample.statistics)
            });
    let finest_pair_rms_relative_change = complete
        .then(|| samples.last()?.rms_relative_change)
        .flatten();
    let finest_pair_p95_relative_change = complete
        .then(|| samples.last()?.p95_relative_change)
        .flatten();
    let finest_pair_max_relative_change = complete
        .then(|| samples.last()?.max_relative_change)
        .flatten();
    let status = match (
        finest_pair_rms_relative_change,
        finest_pair_p95_relative_change,
    ) {
        (Some(rms), Some(p95))
            if rms <= RECOVERED_STRESS_CONVERGENCE_TOLERANCE
                && p95 <= RECOVERED_STRESS_CONVERGENCE_TOLERANCE =>
        {
            "pass"
        }
        (Some(_), Some(_)) => "fail",
        _ => "missing",
    };
    CompositeThermalStressRecovery {
        schema_version: COMPOSITE_THERMAL_STRESS_RECOVERY_SCHEMA_VERSION.to_string(),
        method: method.to_string(),
        refinement_levels: COMPOSITE_THERMAL_REFINEMENT_LEVELS.to_vec(),
        samples,
        finest_pair_rms_relative_change,
        finest_pair_p95_relative_change,
        finest_pair_max_relative_change,
        convergence_tolerance: RECOVERED_STRESS_CONVERGENCE_TOLERANCE,
        pass_metrics: vec![
            "area_weighted_rms_stress_pa".to_string(),
            "area_weighted_p95_stress_pa".to_string(),
        ],
        diagnostic_metrics: vec!["max_stress_pa".to_string(), "max_to_p95_ratio".to_string()],
        status: status.to_string(),
    }
}

fn weighted_percentile(values: &[(f64, f64)], total_weight: f64, quantile: f64) -> f64 {
    let target = total_weight * quantile;
    let mut cumulative = 0.0;
    for (value, weight) in values {
        cumulative += weight;
        if cumulative >= target {
            return *value;
        }
    }
    values.last().map(|item| item.0).unwrap_or(0.0)
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

fn valid_statistics(value: &CompositeThermalRecoveredStressStatistics) -> bool {
    value.area_weighted_mean_stress_pa.is_finite()
        && value.area_weighted_rms_stress_pa.is_finite()
        && value.area_weighted_p95_stress_pa.is_finite()
        && value.max_stress_pa.is_finite()
        && value.max_to_p95_ratio.is_finite()
}

#[cfg(test)]
mod tests {
    use super::{
        CompositeThermalRecoveredStressStatistics,
        composite_thermal_interface_graded_stress_recovery,
        composite_thermal_interface_grading_assessment, composite_thermal_stress_recovery,
        weighted_percentile,
    };
    use crate::{
        composite_thermal_interface_graded_mesh_convergence, composite_thermal_mesh_convergence,
    };

    #[test]
    fn weighted_percentile_ignores_a_tiny_isolated_peak_below_tail_mass() {
        let values = [(10.0, 0.96), (1_000.0, 0.04)];

        assert_eq!(weighted_percentile(&values, 1.0, 0.95), 10.0);
    }

    #[test]
    fn recovered_stress_requires_complete_converged_rms_and_p95_series() {
        let samples = [
            (1, statistics(100.0, 90.0, 130.0)),
            (2, statistics(95.0, 86.0, 145.0)),
            (4, statistics(93.0, 84.0, 165.0)),
            (8, statistics(92.0, 83.0, 210.0)),
        ];
        let result = composite_thermal_stress_recovery(&samples);
        let missing = composite_thermal_stress_recovery(&samples[..2]);

        assert_eq!(result.status, "pass");
        assert!(result.finest_pair_max_relative_change.unwrap() > 0.2);
        assert_eq!(missing.status, "missing");
    }

    #[test]
    fn grading_assessment_keeps_local_tail_improvement_diagnostic_only() {
        let uniform_mesh = composite_thermal_mesh_convergence(&[
            (1, 2.0e-4, 0.040, 8.0e7),
            (2, 1.9e-4, 0.030, 9.0e7),
            (4, 1.8e-4, 0.020, 1.0e8),
            (8, 1.7e-4, 0.014, 1.2e8),
        ]);
        let graded_mesh = composite_thermal_interface_graded_mesh_convergence(&[
            (1, 2.0e-4, 0.040, 8.0e7),
            (2, 1.9e-4, 0.030, 9.0e7),
            (4, 1.8e-4, 0.020, 1.0e8),
            (8, 1.75e-4, 0.014, 1.4e8),
        ]);
        let uniform_stress = composite_thermal_stress_recovery(&[
            (1, statistics(120.0, 100.0, 150.0)),
            (2, statistics(110.0, 90.0, 170.0)),
            (4, statistics(100.0, 80.0, 190.0)),
            (8, statistics(90.0, 70.0, 220.0)),
        ]);
        let graded_stress = composite_thermal_interface_graded_stress_recovery(&[
            (1, statistics(120.0, 100.0, 150.0)),
            (2, statistics(110.0, 90.0, 180.0)),
            (4, statistics(100.0, 80.0, 210.0)),
            (8, statistics(95.0, 78.0, 260.0)),
        ]);

        let assessment = composite_thermal_interface_grading_assessment(
            &uniform_mesh,
            &uniform_stress,
            graded_mesh,
            graded_stress,
        );

        assert_eq!(
            assessment.diagnosis,
            "localized_tail_resolution_improved_but_global_energy_and_peak_unstable"
        );
        assert!(assessment.p95_change_ratio_graded_to_uniform.unwrap() < 0.5);
        assert_eq!(
            assessment.qualification_effect,
            "diagnostic_only_does_not_override_uniform_mesh_gates"
        );
    }

    fn statistics(rms: f64, p95: f64, max: f64) -> CompositeThermalRecoveredStressStatistics {
        CompositeThermalRecoveredStressStatistics {
            area_weighted_mean_stress_pa: rms * 0.8,
            area_weighted_rms_stress_pa: rms,
            area_weighted_p95_stress_pa: p95,
            max_stress_pa: max,
            max_to_p95_ratio: max / p95,
        }
    }
}
