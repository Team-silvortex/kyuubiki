use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use super::{
    RUST_QUALITY_SOURCES, RunnerResult, first_quoted_string_with_tail, read_text,
    two_quoted_strings,
};

pub(super) fn rust_quality_terms(root: &Path) -> RunnerResult<BTreeSet<(String, String)>> {
    let mut terms = BTreeSet::new();
    for (domain, relative_path) in RUST_QUALITY_SOURCES {
        let source = read_text(root, relative_path)?;
        terms.extend(rust_quality_terms_from_source(domain, &source));
    }
    Ok(terms)
}

pub(super) fn rust_quality_terms_from_source(
    domain: &str,
    source: &str,
) -> BTreeSet<(String, String)> {
    source
        .lines()
        .map(str::trim_start)
        .filter(|line| line.starts_with("field: \""))
        .filter_map(|line| first_quoted_string_with_tail(line).map(|(field, _tail)| field))
        .map(|field| (domain.to_string(), field))
        .collect()
}

pub(super) fn web_quality_terms(source: &str) -> BTreeSet<(String, String)> {
    let mut terms = BTreeSet::new();
    let mut in_domains = false;
    let mut current_domain = None::<String>;

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
            if line.starts_with("id: \"") {
                current_domain = first_quoted_string_with_tail(line).map(|(domain, _tail)| domain);
                continue;
            }
            if line.starts_with("{\"") {
                if let (Some(domain), Some((field, _tail))) =
                    (&current_domain, first_quoted_string_with_tail(line))
                {
                    terms.insert((domain.clone(), field));
                }
            }
        }
        if line.starts_with("defp extra_quality_term(") {
            if let Some((domain, field)) = two_quoted_strings(line) {
                terms.insert((domain, field));
            }
        }
    }
    terms
}

pub(super) fn compare_quality_terms(
    rust: &BTreeSet<(String, String)>,
    web: &BTreeSet<(String, String)>,
) -> Vec<String> {
    let mut issues = Vec::new();
    if rust.is_empty() {
        issues.push("Rust quality term contract extracted no fields".to_string());
    }
    if web.is_empty() {
        issues.push("Web quality term contract extracted no fields".to_string());
    }
    let missing_from_web = rust.difference(web).cloned().collect::<Vec<_>>();
    if !missing_from_web.is_empty() {
        issues.push(format!(
            "quality score terms missing from Web runtime: {}",
            format_domain_fields(&missing_from_web)
        ));
    }
    let missing_from_rust = web.difference(rust).cloned().collect::<Vec<_>>();
    if !missing_from_rust.is_empty() {
        issues.push(format!(
            "quality score terms missing from Rust engine: {}",
            format_domain_fields(&missing_from_rust)
        ));
    }
    issues
}

fn format_domain_fields(pairs: &[(String, String)]) -> String {
    pairs
        .iter()
        .map(format_domain_field)
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_domain_field((domain, field): &(String, String)) -> String {
    format!("{domain}:{field}")
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct QualityTermSignature {
    label: String,
    target: String,
    weight: String,
    goal: String,
}

impl QualityTermSignature {
    fn describe(&self) -> String {
        format!(
            "label={:?}, target={}, weight={}, goal={}",
            self.label, self.target, self.weight, self.goal
        )
    }
}

#[derive(Default)]
struct PartialQualityTerm {
    field: Option<String>,
    label: Option<String>,
    target: Option<String>,
    weight: Option<String>,
    goal: Option<String>,
}

impl PartialQualityTerm {
    fn finish(self) -> Option<((String, String), QualityTermSignature)> {
        let field = self.field?;
        let signature = QualityTermSignature {
            label: self.label?,
            target: self.target?,
            weight: self.weight?,
            goal: self.goal.unwrap_or_else(|| "min".to_string()),
        };
        Some(((String::new(), field), signature))
    }
}

pub(super) fn rust_quality_term_signatures(
    root: &Path,
) -> RunnerResult<BTreeMap<(String, String), QualityTermSignature>> {
    let mut signatures = BTreeMap::new();
    for (domain, relative_path) in RUST_QUALITY_SOURCES {
        let source = read_text(root, relative_path)?;
        signatures.extend(rust_quality_term_signatures_from_source(domain, &source));
    }
    Ok(signatures)
}

pub(super) fn rust_quality_term_signatures_from_source(
    domain: &str,
    source: &str,
) -> BTreeMap<(String, String), QualityTermSignature> {
    let mut signatures = BTreeMap::new();
    let mut current = None::<PartialQualityTerm>;
    for line in source.lines().map(str::trim) {
        if line.contains("QualityTerm {") {
            current = Some(PartialQualityTerm::default());
            continue;
        }
        let Some(term) = current.as_mut() else {
            continue;
        };
        if line.starts_with("field: \"") {
            term.field = first_quoted_string_with_tail(line).map(|(field, _tail)| field);
        } else if line.starts_with("label: \"") {
            term.label = first_quoted_string_with_tail(line).map(|(label, _tail)| label);
        } else if line.starts_with("target:") {
            term.target = rust_value_after_colon(line);
        } else if line.starts_with("weight:") {
            term.weight = rust_value_after_colon(line);
        } else if line.starts_with("goal:") {
            term.goal = normalize_goal(line);
        }
        if line.starts_with("},") || line.starts_with("})") || line == "}" {
            if let Some(((_parsed_domain, field), signature)) =
                current.take().and_then(|term| term.finish())
            {
                signatures.insert((domain.to_string(), field), signature);
            }
        }
    }
    signatures
}

pub(super) fn web_quality_term_signatures(
    source: &str,
) -> BTreeMap<(String, String), QualityTermSignature> {
    let mut signatures = BTreeMap::new();
    let mut in_domains = false;
    let mut current_domain = None::<String>;
    let mut pending_tuple_domain = None::<String>;
    let mut pending_tuple = String::new();
    let mut pending_extra_domain = None::<String>;

    for line in source.lines().map(str::trim) {
        if continue_web_quality_tuple(
            &mut signatures,
            &mut pending_tuple_domain,
            &mut pending_tuple,
            line,
        ) {
            continue;
        }
        if let Some(domain) = pending_extra_domain.clone() {
            if let Some(start) = line.find('{') {
                collect_web_quality_tuple(
                    &mut signatures,
                    &mut pending_tuple_domain,
                    &mut pending_tuple,
                    &domain,
                    &line[start..],
                );
                pending_extra_domain = None;
                continue;
            }
        }
        if line.starts_with("@domains %{") {
            in_domains = true;
            continue;
        }
        if in_domains && line.starts_with("def supported_operator_ids") {
            in_domains = false;
            current_domain = None;
        }
        if in_domains {
            if line.starts_with("id: \"") {
                current_domain = first_quoted_string_with_tail(line).map(|(domain, _tail)| domain);
                continue;
            }
            if line.starts_with("{\"") {
                if let Some(domain) = &current_domain {
                    collect_web_quality_tuple(
                        &mut signatures,
                        &mut pending_tuple_domain,
                        &mut pending_tuple,
                        domain,
                        line,
                    );
                }
            }
        }
        if line.starts_with("defp extra_quality_term(") {
            if let Some((domain, _field)) = two_quoted_strings(line) {
                if let Some(start) = line.find('{') {
                    collect_web_quality_tuple(
                        &mut signatures,
                        &mut pending_tuple_domain,
                        &mut pending_tuple,
                        &domain,
                        &line[start..],
                    );
                } else {
                    pending_extra_domain = Some(domain);
                }
            }
        }
    }
    signatures
}

fn continue_web_quality_tuple(
    signatures: &mut BTreeMap<(String, String), QualityTermSignature>,
    pending_tuple_domain: &mut Option<String>,
    pending_tuple: &mut String,
    line: &str,
) -> bool {
    let Some(domain) = pending_tuple_domain.clone() else {
        return false;
    };
    pending_tuple.push(' ');
    pending_tuple.push_str(line);
    if line.contains('}') {
        if let Some((key, signature)) = web_quality_signature_from_tuple(&domain, pending_tuple) {
            signatures.insert(key, signature);
        }
        pending_tuple_domain.take();
        pending_tuple.clear();
    }
    true
}

fn collect_web_quality_tuple(
    signatures: &mut BTreeMap<(String, String), QualityTermSignature>,
    pending_tuple_domain: &mut Option<String>,
    pending_tuple: &mut String,
    domain: &str,
    tuple: &str,
) {
    if tuple.contains('}') {
        if let Some((key, signature)) = web_quality_signature_from_tuple(domain, tuple) {
            signatures.insert(key, signature);
        }
    } else {
        *pending_tuple_domain = Some(domain.to_string());
        pending_tuple.clear();
        pending_tuple.push_str(tuple);
    }
}

fn web_quality_signature_from_tuple(
    domain: &str,
    tuple: &str,
) -> Option<((String, String), QualityTermSignature)> {
    let (field, tail) = first_quoted_string_with_tail(tuple)?;
    let (label, tail) = first_quoted_string_with_tail(tail)?;
    let mut pieces = tail
        .split(',')
        .map(clean_tuple_piece)
        .filter(|piece| !piece.is_empty());
    let target = pieces.next()?;
    let weight = pieces.next()?;
    let goal = pieces.find_map(|piece| normalize_goal(&piece))?;
    Some((
        (domain.to_string(), field),
        QualityTermSignature {
            label,
            target,
            weight,
            goal,
        },
    ))
}

pub(super) fn compare_quality_term_signatures(
    rust: &BTreeMap<(String, String), QualityTermSignature>,
    web: &BTreeMap<(String, String), QualityTermSignature>,
) -> Vec<String> {
    let mut issues = Vec::new();
    if rust.is_empty() {
        issues.push("Rust quality term signature contract extracted no fields".to_string());
    }
    if web.is_empty() {
        issues.push("Web quality term signature contract extracted no fields".to_string());
    }
    let missing_from_web = rust
        .keys()
        .filter(|key| !web.contains_key(*key))
        .cloned()
        .collect::<Vec<_>>();
    if !missing_from_web.is_empty() {
        issues.push(format!(
            "quality score term signatures missing from Web runtime: {}",
            format_domain_fields(&missing_from_web)
        ));
    }
    let missing_from_rust = web
        .keys()
        .filter(|key| !rust.contains_key(*key))
        .cloned()
        .collect::<Vec<_>>();
    if !missing_from_rust.is_empty() {
        issues.push(format!(
            "quality score term signatures missing from Rust engine: {}",
            format_domain_fields(&missing_from_rust)
        ));
    }
    for (key, rust_signature) in rust {
        if let Some(web_signature) = web.get(key) {
            if rust_signature != web_signature {
                issues.push(format!(
                    "quality score term signature drift for {}: Rust {}; Web {}",
                    format_domain_field(key),
                    rust_signature.describe(),
                    web_signature.describe()
                ));
            }
        }
    }
    issues
}

pub(super) fn compare_quality_signature_coverage(
    label: &str,
    terms: &BTreeSet<(String, String)>,
    signatures: &BTreeMap<(String, String), QualityTermSignature>,
) -> Vec<String> {
    let missing = terms
        .iter()
        .filter(|key| !signatures.contains_key(*key))
        .cloned()
        .collect::<Vec<_>>();
    if missing.is_empty() {
        Vec::new()
    } else {
        vec![format!(
            "{label} quality term signatures missing parser coverage: {}",
            format_domain_fields(&missing)
        )]
    }
}

fn rust_value_after_colon(line: &str) -> Option<String> {
    line.split_once(':')
        .map(|(_key, value)| value.trim().trim_end_matches(',').to_string())
}

fn clean_tuple_piece(piece: &str) -> String {
    piece
        .trim()
        .trim_start_matches("do:")
        .trim()
        .trim_start_matches('{')
        .trim_end_matches('}')
        .trim_end_matches(')')
        .trim_end_matches(',')
        .trim()
        .to_string()
}

fn normalize_goal(value: &str) -> Option<String> {
    if value.contains("Min") || value.contains(":min") {
        Some("min".to_string())
    } else if value.contains("Max") || value.contains(":max") {
        Some("max".to_string())
    } else {
        None
    }
}
