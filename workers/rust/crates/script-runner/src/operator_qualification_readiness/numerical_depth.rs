use super::{array, field};
use serde_json::{Value, json};
use std::collections::BTreeSet;

const REQUIRED_DIMENSIONS: &[&str] =
    &["reference", "convergence", "robustness", "retained_release"];

pub(super) fn assess(artifacts: &[Value]) -> Value {
    let present_kinds = artifacts
        .iter()
        .filter(|artifact| field(artifact, "state") == "present")
        .map(|artifact| field(artifact, "kind"))
        .collect::<BTreeSet<_>>();
    let mut present_dimensions = Vec::new();
    if has_any(
        &present_kinds,
        &["derivation_note", "reference_note", "versioned_baseline"],
    ) {
        present_dimensions.push("reference");
    }
    if present_kinds
        .iter()
        .any(|kind| kind.starts_with("convergence_"))
    {
        present_dimensions.push("convergence");
    }
    if has_any(
        &present_kinds,
        &[
            "boundary_regression",
            "boundary_report",
            "orientation_regression",
            "regression_test",
            "tolerance_policy",
        ],
    ) {
        present_dimensions.push("robustness");
    }
    if present_kinds.contains("release_retained_regression_output") {
        present_dimensions.push("retained_release");
    }
    let missing_dimensions = REQUIRED_DIMENSIONS
        .iter()
        .filter(|dimension| !present_dimensions.contains(dimension))
        .copied()
        .collect::<Vec<_>>();
    let level = match missing_dimensions.len() {
        0 => "complete",
        1 => "partial",
        _ => "thin",
    };
    json!({
        "level": level,
        "required_dimensions": REQUIRED_DIMENSIONS,
        "present_dimensions": present_dimensions,
        "missing_dimensions": missing_dimensions,
        "independent_reference": present_kinds.contains("reference_note"),
        "artifact_kinds": present_kinds
    })
}

pub(super) fn summarize(candidates: &[Value]) -> Value {
    let mut complete = 0;
    let mut partial = 0;
    let mut thin = 0;
    let mut independent_reference = 0;
    let mut missing = REQUIRED_DIMENSIONS
        .iter()
        .map(|dimension| ((*dimension).to_string(), Value::from(0_u64)))
        .collect::<serde_json::Map<_, _>>();
    for candidate in candidates {
        let depth = candidate
            .get("numerical_validation_depth")
            .cloned()
            .unwrap_or_else(|| assess_owned(candidate));
        match field(&depth, "level") {
            "complete" => complete += 1,
            "partial" => partial += 1,
            _ => thin += 1,
        }
        if depth
            .get("independent_reference")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            independent_reference += 1;
        }
        for dimension in array(&depth, "missing_dimensions")
            .into_iter()
            .filter_map(Value::as_str)
        {
            if let Some(count) = missing.get(dimension).and_then(Value::as_u64) {
                missing.insert(dimension.to_string(), Value::from(count + 1));
            }
        }
    }
    json!({
        "complete": complete,
        "partial": partial,
        "thin": thin,
        "with_independent_reference": independent_reference,
        "missing_dimensions": missing
    })
}

pub(super) fn validate_report(report: &Value, relative_input: &str, errors: &mut Vec<String>) {
    let candidates = array(report, "candidates")
        .into_iter()
        .cloned()
        .collect::<Vec<_>>();
    for candidate in &candidates {
        let expected = assess_owned(candidate);
        if candidate.get("numerical_validation_depth") != Some(&expected) {
            errors.push(format!(
                "{relative_input}: {} numerical_validation_depth is stale",
                field(candidate, "candidate_id")
            ));
        }
    }
    let expected_summary = summarize(&candidates);
    if report.pointer("/summary/numerical_validation_depth") != Some(&expected_summary) {
        errors.push(format!(
            "{relative_input}: summary.numerical_validation_depth is stale"
        ));
    }
}

fn assess_owned(candidate: &Value) -> Value {
    assess(
        &array(candidate, "artifacts")
            .into_iter()
            .cloned()
            .collect::<Vec<_>>(),
    )
}

fn has_any(present: &BTreeSet<&str>, expected: &[&str]) -> bool {
    expected.iter().any(|kind| present.contains(kind))
}

#[cfg(test)]
mod tests {
    use super::{assess, summarize};
    use serde_json::{Value, json};

    #[test]
    fn depth_requires_all_four_evidence_dimensions() {
        let depth = assess(&[
            artifact("derivation_note"),
            artifact("convergence_regression"),
            artifact("boundary_regression"),
            artifact("release_retained_regression_output"),
        ]);
        assert_eq!(depth["level"], "complete");
        assert_eq!(depth["missing_dimensions"], json!([]));
    }

    #[test]
    fn summary_exposes_missing_convergence() {
        let artifacts = vec![
            artifact("derivation_note"),
            artifact("boundary_regression"),
            artifact("release_retained_regression_output"),
        ];
        let candidate = json!({
            "candidate_id": "sample",
            "artifacts": artifacts,
            "numerical_validation_depth": assess(&artifacts)
        });
        let summary = summarize(&[candidate]);
        assert_eq!(summary["partial"], 1);
        assert_eq!(summary["missing_dimensions"]["convergence"], 1);
    }

    fn artifact(kind: &str) -> Value {
        json!({"kind": kind, "state": "present"})
    }
}
