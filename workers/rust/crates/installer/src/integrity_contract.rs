use std::fs;
use std::path::Path;

use serde_json::Value;

use crate::Platform;

const CONTRACT_PATH: &str = "deploy/installation-integrity-contract.json";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IntegrityContract {
    pub schema_version: String,
    pub product_line: String,
    pub shipping_version: String,
    pub required_layout: Vec<IntegrityLayoutRule>,
    pub protected_paths: Vec<String>,
    pub removable_patterns: Vec<String>,
    pub allowed_dist_children: Vec<String>,
    pub visible_rules: Vec<IntegrityVisibleRule>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IntegrityLayoutRule {
    pub label: String,
    pub relative_path: String,
    pub required: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IntegrityVisibleRule {
    pub category: String,
    pub label: String,
    pub value: String,
    pub editable: bool,
    pub description: String,
}

pub fn load_integrity_contract(root: &Path, platform: Platform) -> Result<IntegrityContract, String> {
    let path = root.join(CONTRACT_PATH);
    let contents = fs::read_to_string(&path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let payload: Value = serde_json::from_str(&contents)
        .map_err(|error| format!("invalid JSON in {}: {error}", path.display()))?;

    Ok(IntegrityContract {
        schema_version: string_field(&payload, "schema_version")?,
        product_line: string_field(&payload, "product_line")?,
        shipping_version: string_field(&payload, "shipping_version")?,
        required_layout: layout_rules(&payload, platform)?,
        protected_paths: string_array_field(&payload, "protected_paths")?,
        removable_patterns: string_array_field(&payload, "removable_patterns")?,
        allowed_dist_children: string_array_field(&payload, "allowed_dist_children")?,
        visible_rules: visible_rules(&payload, platform)?,
    })
}

pub fn contract_path() -> &'static str {
    CONTRACT_PATH
}

fn layout_rules(payload: &Value, platform: Platform) -> Result<Vec<IntegrityLayoutRule>, String> {
    let rules = payload
        .get("required_layout")
        .and_then(Value::as_array)
        .ok_or_else(|| "installation contract missing required_layout".to_string())?;

    rules
        .iter()
        .map(|rule| {
            Ok(IntegrityLayoutRule {
                label: string_field(rule, "label")?,
                relative_path: resolve_platform_tokens(&string_field(rule, "path")?, platform),
                required: bool_field(rule, "required")?,
            })
        })
        .collect()
}

fn visible_rules(payload: &Value, platform: Platform) -> Result<Vec<IntegrityVisibleRule>, String> {
    let rules = payload
        .get("visible_rules")
        .and_then(Value::as_array)
        .ok_or_else(|| "installation contract missing visible_rules".to_string())?;

    let mut parsed: Vec<IntegrityVisibleRule> = rules
        .iter()
        .map(|rule| {
            Ok(IntegrityVisibleRule {
                category: string_field(rule, "category")?,
                label: string_field(rule, "label")?,
                value: resolve_platform_tokens(&string_field(rule, "value")?, platform),
                editable: bool_field(rule, "editable")?,
                description: string_field(rule, "description")?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;

    parsed.sort_by(|left, right| {
        left.category
            .cmp(&right.category)
            .then(left.label.cmp(&right.label))
    });
    Ok(parsed)
}

fn string_array_field(payload: &Value, key: &str) -> Result<Vec<String>, String> {
    let items = payload
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("installation contract missing {key}"))?;

    items
        .iter()
        .map(|item| {
            item.as_str()
                .map(str::to_string)
                .ok_or_else(|| format!("installation contract field {key} must be a string array"))
        })
        .collect()
}

fn string_field(payload: &Value, key: &str) -> Result<String, String> {
    payload
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| format!("installation contract missing {key}"))
}

fn bool_field(payload: &Value, key: &str) -> Result<bool, String> {
    payload
        .get(key)
        .and_then(Value::as_bool)
        .ok_or_else(|| format!("installation contract missing {key}"))
}

fn resolve_platform_tokens(value: &str, platform: Platform) -> String {
    value.replace("{platform}", platform.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replaces_platform_tokens_in_rule_values() {
        assert_eq!(
            resolve_platform_tokens("dist/{platform}", Platform::Linux),
            "dist/linux"
        );
    }
}
