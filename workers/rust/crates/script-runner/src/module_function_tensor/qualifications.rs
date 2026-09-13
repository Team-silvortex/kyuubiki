use super::{RunnerResult, read_text, string_array, string_field};
use serde_json::{Value, json};
use std::{collections::BTreeSet, path::Path};

const GRADES: &[&str] = &[
    "unassessed",
    "declared",
    "exercised",
    "verified",
    "qualified",
    "operational",
];

pub(super) fn validate_config(root: &Path, tensor: &Value, matrix: &Value) -> RunnerResult<()> {
    let mut ids = BTreeSet::new();
    let mut scopes = BTreeSet::new();
    let Some(requirements) = tensor.get("qualification_requirements") else {
        return Ok(());
    };
    let requirements = requirements
        .as_array()
        .ok_or("qualification_requirements must be an array")?;
    for requirement in requirements {
        let id = required(requirement, "id")?;
        let module_id = required(requirement, "module_id")?;
        let paradigm = required(requirement, "paradigm")?;
        let dimension = required(requirement, "dimension")?;
        let scope = required(requirement, "scope")?;
        required(requirement, "acceptance")?;
        let target = required(requirement, "target")?;
        if !GRADES[1..].contains(&target) {
            return Err(format!("{id}: unknown qualification target {target}"));
        }
        if !ids.insert(id) || !scopes.insert((module_id, paradigm, dimension, scope)) {
            return Err(format!(
                "{id}: duplicate qualification id or scoped coordinate"
            ));
        }
        if !string_array(
            matrix.get("required_by_module").unwrap_or(&Value::Null),
            module_id,
        )
        .iter()
        .any(|value| value == paradigm)
        {
            return Err(format!(
                "{id}: qualification must address a required matrix coordinate"
            ));
        }
        if !super::maturity::required_dimensions(tensor, module_id, paradigm)
            .iter()
            .any(|value| value == dimension)
        {
            return Err(format!(
                "{id}: qualification dimension {dimension} is not required by the cell"
            ));
        }
        let claim_ids = requirement
            .get("claims")
            .and_then(Value::as_array)
            .ok_or_else(|| {
                format!("{id}: claims must be an explicit array, empty for an open scope")
            })?;
        let mut seen_claims = BTreeSet::new();
        for claim_id in claim_ids {
            let claim_id = claim_id
                .as_str()
                .ok_or_else(|| format!("{id}: claim id must be a string"))?;
            if !seen_claims.insert(claim_id) {
                return Err(format!("{id}: duplicate claim {claim_id}"));
            }
            let claim = array(tensor, "evidence_claims")
                .iter()
                .find(|claim| string_field(claim, "id") == Some(claim_id))
                .ok_or_else(|| format!("{id}: unknown claim {claim_id}"))?;
            if !matches(claim, module_id, paradigm, dimension) {
                return Err(format!(
                    "{id}: claim {claim_id} does not cover this module/paradigm/dimension"
                ));
            }
        }
        let basis = requirement
            .get("basis")
            .and_then(Value::as_array)
            .filter(|paths| !paths.is_empty())
            .ok_or_else(|| format!("{id}: basis must not be empty"))?;
        for path in basis {
            read_text(
                root,
                path.as_str()
                    .ok_or_else(|| format!("{id}: basis path must be a string"))?,
            )?;
        }
    }
    Ok(())
}

// The binding is explicit: unrelated high-grade claims cannot close this scope.
// Basis files explain the requirement; merely adding a file never satisfies it.
pub(super) fn evaluate(tensor: &Value, module_id: &str, paradigm: &str) -> Vec<Value> {
    array(tensor, "qualification_requirements").iter()
        .filter(|requirement| string_field(requirement, "module_id") == Some(module_id)
            && string_field(requirement, "paradigm") == Some(paradigm))
        .map(|requirement| {
            let ids = string_array(requirement, "claims");
            let dimension = string_field(requirement, "dimension").unwrap_or_default();
            let achieved = array(tensor, "evidence_claims").iter()
                .filter(|claim| string_field(claim, "status") == Some("proven")
                    && ids.iter().any(|id| string_field(claim, "id") == Some(id.as_str()))
                    && matches(claim, module_id, paradigm, dimension))
                .map(|claim| rank(string_field(claim, "grade").unwrap_or_default()))
                .max().unwrap_or(0);
            let target = rank(string_field(requirement, "target").unwrap_or_default());
            json!({
                "id": requirement["id"], "dimension": dimension, "scope": requirement["scope"],
                "target_grade": requirement["target"], "achieved_grade": GRADES[achieved],
                "met": achieved >= target, "gap_steps": target.saturating_sub(achieved),
                "claims": ids, "acceptance": requirement["acceptance"], "basis": requirement["basis"]
            })
        }).collect()
}

fn matches(claim: &Value, module_id: &str, paradigm: &str, dimension: &str) -> bool {
    [
        ("modules", module_id),
        ("paradigms", paradigm),
        ("dimensions", dimension),
    ]
    .iter()
    .all(|(key, value)| string_array(claim, key).iter().any(|entry| entry == *value))
}

fn required<'a>(value: &'a Value, key: &str) -> RunnerResult<&'a str> {
    string_field(value, key)
        .filter(|text| !text.trim().is_empty())
        .ok_or_else(|| format!("qualification requirement missing {key}"))
}

fn rank(grade: &str) -> usize {
    GRADES.iter().position(|value| *value == grade).unwrap_or(0)
}

fn array<'a>(value: &'a Value, key: &str) -> &'a [Value] {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}
