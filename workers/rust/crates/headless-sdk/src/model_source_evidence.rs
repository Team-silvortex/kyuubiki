use crate::workflow_bindings::parse_binding;
use crate::{
    HeadlessExecutionBatchStep, HeadlessExecutionStepReport, HeadlessModelSource,
    HeadlessModelSourceKind,
};
use serde_json::Value;

pub(crate) fn validate_saved_source_evidence(
    step: &HeadlessExecutionBatchStep,
    report: &HeadlessExecutionStepReport,
    previous: &[HeadlessExecutionStepReport],
) -> Result<(), String> {
    let version = matches!(
        step.action.as_str(),
        "solve_from_model_version" | "solve_and_wait_from_model_version"
    ) || (step.action == "direct_mesh_solve"
        && has_field(&step.payload, &["model_version_id", "modelVersionId"]));
    let mutable =
        step.action == "direct_mesh_solve" && has_field(&step.payload, &["model_id", "modelId"]);
    if !version && !mutable {
        return Ok(());
    }
    let fail = || {
        "headless research round saved model source is missing, invalid, or does not match the effective reference; execute the pinned snapshot again".to_string()
    };
    let source = report
        .result_preview
        .get("model_source")
        .ok_or_else(fail)
        .and_then(|value| HeadlessModelSource::decode(value).map_err(|_| fail()))?;
    let kind = if version {
        HeadlessModelSourceKind::ModelVersion
    } else {
        HeadlessModelSourceKind::Model
    };
    if source.source_kind != kind {
        return Err(fail());
    }
    for (keys, actual, required) in [
        (
            &["project_id", "projectId"][..],
            Some(source.project_id.as_str()),
            false,
        ),
        (
            &["model_id", "modelId"][..],
            Some(source.model_id.as_str()),
            !version,
        ),
        (
            &["model_version_id", "modelVersionId"][..],
            source.model_version_id.as_deref(),
            version,
        ),
    ] {
        if required && !has_field(&step.payload, keys) {
            return Err(fail());
        }
        for key in keys {
            if let Some(value) = step.payload.get(*key) {
                let expected = resolve_source_field(value, previous).map_err(|_| fail())?;
                if expected.as_str().is_none() || expected.as_str() != actual {
                    return Err(fail());
                }
            }
        }
    }
    if let Some(value) = step.payload.get("expected_model_source") {
        let expected = resolve_source_field(value, previous).map_err(|_| fail())?;
        let expected = HeadlessModelSource::decode(&expected).map_err(|_| fail())?;
        if source != expected {
            return Err(fail());
        }
    }
    if step.action == "solve_and_wait_from_model_version"
        && report.result_preview.pointer("/solve/model_source")
            != report.result_preview.get("model_source")
    {
        return Err(fail());
    }
    Ok(())
}

fn has_field(payload: &Value, keys: &[&str]) -> bool {
    keys.iter().any(|key| payload.get(*key).is_some())
}

// Resolve only source identities/pins against prior retained outputs. Large
// model payloads may be compacted and are deliberately not reconstructed here.
fn resolve_source_field(
    value: &Value,
    previous: &[HeadlessExecutionStepReport],
) -> Result<Value, ()> {
    match value {
        Value::String(text) => match parse_binding(text) {
            Some((index, output)) => previous
                .iter()
                .find(|step| step.index == index)
                .and_then(|step| step.result_preview.get(output))
                .cloned()
                .ok_or(()),
            None => Ok(value.clone()),
        },
        Value::Object(fields) => fields
            .iter()
            .map(|(key, value)| Ok((key.clone(), resolve_source_field(value, previous)?)))
            .collect::<Result<_, ()>>()
            .map(Value::Object),
        Value::Array(values) => values
            .iter()
            .map(|value| resolve_source_field(value, previous))
            .collect::<Result<_, ()>>()
            .map(Value::Array),
        _ => Ok(value.clone()),
    }
}
