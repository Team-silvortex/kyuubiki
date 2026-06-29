use serde_json::Value;

use crate::IntegrityContract;

const COMPONENT_PROTOCOL_SCHEMA: &str = "kyuubiki.component-integrity/v1";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComponentIntegritySpec {
    pub id: String,
    pub kind: String,
    pub version: String,
    pub owner: String,
    pub owned_paths: Vec<String>,
    pub required_paths: Vec<String>,
    pub protected_paths: Vec<String>,
    pub removable_patterns: Vec<String>,
    pub visible_rules: Vec<ComponentVisibleRule>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComponentVisibleRule {
    pub label: String,
    pub value: String,
    pub editable: bool,
    pub description: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComponentIntegrityIssue {
    pub component_id: String,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComponentIntegrityProtocolReport {
    pub schema_version: String,
    pub component_count: usize,
    pub required_path_count: usize,
    pub covered_required_path_count: usize,
    pub components: Vec<ComponentIntegritySpec>,
    pub issues: Vec<ComponentIntegrityIssue>,
}

impl ComponentIntegrityProtocolReport {
    pub fn render_lines(&self) -> Vec<String> {
        let mut lines = vec![
            format!("component_protocol: {}", self.schema_version),
            format!("component_count: {}", self.component_count),
            format!(
                "required_path_coverage: {}/{}",
                self.covered_required_path_count, self.required_path_count
            ),
        ];
        for component in &self.components {
            lines.push(format!(
                "  [{}] {} kind={} version={} owner={}",
                if component_has_issue(&component.id, &self.issues) {
                    "issue"
                } else {
                    "ok"
                },
                component.id,
                component.kind,
                component.version,
                component.owner
            ));
        }
        lines
    }
}

pub fn component_integrity_protocol_report(
    contract: &IntegrityContract,
) -> ComponentIntegrityProtocolReport {
    let mut issues = Vec::new();
    for component in &contract.components {
        collect_component_issues(component, contract, &mut issues);
    }
    let required_path_count = contract
        .required_layout
        .iter()
        .filter(|rule| rule.required)
        .count();
    let covered_required_path_count = collect_required_path_coverage_issues(contract, &mut issues);

    ComponentIntegrityProtocolReport {
        schema_version: COMPONENT_PROTOCOL_SCHEMA.to_string(),
        component_count: contract.components.len(),
        required_path_count,
        covered_required_path_count,
        components: contract.components.clone(),
        issues,
    }
}

pub(crate) fn parse_component_specs(
    payload: &Value,
) -> Result<Vec<ComponentIntegritySpec>, String> {
    let Some(components) = payload.get("components") else {
        return Ok(Vec::new());
    };
    let components = components
        .as_array()
        .ok_or_else(|| "installation contract components must be an array".to_string())?;

    components
        .iter()
        .map(|component| {
            Ok(ComponentIntegritySpec {
                id: string_field(component, "id")?,
                kind: string_field(component, "kind")?,
                version: string_field(component, "version")?,
                owner: string_field(component, "owner")?,
                owned_paths: string_array_field(component, "owned_paths")?,
                required_paths: optional_string_array_field(component, "required_paths")?,
                protected_paths: optional_string_array_field(component, "protected_paths")?,
                removable_patterns: optional_string_array_field(component, "removable_patterns")?,
                visible_rules: component_visible_rules(component)?,
            })
        })
        .collect()
}

fn collect_component_issues(
    component: &ComponentIntegritySpec,
    contract: &IntegrityContract,
    issues: &mut Vec<ComponentIntegrityIssue>,
) {
    if component.owned_paths.is_empty() {
        push_issue(
            issues,
            &component.id,
            "component must own at least one path",
        );
    }
    for required in &component.required_paths {
        if !component_path_covers(component, required) {
            push_issue(
                issues,
                &component.id,
                &format!("required path {required} is outside owned paths"),
            );
        }
    }
    for protected in &component.protected_paths {
        if !contract
            .protected_paths
            .iter()
            .any(|entry| entry == protected)
        {
            push_issue(
                issues,
                &component.id,
                &format!("protected path {protected} is not in the global protected set"),
            );
        }
    }
}

fn collect_required_path_coverage_issues(
    contract: &IntegrityContract,
    issues: &mut Vec<ComponentIntegrityIssue>,
) -> usize {
    let mut covered = 0usize;
    for rule in contract.required_layout.iter().filter(|rule| rule.required) {
        if contract
            .components
            .iter()
            .any(|component| component_path_covers(component, &rule.relative_path))
        {
            covered += 1;
            continue;
        }
        push_issue(
            issues,
            "__contract__",
            &format!(
                "required layout path {} is not owned by any component",
                rule.relative_path
            ),
        );
    }
    covered
}

fn component_path_covers(component: &ComponentIntegritySpec, path: &str) -> bool {
    component.owned_paths.iter().any(|owned| {
        path == owned
            || owned == "."
            || path
                .strip_prefix(owned)
                .is_some_and(|rest| rest.starts_with('/'))
    })
}

fn component_has_issue(component_id: &str, issues: &[ComponentIntegrityIssue]) -> bool {
    issues
        .iter()
        .any(|issue| issue.component_id == component_id)
}

fn push_issue(issues: &mut Vec<ComponentIntegrityIssue>, component_id: &str, message: &str) {
    issues.push(ComponentIntegrityIssue {
        component_id: component_id.to_string(),
        message: message.to_string(),
    });
}

fn component_visible_rules(component: &Value) -> Result<Vec<ComponentVisibleRule>, String> {
    let Some(rules) = component.get("visible_rules") else {
        return Ok(Vec::new());
    };
    rules
        .as_array()
        .ok_or_else(|| "component visible_rules must be an array".to_string())?
        .iter()
        .map(|rule| {
            Ok(ComponentVisibleRule {
                label: string_field(rule, "label")?,
                value: string_field(rule, "value")?,
                editable: bool_field(rule, "editable")?,
                description: string_field(rule, "description")?,
            })
        })
        .collect()
}

fn optional_string_array_field(payload: &Value, key: &str) -> Result<Vec<String>, String> {
    payload
        .get(key)
        .map(|_| string_array_field(payload, key))
        .unwrap_or_else(|| Ok(Vec::new()))
}

fn string_array_field(payload: &Value, key: &str) -> Result<Vec<String>, String> {
    payload
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("component integrity spec missing {key}"))?
        .iter()
        .map(|entry| {
            entry
                .as_str()
                .map(str::to_string)
                .ok_or_else(|| format!("component integrity spec {key} must contain strings"))
        })
        .collect()
}

fn string_field(payload: &Value, key: &str) -> Result<String, String> {
    payload
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| format!("component integrity spec missing {key}"))
}

fn bool_field(payload: &Value, key: &str) -> Result<bool, String> {
    payload
        .get(key)
        .and_then(Value::as_bool)
        .ok_or_else(|| format!("component integrity spec missing {key}"))
}
