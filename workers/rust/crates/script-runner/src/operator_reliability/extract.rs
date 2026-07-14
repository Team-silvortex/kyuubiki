use std::collections::HashSet;
use std::fs;
use std::path::Path;

const CATALOG_PATH: &str = "workers/rust/crates/benchmark/src/catalog_defaults.rs";
const WORKFLOW_PAYLOAD_PATH: &str = "workers/rust/crates/benchmark/src/workflow_payloads.rs";

type RunnerResult<T> = Result<T, String>;

pub(crate) fn physics_coverage_templates(root: &Path) -> RunnerResult<HashSet<String>> {
    let source = read_text(root, CATALOG_PATH)?;
    let block = extract_array_block(&source, "\"physics-coverage\"")?;
    Ok(extract_quoted_strings(block).into_iter().collect())
}

pub(crate) fn workflow_operator_ids(root: &Path) -> RunnerResult<HashSet<String>> {
    let source = read_text(root, WORKFLOW_PAYLOAD_PATH)?;
    Ok(source
        .split("payload(\"")
        .skip(1)
        .filter_map(|rest| rest.split('"').next())
        .map(ToString::to_string)
        .collect())
}

fn extract_array_block<'a>(source: &'a str, marker: &str) -> RunnerResult<&'a str> {
    let marker_index = source
        .find(marker)
        .ok_or_else(|| format!("missing marker {marker}"))?;
    let start = source[marker_index..]
        .find("&[")
        .map(|index| marker_index + index)
        .ok_or_else(|| format!("missing string array after {marker}"))?;
    let mut depth = 0isize;
    for (offset, ch) in source[start..].char_indices() {
        if ch == '[' {
            depth += 1;
        } else if ch == ']' {
            depth -= 1;
            if depth == 0 {
                return Ok(&source[start..=start + offset]);
            }
        }
    }
    Err(format!("unterminated array after {marker}"))
}

fn extract_quoted_strings(source: &str) -> Vec<String> {
    source
        .split('"')
        .skip(1)
        .step_by(2)
        .map(ToString::to_string)
        .collect()
}

fn read_text(root: &Path, relative: &str) -> RunnerResult<String> {
    fs::read_to_string(root.join(relative))
        .map_err(|error| format!("failed to read {relative}: {error}"))
}

#[cfg(test)]
mod tests {
    use super::{extract_array_block, extract_quoted_strings};

    #[test]
    fn extracts_rust_string_array_after_marker() {
        let source = r#"x "physics-coverage" y &["a", "b"] z"#;
        let block = extract_array_block(source, "\"physics-coverage\"").unwrap();
        assert_eq!(extract_quoted_strings(block), vec!["a", "b"]);
    }
}
