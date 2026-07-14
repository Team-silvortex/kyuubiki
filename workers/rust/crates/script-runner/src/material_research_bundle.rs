use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

const DEFAULT_INPUT: &str = "tmp/material-research-bundle.json";
const BUNDLE_SCHEMA_VERSION: &str = "kyuubiki.material-research-bundle/v1";
const POSTURE: &str = "screening_research_bundle";
const EXPLORATION_SCHEMA_VERSION: &str = "kyuubiki.material-exploration-run/v1";
const EXECUTION_SCHEMA_VERSION: &str = "kyuubiki.material-exploration-next-round-execution/v1";
const CHAIN_SCHEMA_VERSION: &str = "kyuubiki.material-exploration-chain/v1";
const SUPPORTED_STUDIES: &[&str] = &["heat-spreader", "composite-thermo-electric-panel"];

type RunnerResult<T> = Result<T, String>;

pub(crate) fn run_check_material_research_bundle(
    root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    let options = parse_args(args)?;
    if options.self_test {
        return run_self_test();
    }
    let (absolute, relative) = repo_local_path(root, &options.input, "--in")?;
    if !absolute.exists() {
        eprintln!("material research bundle check failed: input does not exist: {relative}");
        return Ok(1);
    }
    let text = fs::read_to_string(&absolute)
        .map_err(|error| format!("failed to read {relative}: {error}"))?;
    let bundle: Value = serde_json::from_str(&text)
        .map_err(|error| format!("{relative}: invalid json: {error}"))?;
    match validate_material_research_bundle_value(root, &bundle, Some(&text)) {
        Ok(()) => {
            println!("material research bundle ok: {}", options.input);
            Ok(0)
        }
        Err(issue) => {
            eprintln!("material research bundle check failed: {issue}");
            Ok(1)
        }
    }
}

struct Options {
    input: String,
    self_test: bool,
}

fn parse_args(args: Vec<OsString>) -> RunnerResult<Options> {
    let mut options = Options {
        input: DEFAULT_INPUT.to_string(),
        self_test: false,
    };
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.to_string_lossy().as_ref() {
            "--help" | "-h" => {
                println!(
                    "usage: kyuubiki-script-runner check-material-research-bundle [--self-test] [--in tmp/material-research-bundle.json]"
                );
                return Ok(options);
            }
            "--self-test" => options.self_test = true,
            "--in" => {
                options.input = iter
                    .next()
                    .map(|value| value.to_string_lossy().to_string())
                    .filter(|value| !value.is_empty())
                    .ok_or_else(|| "--in requires a repo-local path".to_string())?;
            }
            other => return Err(format!("unknown argument {other}")),
        }
    }
    Ok(options)
}

fn run_self_test() -> RunnerResult<u8> {
    let bad_bundle = json!({
        "schema_version": BUNDLE_SCHEMA_VERSION,
        "posture": POSTURE,
        "study": "unsupported-study",
        "artifact_checksums": { "initial_exploration_sha256": "bad" },
        "initial_exploration": {},
        "next_round_execution_plan": {},
        "next_exploration": {},
        "chain": {},
        "summary": {},
        "reproducibility": { "initial_command": [] },
    });
    expect_failure(Path::new("."), &bad_bundle, "bad checksum")?;
    let artifact = json!({ "schema_version": EXPLORATION_SCHEMA_VERSION, "iteration": 2 });
    let plan = json!({
        "schema_version": EXECUTION_SCHEMA_VERSION,
        "decision": "repair_validation",
        "iteration": 2,
        "runnable_step_count": 1,
    });
    let chain = json!({ "schema_version": CHAIN_SCHEMA_VERSION, "stop_reason": "validation_repair_required" });
    let mismatch = json!({
        "schema_version": BUNDLE_SCHEMA_VERSION,
        "posture": POSTURE,
        "study": "heat-spreader",
        "artifact_checksums": {
            "initial_exploration_sha256": sha256_json(&artifact)?,
            "next_round_execution_plan_sha256": sha256_json(&plan)?,
            "next_exploration_sha256": sha256_json(&artifact)?,
            "chain_sha256": sha256_json(&chain)?,
        },
        "initial_exploration": artifact,
        "next_round_execution_plan": plan,
        "next_exploration": artifact,
        "chain": chain,
        "summary": {
            "winner_candidate_id": "candidate-a",
            "reliability_decision": "blocked_by_quality_gates",
            "next_round_decision": "mitigate_design_risk",
            "runnable_next_step_count": 1,
            "next_iteration": 2,
            "chain_stop_reason": "validation_repair_required",
        },
        "reproducibility": { "initial_command": ["kyuubiki-material-explore"] },
    });
    expect_failure(Path::new("."), &mismatch, "summary/plan decision mismatch")?;
    println!("material research bundle check self-test passed");
    Ok(0)
}

fn expect_failure(root: &Path, bundle: &Value, label: &str) -> RunnerResult<()> {
    if validate_material_research_bundle_value(root, bundle, None).is_ok() {
        Err(format!("self-test did not reject {label}"))
    } else {
        Ok(())
    }
}

pub(crate) fn validate_material_research_bundle_value(
    root: &Path,
    bundle: &Value,
    raw_text: Option<&str>,
) -> RunnerResult<()> {
    validate_bundle(root, bundle, raw_text)
}

fn validate_bundle(root: &Path, bundle: &Value, raw_text: Option<&str>) -> RunnerResult<()> {
    assert_eq_str(
        field(bundle, "schema_version"),
        BUNDLE_SCHEMA_VERSION,
        "schema_version",
    )?;
    assert_eq_str(field(bundle, "posture"), POSTURE, "posture")?;
    if !SUPPORTED_STUDIES.contains(&field(bundle, "study")) {
        return Err(format!(
            "study: unsupported retained bundle study {:?}",
            field(bundle, "study")
        ));
    }
    assert_no_absolute_repo_path(root, bundle, "bundle")?;
    assert_checksum(
        bundle,
        raw_text,
        "initial_exploration_sha256",
        "initial_exploration",
    )?;
    assert_checksum(
        bundle,
        raw_text,
        "next_round_execution_plan_sha256",
        "next_round_execution_plan",
    )?;
    assert_checksum(
        bundle,
        raw_text,
        "next_exploration_sha256",
        "next_exploration",
    )?;
    assert_checksum(bundle, raw_text, "chain_sha256", "chain")?;
    assert_eq_str(
        pointer_str(bundle, "/initial_exploration/schema_version"),
        EXPLORATION_SCHEMA_VERSION,
        "initial exploration schema",
    )?;
    assert_eq_str(
        pointer_str(bundle, "/next_round_execution_plan/schema_version"),
        EXECUTION_SCHEMA_VERSION,
        "next round execution schema",
    )?;
    assert_eq_str(
        pointer_str(bundle, "/next_exploration/schema_version"),
        EXPLORATION_SCHEMA_VERSION,
        "next exploration schema",
    )?;
    assert_eq_str(
        pointer_str(bundle, "/chain/schema_version"),
        CHAIN_SCHEMA_VERSION,
        "chain schema",
    )?;
    for pointer in [
        "/summary/winner_candidate_id",
        "/summary/reliability_decision",
        "/summary/next_round_decision",
        "/summary/chain_stop_reason",
    ] {
        if pointer_str(bundle, pointer).is_empty() {
            return Err(format!("{}: expected non-empty string", &pointer[1..]));
        }
    }
    assert_eq_value(
        bundle.pointer("/next_round_execution_plan/decision"),
        bundle.pointer("/summary/next_round_decision"),
        "next_round_execution_plan.decision",
    )?;
    assert_eq_value(
        bundle.pointer("/next_round_execution_plan/runnable_step_count"),
        bundle.pointer("/summary/runnable_next_step_count"),
        "next_round_execution_plan.runnable_step_count",
    )?;
    assert_eq_value(
        bundle.pointer("/next_round_execution_plan/iteration"),
        bundle.pointer("/summary/next_iteration"),
        "next_round_execution_plan.iteration",
    )?;
    assert_eq_value(
        bundle.pointer("/next_exploration/iteration"),
        bundle.pointer("/summary/next_iteration"),
        "next_exploration.iteration",
    )?;
    assert_eq_value(
        bundle.pointer("/chain/stop_reason"),
        bundle.pointer("/summary/chain_stop_reason"),
        "chain.stop_reason",
    )?;
    if !bundle
        .pointer("/reproducibility/initial_command")
        .and_then(Value::as_array)
        .is_some_and(|items| !items.is_empty() || items.is_empty())
    {
        return Err("reproducibility.initial_command must be an argv array".to_string());
    }
    Ok(())
}

fn assert_checksum(
    bundle: &Value,
    raw_text: Option<&str>,
    checksum_key: &str,
    artifact_key: &str,
) -> RunnerResult<()> {
    let actual = bundle
        .pointer(&format!("/artifact_checksums/{checksum_key}"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    let expected = if let Some(raw_text) = raw_text {
        sha256_compact_json_slice(
            top_level_value_slice(raw_text, artifact_key)
                .ok_or_else(|| format!("{artifact_key}: missing raw artifact"))?,
        )?
    } else {
        sha256_json(bundle.get(artifact_key).unwrap_or(&Value::Null))?
    };
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "checksum {checksum_key}: expected {expected}, got {actual}"
        ))
    }
}

fn assert_no_absolute_repo_path(root: &Path, value: &Value, context: &str) -> RunnerResult<()> {
    if let Some(text) = value.as_str() {
        if text.contains(root.to_string_lossy().as_ref()) {
            return Err(format!(
                "{context}: contains local absolute repository path"
            ));
        }
    }
    if let Some(items) = value.as_array() {
        for (index, item) in items.iter().enumerate() {
            assert_no_absolute_repo_path(root, item, &format!("{context}[{index}]"))?;
        }
    }
    if let Some(object) = value.as_object() {
        for (key, nested) in object {
            assert_no_absolute_repo_path(root, nested, &format!("{context}.{key}"))?;
        }
    }
    Ok(())
}

fn sha256_json(value: &Value) -> RunnerResult<String> {
    let mut hasher = Sha256::new();
    hasher.update(json_stringify(value)?.as_bytes());
    hasher.update(b"\n");
    Ok(format!("{:x}", hasher.finalize()))
}

fn sha256_compact_json_slice(slice: &str) -> RunnerResult<String> {
    let mut hasher = Sha256::new();
    hasher.update(compact_json_slice(slice).as_bytes());
    hasher.update(b"\n");
    Ok(format!("{:x}", hasher.finalize()))
}

fn top_level_value_slice<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    let bytes = text.as_bytes();
    let mut index = 0usize;
    let mut depth = 0isize;
    while index < bytes.len() {
        match bytes[index] {
            b'"' => {
                let end = string_end(bytes, index + 1)?;
                if depth == 1 && &text[index + 1..end] == key {
                    let mut value_start = end + 1;
                    while value_start < bytes.len() && bytes[value_start].is_ascii_whitespace() {
                        value_start += 1;
                    }
                    if bytes.get(value_start) != Some(&b':') {
                        index = end + 1;
                        continue;
                    }
                    value_start += 1;
                    while value_start < bytes.len() && bytes[value_start].is_ascii_whitespace() {
                        value_start += 1;
                    }
                    let value_end = value_end(bytes, value_start)?;
                    return Some(&text[value_start..value_end]);
                }
                index = end + 1;
            }
            b'{' | b'[' => {
                depth += 1;
                index += 1;
            }
            b'}' | b']' => {
                depth -= 1;
                index += 1;
            }
            _ => index += 1,
        }
    }
    None
}

fn value_end(bytes: &[u8], start: usize) -> Option<usize> {
    let mut index = start;
    let mut depth = 0isize;
    while index < bytes.len() {
        match bytes[index] {
            b'"' => index = string_end(bytes, index + 1)? + 1,
            b'{' | b'[' => {
                depth += 1;
                index += 1;
            }
            b'}' | b']' => {
                if depth == 0 {
                    return Some(index);
                }
                depth -= 1;
                index += 1;
            }
            b',' if depth == 0 => return Some(index),
            _ => index += 1,
        }
    }
    Some(index)
}

fn string_end(bytes: &[u8], mut index: usize) -> Option<usize> {
    while index < bytes.len() {
        match bytes[index] {
            b'\\' => index += 2,
            b'"' => return Some(index),
            _ => index += 1,
        }
    }
    None
}

fn compact_json_slice(slice: &str) -> String {
    let bytes = slice.as_bytes();
    let mut output = String::with_capacity(slice.len());
    let mut index = 0usize;
    while index < bytes.len() {
        if bytes[index] == b'"' {
            let end = string_end(bytes, index + 1).unwrap_or(bytes.len().saturating_sub(1));
            output.push_str(&slice[index..=end]);
            index = end + 1;
        } else {
            if !bytes[index].is_ascii_whitespace() {
                output.push(bytes[index] as char);
            }
            index += 1;
        }
    }
    output
}

fn json_stringify(value: &Value) -> RunnerResult<String> {
    match value {
        Value::Null => Ok("null".to_string()),
        Value::Bool(value) => Ok(value.to_string()),
        Value::Number(number) => Ok(js_number(number)?),
        Value::String(value) => serde_json::to_string(value)
            .map_err(|error| format!("failed to encode string: {error}")),
        Value::Array(items) => Ok(format!(
            "[{}]",
            items
                .iter()
                .map(json_stringify)
                .collect::<RunnerResult<Vec<_>>>()?
                .join(",")
        )),
        Value::Object(object) => Ok(format!(
            "{{{}}}",
            object
                .iter()
                .map(|(key, value)| {
                    Ok(format!(
                        "{}:{}",
                        serde_json::to_string(key)
                            .map_err(|error| format!("failed to encode key: {error}"))?,
                        json_stringify(value)?
                    ))
                })
                .collect::<RunnerResult<Vec<_>>>()?
                .join(",")
        )),
    }
}

fn js_number(number: &serde_json::Number) -> RunnerResult<String> {
    if let Some(value) = number.as_i64() {
        return Ok(value.to_string());
    }
    if let Some(value) = number.as_u64() {
        return Ok(value.to_string());
    }
    let value = number
        .as_f64()
        .ok_or_else(|| "failed to convert JSON number".to_string())?;
    if !value.is_finite() {
        return Err("JSON number must be finite".to_string());
    }
    if value == 0.0 {
        return Ok("0".to_string());
    }
    if value.fract() == 0.0 && value.abs() < 1e21 {
        return Ok(format!("{value:.0}"));
    }
    Ok(value.to_string())
}

fn repo_local_path(root: &Path, path: &str, label: &str) -> RunnerResult<(PathBuf, String)> {
    let absolute = root.join(path);
    let relative = absolute
        .strip_prefix(root)
        .map_err(|_| format!("{label} must stay inside the repository"))?
        .to_string_lossy()
        .replace('\\', "/");
    if relative.starts_with("..") || Path::new(&relative).is_absolute() {
        return Err(format!("{label} must stay inside the repository"));
    }
    Ok((absolute, relative))
}

fn assert_eq_str(actual: &str, expected: &str, context: &str) -> RunnerResult<()> {
    if actual == expected {
        Ok(())
    } else {
        Err(format!("{context}: expected {expected:?}, got {actual:?}"))
    }
}

fn assert_eq_value(
    actual: Option<&Value>,
    expected: Option<&Value>,
    context: &str,
) -> RunnerResult<()> {
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "{context}: expected {}, got {}",
            expected.unwrap_or(&Value::Null),
            actual.unwrap_or(&Value::Null)
        ))
    }
}

fn pointer_str<'a>(value: &'a Value, pointer: &str) -> &'a str {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .unwrap_or_default()
}

fn field<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::sha256_json;
    use serde_json::json;

    #[test]
    fn checksum_matches_json_stringify_newline_shape() {
        let value = json!({ "a": 1, "b": [true] });
        assert_eq!(
            sha256_json(&value).unwrap(),
            "a220f9efdeb6af86dccc949ec88e884b7c0a076e2efe208d8ef39aff8637ff08"
        );
    }
}
