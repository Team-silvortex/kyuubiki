use super::{CellEvaluationInput, RunnerResult, string_array, string_field};
use serde_json::{Map, Value, json};
use std::collections::BTreeSet;

const EXPECTED_LEVELS: &[&str] = &[
    "unassessed",
    "declared",
    "exercised",
    "verified",
    "qualified",
    "operational",
];

pub(super) fn validate_config(
    tensor: &Value,
    paradigms: &[String],
    module_ids: &BTreeSet<String>,
) -> RunnerResult<()> {
    let policy = policy(tensor)?;
    if string_field(policy, "gate_mode") != Some("advisory") {
        return Err(
            "evidence_grade_policy.gate_mode must remain advisory until an enforced release gate exists"
                .to_string(),
        );
    }
    let levels = policy
        .get("levels")
        .and_then(Value::as_array)
        .ok_or_else(|| "evidence_grade_policy.levels must be an array".to_string())?;
    if levels.len() != EXPECTED_LEVELS.len() {
        return Err(format!(
            "evidence_grade_policy.levels must contain {} calibrated levels",
            EXPECTED_LEVELS.len()
        ));
    }
    let mut previous_score = None;
    for (index, (level, expected)) in levels.iter().zip(EXPECTED_LEVELS).enumerate() {
        let id = required_string(level, "id", "evidence grade level", index)?;
        if id != *expected {
            return Err(format!(
                "evidence grade level {index} must be {expected}, found {id}"
            ));
        }
        let score = level
            .get("score")
            .and_then(Value::as_u64)
            .filter(|score| *score <= 100)
            .ok_or_else(|| format!("evidence grade {id} score must be between 0 and 100"))?;
        if previous_score.is_some_and(|previous| score <= previous) {
            return Err("evidence grade scores must increase strictly".to_string());
        }
        if level
            .get("description")
            .and_then(Value::as_str)
            .is_none_or(|description| description.is_empty())
        {
            return Err(format!("evidence grade {id} must describe itself"));
        }
        previous_score = Some(score);
    }
    if levels
        .first()
        .and_then(|level| level.get("score"))
        .and_then(Value::as_u64)
        != Some(0)
        || levels
            .last()
            .and_then(|level| level.get("score"))
            .and_then(Value::as_u64)
            != Some(100)
    {
        return Err("evidence grade scores must span 0 through 100".to_string());
    }

    let targets = policy
        .get("targets")
        .and_then(Value::as_object)
        .ok_or_else(|| "evidence_grade_policy.targets must be an object".to_string())?;
    for paradigm in paradigms {
        let target = targets
            .get(paradigm)
            .and_then(Value::as_str)
            .ok_or_else(|| format!("evidence grade target missing {paradigm}"))?;
        validate_target(target, &format!("target {paradigm}"))?;
    }
    for paradigm in targets.keys() {
        if !paradigms.contains(paradigm) {
            return Err(format!(
                "evidence grade targets unknown paradigm {paradigm}"
            ));
        }
    }
    validate_priority_weights(policy, paradigms)?;
    validate_overrides(policy, paradigms, module_ids)?;
    validate_claim_grades(tensor)?;
    let weakest_limit = policy
        .get("weakest_limit")
        .and_then(Value::as_u64)
        .unwrap_or(20);
    if !(1..=100).contains(&weakest_limit) {
        return Err("evidence_grade_policy.weakest_limit must be between 1 and 100".to_string());
    }
    Ok(())
}

pub(super) fn evaluate(input: &CellEvaluationInput<'_>) -> Value {
    let CellEvaluationInput {
        tensor,
        module_id,
        paradigm,
        status,
        required,
        benchmark_tests,
        security_tests,
        contract_evidence,
    } = *input;
    let policy = tensor.get("evidence_grade_policy").unwrap_or(&Value::Null);
    let levels = policy
        .get("levels")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let mut achieved_rank = 0;
    let mut sources = Vec::new();
    if status == "covered" || status == "partial" {
        achieved_rank = rank(levels, "declared").unwrap_or(0);
        sources.push("matrix_status".to_string());
    }
    if !contract_evidence.is_empty() {
        achieved_rank = achieved_rank.max(rank(levels, "declared").unwrap_or(0));
        sources.push("contract_evidence".to_string());
    }
    if !benchmark_tests.is_empty() || !security_tests.is_empty() {
        achieved_rank = achieved_rank.max(rank(levels, "exercised").unwrap_or(0));
        sources.push("runnable_lane".to_string());
    }

    let claims = matching_claims(tensor, module_id, paradigm)
        .into_iter()
        .map(|claim| {
            let claim_status = string_field(claim, "status").unwrap_or_default();
            let grade = string_field(claim, "grade").unwrap_or("unassessed");
            if claim_status == "proven" {
                achieved_rank = achieved_rank.max(rank(levels, grade).unwrap_or(0));
                sources.push(format!(
                    "claim:{}",
                    string_field(claim, "id").unwrap_or_default()
                ));
            }
            json!({
                "id": string_field(claim, "id").unwrap_or_default(),
                "status": claim_status,
                "grade": grade
            })
        })
        .collect::<Vec<_>>();
    sources.sort();
    sources.dedup();

    let target_grade = target_for(policy, module_id, paradigm);
    let target_rank = rank(levels, target_grade).unwrap_or(0);
    let gap_steps = if required {
        target_rank.saturating_sub(achieved_rank)
    } else {
        0
    };
    let state = if !required {
        "optional"
    } else if status != "covered" {
        "not_ready"
    } else if gap_steps == 0 {
        "target_met"
    } else {
        "below_target"
    };
    let achieved_grade = level_id(levels, achieved_rank);
    let next_grade = if gap_steps > 0 {
        level_id(levels, (achieved_rank + 1).min(target_rank))
    } else {
        achieved_grade
    };
    json!({
        "state": state,
        "achieved_grade": achieved_grade,
        "target_grade": target_grade,
        "next_grade": next_grade,
        "achieved_rank": achieved_rank,
        "target_rank": target_rank,
        "gap_steps": gap_steps,
        "evidence_score": level_score(levels, achieved_rank),
        "target_score": level_score(levels, target_rank),
        "sources": sources,
        "claims": claims
    })
}

pub(super) fn summarize(tensor: &Value, cells: &Map<String, Value>) -> Value {
    let policy = tensor.get("evidence_grade_policy").unwrap_or(&Value::Null);
    let levels = policy
        .get("levels")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let mut achieved_summary = empty_summary(levels);
    let mut target_summary = empty_summary(levels);
    let mut weakest = Vec::new();
    let mut required_count = 0_u64;
    let mut score_sum = 0_u64;
    let mut target_score_sum = 0_u64;
    let mut achieved_toward_target = 0_u64;

    for (module_id, module_cells) in cells {
        for (paradigm, cell) in module_cells.as_object().into_iter().flatten() {
            if cell.get("required").and_then(Value::as_bool) != Some(true) {
                continue;
            }
            required_count += 1;
            let grade = cell.get("evidence_grade").unwrap_or(&Value::Null);
            let achieved = string_field(grade, "achieved_grade").unwrap_or("unassessed");
            let target = string_field(grade, "target_grade").unwrap_or("unassessed");
            increment(&mut achieved_summary, achieved);
            increment(&mut target_summary, target);
            let score = grade
                .get("evidence_score")
                .and_then(Value::as_u64)
                .unwrap_or(0);
            let target_score = grade
                .get("target_score")
                .and_then(Value::as_u64)
                .unwrap_or(0);
            score_sum += score;
            target_score_sum += target_score;
            achieved_toward_target += score.min(target_score);
            let gap_steps = grade.get("gap_steps").and_then(Value::as_u64).unwrap_or(0);
            if gap_steps == 0 {
                continue;
            }
            let missing_dimensions = string_array(
                cell.pointer("/maturity_evidence").unwrap_or(&Value::Null),
                "missing_dimensions",
            );
            let status_penalty = u64::from(string_field(cell, "status") != Some("covered")) * 1000;
            let priority_score = status_penalty
                + gap_steps * 100
                + target_score.saturating_sub(score)
                + missing_dimensions.len() as u64 * 10
                + priority_weight(policy, paradigm);
            weakest.push(json!({
                "module_id": module_id,
                "paradigm": paradigm,
                "status": string_field(cell, "status").unwrap_or_default(),
                "maturity": string_field(cell, "maturity").unwrap_or_default(),
                "achieved_grade": achieved,
                "target_grade": target,
                "next_grade": string_field(grade, "next_grade").unwrap_or_default(),
                "gap_steps": gap_steps,
                "evidence_score": score,
                "target_score": target_score,
                "priority_score": priority_score,
                "missing_dimensions": missing_dimensions,
                "recommended_action": recommendation(achieved),
                "sources": grade.get("sources").cloned().unwrap_or_else(|| json!([]))
            }));
        }
    }
    weakest.sort_by(|left, right| {
        priority(right)
            .cmp(&priority(left))
            .then_with(|| coordinate(left).cmp(&coordinate(right)))
    });
    let gap_count = weakest.len();
    let all_gaps = weakest.clone();
    let weakest_limit = policy
        .get("weakest_limit")
        .and_then(Value::as_u64)
        .unwrap_or(20) as usize;
    weakest.truncate(weakest_limit);
    let average_score = if required_count == 0 {
        0.0
    } else {
        score_sum as f64 / required_count as f64
    };
    let average_target_score = if required_count == 0 {
        0.0
    } else {
        target_score_sum as f64 / required_count as f64
    };
    let proof_completion_percent = if target_score_sum == 0 {
        100.0
    } else {
        achieved_toward_target as f64 * 100.0 / target_score_sum as f64
    };
    let target_met_count = required_count.saturating_sub(gap_count as u64);
    let target_met_percent = if required_count == 0 {
        100.0
    } else {
        target_met_count as f64 * 100.0 / required_count as f64
    };
    json!({
        "ok": gap_count == 0,
        "gate_mode": string_field(policy, "gate_mode").unwrap_or("advisory"),
        "required_cell_count": required_count,
        "target_met_count": target_met_count,
        "target_met_percent": (target_met_percent * 10.0).round() / 10.0,
        "gap_count": gap_count,
        "average_evidence_score": (average_score * 10.0).round() / 10.0,
        "average_target_score": (average_target_score * 10.0).round() / 10.0,
        "achieved_score_toward_target": achieved_toward_target,
        "target_score_total": target_score_sum,
        "remaining_target_score": target_score_sum.saturating_sub(achieved_toward_target),
        "proof_completion_percent": (proof_completion_percent * 10.0).round() / 10.0,
        "achieved_summary": achieved_summary,
        "target_summary": target_summary,
        "gaps": all_gaps,
        "weakest_limit": weakest_limit,
        "weakest_points": weakest
    })
}

fn validate_priority_weights(policy: &Value, paradigms: &[String]) -> RunnerResult<()> {
    let weights = policy
        .get("priority_weights")
        .and_then(Value::as_object)
        .ok_or_else(|| "evidence_grade_policy.priority_weights must be an object".to_string())?;
    for paradigm in paradigms {
        weights
            .get(paradigm)
            .and_then(Value::as_u64)
            .filter(|weight| *weight <= 100)
            .ok_or_else(|| format!("priority weight for {paradigm} must be between 0 and 100"))?;
    }
    for paradigm in weights.keys() {
        if !paradigms.contains(paradigm) {
            return Err(format!("priority weights unknown paradigm {paradigm}"));
        }
    }
    Ok(())
}

fn validate_overrides(
    policy: &Value,
    paradigms: &[String],
    module_ids: &BTreeSet<String>,
) -> RunnerResult<()> {
    let mut seen = BTreeSet::new();
    for (index, entry) in array_field(policy, "cell_overrides").iter().enumerate() {
        let module_id = required_string(entry, "module_id", "cell override", index)?;
        let paradigm = required_string(entry, "paradigm", "cell override", index)?;
        let target = required_string(entry, "target", "cell override", index)?;
        if !module_ids.contains(module_id) {
            return Err(format!(
                "cell override references unknown module {module_id}"
            ));
        }
        if !paradigms.contains(&paradigm.to_string()) {
            return Err(format!(
                "cell override references unknown paradigm {paradigm}"
            ));
        }
        validate_target(target, "cell override")?;
        if !seen.insert((module_id, paradigm)) {
            return Err(format!(
                "duplicate evidence grade override {module_id}/{paradigm}"
            ));
        }
    }
    Ok(())
}

fn validate_claim_grades(tensor: &Value) -> RunnerResult<()> {
    for (index, claim) in array_field(tensor, "evidence_claims").iter().enumerate() {
        let grade = required_string(claim, "grade", "evidence claim", index)?;
        validate_target(grade, string_field(claim, "id").unwrap_or("evidence claim"))?;
    }
    Ok(())
}

fn validate_target(target: &str, context: &str) -> RunnerResult<()> {
    if target == "unassessed" || !EXPECTED_LEVELS.contains(&target) {
        return Err(format!("{context}: invalid evidence target {target}"));
    }
    Ok(())
}

fn target_for<'a>(policy: &'a Value, module_id: &str, paradigm: &str) -> &'a str {
    for entry in array_field(policy, "cell_overrides") {
        if string_field(entry, "module_id") == Some(module_id)
            && string_field(entry, "paradigm") == Some(paradigm)
        {
            return string_field(entry, "target").unwrap_or("unassessed");
        }
    }
    policy
        .pointer(&format!("/targets/{paradigm}"))
        .and_then(Value::as_str)
        .unwrap_or("unassessed")
}

fn matching_claims<'a>(tensor: &'a Value, module_id: &str, paradigm: &str) -> Vec<&'a Value> {
    array_field(tensor, "evidence_claims")
        .iter()
        .filter(|claim| {
            string_array(claim, "modules")
                .iter()
                .any(|id| id == module_id)
                && string_array(claim, "paradigms")
                    .iter()
                    .any(|id| id == paradigm)
        })
        .collect()
}

fn policy(tensor: &Value) -> RunnerResult<&Value> {
    tensor
        .get("evidence_grade_policy")
        .filter(|value| value.is_object())
        .ok_or_else(|| "evidence_grade_policy must be an object".to_string())
}

fn rank(levels: &[Value], id: &str) -> Option<usize> {
    levels
        .iter()
        .position(|level| string_field(level, "id") == Some(id))
}

fn level_id(levels: &[Value], rank: usize) -> &str {
    levels
        .get(rank)
        .and_then(|level| string_field(level, "id"))
        .unwrap_or("unassessed")
}

fn level_score(levels: &[Value], rank: usize) -> u64 {
    levels
        .get(rank)
        .and_then(|level| level.get("score"))
        .and_then(Value::as_u64)
        .unwrap_or(0)
}

fn empty_summary(levels: &[Value]) -> Map<String, Value> {
    levels
        .iter()
        .filter_map(|level| string_field(level, "id"))
        .map(|id| (id.to_string(), json!(0)))
        .collect()
}

fn increment(summary: &mut Map<String, Value>, grade: &str) {
    let next = summary.get(grade).and_then(Value::as_u64).unwrap_or(0) + 1;
    summary.insert(grade.to_string(), json!(next));
}

fn priority(value: &Value) -> u64 {
    value
        .get("priority_score")
        .and_then(Value::as_u64)
        .unwrap_or(0)
}

fn priority_weight(policy: &Value, paradigm: &str) -> u64 {
    policy
        .pointer(&format!("/priority_weights/{paradigm}"))
        .and_then(Value::as_u64)
        .unwrap_or(0)
}

fn coordinate(value: &Value) -> String {
    format!(
        "{}/{}",
        string_field(value, "module_id").unwrap_or_default(),
        string_field(value, "paradigm").unwrap_or_default()
    )
}

fn recommendation(grade: &str) -> &'static str {
    match grade {
        "unassessed" => "declare_contract_and_test_scope",
        "declared" => "add_runnable_smoke_evidence",
        "exercised" => "add_asserted_verification_evidence",
        "verified" => "add_repeatable_qualification_evidence",
        "qualified" => "add_packaged_or_multi_host_operational_evidence",
        _ => "retain_and_monitor_operational_evidence",
    }
}

fn required_string<'a>(
    entry: &'a Value,
    key: &str,
    collection: &str,
    index: usize,
) -> RunnerResult<&'a str> {
    string_field(entry, key)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{collection}[{index}].{key} must be a non-empty string"))
}

fn array_field<'a>(value: &'a Value, key: &str) -> &'a [Value] {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

#[cfg(test)]
mod tests {
    use super::{evaluate, summarize, validate_config};
    use crate::module_function_tensor::CellEvaluationInput;
    use serde_json::{Map, Value, json};
    use std::collections::BTreeSet;

    fn tensor() -> Value {
        json!({
            "evidence_grade_policy": {
                "levels": [
                    {"id": "unassessed", "score": 0, "description": "none"},
                    {"id": "declared", "score": 20, "description": "declared"},
                    {"id": "exercised", "score": 40, "description": "exercised"},
                    {"id": "verified", "score": 60, "description": "verified"},
                    {"id": "qualified", "score": 80, "description": "qualified"},
                    {"id": "operational", "score": 100, "description": "operational"}
                ],
                "targets": {"validation": "qualified"},
                "priority_weights": {"validation": 80},
                "cell_overrides": [],
                "weakest_limit": 10
            },
            "evidence_claims": []
        })
    }

    #[test]
    fn runnable_lane_only_reaches_exercised() {
        let tensor = tensor();
        let benchmark_tests = [json!({"id": "smoke"})];
        let contract_evidence = [json!({"id": "contract"})];
        let grade = evaluate(&CellEvaluationInput {
            tensor: &tensor,
            module_id: "engine",
            paradigm: "validation",
            status: "covered",
            required: true,
            benchmark_tests: &benchmark_tests,
            security_tests: &[],
            contract_evidence: &contract_evidence,
        });
        assert_eq!(grade["achieved_grade"], "exercised");
        assert_eq!(grade["gap_steps"], 2);
    }

    #[test]
    fn only_proven_claim_advances_grade() {
        let mut fixture = tensor();
        fixture["evidence_claims"] = json!([
            {
                "id": "partial-depth",
                "status": "partial",
                "grade": "operational",
                "modules": ["engine"],
                "paradigms": ["validation"]
            },
            {
                "id": "verified-depth",
                "status": "proven",
                "grade": "verified",
                "modules": ["engine"],
                "paradigms": ["validation"]
            }
        ]);
        let grade = evaluate(&CellEvaluationInput {
            tensor: &fixture,
            module_id: "engine",
            paradigm: "validation",
            status: "covered",
            required: true,
            benchmark_tests: &[],
            security_tests: &[],
            contract_evidence: &[],
        });
        assert_eq!(grade["achieved_grade"], "verified");
        assert_eq!(grade["next_grade"], "qualified");
    }

    #[test]
    fn weakest_points_are_ranked_by_depth_gap() {
        let mut cells = Map::new();
        cells.insert(
            "engine".to_string(),
            json!({
                "validation": {
                    "required": true,
                    "status": "covered",
                    "maturity": "strong",
                    "maturity_evidence": {"missing_dimensions": []},
                    "evidence_grade": {
                        "achieved_grade": "exercised",
                        "target_grade": "qualified",
                        "next_grade": "verified",
                        "gap_steps": 2,
                        "evidence_score": 40,
                        "target_score": 80,
                        "sources": []
                    }
                }
            }),
        );
        let report = summarize(&tensor(), &cells);
        assert_eq!(report["gap_count"], 1);
        assert_eq!(report["target_met_percent"], 0.0);
        assert_eq!(report["weakest_points"][0]["module_id"], "engine");
        assert_eq!(report["average_evidence_score"], 40.0);
        assert_eq!(report["average_target_score"], 80.0);
        assert_eq!(report["proof_completion_percent"], 50.0);
    }

    #[test]
    fn evidence_claims_must_declare_a_grade() {
        let mut fixture = tensor();
        fixture["evidence_claims"] = json!([{
            "id": "missing-grade",
            "status": "proven",
            "modules": ["engine"],
            "paradigms": ["validation"]
        }]);
        let modules = BTreeSet::from(["engine".to_string()]);
        let paradigms = vec!["validation".to_string()];
        assert!(validate_config(&fixture, &paradigms, &modules).is_err());
    }
}
