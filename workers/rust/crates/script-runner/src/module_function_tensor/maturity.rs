use super::{CellEvaluationInput, RunnerResult, read_text, string_array, string_field};
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::path::Path;

const ALLOWED_DIMENSIONS: &[&str] = &[
    "execution",
    "benchmark",
    "security",
    "contract",
    "numerical_validation",
    "recovery",
    "sdk_parity",
    "abi_compatibility",
    "ux_closure",
];
const ALLOWED_CLAIM_STATUS: &[&str] = &["proven", "partial", "open"];

pub(super) fn validate_config(
    root: &Path,
    tensor: &Value,
    paradigms: &[String],
    module_ids: &BTreeSet<String>,
) -> RunnerResult<()> {
    let policy = tensor
        .get("maturity_policy")
        .and_then(Value::as_object)
        .ok_or_else(|| "maturity_policy must be an object".to_string())?;
    for paradigm in paradigms {
        let dimensions = policy
            .get(paradigm)
            .and_then(Value::as_array)
            .ok_or_else(|| format!("maturity_policy missing {paradigm}"))?;
        if dimensions.is_empty() {
            return Err(format!("maturity_policy.{paradigm} must not be empty"));
        }
        validate_dimensions(dimensions, &format!("maturity_policy.{paradigm}"))?;
    }
    for paradigm in policy.keys() {
        if !paradigms.contains(paradigm) {
            return Err(format!("maturity_policy maps unknown paradigm {paradigm}"));
        }
    }

    validate_cell_requirements(tensor, paradigms, module_ids)?;
    validate_evidence_claims(root, tensor, paradigms, module_ids)
}

pub(super) fn evaluate(input: &CellEvaluationInput<'_>) -> Value {
    let CellEvaluationInput {
        tensor,
        module_id,
        paradigm,
        status,
        required,
        benchmark_tests,
        security_tests,
        contract_evidence,
    } = *input;
    let mut required_dimensions = string_array(
        tensor.get("maturity_policy").unwrap_or(&Value::Null),
        paradigm,
    );
    for requirement in matching_cell_requirements(tensor, module_id, paradigm) {
        required_dimensions.extend(string_array(requirement, "dimensions"));
    }
    normalize(&mut required_dimensions);

    let mut present_dimensions = Vec::new();
    if !benchmark_tests.is_empty() || !security_tests.is_empty() {
        present_dimensions.push("execution".to_string());
    }
    if !benchmark_tests.is_empty() {
        present_dimensions.push("benchmark".to_string());
    }
    if !security_tests.is_empty() {
        present_dimensions.push("security".to_string());
    }
    if !contract_evidence.is_empty() {
        present_dimensions.push("contract".to_string());
    }

    let claims = matching_entries(tensor, "evidence_claims", module_id, paradigm)
        .into_iter()
        .map(|claim| {
            if string_field(claim, "status") == Some("proven") {
                present_dimensions.extend(string_array(claim, "dimensions"));
            }
            json!({
                "id": string_field(claim, "id").unwrap_or_default(),
                "status": string_field(claim, "status").unwrap_or_default(),
                "grade": string_field(claim, "grade").unwrap_or_default(),
                "dimensions": string_array(claim, "dimensions"),
                "files": string_array(claim, "files")
            })
        })
        .collect::<Vec<_>>();
    normalize(&mut present_dimensions);
    let present_set = present_dimensions.iter().collect::<BTreeSet<_>>();
    let missing_dimensions = required_dimensions
        .iter()
        .filter(|dimension| !present_set.contains(dimension))
        .cloned()
        .collect::<Vec<_>>();
    let maturity = if !required {
        "optional"
    } else if status != "covered" {
        "not_ready"
    } else if missing_dimensions.is_empty() {
        "strong"
    } else if missing_dimensions.len() == 1 {
        "medium"
    } else {
        "thin"
    };
    json!({
        "level": maturity,
        "required_dimensions": required_dimensions,
        "present_dimensions": present_dimensions,
        "missing_dimensions": missing_dimensions,
        "claims": claims
    })
}

fn validate_cell_requirements(
    tensor: &Value,
    paradigms: &[String],
    module_ids: &BTreeSet<String>,
) -> RunnerResult<()> {
    let mut seen = BTreeSet::new();
    for (index, entry) in array_field(tensor, "cell_requirements").iter().enumerate() {
        let module_id = required_string(entry, "module_id", "cell_requirements", index)?;
        let paradigm = required_string(entry, "paradigm", "cell_requirements", index)?;
        validate_coordinate(module_id, paradigm, paradigms, module_ids)?;
        if !seen.insert((module_id, paradigm)) {
            return Err(format!(
                "cell_requirements has duplicate coordinate {module_id}/{paradigm}"
            ));
        }
        let dimensions = entry
            .get("dimensions")
            .and_then(Value::as_array)
            .ok_or_else(|| format!("cell_requirements[{index}].dimensions must be an array"))?;
        validate_dimensions(dimensions, &format!("cell_requirements[{index}]"))?;
    }
    Ok(())
}

fn validate_evidence_claims(
    root: &Path,
    tensor: &Value,
    paradigms: &[String],
    module_ids: &BTreeSet<String>,
) -> RunnerResult<()> {
    let mut seen = BTreeSet::new();
    for (index, claim) in array_field(tensor, "evidence_claims").iter().enumerate() {
        let id = required_string(claim, "id", "evidence_claims", index)?;
        if !seen.insert(id) {
            return Err(format!("evidence_claims has duplicate id {id}"));
        }
        let status = required_string(claim, "status", "evidence_claims", index)?;
        if !ALLOWED_CLAIM_STATUS.contains(&status) {
            return Err(format!("{id}: unknown evidence claim status {status}"));
        }
        let modules = string_array(claim, "modules");
        if modules.is_empty() {
            return Err(format!("{id}: evidence claim modules must not be empty"));
        }
        for module_id in modules {
            if !module_ids.contains(&module_id) {
                return Err(format!("{id}: unknown module {module_id}"));
            }
        }
        let claim_paradigms = string_array(claim, "paradigms");
        if claim_paradigms.is_empty() {
            return Err(format!("{id}: evidence claim paradigms must not be empty"));
        }
        for paradigm in claim_paradigms {
            if !paradigms.contains(&paradigm) {
                return Err(format!("{id}: unknown paradigm {paradigm}"));
            }
        }
        let dimensions = claim
            .get("dimensions")
            .and_then(Value::as_array)
            .ok_or_else(|| format!("{id}.dimensions must be an array"))?;
        validate_dimensions(dimensions, id)?;
        validate_file_evidence(root, claim, id)?;
    }
    Ok(())
}

fn validate_file_evidence(root: &Path, entry: &Value, id: &str) -> RunnerResult<()> {
    let mut combined = String::new();
    let files = string_array(entry, "files");
    if files.is_empty() {
        return Err(format!("{id}: evidence files must not be empty"));
    }
    for file in files {
        combined.push('\n');
        combined.push_str(&read_text(root, &file)?);
    }
    for required_text in string_array(entry, "required_text") {
        if !combined.contains(&required_text) {
            return Err(format!("{id}: evidence bundle missing {required_text}"));
        }
    }
    Ok(())
}

fn validate_dimensions(dimensions: &[Value], context: &str) -> RunnerResult<()> {
    let mut seen = BTreeSet::new();
    for dimension in dimensions {
        let dimension = dimension
            .as_str()
            .ok_or_else(|| format!("{context}: evidence dimension must be a string"))?;
        if !ALLOWED_DIMENSIONS.contains(&dimension) {
            return Err(format!("{context}: unknown evidence dimension {dimension}"));
        }
        if !seen.insert(dimension) {
            return Err(format!(
                "{context}: duplicate evidence dimension {dimension}"
            ));
        }
    }
    Ok(())
}

fn matching_entries<'a>(
    tensor: &'a Value,
    key: &str,
    module_id: &str,
    paradigm: &str,
) -> Vec<&'a Value> {
    array_field(tensor, key)
        .iter()
        .filter(|entry| {
            string_array(entry, "modules")
                .iter()
                .any(|id| id == module_id)
                && string_array(entry, "paradigms")
                    .iter()
                    .any(|id| id == paradigm)
        })
        .collect()
}

fn matching_cell_requirements<'a>(
    tensor: &'a Value,
    module_id: &str,
    paradigm: &str,
) -> Vec<&'a Value> {
    array_field(tensor, "cell_requirements")
        .iter()
        .filter(|entry| {
            string_field(entry, "module_id") == Some(module_id)
                && string_field(entry, "paradigm") == Some(paradigm)
        })
        .collect()
}

fn validate_coordinate(
    module_id: &str,
    paradigm: &str,
    paradigms: &[String],
    module_ids: &BTreeSet<String>,
) -> RunnerResult<()> {
    if !module_ids.contains(module_id) {
        return Err(format!("unknown module {module_id}"));
    }
    if !paradigms.contains(&paradigm.to_string()) {
        return Err(format!("unknown paradigm {paradigm}"));
    }
    Ok(())
}

fn required_string<'a>(
    entry: &'a Value,
    key: &str,
    collection: &str,
    index: usize,
) -> RunnerResult<&'a str> {
    string_field(entry, key)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{collection}[{index}].{key} must be a non-empty string"))
}

fn array_field<'a>(value: &'a Value, key: &str) -> &'a [Value] {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

fn normalize(values: &mut Vec<String>) {
    values.sort();
    values.dedup();
}

#[cfg(test)]
mod tests {
    use super::evaluate;
    use crate::module_function_tensor::CellEvaluationInput;
    use serde_json::{Value, json};

    #[test]
    fn partial_claim_does_not_satisfy_cell_requirement() {
        let tensor = json!({
            "maturity_policy": {
                "validation": ["execution", "contract"]
            },
            "cell_requirements": [{
                "module_id": "engine",
                "paradigm": "validation",
                "dimensions": ["numerical_validation"]
            }],
            "evidence_claims": [{
                "id": "numerical-depth",
                "status": "partial",
                "modules": ["engine"],
                "paradigms": ["validation"],
                "dimensions": ["numerical_validation"],
                "files": ["docs/example.md"]
            }]
        });
        let benchmark_tests = [json!({"id": "run"})];
        let contract_evidence = [json!({"id": "contract"})];
        let maturity = evaluate(&CellEvaluationInput {
            tensor: &tensor,
            module_id: "engine",
            paradigm: "validation",
            status: "covered",
            required: true,
            benchmark_tests: &benchmark_tests,
            security_tests: &[],
            contract_evidence: &contract_evidence,
        });
        assert_eq!(maturity["level"], Value::String("medium".to_string()));
        assert_eq!(
            maturity["missing_dimensions"],
            json!(["numerical_validation"])
        );
        assert_eq!(maturity["claims"][0]["status"], "partial");
    }
}
