use serde::{Deserialize, Serialize};

pub const COMPOSITE_THERMAL_CONVERGENCE_REGIME_SCHEMA_VERSION: &str =
    "kyuubiki.composite-thermal-convergence-regime/v1";

const REFINEMENT_RATIO: f64 = 2.0;
const GCI_SAFETY_FACTOR: f64 = 1.25;
const ORDER_CONSISTENCY_TOLERANCE: f64 = 0.25;
const QUALIFICATION_TOLERANCE: f64 = 2.0e-2;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompositeConvergenceMetricAssessment {
    pub metric: String,
    pub unit: String,
    pub qualification_role: String,
    pub regime: String,
    pub coarse_triplet_observed_order: Option<f64>,
    pub fine_triplet_observed_order: Option<f64>,
    pub observed_order_relative_difference: Option<f64>,
    pub richardson_extrapolated_value: Option<f64>,
    pub fine_grid_gci: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompositeThermalConvergenceRegimeAssessment {
    pub schema_version: String,
    pub method: String,
    pub refinement_ratio: f64,
    pub gci_safety_factor: f64,
    pub order_consistency_tolerance: f64,
    pub qualification_tolerance: f64,
    pub metrics: Vec<CompositeConvergenceMetricAssessment>,
    pub status: String,
    pub diagnosis: String,
    pub qualification_effect: String,
}

pub fn composite_thermal_convergence_regime(
    samples: &[(usize, f64, f64, f64)],
) -> CompositeThermalConvergenceRegimeAssessment {
    let complete = samples.len() == 4
        && samples.iter().zip([1, 2, 4, 8]).all(
            |((level, displacement, energy, stress), expected)| {
                *level == expected
                    && displacement.is_finite()
                    && energy.is_finite()
                    && stress.is_finite()
            },
        );
    let metrics = if complete {
        vec![
            assess_metric(
                "max_displacement_m",
                "m",
                "pass_metric",
                samples.map_values(|value| value.1),
            ),
            assess_metric(
                "total_strain_energy_j",
                "J",
                "pass_metric",
                samples.map_values(|value| value.2),
            ),
            assess_metric(
                "max_von_mises_stress_pa",
                "Pa",
                "diagnostic_metric",
                samples.map_values(|value| value.3),
            ),
        ]
    } else {
        missing_metrics()
    };
    let pass_metrics = metrics
        .iter()
        .filter(|metric| metric.qualification_role == "pass_metric")
        .collect::<Vec<_>>();
    let status = if !complete {
        "missing"
    } else if pass_metrics.iter().all(|metric| {
        metric
            .fine_grid_gci
            .is_some_and(|value| value <= QUALIFICATION_TOLERANCE)
    }) {
        "pass"
    } else {
        "fail"
    };
    let displacement = &metrics[0];
    let energy = &metrics[1];
    let stress = &metrics[2];
    let diagnosis = if !complete {
        "insufficient_evidence"
    } else if displacement.regime == "monotonic_pre_asymptotic"
        && energy.regime == "asymptotic_converging"
        && stress.regime == "monotonic_diverging"
    {
        "displacement_pre_asymptotic_energy_high_uncertainty_peak_stress_diverging"
    } else if status == "pass" {
        "pass_metrics_have_qualified_discretization_uncertainty"
    } else {
        "pass_metrics_lack_qualified_discretization_uncertainty"
    };
    CompositeThermalConvergenceRegimeAssessment {
        schema_version: COMPOSITE_THERMAL_CONVERGENCE_REGIME_SCHEMA_VERSION.to_string(),
        method: "four_level_constant_ratio_observed_order_and_fine_grid_gci".to_string(),
        refinement_ratio: REFINEMENT_RATIO,
        gci_safety_factor: GCI_SAFETY_FACTOR,
        order_consistency_tolerance: ORDER_CONSISTENCY_TOLERANCE,
        qualification_tolerance: QUALIFICATION_TOLERANCE,
        metrics,
        status: status.to_string(),
        diagnosis: diagnosis.to_string(),
        qualification_effect: "gci_gates_require_asymptotic_pass_metric_evidence".to_string(),
    }
}

pub fn missing_thermal_convergence_regime() -> CompositeThermalConvergenceRegimeAssessment {
    composite_thermal_convergence_regime(&[])
}

fn assess_metric(
    metric: &str,
    unit: &str,
    role: &str,
    values: [f64; 4],
) -> CompositeConvergenceMetricAssessment {
    let differences = [
        values[0] - values[1],
        values[1] - values[2],
        values[2] - values[3],
    ];
    let monotonic = differences.windows(2).all(|pair| pair[0] * pair[1] > 0.0);
    let coarse_order = observed_order(differences[0], differences[1]);
    let fine_order = observed_order(differences[1], differences[2]);
    let order_difference = relative_difference(coarse_order, fine_order);
    let asymptotic = monotonic
        && coarse_order.is_some_and(|value| value > 0.0)
        && fine_order.is_some_and(|value| value > 0.0)
        && order_difference.is_some_and(|value| value <= ORDER_CONSISTENCY_TOLERANCE);
    let regime = if !monotonic {
        "oscillatory"
    } else if coarse_order.is_some_and(|value| value <= 0.0)
        || fine_order.is_some_and(|value| value <= 0.0)
    {
        "monotonic_diverging"
    } else if asymptotic {
        "asymptotic_converging"
    } else {
        "monotonic_pre_asymptotic"
    };
    let denominator = fine_order
        .filter(|_| asymptotic)
        .map(|order| REFINEMENT_RATIO.powf(order) - 1.0)
        .filter(|value| *value > f64::EPSILON);
    let richardson = denominator.map(|value| values[3] + (values[3] - values[2]) / value);
    let fine_grid_gci = denominator.map(|value| {
        GCI_SAFETY_FACTOR * (values[3] - values[2]).abs()
            / values[3].abs().max(f64::EPSILON)
            / value
    });
    CompositeConvergenceMetricAssessment {
        metric: metric.to_string(),
        unit: unit.to_string(),
        qualification_role: role.to_string(),
        regime: regime.to_string(),
        coarse_triplet_observed_order: coarse_order,
        fine_triplet_observed_order: fine_order,
        observed_order_relative_difference: order_difference,
        richardson_extrapolated_value: richardson,
        fine_grid_gci,
    }
}

fn observed_order(coarse_difference: f64, fine_difference: f64) -> Option<f64> {
    if coarse_difference.abs() <= f64::EPSILON || fine_difference.abs() <= f64::EPSILON {
        return None;
    }
    Some((coarse_difference.abs() / fine_difference.abs()).ln() / REFINEMENT_RATIO.ln())
}

fn relative_difference(left: Option<f64>, right: Option<f64>) -> Option<f64> {
    match (left, right) {
        (Some(left), Some(right)) => {
            Some((left - right).abs() / left.abs().max(right.abs()).max(f64::EPSILON))
        }
        _ => None,
    }
}

fn missing_metrics() -> Vec<CompositeConvergenceMetricAssessment> {
    [
        ("max_displacement_m", "m", "pass_metric"),
        ("total_strain_energy_j", "J", "pass_metric"),
        ("max_von_mises_stress_pa", "Pa", "diagnostic_metric"),
    ]
    .map(
        |(metric, unit, role)| CompositeConvergenceMetricAssessment {
            metric: metric.to_string(),
            unit: unit.to_string(),
            qualification_role: role.to_string(),
            regime: "insufficient_evidence".to_string(),
            coarse_triplet_observed_order: None,
            fine_triplet_observed_order: None,
            observed_order_relative_difference: None,
            richardson_extrapolated_value: None,
            fine_grid_gci: None,
        },
    )
    .to_vec()
}

trait SampleValues {
    fn map_values(&self, field: impl Fn(&(usize, f64, f64, f64)) -> f64) -> [f64; 4];
}

impl SampleValues for [(usize, f64, f64, f64)] {
    fn map_values(&self, field: impl Fn(&(usize, f64, f64, f64)) -> f64) -> [f64; 4] {
        [
            field(&self[0]),
            field(&self[1]),
            field(&self[2]),
            field(&self[3]),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::composite_thermal_convergence_regime;

    #[test]
    fn separates_pre_asymptotic_convergence_from_divergence() {
        let result = composite_thermal_convergence_regime(&[
            (1, 0.00018909, 0.037979, 80.879e6),
            (2, 0.00017246, 0.018649, 81.163e6),
            (4, 0.00016106, 0.010843, 93.728e6),
            (8, 0.00015686, 0.007847, 119.453e6),
        ]);

        assert_eq!(result.status, "fail");
        assert_eq!(result.metrics[0].regime, "monotonic_pre_asymptotic");
        assert_eq!(result.metrics[1].regime, "asymptotic_converging");
        assert!(result.metrics[1].fine_grid_gci.unwrap() > 0.25);
        assert_eq!(result.metrics[2].regime, "monotonic_diverging");
        assert_eq!(
            result.diagnosis,
            "displacement_pre_asymptotic_energy_high_uncertainty_peak_stress_diverging"
        );
    }

    #[test]
    fn incomplete_series_never_produces_extrapolation() {
        let result = composite_thermal_convergence_regime(&[(1, 1.0, 1.0, 1.0)]);

        assert_eq!(result.status, "missing");
        assert!(
            result
                .metrics
                .iter()
                .all(|metric| metric.fine_grid_gci.is_none())
        );
    }
}
