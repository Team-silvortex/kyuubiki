use super::{
    CoverageTarget, Value, non_empty_string, normalize_case_id, number_field, observed_case_ids,
    string_field,
};
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn coverage_summaries(
    runs: &[Value],
    coverage_targets: &[CoverageTarget],
) -> Vec<Value> {
    coverage_targets
        .iter()
        .map(|target| {
            let observed = runs
                .iter()
                .filter(|run| string_field(run, "matrix") == target.matrix && string_field(run, "profile") == target.profile)
                .flat_map(observed_case_ids)
                .collect::<BTreeSet<_>>();
            let covered_cases = target.expected_cases.iter().filter(|case| observed.contains(*case)).cloned().collect::<Vec<_>>();
            let missing_cases = target.expected_cases.iter().filter(|case| !observed.contains(*case)).cloned().collect::<Vec<_>>();
            let scale_threshold = (target.profile == "one_million").then_some(1_000_000_u64);
            let observed_nodes = runs
                .iter()
                .filter(|run| string_field(run, "matrix") == target.matrix && string_field(run, "profile") == target.profile)
                .flat_map(run_case_shapes_from_row)
                .fold(BTreeMap::<String, u64>::new(), |mut nodes, (id, node_count)| {
                    nodes.entry(normalize_case_id(&id)).and_modify(|current| *current = (*current).max(node_count)).or_insert(node_count);
                    nodes
                });
            let qualified_covered_cases = scale_threshold.map_or_else(Vec::new, |threshold| covered_cases.iter().filter(|case| observed_nodes.get(*case).is_some_and(|nodes| *nodes >= threshold)).cloned().collect());
            let below_scale_threshold_cases = scale_threshold.map_or_else(Vec::new, |_| covered_cases.iter().filter(|case| !qualified_covered_cases.contains(*case)).cloned().collect());
            let below_scale_threshold_details = below_scale_threshold_cases.iter().map(|case| json!({ "id": case, "reason": target.scale_limit_reasons.get(case), "remediation": target.scale_limit_remediations.get(case) })).collect::<Vec<_>>();
            json!({
                "matrix": target.matrix,
                "profile": target.profile,
                "expected_case_count": target.expected_cases.len(),
                "covered_case_count": covered_cases.len(),
                "missing_case_count": missing_cases.len(),
                "covered_cases": covered_cases,
                "missing_cases": missing_cases,
                "scale_qualified_node_threshold": scale_threshold,
                "scale_qualified_covered_case_count": qualified_covered_cases.len(),
                "scale_qualified_covered_cases": qualified_covered_cases,
                "below_scale_threshold_case_count": below_scale_threshold_cases.len(),
                "below_scale_threshold_cases": below_scale_threshold_cases,
                "below_scale_threshold_details": below_scale_threshold_details,
            })
        })
        .collect()
}

pub(super) fn profile_coverage_summaries(coverage: &[Value]) -> Vec<Value> {
    let mut summaries = BTreeMap::<String, (u64, u64, u64, Option<u64>)>::new();
    for entry in coverage {
        let profile = string_field(entry, "profile");
        let summary = summaries.entry(profile).or_insert((0, 0, 0, None));
        summary.0 += number_field(entry, "expected_case_count") as u64;
        summary.1 += number_field(entry, "covered_case_count") as u64;
        summary.2 += number_field(entry, "scale_qualified_covered_case_count") as u64;
        if let Some(threshold) = entry
            .get("scale_qualified_node_threshold")
            .and_then(Value::as_u64)
        {
            summary.3 = Some(threshold);
        }
    }
    summaries.into_iter().map(|(profile, (expected, covered, qualified, threshold))| json!({
        "profile": profile,
        "expected_case_count": expected,
        "covered_case_count": covered,
        "missing_case_count": expected.saturating_sub(covered),
        "scale_qualified_node_threshold": threshold,
        "scale_qualified_covered_case_count": qualified,
        "below_scale_threshold_case_count": threshold.map(|_| covered.saturating_sub(qualified)).unwrap_or(0),
    })).collect()
}

fn run_case_shapes_from_row(run: &Value) -> Vec<(String, u64)> {
    run.get("case_shapes")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|shape| {
            Some((
                non_empty_string(shape, "id")?,
                shape.get("node_count")?.as_u64()?,
            ))
        })
        .collect()
}
