use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use super::{RUST_QUALITY_SOURCES, RunnerResult, first_quoted_string_with_tail, read_text};

pub(super) fn rust_quality_payload_keys(root: &Path) -> RunnerResult<BTreeSet<String>> {
    let mut keys = BTreeSet::new();
    for (_domain, relative_path) in RUST_QUALITY_SOURCES {
        let source = read_text(root, relative_path)?;
        keys.extend(rust_quality_payload_keys_from_source(&source));
    }
    Ok(keys)
}

pub(super) fn rust_quality_payload_keys_from_source(source: &str) -> BTreeSet<String> {
    source
        .lines()
        .filter_map(first_quoted_string_with_tail)
        .map(|(value, _tail)| value)
        .filter(|value| value.contains("_quality_"))
        .collect()
}

pub(super) fn web_quality_payload_keys(source: &str) -> BTreeSet<String> {
    let domains = web_quality_domains(source);
    let mut keys = BTreeSet::new();

    for line in source.lines().map(str::trim_start) {
        if let Some((value, _tail)) = first_quoted_string_with_tail(line) {
            if let Some(suffix) = value.strip_prefix("#{id}_") {
                for domain in &domains {
                    keys.insert(format!("{domain}_{suffix}"));
                }
            } else if value.contains("_quality_") {
                keys.insert(value);
            }
        }
    }
    keys
}

pub(super) fn compare_quality_payload_keys(
    rust: &BTreeSet<String>,
    web: &BTreeSet<String>,
) -> Vec<String> {
    let mut issues = Vec::new();
    if rust.is_empty() {
        issues.push("Rust quality payload contract extracted no keys".to_string());
    }
    if web.is_empty() {
        issues.push("Web quality payload contract extracted no keys".to_string());
    }
    let missing_from_web = rust.difference(web).cloned().collect::<Vec<_>>();
    if !missing_from_web.is_empty() {
        issues.push(format!(
            "quality payload keys missing from Web runtime: {}",
            missing_from_web.join(", ")
        ));
    }
    let missing_from_rust = web.difference(rust).cloned().collect::<Vec<_>>();
    if !missing_from_rust.is_empty() {
        issues.push(format!(
            "quality payload keys missing from Rust engine: {}",
            missing_from_rust.join(", ")
        ));
    }
    issues
}

fn web_quality_domains(source: &str) -> BTreeSet<String> {
    let mut domains = BTreeSet::new();
    let mut in_domains = false;
    for line in source.lines().map(str::trim_start) {
        if line.starts_with("@domains %{") {
            in_domains = true;
            continue;
        }
        if in_domains && line.starts_with("def supported_operator_ids") {
            break;
        }
        if in_domains && line.starts_with("id: \"") {
            if let Some((domain, _tail)) = first_quoted_string_with_tail(line) {
                domains.insert(domain);
            }
        }
    }
    domains
}

pub(super) fn rust_quality_term_entry_schemas(
    root: &Path,
) -> RunnerResult<BTreeMap<(String, String), BTreeSet<String>>> {
    let mut schemas = BTreeMap::new();
    for (domain, relative_path) in RUST_QUALITY_SOURCES {
        let source = read_text(root, relative_path)?;
        schemas.extend(rust_quality_term_entry_schemas_from_source(domain, &source));
    }
    Ok(schemas)
}

pub(super) fn rust_quality_term_entry_schemas_from_source(
    domain: &str,
    source: &str,
) -> BTreeMap<(String, String), BTreeSet<String>> {
    let mut schemas = BTreeMap::new();
    let mut in_score = false;
    let mut in_compact = false;
    let mut score_json_count = 0usize;
    let mut current_variant = None::<String>;
    let mut current_keys = BTreeSet::new();

    for line in source.lines().map(str::trim_start) {
        if line.starts_with("fn score_quality_term(") {
            in_score = true;
            score_json_count = 0;
        } else if line.starts_with("fn compact_quality_term(") {
            in_compact = true;
        } else if line.starts_with("fn ") && !line.starts_with("fn score_quality_term(") {
            in_score = false;
            in_compact = false;
        }
        if current_variant.is_none() && line.contains("serde_json::json!({") {
            if in_score {
                score_json_count += 1;
                current_variant = Some(if score_json_count == 1 {
                    "present".to_string()
                } else {
                    "missing".to_string()
                });
            } else if in_compact {
                current_variant = Some("compact".to_string());
            }
        }
        if current_variant.is_some() {
            if let Some((key, tail)) = first_quoted_string_with_tail(line) {
                if tail.trim_start().starts_with(':') {
                    current_keys.insert(key);
                }
            }
            if line.starts_with("})") || line.starts_with("}),") {
                let variant = current_variant.take().expect("variant is set");
                schemas.insert(
                    (domain.to_string(), variant),
                    std::mem::take(&mut current_keys),
                );
            }
        }
    }
    schemas
}

pub(super) fn web_quality_term_entry_schema(source: &str) -> BTreeMap<String, BTreeSet<String>> {
    let mut schemas = BTreeMap::new();
    let mut in_score = false;
    let mut in_compact = false;
    let mut score_map_count = 0usize;
    let mut current_variant = None::<String>;
    let mut current_keys = BTreeSet::new();

    for line in source.lines().map(str::trim_start) {
        if line.starts_with("defp score_term(") {
            in_score = true;
            score_map_count = 0;
        } else if line.starts_with("defp compact_quality_term(term)") {
            in_compact = true;
        } else if line.starts_with("defp ") && !line.starts_with("defp score_term(") {
            in_score = false;
            in_compact = false;
        }
        if current_variant.is_none() && line.starts_with("%{") {
            if in_score {
                score_map_count += 1;
                current_variant = Some(if score_map_count == 1 {
                    "present".to_string()
                } else {
                    "missing".to_string()
                });
            } else if in_compact {
                current_variant = Some("compact".to_string());
            }
        }
        if current_variant.is_some() {
            if let Some((key, tail)) = first_quoted_string_with_tail(line) {
                if tail.trim_start().starts_with("=>") {
                    current_keys.insert(key);
                }
            }
            if line.starts_with('}') {
                let variant = current_variant.take().expect("variant is set");
                schemas.insert(variant, std::mem::take(&mut current_keys));
            }
        }
    }
    schemas
}

pub(super) fn compare_quality_term_entry_schemas(
    rust: &BTreeMap<(String, String), BTreeSet<String>>,
    web: &BTreeMap<String, BTreeSet<String>>,
) -> Vec<String> {
    let mut issues = Vec::new();
    if rust.is_empty() {
        issues.push("Rust quality term entry schema contract extracted no keys".to_string());
    }
    if web.is_empty() {
        issues.push("Web quality term entry schema contract extracted no keys".to_string());
    }
    for ((domain, variant), rust_keys) in rust {
        let Some(web_keys) = web.get(variant) else {
            issues.push(format!(
                "Web quality term entry schema missing variant {variant}"
            ));
            continue;
        };
        let missing_from_rust = web_keys.difference(rust_keys).cloned().collect::<Vec<_>>();
        if !missing_from_rust.is_empty() {
            issues.push(format!(
                "quality term entry keys missing from Rust {domain}:{variant}: {}",
                missing_from_rust.join(", ")
            ));
        }
        let missing_from_web = rust_keys.difference(web_keys).cloned().collect::<Vec<_>>();
        if !missing_from_web.is_empty() {
            issues.push(format!(
                "quality term entry keys missing from Web {domain}:{variant}: {}",
                missing_from_web.join(", ")
            ));
        }
    }
    issues
}
