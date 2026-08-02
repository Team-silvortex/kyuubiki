use crate::{CompositePanelCandidateReport, MaterialQualityGate, material_quality_gate};

pub(crate) fn composite_coupling_quality_gates(
    rows: &[CompositePanelCandidateReport],
) -> Vec<MaterialQualityGate> {
    vec![
        material_quality_gate(
            "gate.electrothermal_loss.energy_balance",
            "Electrothermal loss projection energy balance",
            "electrothermal_loss_energy_balance_relative_error",
            "<=",
            1.0e-12,
            max_optional(rows.iter().filter_map(|row| {
                row.electrothermal_loss_projection
                    .as_ref()
                    .map(|projection| projection.energy_balance_relative_error)
            })),
            "Projected dielectric loss must equal the total heat load distributed to heat nodes.",
        ),
        material_quality_gate(
            "gate.joule_heating.energy_balance",
            "Conductor Joule heating projection energy balance",
            "joule_heating_energy_balance_relative_error",
            "<=",
            1.0e-12,
            max_optional(rows.iter().filter_map(|row| {
                row.joule_heating_projection
                    .as_ref()
                    .map(|projection| projection.energy_balance_relative_error)
            })),
            "Solved sigma-E-squared conductor loss must equal its added heat load.",
        ),
        material_quality_gate(
            "gate.electric_conduction.current_balance",
            "Electric-conduction terminal current balance",
            "electric_conduction_current_balance_relative_error",
            "<=",
            1.0e-12,
            max_optional(rows.iter().filter_map(|row| {
                row.joule_heating_projection
                    .as_ref()
                    .map(|projection| projection.current_balance_relative_error)
            })),
            "Recovered injected and extracted terminal currents must balance.",
        ),
        material_quality_gate(
            "gate.electric_conduction.power_balance",
            "Electric-conduction input and Joule power balance",
            "electric_conduction_power_balance_relative_error",
            "<=",
            1.0e-12,
            max_optional(rows.iter().filter_map(|row| {
                row.joule_heating_projection
                    .as_ref()
                    .map(|projection| projection.electrical_power_balance_relative_error)
            })),
            "Recovered terminal VI power must equal the integrated sigma-E-squared loss.",
        ),
        material_quality_gate(
            "gate.electric_conduction.free_node_residual",
            "Electric-conduction free-node residual",
            "electric_conduction_free_current_residual_relative_error",
            "<=",
            1.0e-10,
            max_optional(rows.iter().filter_map(|row| {
                row.joule_heating_projection
                    .as_ref()
                    .map(|projection| projection.free_current_residual_relative_error)
            })),
            "Every unconstrained potential must satisfy its applied-current equation.",
        ),
        material_quality_gate(
            "gate.electric_conduction.source_power_balance",
            "Electric-conduction source and total dissipation balance",
            "electric_conduction_source_power_balance_relative_error",
            "<=",
            1.0e-12,
            max_optional(rows.iter().filter_map(|row| {
                row.joule_heating_projection
                    .as_ref()
                    .map(|projection| projection.source_power_balance_relative_error)
            })),
            "Constraint, current-source, and impedance-terminal power must equal all electrical losses.",
        ),
        material_quality_gate(
            "gate.electrothermal_feedback.temperature_residual",
            "Electrothermal feedback temperature residual",
            "electrothermal_feedback_temperature_residual_ratio",
            "<=",
            1.0,
            max_optional(rows.iter().filter_map(|row| {
                row.electrothermal_feedback_convergence
                    .as_ref()
                    .and_then(|convergence| {
                        convergence
                            .final_temperature_residual_c
                            .map(|residual| residual / convergence.temperature_residual_tolerance_c)
                    })
            })),
            "Temperature-dependent dielectric feedback must reach its fixed-point temperature tolerance.",
        ),
        material_quality_gate(
            "gate.electrothermal_feedback.loss_change",
            "Electrothermal feedback loss stability",
            "electrothermal_feedback_loss_change_ratio",
            "<=",
            1.0,
            max_optional(rows.iter().filter_map(|row| {
                row.electrothermal_feedback_convergence
                    .as_ref()
                    .and_then(|convergence| {
                        convergence
                            .final_loss_relative_change
                            .map(|change| change / convergence.loss_relative_change_tolerance)
                    })
            })),
            "Successive dielectric-loss estimates must stabilize before downstream thermal stress is accepted.",
        ),
        material_quality_gate(
            "gate.electrothermal_feedback.conductivity_change",
            "Electrothermal feedback conductivity stability",
            "electrothermal_feedback_conductivity_change_ratio",
            "<=",
            1.0,
            max_optional(rows.iter().filter_map(|row| {
                row.electrothermal_feedback_convergence
                    .as_ref()
                    .and_then(|convergence| {
                        convergence
                            .final_max_conductivity_relative_change
                            .map(|change| {
                                change / convergence.conductivity_relative_change_tolerance
                            })
                    })
            })),
            "Every temperature-dependent thermal conductivity must stabilize before downstream thermal stress is accepted.",
        ),
        material_quality_gate(
            "gate.heat_to_thermal.coordinate_alignment",
            "Heat-to-thermal node coordinate alignment",
            "heat_to_thermal_maximum_coordinate_error_m",
            "<=",
            1.0e-12,
            max_optional(rows.iter().filter_map(|row| {
                row.heat_to_thermal_projection
                    .as_ref()
                    .map(|projection| projection.maximum_coordinate_error_m)
            })),
            "Every projected temperature must target a structurally identical node coordinate.",
        ),
        material_quality_gate(
            "gate.thermal_expansion.coverage",
            "Temperature-dependent thermal-expansion region coverage",
            "thermal_expansion_projection_coverage_fraction",
            ">=",
            1.0,
            min_optional(rows.iter().filter_map(|row| {
                row.thermal_expansion_projection
                    .as_ref()
                    .map(|projection| projection.coverage_fraction)
            })),
            "Every declared structural material region must receive its temperature-adjusted expansion coefficient.",
        ),
    ]
}

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

fn min_optional(values: impl Iterator<Item = f64>) -> Option<f64> {
    values.fold(None, |current: Option<f64>, value| {
        Some(current.map_or(value, |min| min.min(value)))
    })
}
