use super::{RunnerResult, read_json, string_array, string_field};
use serde_json::{Map, Value, json};
use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::path::Path;

const CRITICALITIES: &[&str] = &["p0", "p1", "p2"];

pub(super) fn validate_config(
    root: &Path,
    tensor: &Value,
    matrix: &Value,
    paradigms: &[String],
    module_ids: &BTreeSet<String>,
) -> RunnerResult<()> {
    let profile = profile(tensor)?;
    required_text(profile, "id")?;
    required_text(profile, "current_checkpoint")?;
    let target_release = required_text(profile, "target_release")?;
    required_text(profile, "enforce_from")?;
    if !matches!(
        string_field(profile, "gate_mode"),
        Some("advisory" | "enforced")
    ) {
        return Err("release_profile.gate_mode must be advisory or enforced".to_string());
    }
    let target_percent = profile
        .get("required_target_met_percent")
        .and_then(Value::as_f64)
        .filter(|value| (0.0..=100.0).contains(value))
        .ok_or_else(|| {
            "release_profile.required_target_met_percent must be between 0 and 100".to_string()
        })?;
    if target_release == "daji 3.0.0" && target_percent != 100.0 {
        return Err("daji 3.0.0 requires every required tensor coordinate to meet target".into());
    }
    if string_array(profile, "criticality_order") != CRITICALITIES {
        return Err("release_profile.criticality_order must be p0, p1, p2".to_string());
    }

    let paradigm_criticality = profile
        .get("paradigm_criticality")
        .and_then(Value::as_object)
        .ok_or_else(|| "release_profile.paradigm_criticality must be an object".to_string())?;
    for paradigm in paradigms {
        validate_criticality(
            paradigm_criticality.get(paradigm).and_then(Value::as_str),
            &format!("paradigm criticality {paradigm}"),
        )?;
    }
    for paradigm in paradigm_criticality.keys() {
        if !paradigms.contains(paradigm) {
            return Err(format!("release profile maps unknown paradigm {paradigm}"));
        }
    }

    let mut coordinates = BTreeSet::new();
    for entry in array_field(profile, "cell_criticality_overrides") {
        let module_id = required_text(entry, "module_id")?;
        let paradigm = required_text(entry, "paradigm")?;
        validate_coordinate(module_id, paradigm, paradigms, module_ids)?;
        validate_required_coordinate(matrix, module_id, paradigm)?;
        validate_criticality(
            string_field(entry, "criticality"),
            &format!("cell criticality {module_id}/{paradigm}"),
        )?;
        if !coordinates.insert((module_id, paradigm)) {
            return Err(format!(
                "duplicate release criticality override {module_id}/{paradigm}"
            ));
        }
    }

    let gates = array_field(profile, "external_gates");
    if target_release == "daji 3.0.0" && gates.is_empty() {
        return Err("daji 3.0.0 release profile requires an external release gate".into());
    }
    let mut gate_ids = BTreeSet::new();
    for gate in gates {
        let id = required_text(gate, "id")?;
        if !gate_ids.insert(id) {
            return Err(format!("duplicate external release gate {id}"));
        }
        let path = required_text(gate, "path")?;
        let pointer = required_text(gate, "pointer")?;
        required_text(gate, "recommended_action")?;
        validate_criticality(string_field(gate, "criticality"), id)?;
        let expected = gate
            .get("expected")
            .filter(|value| is_scalar(value))
            .ok_or_else(|| format!("{id}: expected must be a JSON scalar"))?;
        let source = read_json(root, path)?;
        let actual = source
            .pointer(pointer)
            .ok_or_else(|| format!("{id}: pointer {pointer} does not exist in {path}"))?;
        if !is_scalar(actual) || expected.is_null() {
            return Err(format!("{id}: gate values must be non-null JSON scalars"));
        }
    }
    Ok(())
}

pub(super) fn evaluate(
    root: &Path,
    tensor: &Value,
    cells: &Map<String, Value>,
    structural_ok: bool,
    maturity_ok: bool,
    calibration: &Value,
) -> RunnerResult<Value> {
    let profile = profile(tensor)?;
    let mut coordinate_gaps = calibration
        .get("gaps")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for gap in &mut coordinate_gaps {
        let module_id = string_field(gap, "module_id").unwrap_or_default();
        let paradigm = string_field(gap, "paradigm").unwrap_or_default();
        let criticality = criticality_for(profile, module_id, paradigm);
        let score = gap
            .get("priority_score")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            + criticality_weight(criticality);
        if let Some(object) = gap.as_object_mut() {
            object.insert("criticality".to_string(), json!(criticality));
            object.insert("release_priority_score".to_string(), json!(score));
        }
    }
    coordinate_gaps.sort_by(release_gap_order);

    let criticality_summary = summarize_criticalities(profile, cells);
    let external_gates = evaluate_external_gates(root, profile)?;
    let external_gates_met = external_gates
        .iter()
        .all(|gate| gate.get("met").and_then(Value::as_bool) == Some(true));
    let p0_gap_count = criticality_summary
        .pointer("/p0/gap_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let p0_external_gap_count = external_gates
        .iter()
        .filter(|gate| {
            string_field(gate, "criticality") == Some("p0")
                && gate.get("met").and_then(Value::as_bool) != Some(true)
        })
        .count();
    let target_met_percent = calibration
        .get("target_met_percent")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let required_target = profile
        .get("required_target_met_percent")
        .and_then(Value::as_f64)
        .unwrap_or(100.0);
    let evidence_targets_met = target_met_percent >= required_target;
    let release_ready = structural_ok
        && maturity_ok
        && evidence_targets_met
        && external_gates_met
        && p0_gap_count == 0;
    let status = if release_ready {
        "ready"
    } else if !structural_ok {
        "structurally_blocked"
    } else if p0_gap_count > 0 || p0_external_gap_count > 0 {
        "blocked"
    } else {
        "hardening"
    };
    let blocking_coordinates = coordinate_gaps
        .iter()
        .filter(|gap| string_field(gap, "criticality") == Some("p0"))
        .cloned()
        .collect::<Vec<_>>();

    Ok(json!({
        "profile_id": string_field(profile, "id").unwrap_or_default(),
        "current_checkpoint": string_field(profile, "current_checkpoint").unwrap_or_default(),
        "target_release": string_field(profile, "target_release").unwrap_or_default(),
        "gate_mode": string_field(profile, "gate_mode").unwrap_or("advisory"),
        "enforce_from": string_field(profile, "enforce_from").unwrap_or_default(),
        "status": status,
        "release_claim_allowed": release_ready,
        "criteria": [
            criterion("structural_complete", structural_ok, json!(true), json!(structural_ok)),
            criterion("maturity_complete", maturity_ok, json!(true), json!(maturity_ok)),
            criterion(
                "required_coordinates_at_target",
                evidence_targets_met,
                json!(required_target),
                json!(target_met_percent)
            ),
            criterion("external_release_gates", external_gates_met, json!(true), json!(external_gates_met))
        ],
        "coordinate_gap_count": coordinate_gaps.len(),
        "p0_coordinate_gap_count": p0_gap_count,
        "p0_external_gate_gap_count": p0_external_gap_count,
        "criticality_summary": criticality_summary,
        "blocking_coordinates": blocking_coordinates,
        "planning_queue": coordinate_gaps,
        "external_gates": external_gates
    }))
}

pub(super) fn gate_is_enforced(tensor: &Value) -> bool {
    tensor
        .pointer("/release_profile/gate_mode")
        .and_then(Value::as_str)
        == Some("enforced")
}

fn summarize_criticalities(profile: &Value, cells: &Map<String, Value>) -> Value {
    let mut summary = Map::new();
    for criticality in CRITICALITIES {
        summary.insert(
            (*criticality).to_string(),
            json!({"required_cell_count": 0, "target_met_count": 0, "gap_count": 0, "target_met_percent": 100.0}),
        );
    }
    for (module_id, module_cells) in cells {
        for (paradigm, cell) in module_cells.as_object().into_iter().flatten() {
            if cell.get("required").and_then(Value::as_bool) != Some(true) {
                continue;
            }
            let criticality = criticality_for(profile, module_id, paradigm);
            let entry = summary.get_mut(criticality).and_then(Value::as_object_mut);
            let Some(entry) = entry else { continue };
            increment(entry, "required_cell_count");
            let met = cell
                .pointer("/evidence_grade/gap_steps")
                .and_then(Value::as_u64)
                == Some(0);
            increment(entry, if met { "target_met_count" } else { "gap_count" });
        }
    }
    for entry in summary.values_mut().filter_map(Value::as_object_mut) {
        let required = entry
            .get("required_cell_count")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let met = entry
            .get("target_met_count")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let percent = if required == 0 {
            100.0
        } else {
            met as f64 * 100.0 / required as f64
        };
        entry.insert(
            "target_met_percent".to_string(),
            json!((percent * 10.0).round() / 10.0),
        );
    }
    Value::Object(summary)
}

fn evaluate_external_gates(root: &Path, profile: &Value) -> RunnerResult<Vec<Value>> {
    array_field(profile, "external_gates")
        .iter()
        .map(|gate| {
            let path = required_text(gate, "path")?;
            let pointer = required_text(gate, "pointer")?;
            let source = read_json(root, path)?;
            let actual = source
                .pointer(pointer)
                .ok_or_else(|| format!("external gate pointer {pointer} missing from {path}"))?;
            let expected = gate.get("expected").cloned().unwrap_or(Value::Null);
            Ok(json!({
                "id": string_field(gate, "id").unwrap_or_default(),
                "path": path,
                "pointer": pointer,
                "expected": expected,
                "actual": actual,
                "met": actual == &expected,
                "criticality": string_field(gate, "criticality").unwrap_or("p1"),
                "recommended_action": string_field(gate, "recommended_action").unwrap_or_default()
            }))
        })
        .collect()
}

fn criticality_for<'a>(profile: &'a Value, module_id: &str, paradigm: &str) -> &'a str {
    for entry in array_field(profile, "cell_criticality_overrides") {
        if string_field(entry, "module_id") == Some(module_id)
            && string_field(entry, "paradigm") == Some(paradigm)
        {
            return string_field(entry, "criticality").unwrap_or("p1");
        }
    }
    profile
        .pointer(&format!("/paradigm_criticality/{paradigm}"))
        .and_then(Value::as_str)
        .unwrap_or("p1")
}

fn release_gap_order(left: &Value, right: &Value) -> Ordering {
    let left_criticality = criticality_rank(string_field(left, "criticality").unwrap_or("p2"));
    let right_criticality = criticality_rank(string_field(right, "criticality").unwrap_or("p2"));
    left_criticality
        .cmp(&right_criticality)
        .then_with(|| {
            right
                .get("release_priority_score")
                .and_then(Value::as_u64)
                .cmp(&left.get("release_priority_score").and_then(Value::as_u64))
        })
        .then_with(|| coordinate(left).cmp(&coordinate(right)))
}

fn criticality_rank(criticality: &str) -> usize {
    CRITICALITIES
        .iter()
        .position(|value| *value == criticality)
        .unwrap_or(CRITICALITIES.len())
}

fn criticality_weight(criticality: &str) -> u64 {
    match criticality {
        "p0" => 1_000,
        "p1" => 500,
        _ => 0,
    }
}

fn validate_required_coordinate(
    matrix: &Value,
    module_id: &str,
    paradigm: &str,
) -> RunnerResult<()> {
    let required = matrix
        .pointer(&format!("/required_by_module/{module_id}"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .any(|value| value.as_str() == Some(paradigm));
    if !required {
        return Err(format!(
            "release criticality override {module_id}/{paradigm} must target a required cell"
        ));
    }
    Ok(())
}

fn validate_coordinate(
    module_id: &str,
    paradigm: &str,
    paradigms: &[String],
    module_ids: &BTreeSet<String>,
) -> RunnerResult<()> {
    if !module_ids.contains(module_id) {
        return Err(format!("release profile maps unknown module {module_id}"));
    }
    if !paradigms.contains(&paradigm.to_string()) {
        return Err(format!("release profile maps unknown paradigm {paradigm}"));
    }
    Ok(())
}

fn validate_criticality(value: Option<&str>, context: &str) -> RunnerResult<()> {
    if value.is_some_and(|value| CRITICALITIES.contains(&value)) {
        Ok(())
    } else {
        Err(format!("{context}: criticality must be p0, p1, or p2"))
    }
}

fn profile(tensor: &Value) -> RunnerResult<&Value> {
    tensor
        .get("release_profile")
        .filter(|value| value.is_object())
        .ok_or_else(|| "release_profile must be an object".to_string())
}

fn required_text<'a>(value: &'a Value, key: &str) -> RunnerResult<&'a str> {
    string_field(value, key)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("{key} must be a non-empty string"))
}

fn array_field<'a>(value: &'a Value, key: &str) -> &'a [Value] {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

fn is_scalar(value: &Value) -> bool {
    value.is_boolean() || value.is_number() || value.is_string() || value.is_null()
}

fn criterion(id: &str, met: bool, expected: Value, actual: Value) -> Value {
    json!({"id": id, "met": met, "expected": expected, "actual": actual})
}

fn increment(values: &mut Map<String, Value>, key: &str) {
    let next = values.get(key).and_then(Value::as_u64).unwrap_or(0) + 1;
    values.insert(key.to_string(), json!(next));
}

fn coordinate(value: &Value) -> String {
    format!(
        "{}/{}",
        string_field(value, "module_id").unwrap_or_default(),
        string_field(value, "paradigm").unwrap_or_default()
    )
}

#[cfg(test)]
mod tests {
    use super::{criticality_for, evaluate};
    use serde_json::{Map, json};
    use std::path::Path;

    fn profile() -> serde_json::Value {
        json!({
            "release_profile": {
                "id": "test-release",
                "current_checkpoint": "test",
                "target_release": "test",
                "gate_mode": "advisory",
                "enforce_from": "later",
                "required_target_met_percent": 100.0,
                "criticality_order": ["p0", "p1", "p2"],
                "paradigm_criticality": {"security": "p0", "validation": "p1"},
                "cell_criticality_overrides": [
                    {"module_id": "engine", "paradigm": "validation", "criticality": "p0"}
                ],
                "external_gates": []
            }
        })
    }

    #[test]
    fn cell_criticality_overrides_paradigm_default() {
        let tensor = profile();
        assert_eq!(
            criticality_for(&tensor["release_profile"], "engine", "validation"),
            "p0"
        );
    }

    #[test]
    fn open_p0_coordinate_blocks_release_claim() {
        let tensor = profile();
        let mut cells = Map::new();
        cells.insert(
            "engine".into(),
            json!({
                "validation": {
                    "required": true,
                    "evidence_grade": {"gap_steps": 1}
                }
            }),
        );
        let calibration = json!({
            "target_met_percent": 0.0,
            "gaps": [{
                "module_id": "engine",
                "paradigm": "validation",
                "priority_score": 100
            }]
        });
        let report = evaluate(Path::new("."), &tensor, &cells, true, true, &calibration)
            .expect("release readiness");
        assert_eq!(report["status"], "blocked");
        assert_eq!(report["p0_coordinate_gap_count"], 1);
        assert_eq!(report["release_claim_allowed"], false);
    }
}
