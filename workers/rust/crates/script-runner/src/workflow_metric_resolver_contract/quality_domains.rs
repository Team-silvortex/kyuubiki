use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use super::{
    RUST_QUALITY_SOURCES, RunnerResult, first_quoted_string_with_tail, read_text,
    two_quoted_strings,
};

const RUST_WORKFLOW_EXECUTOR_PATH: &str = "workers/rust/crates/engine/src/workflow_executor.rs";

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct QualityDomainMetadata {
    ready: String,
    contract: String,
}

impl QualityDomainMetadata {
    fn describe(&self) -> String {
        format!("ready={}, contract={}", self.ready, self.contract)
    }
}

pub(super) fn rust_quality_domain_metadata(
    root: &Path,
) -> RunnerResult<BTreeMap<String, QualityDomainMetadata>> {
    let mut metadata = BTreeMap::new();
    for (domain, relative_path) in RUST_QUALITY_SOURCES {
        let source = read_text(root, relative_path)?;
        if let Some(signature) = rust_quality_domain_metadata_from_source(&source) {
            metadata.insert((*domain).to_string(), signature);
        }
    }
    Ok(metadata)
}

pub(super) fn rust_quality_domain_metadata_from_source(
    source: &str,
) -> Option<QualityDomainMetadata> {
    let mut ready = None::<String>;
    let mut contract = None::<String>;
    for line in source.lines().map(str::trim_start) {
        if line.contains("config_number(&config, \"max_ready_score\"") {
            ready = config_number_default(line);
        }
        if line.contains("_quality_contract") {
            contract = two_quoted_strings(line).map(|(_key, value)| value);
        }
    }
    Some(QualityDomainMetadata {
        ready: ready?,
        contract: contract?,
    })
}

pub(super) fn web_quality_domain_metadata(source: &str) -> BTreeMap<String, QualityDomainMetadata> {
    let mut ready_values = BTreeMap::new();
    let mut in_domains = false;
    let mut current_domain = None::<String>;
    let mut contract_template = None::<String>;

    for line in source.lines().map(str::trim_start) {
        if line.starts_with("@domains %{") {
            in_domains = true;
            continue;
        }
        if in_domains && line.starts_with("def supported_operator_ids") {
            in_domains = false;
            current_domain = None;
        }
        if in_domains {
            if line.contains("id: \"") {
                current_domain = first_quoted_string_with_tail(line).map(|(domain, _tail)| domain);
                continue;
            }
            if line.starts_with("ready:") {
                if let (Some(domain), Some(ready)) = (&current_domain, ready_value(line)) {
                    ready_values.insert(domain.clone(), ready);
                }
            }
        }
        if line.contains("_quality_contract") {
            contract_template = two_quoted_strings(line).map(|(_key, value)| value);
        }
    }

    let mut metadata = BTreeMap::new();
    for (domain, ready) in ready_values {
        metadata.insert(
            domain.clone(),
            QualityDomainMetadata {
                ready,
                contract: contract_template
                    .as_deref()
                    .unwrap_or("")
                    .replace("#{id}", &domain),
            },
        );
    }
    metadata
}

pub(super) fn compare_quality_domain_metadata(
    rust: &BTreeMap<String, QualityDomainMetadata>,
    web: &BTreeMap<String, QualityDomainMetadata>,
) -> Vec<String> {
    let mut issues = Vec::new();
    if rust.is_empty() {
        issues.push("Rust quality domain metadata contract extracted no domains".to_string());
    }
    if web.is_empty() {
        issues.push("Web quality domain metadata contract extracted no domains".to_string());
    }
    let missing_from_web = rust
        .keys()
        .filter(|domain| !web.contains_key(*domain))
        .cloned()
        .collect::<Vec<_>>();
    if !missing_from_web.is_empty() {
        issues.push(format!(
            "quality domain metadata missing from Web runtime: {}",
            missing_from_web.join(", ")
        ));
    }
    let missing_from_rust = web
        .keys()
        .filter(|domain| !rust.contains_key(*domain))
        .cloned()
        .collect::<Vec<_>>();
    if !missing_from_rust.is_empty() {
        issues.push(format!(
            "quality domain metadata missing from Rust engine: {}",
            missing_from_rust.join(", ")
        ));
    }
    for (domain, rust_metadata) in rust {
        if let Some(web_metadata) = web.get(domain) {
            if rust_metadata != web_metadata {
                issues.push(format!(
                    "quality domain metadata drift for {domain}: Rust {}; Web {}",
                    rust_metadata.describe(),
                    web_metadata.describe()
                ));
            }
        }
    }
    issues
}

pub(super) fn rust_quality_operator_ids(root: &Path) -> RunnerResult<BTreeSet<String>> {
    let source = read_text(root, RUST_WORKFLOW_EXECUTOR_PATH)?;
    Ok(quality_operator_ids_from_source(&source))
}

pub(super) fn web_quality_operator_ids(source: &str) -> BTreeSet<String> {
    quality_operator_ids_from_source(source)
}

pub(super) fn compare_quality_operator_ids(
    rust: &BTreeSet<String>,
    web: &BTreeSet<String>,
) -> Vec<String> {
    let mut issues = Vec::new();
    if rust.is_empty() {
        issues.push("Rust quality operator contract extracted no operator ids".to_string());
    }
    if web.is_empty() {
        issues.push("Web quality operator contract extracted no operator ids".to_string());
    }
    let missing_from_web = rust.difference(web).cloned().collect::<Vec<_>>();
    if !missing_from_web.is_empty() {
        issues.push(format!(
            "quality operator ids missing from Web runtime: {}",
            missing_from_web.join(", ")
        ));
    }
    let missing_from_rust = web.difference(rust).cloned().collect::<Vec<_>>();
    if !missing_from_rust.is_empty() {
        issues.push(format!(
            "quality operator ids missing from Rust engine: {}",
            missing_from_rust.join(", ")
        ));
    }
    issues
}

fn quality_operator_ids_from_source(source: &str) -> BTreeSet<String> {
    source
        .lines()
        .filter_map(first_quoted_string_with_tail)
        .map(|(value, _tail)| value)
        .filter(|value| value.starts_with("transform.score_") && value.ends_with("_quality"))
        .collect()
}

pub(super) fn run_self_test() -> RunnerResult<()> {
    let rust = r#"
let max_ready_score = config_number(&config, "max_ready_score", 7.0);
"acoustic_quality_contract": "kyuubiki.acoustic_quality_score/v1",
"#;
    let web = r##"
@domains %{
  "transform.score_acoustic_quality" => %{
    id: "acoustic",
    ready: 7.0,
    terms: []
  }
}
def supported_operator_ids, do: Map.keys(@domains)
"#{id}_quality_contract" => "kyuubiki.#{id}_quality_score/v1",
"##;
    let mut rust_metadata = BTreeMap::new();
    rust_metadata.insert(
        "acoustic".to_string(),
        rust_quality_domain_metadata_from_source(rust)
            .ok_or_else(|| "self-test failed to parse Rust domain metadata".to_string())?,
    );
    let web_metadata = web_quality_domain_metadata(web);
    let issues = compare_quality_domain_metadata(&rust_metadata, &web_metadata);
    if !issues.is_empty() {
        return Err(format!(
            "self-test matching quality domain metadata fixtures drifted: {issues:?}"
        ));
    }

    let drifted_web = web.replace("ready: 7.0,", "ready: 8.0,");
    expect_issue(
        compare_quality_domain_metadata(&rust_metadata, &web_quality_domain_metadata(&drifted_web)),
        "quality domain metadata drift for acoustic",
    )?;

    let rust_operator_source = r#"
const SUPPORTED_TRANSFORM_OPERATORS: &[&str] = &[
  "transform.score_acoustic_quality",
  "transform.score_cfd_quality",
];
"#;
    let web_operator_source = r#"
@domains %{
  "transform.score_acoustic_quality" => %{id: "acoustic"},
  "transform.score_cfd_quality" => %{id: "cfd"}
}
"#;
    let rust_operator_ids = quality_operator_ids_from_source(rust_operator_source);
    let web_operator_ids = web_quality_operator_ids(web_operator_source);
    let issues = compare_quality_operator_ids(&rust_operator_ids, &web_operator_ids);
    if !issues.is_empty() {
        return Err(format!(
            "self-test matching quality operator fixtures drifted: {issues:?}"
        ));
    }

    let missing_web_operator =
        web_operator_source.replace("  \"transform.score_cfd_quality\" => %{id: \"cfd\"}\n", "");
    expect_issue(
        compare_quality_operator_ids(
            &rust_operator_ids,
            &web_quality_operator_ids(&missing_web_operator),
        ),
        "quality operator ids missing from Web runtime: transform.score_cfd_quality",
    )
}

fn config_number_default(line: &str) -> Option<String> {
    let marker = "\"max_ready_score\",";
    let after_marker = line.split_once(marker)?.1;
    let value = after_marker.split_once(')')?.0;
    Some(value.trim().trim_end_matches(',').to_string())
}

fn ready_value(line: &str) -> Option<String> {
    let after_ready = line.split_once(':')?.1;
    Some(after_ready.trim().trim_end_matches(',').to_string())
}

fn expect_issue(issues: Vec<String>, expected: &str) -> RunnerResult<()> {
    if issues.iter().any(|issue| issue.contains(expected)) {
        Ok(())
    } else {
        Err(format!(
            "self-test expected issue containing {expected:?}, got {issues:?}"
        ))
    }
}
