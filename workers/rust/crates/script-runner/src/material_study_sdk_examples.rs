use serde_json::Value;
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

mod bundle;

const PLAN_SCHEMA_VERSION: &str = "kyuubiki.material-study-execution-plan/v1";
const MATERIAL_CARD_SCHEMA_VERSION: &str = "kyuubiki.material-card/v1";

type RunnerResult<T> = Result<T, String>;

pub(crate) fn run_check_material_study_sdk_examples(
    root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    let self_test = parse_args(args)?;
    let result = if self_test {
        run_self_test()
    } else {
        check_examples(root)
    };
    match result {
        Ok(message) => {
            println!("{message}");
            Ok(0)
        }
        Err(issue) => {
            eprintln!("material study SDK example check failed: {issue}");
            Ok(1)
        }
    }
}

fn parse_args(args: Vec<OsString>) -> RunnerResult<bool> {
    let mut self_test = false;
    for arg in args {
        match arg.to_string_lossy().as_ref() {
            "--self-test" => self_test = true,
            "--help" | "-h" => {
                println!(
                    "usage: kyuubiki-script-runner check-material-study-sdk-examples [--self-test]"
                );
                return Ok(self_test);
            }
            other => return Err(format!("unknown argument {other}")),
        }
    }
    Ok(self_test)
}

fn run_self_test() -> RunnerResult<&'static str> {
    let bad_plan = parse_json_output(
        "self-test",
        "compiled\n{\"schema_version\":\"wrong\",\"steps\":[],\"step_count\":0}",
    )?;
    expect_failure("invalid schema version", || {
        check_plan("self-test", &bad_plan)
    })?;
    let bad_report = parse_key_value_output(
        "self-test-report",
        "study=wrong\nwinner=copper_polyimide_aluminum\nreliability=ready_for_next_round\n",
    )?;
    expect_failure("invalid report example", || {
        check_report_example("self-test-report", &bad_report)
    })?;
    let bad_bundle = parse_key_value_output(
        "self-test-bundle",
        "schema=wrong\nstudy=heat-spreader\nwinner=pyrolytic_graphite_in_plane\nreliability=blocked_by_quality_gates\n",
    )?;
    expect_failure("invalid bundle example", || {
        check_bundle_example(
            "self-test-bundle",
            &bad_bundle,
            ExpectedBundle {
                study: "heat-spreader",
                winner: "pyrolytic_graphite_in_plane",
                reliability: "blocked_by_quality_gates",
            },
        )
    })?;
    Ok("material study SDK example check self-test passed")
}

fn expect_failure(label: &str, check: impl FnOnce() -> RunnerResult<()>) -> RunnerResult<()> {
    if check().is_ok() {
        Err(format!("self-test did not reject {label}"))
    } else {
        Ok(())
    }
}

fn check_examples(root: &Path) -> RunnerResult<&'static str> {
    for example in plan_examples(root) {
        let output = run_example(&example)?;
        check_plan(&example.name, &parse_json_output(&example.name, &output)?)?;
    }
    for example in report_examples(root) {
        let output = run_example(&example)?;
        check_report_example(
            &example.name,
            &parse_key_value_output(&example.name, &output)?,
        )?;
    }
    for example in bundle_examples(root, None) {
        let output = run_example(&example)?;
        check_bundle_example(
            &example.name,
            &parse_key_value_output(&example.name, &output)?,
            ExpectedBundle {
                study: "heat-spreader",
                winner: "pyrolytic_graphite_in_plane",
                reliability: "blocked_by_quality_gates",
            },
        )?;
    }
    let composite = bundle::ensure_composite_bundle(root)?;
    for example in bundle_examples(root, Some(&composite)) {
        let output = run_example(&example)?;
        check_bundle_example(
            &example.name,
            &parse_key_value_output(&example.name, &output)?,
            ExpectedBundle {
                study: "composite-thermo-electric-panel",
                winner: "copper_ptfe_glass_epoxy",
                reliability: "blocked_by_quality_gates",
            },
        )?;
    }
    Ok("material study SDK example check passed")
}

struct Example {
    name: String,
    command: &'static str,
    args: Vec<String>,
    cwd: PathBuf,
    env: Vec<(&'static str, String)>,
}

fn plan_examples(root: &Path) -> Vec<Example> {
    vec![
        Example {
            name: "rust".to_string(),
            command: "cargo",
            args: vec![
                "run",
                "--quiet",
                "--manifest-path",
                "sdks/rust/Cargo.toml",
                "--example",
                "plan_material_study",
            ]
            .into_iter()
            .map(str::to_string)
            .collect(),
            cwd: root.to_path_buf(),
            env: vec![],
        },
        Example {
            name: "python".to_string(),
            command: "python3",
            args: vec!["sdks/python/examples/plan_material_study.py".to_string()],
            cwd: root.to_path_buf(),
            env: vec![("PYTHONPATH", root.join("sdks/python").display().to_string())],
        },
        Example {
            name: "elixir".to_string(),
            command: "mix",
            args: vec![
                "run".to_string(),
                "examples/plan_material_study.exs".to_string(),
            ],
            cwd: root.join("sdks/elixir"),
            env: vec![],
        },
    ]
}

fn report_examples(root: &Path) -> Vec<Example> {
    vec![
        Example {
            name: "python-report".to_string(),
            command: "python3",
            args: vec!["sdks/python/examples/run_material_report.py".to_string()],
            cwd: root.to_path_buf(),
            env: vec![("PYTHONPATH", root.join("sdks/python").display().to_string())],
        },
        Example {
            name: "elixir-report".to_string(),
            command: "mix",
            args: vec![
                "run".to_string(),
                "examples/run_material_report.exs".to_string(),
            ],
            cwd: root.join("sdks/elixir"),
            env: vec![],
        },
    ]
}

fn bundle_examples(root: &Path, bundle: Option<&Path>) -> Vec<Example> {
    let bundle_arg = bundle.map(|path| path.display().to_string());
    let mut examples = vec![
        Example {
            name: "rust-bundle".to_string(),
            command: "cargo",
            args: vec![
                "run",
                "--quiet",
                "--manifest-path",
                "sdks/rust/Cargo.toml",
                "--example",
                "validate_material_research_bundle",
            ]
            .into_iter()
            .map(str::to_string)
            .collect(),
            cwd: root.to_path_buf(),
            env: vec![],
        },
        Example {
            name: "python-bundle".to_string(),
            command: "python3",
            args: vec!["sdks/python/examples/validate_material_research_bundle.py".to_string()],
            cwd: root.to_path_buf(),
            env: vec![("PYTHONPATH", root.join("sdks/python").display().to_string())],
        },
        Example {
            name: "elixir-bundle".to_string(),
            command: "mix",
            args: vec![
                "run".to_string(),
                "examples/validate_material_research_bundle.exs".to_string(),
            ],
            cwd: root.join("sdks/elixir"),
            env: vec![],
        },
    ];
    if let Some(arg) = bundle_arg {
        for example in &mut examples {
            example.name = example.name.replace("-bundle", "-composite-bundle");
            example.args.push(arg.clone());
        }
    }
    examples
}

fn run_example(example: &Example) -> RunnerResult<String> {
    let mut command = Command::new(example.command);
    command.args(&example.args).current_dir(&example.cwd);
    for (key, value) in &example.env {
        command.env(key, value);
    }
    let output = command
        .output()
        .map_err(|error| format!("{}: failed to run: {error}", example.name))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(format!(
            "{}: {}",
            example.name,
            if stderr.is_empty() {
                "command failed".to_string()
            } else {
                stderr
            }
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn parse_json_output(name: &str, output: &str) -> RunnerResult<Value> {
    let start = output
        .find('{')
        .ok_or_else(|| format!("{name}: output did not contain a JSON object"))?;
    let end = output
        .rfind('}')
        .ok_or_else(|| format!("{name}: output did not contain a JSON object"))?;
    serde_json::from_str(&output[start..=end])
        .map_err(|error| format!("{name}: output JSON could not be parsed: {error}"))
}

fn check_plan(name: &str, plan: &Value) -> RunnerResult<()> {
    if field(plan, "schema_version") != PLAN_SCHEMA_VERSION {
        return Err(format!(
            "{name}: schema_version must be {PLAN_SCHEMA_VERSION}"
        ));
    }
    if field(plan, "study_id") != "material_heat_spreader_screening" {
        return Err(format!(
            "{name}: unexpected study_id {}",
            field(plan, "study_id")
        ));
    }
    if array_len(plan, "steps") != plan.get("step_count").and_then(Value::as_u64).unwrap_or(0) {
        return Err(format!("{name}: step_count must match steps length"));
    }
    let candidate_count = plan
        .get("candidate_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    if array_len(plan, "candidate_ids") != candidate_count {
        return Err(format!(
            "{name}: candidate_count must match candidate_ids length"
        ));
    }
    if plan
        .get("material_card_contract_required")
        .and_then(Value::as_bool)
        != Some(true)
    {
        return Err(format!(
            "{name}: material_card_contract_required must be true"
        ));
    }
    if field(plan, "material_card_schema_version") != MATERIAL_CARD_SCHEMA_VERSION {
        return Err(format!(
            "{name}: material_card_schema_version must be {MATERIAL_CARD_SCHEMA_VERSION}"
        ));
    }
    if plan.get("material_card_ref_count").and_then(Value::as_u64) != Some(candidate_count) {
        return Err(format!(
            "{name}: material_card_ref_count must match candidate_count"
        ));
    }
    if !plan
        .get("candidate_ids")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .any(|value| value.as_str() == Some("copper_c110"))
    {
        return Err(format!("{name}: expected copper_c110 candidate"));
    }
    if !field(plan, "recommended_command").contains("heat-spreader") {
        return Err(format!(
            "{name}: recommended_command must expose the heat-spreader alias"
        ));
    }
    Ok(())
}

fn parse_key_value_output(name: &str, output: &str) -> RunnerResult<BTreeMap<String, String>> {
    let mut pairs = BTreeMap::new();
    for line in output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        if let Some((key, value)) = line.split_once('=') {
            pairs.insert(key.to_string(), value.to_string());
        }
    }
    for key in ["study", "winner", "reliability"] {
        if !pairs.contains_key(key) {
            return Err(format!(
                "{name}: expected study, winner, and reliability key-value lines"
            ));
        }
    }
    Ok(pairs)
}

fn check_report_example(name: &str, report: &BTreeMap<String, String>) -> RunnerResult<()> {
    expect_pair(
        name,
        report,
        "study",
        "material.composite_thermo_electric_panel.v1",
    )?;
    expect_pair(name, report, "winner", "copper_polyimide_aluminum")?;
    let reliability = report.get("reliability").map(String::as_str).unwrap_or("");
    if !["ready_for_next_round", "blocked_by_quality_gates"].contains(&reliability) {
        return Err(format!(
            "{name}: unexpected reliability decision {reliability}"
        ));
    }
    Ok(())
}

struct ExpectedBundle<'a> {
    study: &'a str,
    winner: &'a str,
    reliability: &'a str,
}

fn check_bundle_example(
    name: &str,
    bundle: &BTreeMap<String, String>,
    expected: ExpectedBundle<'_>,
) -> RunnerResult<()> {
    expect_pair(
        name,
        bundle,
        "schema",
        "kyuubiki.material-research-bundle/v1",
    )?;
    expect_pair(name, bundle, "study", expected.study)?;
    expect_pair(name, bundle, "winner", expected.winner)?;
    expect_pair(name, bundle, "reliability", expected.reliability)
}

fn expect_pair(
    name: &str,
    pairs: &BTreeMap<String, String>,
    key: &str,
    expected: &str,
) -> RunnerResult<()> {
    let actual = pairs.get(key).map(String::as_str).unwrap_or("");
    if actual == expected {
        Ok(())
    } else {
        Err(format!("{name}: unexpected {key} {actual}"))
    }
}

fn field<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or_default()
}

fn array_len(value: &Value, key: &str) -> u64 {
    value
        .get(key)
        .and_then(Value::as_array)
        .map_or(0, |items| items.len() as u64)
}

#[cfg(test)]
mod tests {
    use super::{ExpectedBundle, check_bundle_example, check_plan, parse_key_value_output};
    use serde_json::json;

    #[test]
    fn rejects_wrong_plan_schema() {
        let plan = json!({"schema_version":"wrong","study_id":"material_heat_spreader_screening"});
        assert!(check_plan("fixture", &plan).is_err());
    }

    #[test]
    fn parses_and_checks_bundle_key_values() {
        let pairs = parse_key_value_output(
            "fixture",
            "schema=kyuubiki.material-research-bundle/v1\nstudy=heat-spreader\nwinner=pyrolytic_graphite_in_plane\nreliability=blocked_by_quality_gates\n",
        )
        .unwrap();
        check_bundle_example(
            "fixture",
            &pairs,
            ExpectedBundle {
                study: "heat-spreader",
                winner: "pyrolytic_graphite_in_plane",
                reliability: "blocked_by_quality_gates",
            },
        )
        .unwrap();
    }
}
