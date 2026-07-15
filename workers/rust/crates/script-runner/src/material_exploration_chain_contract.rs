use serde_json::Value;
use std::ffi::OsString;
use std::fs;
use std::path::Path;

type RunnerResult<T> = Result<T, String>;

const SCHEMA_PATH: &str = "schemas/material-exploration-chain.schema.json";
const EXAMPLE_PATH: &str = "schemas/examples.material-exploration-chain.json";
const SCHEMAS_README_PATH: &str = "schemas/README.md";
const SDK_README_PATH: &str = "sdks/README.md";

const CHAIN_SCHEMA_VERSION: &str = "kyuubiki.material-exploration-chain/v1";
const CONVERGENCE_SCHEMA_VERSION: &str = "kyuubiki.material-chain-convergence-assessment/v1";
const OBJECTIVES_SCHEMA_VERSION: &str = "kyuubiki.material-next-round-optimization-objectives/v1";

pub(crate) fn run_check_material_exploration_chain_contract(
    root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    if args.iter().any(|arg| arg == "--self-test") {
        run_self_test(root)?;
        println!("material exploration chain contract check self-test passed");
        return Ok(0);
    }
    if !args.is_empty() {
        return Err(
            "check-material-exploration-chain-contract only accepts --self-test".to_string(),
        );
    }
    let issues = check_contracts(root)?;
    if let Some(issue) = issues.first() {
        eprintln!("material exploration chain contract check failed: {issue}");
        return Ok(1);
    }
    println!("material exploration chain contract check passed");
    Ok(0)
}

fn check_contracts(root: &Path) -> RunnerResult<Vec<String>> {
    let mut issues = Vec::new();
    check_schema(&read_json(root, SCHEMA_PATH)?, &mut issues);
    check_example(&read_json(root, EXAMPLE_PATH)?, &mut issues);
    check_documentation(root, &mut issues)?;
    Ok(issues)
}

fn run_self_test(root: &Path) -> RunnerResult<()> {
    let mut example = read_json(root, EXAMPLE_PATH)?;
    if let Some(count) = example.get("round_count").and_then(Value::as_i64) {
        example["round_count"] = Value::from(count + 1);
    }
    let mut issues = Vec::new();
    check_example(&example, &mut issues);
    if issues.is_empty() {
        return Err("self-test did not reject round count mismatch".to_string());
    }
    Ok(())
}

fn check_schema(schema: &Value, issues: &mut Vec<String>) {
    if schema
        .pointer("/properties/schema_version/const")
        .and_then(Value::as_str)
        != Some(CHAIN_SCHEMA_VERSION)
    {
        issues.push(format!(
            "{SCHEMA_PATH}: schema_version const must be {CHAIN_SCHEMA_VERSION}"
        ));
    }
    if schema
        .pointer("/$defs/convergenceAssessment/properties/schema_version/const")
        .and_then(Value::as_str)
        != Some(CONVERGENCE_SCHEMA_VERSION)
    {
        issues.push(format!(
            "{SCHEMA_PATH}: convergence schema_version const must be {CONVERGENCE_SCHEMA_VERSION}"
        ));
    }
    if schema
        .pointer("/$defs/optimizationObjectives/properties/schema_version/const")
        .and_then(Value::as_str)
        != Some(OBJECTIVES_SCHEMA_VERSION)
    {
        issues.push(format!(
            "{SCHEMA_PATH}: objectives schema_version const must be {OBJECTIVES_SCHEMA_VERSION}"
        ));
    }
    if schema
        .pointer("/$defs/materialCardRef/properties/schema_version/const")
        .and_then(Value::as_str)
        != Some("kyuubiki.material-card/v1")
    {
        issues.push(format!(
            "{SCHEMA_PATH}: materialCardRef schema_version const must be kyuubiki.material-card/v1"
        ));
    }
    for field in [
        "convergence_assessment",
        "optimization_trace",
        "summaries",
        "runs",
    ] {
        if !required_fields(schema)
            .iter()
            .any(|required| *required == field)
        {
            issues.push(format!("{SCHEMA_PATH}: missing required field {field}"));
        }
    }
}

fn check_example(example: &Value, issues: &mut Vec<String>) {
    if field(example, "schema_version") != CHAIN_SCHEMA_VERSION {
        issues.push(format!(
            "{EXAMPLE_PATH}: schema_version must be {CHAIN_SCHEMA_VERSION}"
        ));
    }
    if example
        .pointer("/convergence_assessment/schema_version")
        .and_then(Value::as_str)
        != Some(CONVERGENCE_SCHEMA_VERSION)
    {
        issues.push(format!(
            "{EXAMPLE_PATH}: convergence schema_version must be {CONVERGENCE_SCHEMA_VERSION}"
        ));
    }
    let optimization_trace = non_empty_array(example, "optimization_trace", EXAMPLE_PATH, issues);
    let summaries = non_empty_array(example, "summaries", EXAMPLE_PATH, issues);
    let runs = non_empty_array(example, "runs", EXAMPLE_PATH, issues);
    if example.get("round_count").and_then(Value::as_u64) != Some(runs.len() as u64) {
        issues.push(format!(
            "{EXAMPLE_PATH}: round_count must match runs length"
        ));
    }
    if optimization_trace.len() != summaries.len() {
        issues.push(format!(
            "{EXAMPLE_PATH}: optimization_trace length must match summaries length"
        ));
    }
    if summaries.len() != runs.len() {
        issues.push(format!(
            "{EXAMPLE_PATH}: summaries length must match runs length"
        ));
    }
    for (index, entry) in optimization_trace.iter().enumerate() {
        let context = format!("{EXAMPLE_PATH}#optimization_trace/{index}");
        require_string(entry.get("decision"), "decision", &context, issues);
        require_string(entry.get("mode"), "mode", &context, issues);
        require_string(
            entry.get("winner_candidate_id"),
            "winner_candidate_id",
            &context,
            issues,
        );
        non_empty_array(entry, "primary_metric_ids", &context, issues);
    }
    for (index, summary) in summaries.iter().enumerate() {
        let context = format!("{EXAMPLE_PATH}#summaries/{index}");
        require_string(
            summary.get("winner_candidate_id"),
            "winner_candidate_id",
            &context,
            issues,
        );
        if !summary
            .get("winner_score")
            .and_then(Value::as_f64)
            .is_some_and(f64::is_finite)
        {
            issues.push(format!("{context}: winner_score must be finite"));
        }
        if summary
            .pointer("/optimization_objectives/schema_version")
            .and_then(Value::as_str)
            != Some(OBJECTIVES_SCHEMA_VERSION)
        {
            issues.push(format!(
                "{context}: optimization_objectives schema_version must be {OBJECTIVES_SCHEMA_VERSION}"
            ));
        }
        let objectives = summary
            .get("optimization_objectives")
            .unwrap_or(&Value::Null);
        non_empty_array(objectives, "primary_metric_ids", &context, issues);
        for (ref_index, reference) in
            non_empty_array(summary, "material_card_refs", &context, issues)
                .iter()
                .enumerate()
        {
            let ref_context = format!("{context}.material_card_refs/{ref_index}");
            require_string(
                reference.get("material_card_id"),
                "material_card_id",
                &ref_context,
                issues,
            );
            if reference.get("schema_version").and_then(Value::as_str)
                != Some("kyuubiki.material-card/v1")
            {
                issues.push(format!(
                    "{ref_context}: schema_version must be kyuubiki.material-card/v1"
                ));
            }
            for field in [
                "candidate_id",
                "confidence",
                "unit_system",
                "parameter_scope",
            ] {
                require_string(reference.get(field), field, &ref_context, issues);
            }
        }
    }
}

fn check_documentation(root: &Path, issues: &mut Vec<String>) -> RunnerResult<()> {
    let schemas_readme = read_text(root, SCHEMAS_README_PATH)?;
    for required_path in [SCHEMA_PATH, EXAMPLE_PATH] {
        let file_name = Path::new(required_path)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(required_path);
        if !schemas_readme.contains(file_name) {
            issues.push(format!(
                "{SCHEMAS_README_PATH}: missing entry for {file_name}"
            ));
        }
    }
    let sdk_readme = read_text(root, SDK_README_PATH)?;
    for required_path in [SCHEMA_PATH, EXAMPLE_PATH] {
        if !sdk_readme.contains(required_path) {
            issues.push(format!(
                "{SDK_README_PATH}: missing link to {required_path}"
            ));
        }
    }
    Ok(())
}

fn required_fields(schema: &Value) -> Vec<&str> {
    schema
        .get("required")
        .and_then(Value::as_array)
        .map(|fields| fields.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default()
}

fn non_empty_array<'a>(
    value: &'a Value,
    field: &str,
    context: &str,
    issues: &mut Vec<String>,
) -> Vec<&'a Value> {
    let Some(items) = value.get(field).and_then(Value::as_array) else {
        issues.push(format!("{context}: {field} must be a non-empty array"));
        return Vec::new();
    };
    if items.is_empty() {
        issues.push(format!("{context}: {field} must be a non-empty array"));
    }
    items.iter().collect()
}

fn require_string(value: Option<&Value>, field: &str, context: &str, issues: &mut Vec<String>) {
    if value.and_then(Value::as_str).is_none_or(str::is_empty) {
        issues.push(format!("{context}: {field} must be a non-empty string"));
    }
}

fn read_json(root: &Path, relative_path: &str) -> RunnerResult<Value> {
    let text = read_text(root, relative_path)?;
    serde_json::from_str(&text).map_err(|error| format!("{relative_path}: invalid json: {error}"))
}

fn read_text(root: &Path, relative_path: &str) -> RunnerResult<String> {
    fs::read_to_string(root.join(relative_path))
        .map_err(|error| format!("failed to read {relative_path}: {error}"))
}

fn field<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{CHAIN_SCHEMA_VERSION, CONVERGENCE_SCHEMA_VERSION, OBJECTIVES_SCHEMA_VERSION};

    #[test]
    fn schema_versions_are_stable() {
        assert_eq!(
            CHAIN_SCHEMA_VERSION,
            "kyuubiki.material-exploration-chain/v1"
        );
        assert_eq!(
            CONVERGENCE_SCHEMA_VERSION,
            "kyuubiki.material-chain-convergence-assessment/v1"
        );
        assert_eq!(
            OBJECTIVES_SCHEMA_VERSION,
            "kyuubiki.material-next-round-optimization-objectives/v1"
        );
    }
}
