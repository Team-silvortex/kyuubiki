use crate::run::compact_report_value;
use crate::workflow_batch::missing_required_keys;
use crate::{
    HEADLESS_FAILURE_RECEIPT_SCHEMA_VERSION, HeadlessExecutionBatchStep,
    HeadlessExecutionStepReport, HeadlessFailureReceipt, HeadlessRisk,
};
use serde_json::{Value, json};
use std::borrow::Cow;
use std::collections::HashMap;

pub(crate) fn parse_binding(text: &str) -> Option<(usize, &str)> {
    let inner = text.trim().strip_prefix("{{")?.strip_suffix("}}")?.trim();
    let rest = inner.strip_prefix("steps.")?;
    let (step_text, output_path) = rest.split_once(".result.")?;
    Some((step_text.parse().ok()?, output_path.trim()))
}

pub(crate) fn resolve_step_payload<'a>(
    step: &'a HeadlessExecutionBatchStep,
    results: &HashMap<usize, Value>,
) -> Result<Cow<'a, Value>, String> {
    // Most large solver payloads contain no bindings and need no second copy.
    let payload = if contains_binding(&step.payload) {
        Cow::Owned(resolve_value(&step.payload, results)?)
    } else {
        Cow::Borrowed(&step.payload)
    };
    let missing = missing_required_keys(&step.action, &payload);
    if !missing.is_empty() {
        return Err(format!(
            "step {} resolved payload is missing required key {}",
            step.index,
            missing.join(", ")
        ));
    }
    Ok(payload)
}

fn contains_binding(value: &Value) -> bool {
    match value {
        Value::String(text) => parse_binding(text).is_some(),
        Value::Array(items) => items.iter().any(contains_binding),
        Value::Object(fields) => fields.values().any(contains_binding),
        _ => false,
    }
}

fn resolve_value(value: &Value, results: &HashMap<usize, Value>) -> Result<Value, String> {
    match value {
        Value::String(text) => match parse_binding(text) {
            Some((step, output)) => results
                .get(&step)
                .and_then(Value::as_object)
                .and_then(|result| result.get(output))
                .cloned()
                .ok_or_else(|| format!("binding source step {step} has no output {output}")),
            None => Ok(value.clone()),
        },
        Value::Array(items) => items
            .iter()
            .map(|item| resolve_value(item, results))
            .collect::<Result<Vec<_>, _>>()
            .map(Value::Array),
        Value::Object(fields) => fields
            .iter()
            .map(|(key, value)| Ok((key.clone(), resolve_value(value, results)?)))
            .collect::<Result<_, String>>()
            .map(Value::Object),
        _ => Ok(value.clone()),
    }
}

pub(crate) fn binding_failure_step(
    step: &HeadlessExecutionBatchStep,
    message: String,
) -> HeadlessExecutionStepReport {
    let receipt = HeadlessFailureReceipt {
        schema_version: HEADLESS_FAILURE_RECEIPT_SCHEMA_VERSION.into(),
        error_code: "kyuubiki.headless.binding_resolution".into(),
        category: "binding_resolution".into(),
        stage: "payload_resolution".into(),
        step_index: step.index,
        action: step.action.clone(),
        message,
        retryable: false,
        retry_strategy: "none".into(),
        recommended_action: "Inspect the source output and repair the dependent payload; do not replay completed source side effects automatically. Run-report previews are not checkpoints or full computation results.".into(),
    };
    HeadlessExecutionStepReport {
        index: step.index,
        action: step.action.clone(),
        risk: step.risk,
        status: "failed".into(),
        payload: compact_report_value(&step.payload),
        result_preview: json!({
            "error":receipt.message, "error_code":receipt.error_code,
            "failure_receipt":receipt,
        }),
        requires_confirmation: matches!(
            step.risk,
            HeadlessRisk::Sensitive | HeadlessRisk::Destructive
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step(payload: Value) -> HeadlessExecutionBatchStep {
        HeadlessExecutionBatchStep {
            index: 2,
            action: "service_health".into(),
            risk: HeadlessRisk::Normal,
            payload,
        }
    }

    #[test]
    fn unbound_large_payload_is_borrowed_even_after_a_source_result_exists() {
        let step = step(json!({"model":{"nodes":(0..1_000).collect::<Vec<_>>()}}));
        let results = HashMap::from([(1, json!({"status":"ok"}))]);
        assert!(matches!(
            resolve_step_payload(&step, &results).unwrap(),
            Cow::Borrowed(_)
        ));
    }

    #[test]
    fn missing_source_is_not_kept_as_a_literal_when_result_map_is_empty() {
        let step = step(json!({"forwarded":"{{steps.1.result.status}}"}));
        assert!(
            resolve_step_payload(&step, &HashMap::new())
                .unwrap_err()
                .contains("step 1")
        );
    }
}
