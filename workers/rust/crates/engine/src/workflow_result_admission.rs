use serde_json::{Map, Value};

/// Reject explicit solver failure before reducing results or making quality decisions.
/// An absent marker remains valid for metric-only and older linear-result contracts;
/// it is not proof of convergence. Trial histories are deliberately not scanned.
pub(crate) fn require_converged_result(
    object: &Map<String, Value>,
    operator: &str,
    path: &str,
) -> Result<(), String> {
    check_marker(object, operator, path)?;
    // Material P-Delta results carry their final status in this contract-defined wrapper.
    if let Some(stability) = object.get("stability_result") {
        let stability = stability
            .as_object()
            .ok_or_else(|| format!("{operator} expects an object {path}.stability_result"))?;
        check_marker(stability, operator, &format!("{path}.stability_result"))?;
    }
    Ok(())
}

fn check_marker(object: &Map<String, Value>, operator: &str, path: &str) -> Result<(), String> {
    match object.get("converged") {
        None | Some(Value::Bool(true)) => Ok(()),
        Some(Value::Bool(false)) => {
            // Load-factor scales vary by operator. Report the factor, do not reinterpret it.
            let achieved = object
                .get("achieved_load_factor")
                .and_then(Value::as_f64)
                .filter(|value| value.is_finite())
                .map(|value| value.to_string())
                .unwrap_or_else(|| "unknown".to_string());
            Err(format!(
                "{operator} rejects nonconverged result: {path}.converged=false \
                 (achieved_load_factor={achieved}); inspect the raw solver result before assessment"
            ))
        }
        Some(_) => Err(format!("{operator} expects a boolean {path}.converged")),
    }
}
