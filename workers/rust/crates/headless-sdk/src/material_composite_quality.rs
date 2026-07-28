use crate::{CompositePanelCandidateReport, MaterialQualityGate, material_quality_gate};

pub(crate) fn composite_structural_quality_gates(
    rows: &[CompositePanelCandidateReport],
) -> Vec<MaterialQualityGate> {
    let mut gates = vec![
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
    ];
    for (metric, id_suffix, label) in [
        ("max_displacement_m", "displacement", "displacement"),
        ("total_strain_energy_j", "strain_energy", "strain energy"),
    ] {
        gates.push(material_quality_gate(
            &format!("gate.thermal_mesh_gci.{id_suffix}"),
            &format!("Thermal-structural {label} fine-grid GCI"),
            &format!("thermal_mesh_{id_suffix}_fine_grid_gci"),
            "<=",
            2.0e-2,
            max_optional(rows.iter().filter_map(|row| metric_gci(row, metric))),
            "Fine-grid GCI is accepted only after the observed order is asymptotically consistent.",
        ));
    }
    gates.push(material_quality_gate(
        "gate.thermal_solver.relative_residual",
        "Thermal-structural algebraic residual",
        "thermal_solver_max_relative_residual",
        "<=",
        1.0e-10,
        max_optional(rows.iter().filter_map(|row| {
            row.thermal_mesh_convergence
                .algebraic_validation
                .max_relative_residual
        })),
        "Every retained uniform, regularized, and graded solve must satisfy the algebraic residual tolerance.",
    ));
    gates
}

fn metric_gci(row: &CompositePanelCandidateReport, metric: &str) -> Option<f64> {
    row.thermal_mesh_convergence
        .regime_assessment
        .metrics
        .iter()
        .find(|assessment| assessment.metric == metric)
        .and_then(|assessment| assessment.fine_grid_gci)
}

fn max_optional(values: impl Iterator<Item = f64>) -> Option<f64> {
    values.fold(None, |current: Option<f64>, value| {
        Some(current.map_or(value, |max| max.max(value)))
    })
}
