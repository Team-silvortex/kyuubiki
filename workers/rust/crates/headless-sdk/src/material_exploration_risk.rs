use crate::material_exploration::MaterialExplorationRiskMitigationHint;
use serde_json::Value;

pub(crate) fn risk_mitigation_hints(
    decision: &str,
    report: &Value,
    focus_candidate_ids: &[String],
    violated_gates: &[String],
) -> Vec<MaterialExplorationRiskMitigationHint> {
    if decision != "mitigate_design_risk" {
        return Vec::new();
    }
    let warnings = report_warnings(report);
    report
        .get("candidates")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|candidate| candidate_is_focused(candidate, focus_candidate_ids))
        .flat_map(|candidate| {
            let candidate_id = candidate
                .get("candidate_id")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string();
            violated_gates.iter().map({
                let warnings = warnings.clone();
                move |gate_id| build_risk_hint(candidate, &candidate_id, gate_id, &warnings)
            })
        })
        .collect()
}

fn candidate_is_focused(candidate: &Value, focus_candidate_ids: &[String]) -> bool {
    let Some(candidate_id) = candidate.get("candidate_id").and_then(Value::as_str) else {
        return false;
    };
    focus_candidate_ids.iter().any(|id| id == candidate_id)
}

fn build_risk_hint(
    candidate: &Value,
    candidate_id: &str,
    gate_id: &str,
    warnings: &[String],
) -> MaterialExplorationRiskMitigationHint {
    let driver = candidate
        .get("weakest_interface")
        .and_then(|interface| interface.get("dominant_driver"))
        .and_then(Value::as_str)
        .or_else(|| risk_driver_from_gate(gate_id))
        .unwrap_or("quality_gate_violation");
    MaterialExplorationRiskMitigationHint {
        candidate_id: candidate_id.to_string(),
        gate_id: gate_id.to_string(),
        driver: driver.to_string(),
        recommendation: risk_recommendation(gate_id, driver, warnings),
    }
}

fn report_warnings(report: &Value) -> Vec<String> {
    report
        .get("warnings")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(ToString::to_string)
        .collect()
}

fn risk_driver_from_gate(gate_id: &str) -> Option<&'static str> {
    if gate_id.contains("interface") {
        Some("interface_mismatch")
    } else if gate_id.contains("temperature") {
        Some("thermal_load")
    } else if gate_id.contains("stress") {
        Some("mechanical_stress")
    } else if gate_id.contains("breakdown") {
        Some("electrical_margin")
    } else {
        None
    }
}

fn risk_recommendation(gate_id: &str, driver: &str, warnings: &[String]) -> String {
    let warning_note = if warnings.is_empty() {
        "no report warning text"
    } else {
        "see report warnings"
    };
    if gate_id.contains("interface") || driver.contains("expansion") {
        format!("try lower CTE mismatch or add compliant interlayer; {warning_note}")
    } else if gate_id.contains("temperature") {
        format!("increase thermal conductivity or reduce heat load; {warning_note}")
    } else if gate_id.contains("stress") {
        format!("reduce stiffness contrast or relax fixtures; {warning_note}")
    } else if gate_id.contains("breakdown") {
        format!("increase dielectric strength or reduce electric field; {warning_note}")
    } else {
        format!("generate conservative neighbors around focused candidates; {warning_note}")
    }
}
