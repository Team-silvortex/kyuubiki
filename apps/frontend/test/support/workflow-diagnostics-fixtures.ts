export function buildDiagnosticsSummaryPayload(domain: "electrostatic" | "thermal" | "thermo") {
  if (domain === "electrostatic") {
    return {
      summary: {
        diagnostic_contract: "kyuubiki.workflow_diagnostics/v1",
        diagnostic_domain: "electrostatic",
        diagnostic_subject: "electrostatic_result",
        diagnostic_prefix: "electrostatic",
        diagnostic_node_count: 4,
        diagnostic_element_count: 2,
        diagnostic_metric_groups: ["field", "potential"],
        electrostatic_field_peak_magnitude: 8.4,
        electrostatic_potential_max: 12,
      },
    };
  }
  if (domain === "thermal") {
    return {
      summary: {
        diagnostic_contract: "kyuubiki.workflow_diagnostics/v1",
        diagnostic_domain: "thermal",
        diagnostic_subject: "thermal_result",
        diagnostic_prefix: "thermal",
        diagnostic_node_count: 4,
        diagnostic_element_count: 3,
        diagnostic_metric_groups: ["temperature", "flux"],
        thermal_temperature_max: 92,
        thermal_heat_flux_peak_magnitude: 16.5,
      },
    };
  }
  return {
    summary: {
      diagnostic_contract: "kyuubiki.workflow_diagnostics/v1",
      diagnostic_domain: "thermo",
      diagnostic_subject: "thermo_result",
      diagnostic_prefix: "thermo",
      diagnostic_node_count: 4,
      diagnostic_element_count: 3,
      diagnostic_metric_groups: ["stress", "displacement"],
      thermo_peak_stress: 142,
      thermo_peak_displacement: 0.0028,
    },
  };
}

export function buildDiagnosticsGuardNode() {
  return {
    id: "evaluate_diagnostics_guard",
    kind: "transform",
    operator_id: "transform.evaluate_diagnostics_bundle_guard",
    config: {
      rules: [
        {
          source: "thermal",
          field: "thermal_temperature_max",
          threshold: 120,
          severity: "warn",
          comparison: "gt",
          label: "thermal temperature",
        },
        {
          source: "thermo",
          field: "thermo_peak_stress",
          threshold: 180,
          severity: "block",
          comparison: "gt",
          label: "stress ceiling",
        },
        {
          source: "electrostatic",
          field: "electrostatic_field_peak_magnitude",
          threshold: 9,
          severity: "warn",
          comparison: "gt",
          label: "field ceiling",
        },
      ],
    },
  };
}
