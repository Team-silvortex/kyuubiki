use serde_json::Value;
use std::path::Path;

pub(crate) fn validate_model_research_bootstrap(
    root: &Path,
    bootstrap_path: &str,
    bootstrap: &Value,
    issues: &mut Vec<String>,
) {
    if bootstrap.get("schema_version").and_then(Value::as_str)
        != Some("kyuubiki.model-research-bootstrap/v1")
    {
        issues.push(format!(
            "{bootstrap_path}: unsupported or missing schema_version"
        ));
    }

    let mut paths = Vec::new();
    if let Some(path) = bootstrap.get("entrypoint").and_then(Value::as_str) {
        paths.push(path);
    } else {
        issues.push(format!("{bootstrap_path}: missing entrypoint"));
    }
    if let Some(documents) = bootstrap
        .get("required_documents")
        .and_then(Value::as_array)
    {
        paths.extend(
            documents
                .iter()
                .filter_map(|entry| entry.get("path").and_then(Value::as_str)),
        );
    } else {
        issues.push(format!("{bootstrap_path}: missing required_documents"));
    }
    if let Some(surfaces) = bootstrap.get("sdk_surfaces").and_then(Value::as_object) {
        paths.extend(
            surfaces
                .values()
                .filter_map(|entry| entry.get("path").and_then(Value::as_str)),
        );
    } else {
        issues.push(format!("{bootstrap_path}: missing sdk_surfaces"));
    }
    if let Some(execution) = bootstrap
        .get("execution_contract")
        .and_then(Value::as_object)
    {
        paths.extend(
            [
                "approval_schema",
                "approval_fixture",
                "receipt_schema",
                "frontier_schema",
                "frontier_fixture",
            ]
            .iter()
            .filter_map(|key| execution.get(*key).and_then(Value::as_str)),
        );
        if let Some(surfaces) = execution.get("surfaces").and_then(Value::as_object) {
            paths.extend(
                surfaces
                    .values()
                    .filter_map(|surface| surface.get("path").and_then(Value::as_str)),
            );
            paths.extend(
                surfaces
                    .values()
                    .filter_map(|surface| surface.get("frontier_path").and_then(Value::as_str)),
            );
        } else {
            issues.push(format!(
                "{bootstrap_path}: missing execution_contract.surfaces"
            ));
        }
    } else {
        issues.push(format!("{bootstrap_path}: missing execution_contract"));
    }
    if let Some(first_research) = bootstrap.get("first_research").and_then(Value::as_object) {
        paths.extend(
            [
                "session_fixture",
                "proposal_fixture",
                "catalog_request_fixture",
            ]
            .iter()
            .filter_map(|key| first_research.get(*key).and_then(Value::as_str)),
        );
    } else {
        issues.push(format!("{bootstrap_path}: missing first_research"));
    }

    for path in paths {
        if Path::new(path).is_absolute() {
            issues.push(format!(
                "{bootstrap_path}: absolute path is forbidden: {path}"
            ));
        } else if !root.join(path).is_file() {
            issues.push(format!("{bootstrap_path}: missing referenced file {path}"));
        }
    }
}
