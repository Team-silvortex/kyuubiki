use crate::{CompositePanelCandidateReport, MaterialQualityGate, material_quality_gate};

pub(crate) fn composite_stress_recovery_quality_gates(
    rows: &[CompositePanelCandidateReport],
) -> Vec<MaterialQualityGate> {
    vec![
        material_quality_gate(
            "gate.thermal_stress_recovery.rms",
            "Recovered thermal-stress RMS convergence",
            "thermal_stress_recovery_finest_pair_rms_relative_change",
            "<=",
            2.0e-2,
            max_optional(
                rows.iter()
                    .filter_map(|row| row.thermal_stress_recovery.finest_pair_rms_relative_change),
            ),
            "Area-weighted RMS von Mises stress should stabilize between the two finest meshes.",
        ),
        material_quality_gate(
            "gate.thermal_stress_recovery.p95",
            "Recovered thermal-stress P95 convergence",
            "thermal_stress_recovery_finest_pair_p95_relative_change",
            "<=",
            2.0e-2,
            max_optional(
                rows.iter()
                    .filter_map(|row| row.thermal_stress_recovery.finest_pair_p95_relative_change),
            ),
            "Area-weighted P95 von Mises stress should stabilize between the two finest meshes.",
        ),
    ]
}

fn max_optional(values: impl Iterator<Item = f64>) -> Option<f64> {
    values.fold(None, |current: Option<f64>, value| {
        Some(current.map_or(value, |max| max.max(value)))
    })
}
