use super::{EVIDENCE_INCLUDE_SCHEMA_VERSION, RunnerResult, read_json, string_array, string_field};
use serde_json::{Value, json};
use std::path::Path;

pub(super) fn load_tensor_with_includes(root: &Path, mut tensor: Value) -> RunnerResult<Value> {
    let include_paths = string_array(&tensor, "evidence_includes");
    for include_path in include_paths {
        let include = read_json(root, &include_path)?;
        if string_field(&include, "schema_version") != Some(EVIDENCE_INCLUDE_SCHEMA_VERSION) {
            return Err(format!(
                "{include_path}: schema_version must be {EVIDENCE_INCLUDE_SCHEMA_VERSION}"
            ));
        }
        merge_paradigm_contract_evidence(&mut tensor, &include, &include_path)?;
        merge_array_section(&mut tensor, &include, "evidence_claims", &include_path)?;
        merge_array_section(&mut tensor, &include, "cell_requirements", &include_path)?;
    }
    Ok(tensor)
}

fn merge_paradigm_contract_evidence(
    tensor: &mut Value,
    include: &Value,
    include_path: &str,
) -> RunnerResult<()> {
    let Some(included) = include.get("paradigm_contract_evidence") else {
        return Ok(());
    };
    let included = included
        .as_object()
        .ok_or_else(|| format!("{include_path}: paradigm_contract_evidence must be an object"))?;
    let target_value = ensure_object_section(tensor, "paradigm_contract_evidence")?;
    let target = target_value
        .as_object_mut()
        .ok_or_else(|| "tensor paradigm_contract_evidence must be an object".to_string())?;
    for (paradigm, entries) in included {
        let entries = entries
            .as_array()
            .ok_or_else(|| format!("{include_path}: {paradigm} evidence must be an array"))?;
        let target_entries = target.entry(paradigm.clone()).or_insert_with(|| json!([]));
        let target_entries = target_entries
            .as_array_mut()
            .ok_or_else(|| format!("tensor {paradigm} evidence must be an array"))?;
        target_entries.extend(entries.iter().cloned());
    }
    Ok(())
}

fn ensure_object_section<'a>(tensor: &'a mut Value, key: &str) -> RunnerResult<&'a mut Value> {
    let object = tensor
        .as_object_mut()
        .ok_or_else(|| "tensor must be an object".to_string())?;
    if !object.contains_key(key) {
        object.insert(key.to_string(), json!({}));
    }
    Ok(object.get_mut(key).expect("inserted object section"))
}

fn merge_array_section(
    tensor: &mut Value,
    include: &Value,
    key: &str,
    include_path: &str,
) -> RunnerResult<()> {
    let Some(included) = include.get(key) else {
        return Ok(());
    };
    let included = included
        .as_array()
        .ok_or_else(|| format!("{include_path}: {key} must be an array"))?;
    let target_value = ensure_array_section(tensor, key)?;
    let target = target_value
        .as_array_mut()
        .ok_or_else(|| format!("tensor {key} must be an array"))?;
    target.extend(included.iter().cloned());
    Ok(())
}

fn ensure_array_section<'a>(tensor: &'a mut Value, key: &str) -> RunnerResult<&'a mut Value> {
    let object = tensor
        .as_object_mut()
        .ok_or_else(|| "tensor must be an object".to_string())?;
    if !object.contains_key(key) {
        object.insert(key.to_string(), json!([]));
    }
    Ok(object.get_mut(key).expect("inserted array section"))
}
