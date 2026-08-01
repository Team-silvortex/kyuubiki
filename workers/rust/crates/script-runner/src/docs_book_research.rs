use serde_json::Value;
use std::fs;
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
                "validation_report_schema",
                "validation_report_fixture",
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
            paths.extend(
                surfaces
                    .values()
                    .filter_map(|surface| surface.get("validation_path").and_then(Value::as_str)),
            );
        } else {
            issues.push(format!(
                "{bootstrap_path}: missing execution_contract.surfaces"
            ));
        }
        validate_research_validation_report(root, bootstrap_path, execution, issues);
    } else {
        issues.push(format!("{bootstrap_path}: missing execution_contract"));
    }
    if let Some(preflight) = bootstrap.get("preflight").and_then(Value::as_object) {
        if preflight.get("execution_authority").and_then(Value::as_str)
            != Some("none_preflight_only")
        {
            issues.push(format!(
                "{bootstrap_path}: preflight execution authority must be none_preflight_only"
            ));
        }
        paths.extend(
            ["report_schema", "report_fixture"]
                .iter()
                .filter_map(|key| preflight.get(*key).and_then(Value::as_str)),
        );
        if let Some(surfaces) = preflight.get("surfaces").and_then(Value::as_object) {
            paths.extend(
                surfaces
                    .values()
                    .filter_map(|surface| surface.get("path").and_then(Value::as_str)),
            );
        } else {
            issues.push(format!("{bootstrap_path}: missing preflight.surfaces"));
        }
        validate_research_readiness_report(root, bootstrap_path, preflight, issues);
    } else {
        issues.push(format!("{bootstrap_path}: missing preflight"));
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

fn validate_research_readiness_report(
    root: &Path,
    bootstrap_path: &str,
    preflight: &serde_json::Map<String, Value>,
    issues: &mut Vec<String>,
) {
    let Some(schema_path) = preflight.get("report_schema").and_then(Value::as_str) else {
        issues.push(format!("{bootstrap_path}: missing preflight.report_schema"));
        return;
    };
    let Some(fixture_path) = preflight.get("report_fixture").and_then(Value::as_str) else {
        issues.push(format!(
            "{bootstrap_path}: missing preflight.report_fixture"
        ));
        return;
    };
    let Some(schema) = read_json(root, schema_path, issues) else {
        return;
    };
    let expected = "kyuubiki.model-research-readiness-report/v1";
    if schema
        .pointer("/properties/schema_version/const")
        .and_then(Value::as_str)
        != Some(expected)
    {
        issues.push(format!(
            "{schema_path}: readiness schema_version const is invalid"
        ));
    }
    let Some(fixture) = read_json(root, fixture_path, issues) else {
        return;
    };
    let valid = fixture.get("schema_version").and_then(Value::as_str) == Some(expected)
        && fixture.get("ready_for_planning").and_then(Value::as_bool) == Some(true)
        && fixture.get("execution_authority").and_then(Value::as_str)
            == Some("none_preflight_only")
        && fixture
            .get("selected_surface")
            .is_some_and(Value::is_object)
        && fixture
            .get("missing_resources")
            .and_then(Value::as_array)
            .is_some_and(Vec::is_empty)
        && fixture
            .get("blockers")
            .and_then(Value::as_array)
            .is_some_and(Vec::is_empty)
        && fixture
            .get("hard_rules")
            .and_then(Value::as_array)
            .is_some_and(|rules| rules.len() >= 8)
        && fixture
            .get("stop_conditions")
            .and_then(Value::as_array)
            .is_some_and(|conditions| conditions.len() >= 4);
    if !valid {
        issues.push(format!(
            "{fixture_path}: readiness trust boundary is invalid"
        ));
    }
}

fn validate_research_validation_report(
    root: &Path,
    bootstrap_path: &str,
    execution: &serde_json::Map<String, Value>,
    issues: &mut Vec<String>,
) {
    let Some(schema_path) = execution
        .get("validation_report_schema")
        .and_then(Value::as_str)
    else {
        issues.push(format!(
            "{bootstrap_path}: missing validation_report_schema"
        ));
        return;
    };
    let Some(fixture_path) = execution
        .get("validation_report_fixture")
        .and_then(Value::as_str)
    else {
        issues.push(format!(
            "{bootstrap_path}: missing validation_report_fixture"
        ));
        return;
    };
    let Some(schema) = read_json(root, schema_path, issues) else {
        return;
    };
    let expected = "kyuubiki.model-research-validation-report/v1";
    if schema
        .pointer("/properties/schema_version/const")
        .and_then(Value::as_str)
        != Some(expected)
    {
        issues.push(format!(
            "{schema_path}: validation report schema_version const is invalid"
        ));
    }
    let Some(fixture) = read_json(root, fixture_path, issues) else {
        return;
    };
    if fixture.get("schema_version").and_then(Value::as_str) != Some(expected)
        || fixture.get("claim_boundary").and_then(Value::as_str)
            != Some("screening_only_not_qualification")
        || fixture
            .get("external_validation_required")
            .and_then(Value::as_bool)
            != Some(true)
    {
        issues.push(format!(
            "{fixture_path}: validation report trust boundary is invalid"
        ));
    }
    let workflow = fixture.get("workflow_result");
    let has_artifacts = workflow
        .and_then(|value| value.get("artifact_keys"))
        .and_then(Value::as_array)
        .is_some_and(|items| {
            !items.is_empty()
                && items
                    .iter()
                    .all(|item| item.as_str().is_some_and(|text| !text.is_empty()))
        });
    let ids_match = workflow
        .and_then(|value| value.get("graph_id"))
        .and_then(Value::as_str)
        == fixture.get("workflow_id").and_then(Value::as_str);
    if !has_artifacts
        || !ids_match
        || workflow
            .and_then(|value| value.get("runtime_status"))
            .and_then(Value::as_str)
            != Some("completed")
    {
        issues.push(format!(
            "{fixture_path}: validation report workflow evidence is invalid"
        ));
    }
    let has_external_action = fixture
        .get("next_actions")
        .and_then(Value::as_array)
        .is_some_and(|items| {
            items
                .iter()
                .any(|item| item.as_str() == Some("external_validation_required"))
        });
    if !has_external_action {
        issues.push(format!(
            "{fixture_path}: validation report must retain external validation action"
        ));
    }
}

fn read_json(root: &Path, path: &str, issues: &mut Vec<String>) -> Option<Value> {
    let text = fs::read_to_string(root.join(path)).ok()?;
    match serde_json::from_str(&text) {
        Ok(value) => Some(value),
        Err(error) => {
            issues.push(format!("{path}: invalid json: {error}"));
            None
        }
    }
}
