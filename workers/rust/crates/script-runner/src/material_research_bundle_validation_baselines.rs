use serde_json::{Value, json};

pub(crate) fn validation_baseline_refs(report: &Value) -> Vec<Value> {
    let mut refs = vec![json!({
        "baseline_id": report.pointer("/optimization/id").cloned().unwrap_or(Value::Null),
        "kind": "built_in_screening_fixture",
        "status": "retained",
        "scope": "internal deterministic solver fixture; external calibration still required",
    })];
    let candidates = report
        .get("candidates")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default();
    append_analytic_baseline(
        &mut refs,
        candidates,
        AnalyticBaseline {
            field: "electrostatic_cross_validation",
            schema: "kyuubiki.composite-electrostatic-cross-validation/v1",
            baseline_id: "material.composite_thermo_electric_panel.layered_dielectric_closed_form.v1",
            physics: "electrostatic",
            scope: "independent layered-dielectric displacement-continuity cross-check; not external solver calibration",
        },
    );
    append_mesh_baseline(
        &mut refs,
        candidates,
        MeshBaseline {
            field: "electrostatic_mesh_convergence",
            schema: "kyuubiki.composite-electrostatic-mesh-convergence/v1",
            baseline_id: "material.composite_thermo_electric_panel.electrostatic_mesh_convergence.v1",
            physics: "electrostatic",
            scope: "real Rust solver h-refinement for the electrostatic subproblem; coupled-field convergence remains required",
        },
    );
    append_analytic_baseline(
        &mut refs,
        candidates,
        AnalyticBaseline {
            field: "heat_cross_validation",
            schema: "kyuubiki.composite-heat-cross-validation/v1",
            baseline_id: "material.composite_thermo_electric_panel.layered_thermal_resistance.v1",
            physics: "heat",
            scope: "independent layered thermal-resistance cross-check; not external solver calibration",
        },
    );
    append_mesh_baseline(
        &mut refs,
        candidates,
        MeshBaseline {
            field: "heat_mesh_convergence",
            schema: "kyuubiki.composite-heat-mesh-convergence/v1",
            baseline_id: "material.composite_thermo_electric_panel.heat_mesh_convergence.v1",
            physics: "heat",
            scope: "real Rust solver h-refinement for the heat subproblem; structural and coupled-field convergence remain required",
        },
    );
    refs
}

struct AnalyticBaseline {
    field: &'static str,
    schema: &'static str,
    baseline_id: &'static str,
    physics: &'static str,
    scope: &'static str,
}

fn append_analytic_baseline(
    refs: &mut Vec<Value>,
    candidates: &[Value],
    baseline: AnalyticBaseline,
) {
    let Some(validations) = candidate_validations(candidates, baseline.field) else {
        return;
    };
    if validations.is_empty()
        || !validations.iter().all(|validation| {
            valid_status_schema(validation, baseline.schema)
                && finite_non_negative(validation, "relative_error").is_some()
        })
    {
        return;
    }
    let max_relative_error = max_field(&validations, "relative_error");
    refs.push(json!({
        "baseline_id": baseline.baseline_id,
        "kind": "analytic_closed_form",
        "physics": baseline.physics,
        "status": "pass",
        "schema_version": baseline.schema,
        "candidate_count": validations.len(),
        "max_relative_error": max_relative_error,
        "scope": baseline.scope,
    }));
}

struct MeshBaseline {
    field: &'static str,
    schema: &'static str,
    baseline_id: &'static str,
    physics: &'static str,
    scope: &'static str,
}

fn append_mesh_baseline(refs: &mut Vec<Value>, candidates: &[Value], baseline: MeshBaseline) {
    let analytic_baseline_passed = refs.iter().any(|reference| {
        reference.get("kind").and_then(Value::as_str) == Some("analytic_closed_form")
            && reference.get("physics").and_then(Value::as_str) == Some(baseline.physics)
            && reference.get("status").and_then(Value::as_str) == Some("pass")
    });
    if !analytic_baseline_passed {
        return;
    }
    let Some(validations) = candidate_validations(candidates, baseline.field) else {
        return;
    };
    if validations.is_empty()
        || !validations.iter().all(|validation| {
            valid_status_schema(validation, baseline.schema)
                && validation
                    .get("samples")
                    .and_then(Value::as_array)
                    .is_some_and(|samples| samples.len() == 4)
                && finite_non_negative(validation, "max_analytic_relative_error").is_some()
                && finite_non_negative(validation, "finest_pair_relative_change").is_some()
        })
    {
        return;
    }
    refs.push(json!({
        "baseline_id": baseline.baseline_id,
        "kind": "mesh_convergence",
        "physics": baseline.physics,
        "status": "pass",
        "schema_version": baseline.schema,
        "candidate_count": validations.len(),
        "refinement_levels": [1, 2, 4, 8],
        "max_analytic_relative_error": max_field(&validations, "max_analytic_relative_error"),
        "max_finest_pair_relative_change": max_field(&validations, "finest_pair_relative_change"),
        "scope": baseline.scope,
    }));
}

fn candidate_validations<'a>(candidates: &'a [Value], field: &str) -> Option<Vec<&'a Value>> {
    candidates
        .iter()
        .map(|candidate| candidate.get(field))
        .collect()
}

fn valid_status_schema(value: &Value, schema: &str) -> bool {
    value.get("status").and_then(Value::as_str) == Some("pass")
        && value.get("schema_version").and_then(Value::as_str) == Some(schema)
}

fn max_field(values: &[&Value], field: &str) -> f64 {
    values
        .iter()
        .filter_map(|value| finite_non_negative(value, field))
        .fold(0.0_f64, f64::max)
}

fn finite_non_negative(value: &Value, field: &str) -> Option<f64> {
    value
        .get(field)
        .and_then(Value::as_f64)
        .filter(|number| number.is_finite() && *number >= 0.0)
}

#[cfg(test)]
mod tests {
    use super::validation_baseline_refs;
    use serde_json::{Value, json};

    #[test]
    fn promotes_complete_analytic_and_mesh_evidence_by_physics() {
        let report = json!({
            "optimization": { "id": "composite-screening" },
            "candidates": [candidate(2.0e-16), candidate(4.0e-16)]
        });

        let refs = validation_baseline_refs(&report);

        assert_eq!(refs.len(), 5);
        assert_eq!(refs[1]["physics"], "electrostatic");
        assert_eq!(refs[2]["kind"], "mesh_convergence");
        assert_eq!(refs[3]["physics"], "heat");
        assert_eq!(refs[4]["max_finest_pair_relative_change"], 2.0e-15);
    }

    #[test]
    fn refuses_incomplete_candidate_evidence() {
        let mut incomplete = candidate(2.0e-16);
        incomplete
            .as_object_mut()
            .expect("candidate")
            .remove("heat_mesh_convergence");
        let report = json!({
            "candidates": [candidate(2.0e-16), incomplete]
        });

        let refs = validation_baseline_refs(&report);

        assert_eq!(refs.len(), 4);
        assert!(
            !refs
                .iter()
                .any(|item| { item["kind"] == "mesh_convergence" && item["physics"] == "heat" })
        );
    }

    fn candidate(error: f64) -> Value {
        json!({
            "electrostatic_cross_validation": {
                "schema_version": "kyuubiki.composite-electrostatic-cross-validation/v1",
                "status": "pass",
                "relative_error": error
            },
            "electrostatic_mesh_convergence": mesh(
                "kyuubiki.composite-electrostatic-mesh-convergence/v1"
            ),
            "heat_cross_validation": {
                "schema_version": "kyuubiki.composite-heat-cross-validation/v1",
                "status": "pass",
                "relative_error": error
            },
            "heat_mesh_convergence": mesh(
                "kyuubiki.composite-heat-mesh-convergence/v1"
            )
        })
    }

    fn mesh(schema: &str) -> Value {
        json!({
            "schema_version": schema,
            "status": "pass",
            "samples": [{}, {}, {}, {}],
            "max_analytic_relative_error": 5.0e-15,
            "finest_pair_relative_change": 2.0e-15
        })
    }
}
