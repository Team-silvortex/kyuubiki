use serde_json::Value;
use std::ffi::OsString;
use std::fs;
use std::path::Path;

type RunnerResult<T> = Result<T, String>;

const SCHEMA_PATH: &str = "schemas/material-card.schema.json";
const EXAMPLE_PATH: &str = "schemas/examples.material-card.json";
const SCHEMAS_README_PATH: &str = "schemas/README.md";
const ROADMAP_PATH: &str = "docs/material-research-roadmap.md";
const RUNTIME_PATH: &str = "apps/web/lib/kyuubiki_web/workflow_material_card_runtime.ex";
const TEST_PATH: &str = "apps/web/test/kyuubiki_web/workflow_material_card_runtime_test.exs";
const SCHEMA_VERSION: &str = "kyuubiki.material-card/v1";

pub(crate) fn run_check_material_card_contract(
    root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    if args.iter().any(|arg| arg == "--self-test") {
        run_self_test(root)?;
        println!("material card contract check self-test passed");
        return Ok(0);
    }
    if !args.is_empty() {
        return Err("check-material-card-contract only accepts --self-test".to_string());
    }
    let issues = check_contract(root)?;
    if let Some(issue) = issues.first() {
        eprintln!("material card contract check failed: {issue}");
        return Ok(1);
    }
    println!("material card contract check passed");
    Ok(0)
}

fn check_contract(root: &Path) -> RunnerResult<Vec<String>> {
    let mut issues = Vec::new();
    check_schema(&read_json(root, SCHEMA_PATH)?, &mut issues);
    check_example(&read_json(root, EXAMPLE_PATH)?, &mut issues);
    check_text_anchors(root, &mut issues)?;
    Ok(issues)
}

fn run_self_test(root: &Path) -> RunnerResult<()> {
    let mut missing_unit = read_json(root, EXAMPLE_PATH)?;
    if let Some(parameter) = missing_unit
        .pointer_mut("/parameters/thermal_conductivity")
        .and_then(Value::as_object_mut)
    {
        parameter.remove("unit");
    }
    expect_example_failure(&missing_unit, "missing parameter unit")?;

    let mut bad_version = read_json(root, EXAMPLE_PATH)?;
    bad_version["schema_version"] = Value::from("kyuubiki.material-card/v0");
    expect_example_failure(&bad_version, "unsupported schema version")
}

fn expect_example_failure(example: &Value, label: &str) -> RunnerResult<()> {
    let mut issues = Vec::new();
    check_example(example, &mut issues);
    if issues.is_empty() {
        return Err(format!("self-test did not reject {label}"));
    }
    Ok(())
}

fn check_schema(schema: &Value, issues: &mut Vec<String>) {
    if schema
        .pointer("/properties/schema_version/const")
        .and_then(Value::as_str)
        != Some(SCHEMA_VERSION)
    {
        issues.push(format!(
            "{SCHEMA_PATH}: schema_version const must be {SCHEMA_VERSION}"
        ));
    }
    for field in [
        "schema_version",
        "material_id",
        "display_name",
        "unit_system",
        "provenance",
        "confidence",
        "parameters",
    ] {
        if !required_fields(schema)
            .iter()
            .any(|required| *required == field)
        {
            issues.push(format!("{SCHEMA_PATH}: missing required field {field}"));
        }
    }
    for unit in ["si", "engineering", "mixed"] {
        if !enum_values(schema, "/properties/unit_system/enum").contains(&unit) {
            issues.push(format!("{SCHEMA_PATH}: unit_system enum missing {unit}"));
        }
    }
    for confidence in ["unknown", "screening", "datasheet", "measured", "certified"] {
        if !enum_values(schema, "/properties/confidence/properties/level/enum")
            .contains(&confidence)
        {
            issues.push(format!(
                "{SCHEMA_PATH}: confidence.level enum missing {confidence}"
            ));
        }
    }
    let parameter_required = schema
        .pointer("/$defs/parameter/required")
        .and_then(Value::as_array)
        .map(|values| values.iter().filter_map(Value::as_str).collect::<Vec<_>>())
        .unwrap_or_default();
    for field in ["kind", "unit"] {
        if !parameter_required.iter().any(|required| *required == field) {
            issues.push(format!("{SCHEMA_PATH}: parameter missing required {field}"));
        }
    }
}

fn check_example(example: &Value, issues: &mut Vec<String>) {
    if field(example, "schema_version") != SCHEMA_VERSION {
        issues.push(format!(
            "{EXAMPLE_PATH}: schema_version must be {SCHEMA_VERSION}"
        ));
    }
    for field_name in ["material_id", "display_name", "unit_system"] {
        require_string(example.get(field_name), field_name, EXAMPLE_PATH, issues);
    }
    require_string(
        example.pointer("/provenance/source_id"),
        "provenance.source_id",
        EXAMPLE_PATH,
        issues,
    );
    require_string(
        example.pointer("/provenance/source_label"),
        "provenance.source_label",
        EXAMPLE_PATH,
        issues,
    );
    require_string(
        example.pointer("/confidence/level"),
        "confidence.level",
        EXAMPLE_PATH,
        issues,
    );
    match example.get("parameters").and_then(Value::as_object) {
        Some(parameters) if !parameters.is_empty() => {
            for (name, parameter) in parameters {
                require_string(
                    parameter.get("kind"),
                    &format!("parameters.{name}.kind"),
                    EXAMPLE_PATH,
                    issues,
                );
                require_string(
                    parameter.get("unit"),
                    &format!("parameters.{name}.unit"),
                    EXAMPLE_PATH,
                    issues,
                );
                if parameter.get("value").is_none() {
                    issues.push(format!(
                        "{EXAMPLE_PATH}: parameters.{name}.value is required"
                    ));
                }
            }
        }
        _ => issues.push(format!(
            "{EXAMPLE_PATH}: parameters must be a non-empty object"
        )),
    }
}

fn check_text_anchors(root: &Path, issues: &mut Vec<String>) -> RunnerResult<()> {
    for (path, anchors) in [
        (
            SCHEMAS_README_PATH,
            vec!["material-card.schema.json", "examples.material-card.json"],
        ),
        (
            ROADMAP_PATH,
            vec![
                "schemas/material-card.schema.json",
                "schemas/examples.material-card.json",
            ],
        ),
        (
            RUNTIME_PATH,
            vec![
                SCHEMA_VERSION,
                "validate_material_card",
                "material_card_reliability_envelope",
                "unit_mismatch",
            ],
        ),
        (
            TEST_PATH,
            vec![
                "validates material cards",
                "material_card_reliability_envelope",
                "unit_mismatch",
            ],
        ),
    ] {
        let text = read_text(root, path)?;
        for anchor in anchors {
            if !text.contains(anchor) {
                issues.push(format!("{path}: missing contract anchor {anchor:?}"));
            }
        }
    }
    Ok(())
}

fn read_json(root: &Path, relative: &str) -> RunnerResult<Value> {
    let text = read_text(root, relative)?;
    serde_json::from_str(&text).map_err(|error| format!("{relative}: invalid json: {error}"))
}

fn read_text(root: &Path, relative: &str) -> RunnerResult<String> {
    fs::read_to_string(root.join(relative))
        .map_err(|error| format!("failed to read {relative}: {error}"))
}

fn required_fields(value: &Value) -> Vec<&str> {
    value
        .get("required")
        .and_then(Value::as_array)
        .map(|values| values.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default()
}

fn enum_values<'a>(value: &'a Value, pointer: &str) -> Vec<&'a str> {
    value
        .pointer(pointer)
        .and_then(Value::as_array)
        .map(|values| values.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default()
}

fn require_string(value: Option<&Value>, label: &str, path: &str, issues: &mut Vec<String>) {
    if !matches!(value.and_then(Value::as_str), Some(text) if !text.is_empty()) {
        issues.push(format!("{path}: {label} must be a non-empty string"));
    }
}

fn field<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or_default()
}
