use crate::run_solve_operator;
use crate::workflow_executor::{run_extract_operator, run_transform_operator};
use crate::workflow_reporting::export_summary_json;
use serde_json::{Value, json};

const DOMAINS: &[&str] = &[
    "structural",
    "thermal",
    "acoustic",
    "modal",
    "dynamic",
    "electrostatic",
    "magnetostatic",
    "cfd",
    "transport",
];

fn partial_result(contact: bool) -> Value {
    let mut model = json!({
        "nodes": [{"id":"base","x":0.0,"fix_x":true,"load_x":0.0},
                  {"id":"tip","x":1.0,"fix_x":false,"load_x":2.0}],
        "elements": [{"id":"spring","node_i":0,"node_j":1,"stiffness":1.0,
                      "cubic_stiffness":if contact { 0.0 } else { 1.0 }}],
        "load_steps":if contact { 4 } else { 1 }, "max_iterations":1, "tolerance":1e-12
    });
    let family = if contact {
        "contact_gap_1d"
    } else {
        "nonlinear_spring_1d"
    };
    if contact {
        model["contacts"] = json!([{"id":"stop","node":1,"gap":1.0,"normal_stiffness":10.0}]);
    }
    let result = run_solve_operator(&format!("solve.{family}"), model).unwrap();
    assert_eq!(result["converged"], false);
    assert_eq!(result["residual_norm"], 0.0);
    result
}

fn guard_config() -> Value {
    json!({"rules":[{"field":"max_displacement","comparison":"gt","threshold":10.0}]})
}

fn quality_config() -> Value {
    json!({"enabled_terms":["max_displacement"],"targets":{"max_displacement":10.0}})
}

fn pair_config() -> Value {
    json!({"criteria":[{"field":"max_displacement","goal":"min"}]})
}

fn rejected(result: Result<Value, String>, operator: &str, path: &str) -> String {
    let error = result.expect_err("incomplete result must not produce a successful assessment");
    assert!(error.contains(operator), "{error}");
    assert!(error.contains(path), "{error}");
    error
}

#[test]
fn guard_rejects_uncommitted_spring_despite_zero_displacement_and_residual() {
    let operator = "transform.evaluate_structural_guard";
    let error = rejected(
        run_transform_operator(operator, partial_result(false), guard_config()),
        operator,
        "payload.converged",
    );
    assert!(error.contains("achieved_load_factor=0"), "{error}");
}

#[test]
fn quality_rejects_committed_but_incomplete_contact_result() {
    let operator = "transform.score_structural_quality";
    let error = rejected(
        run_transform_operator(operator, partial_result(true), quality_config()),
        operator,
        "payload.converged",
    );
    assert!(error.contains("achieved_load_factor=0.5"), "{error}");
}

#[test]
fn pair_benchmark_rejects_either_incomplete_candidate() {
    let operator = "transform.benchmark_structural_pair";
    for side in ["left", "right"] {
        let mut payload = json!({"left":{"max_displacement":2.0},"right":{"max_displacement":3.0}});
        payload[side] = partial_result(false);
        rejected(
            run_transform_operator(operator, payload, pair_config()),
            operator,
            &format!("payload.{side}.converged"),
        );
    }
}

#[test]
fn every_domain_guard_rejects_explicit_nonconvergence() {
    for domain in DOMAINS {
        let operator = format!("transform.evaluate_{domain}_guard");
        rejected(
            run_transform_operator(
                &operator,
                json!({"converged":false,"max_displacement":0.0}),
                guard_config(),
            ),
            &operator,
            "payload.converged",
        );
    }
}

#[test]
fn every_domain_quality_score_rejects_explicit_nonconvergence() {
    for domain in DOMAINS {
        let operator = format!("transform.score_{domain}_quality");
        rejected(
            run_transform_operator(
                &operator,
                json!({"converged":false,"max_displacement":0.0}),
                quality_config(),
            ),
            &operator,
            "payload.converged",
        );
    }
}

#[test]
fn every_domain_pair_rejects_explicit_nonconvergence() {
    for domain in DOMAINS {
        let pair_domain = if *domain == "thermal" {
            "coupled_heat"
        } else {
            domain
        };
        let operator = format!("transform.benchmark_{pair_domain}_pair");
        rejected(
            run_transform_operator(
                &operator,
                json!({
                    "left":{"converged":false,"max_displacement":0.0},
                    "right":{"max_displacement":1.0}
                }),
                pair_config(),
            ),
            &operator,
            "payload.left.converged",
        );
    }
}

#[test]
fn invalid_convergence_markers_do_not_fall_back_to_success() {
    for marker in [Value::Null, json!("false"), json!(1), json!([]), json!({})] {
        for operator in [
            "transform.evaluate_structural_guard",
            "transform.score_structural_quality",
        ] {
            let mut config = guard_config();
            config["enabled_terms"] = json!(["max_displacement"]);
            let error = rejected(
                run_transform_operator(
                    operator,
                    json!({"converged":marker,"max_displacement":0.0}),
                    config,
                ),
                operator,
                "payload.converged",
            );
            assert!(error.contains("boolean"), "{error}");
        }
    }
}

#[test]
fn nested_material_stability_failure_cannot_be_overridden_by_outer_success() {
    let operator = "transform.score_structural_quality";
    rejected(
        run_transform_operator(
            operator,
            json!({
                "converged":true,"max_displacement":0.0,
                "stability_result":{"converged":false}
            }),
            quality_config(),
        ),
        operator,
        "payload.stability_result.converged",
    );
}

#[test]
fn malformed_stability_result_is_not_ignored() {
    let operator = "transform.score_structural_quality";
    for value in [Value::Null, json!(false), json!([]), json!("ready")] {
        rejected(
            run_transform_operator(
                operator,
                json!({"max_displacement":0.0,"stability_result":value}),
                quality_config(),
            ),
            operator,
            "payload.stability_result",
        );
    }
}

#[test]
fn legacy_metrics_and_explicit_success_keep_their_existing_quality_scores() {
    let operator = "transform.score_structural_quality";
    let baseline =
        run_transform_operator(operator, json!({"max_displacement":1.0}), quality_config())
            .unwrap();
    assert_eq!(baseline["structural_quality_ready"], true);
    for status in [
        json!({"converged":true}),
        json!({"stability_result":{"converged":true}}),
    ] {
        let mut payload = json!({"max_displacement":1.0});
        payload
            .as_object_mut()
            .unwrap()
            .extend(status.as_object().unwrap().clone());
        assert_eq!(
            run_transform_operator(operator, payload, quality_config()).unwrap(),
            baseline
        );
    }
}

#[test]
fn result_admission_does_not_scan_failed_trials_or_assume_unit_load_targets() {
    let operator = "transform.score_structural_quality";
    for factor in [0.5, 2.5] {
        let quality = run_transform_operator(
            operator,
            json!({
                "converged":true,"achieved_load_factor":factor,"max_displacement":1.0,
                "steps":[{"converged":false}],
                "metadata":{"converged":false},
                "stability_result":{"converged":true,"steps":[{"converged":false}]}
            }),
            quality_config(),
        )
        .unwrap();
        assert_eq!(quality["structural_quality_ready"], true);
    }
}

#[test]
fn extractors_cannot_erase_nonconvergence_before_quality_scoring() {
    for (operator, config) in [
        (
            "extract.result_summary",
            json!({"fields":["max_displacement"]}),
        ),
        (
            "extract.field_statistics",
            json!({"source":"nodes","field":"ux"}),
        ),
        (
            "extract.field_hotspots",
            json!({"source":"nodes","field":"ux","threshold":0.0}),
        ),
    ] {
        rejected(
            run_extract_operator(operator, partial_result(true), config),
            operator,
            "payload.converged",
        );
    }
}

#[test]
fn renaming_summary_fields_cannot_strip_nonconvergence() {
    let operator = "transform.normalize_summary_fields";
    for copy_unmapped in [false, true] {
        rejected(
            run_transform_operator(
                operator,
                partial_result(true),
                json!({
                    "copy_unmapped":copy_unmapped,
                    "rules":[{"source":"max_displacement","target":"peak_displacement"}]
                }),
            ),
            operator,
            "payload.converged",
        );
    }
}

#[test]
fn summary_pair_transforms_reject_either_failed_source() {
    for operator in [
        "transform.merge_summary_pair",
        "transform.compare_summary_pair",
        "transform.validate_summary_tolerance",
    ] {
        for side in ["left", "right"] {
            let mut payload =
                json!({"left":{"max_displacement":0.0},"right":{"max_displacement":0.0}});
            payload[side]["converged"] = json!(false);
            rejected(
                run_transform_operator(operator, payload, json!({"fields":["max_displacement"]})),
                operator,
                &format!("payload.{side}.converged"),
            );
        }
    }
}

#[test]
fn summary_collections_cannot_aggregate_or_select_failed_candidates() {
    for operator in [
        "transform.aggregate_summary_collection",
        "transform.select_best_summary",
    ] {
        rejected(
            run_transform_operator(
                operator,
                json!({
                    "failed":{"max_displacement":0.0,"converged":false},
                    "healthy":{"max_displacement":1.0}
                }),
                pair_config(),
            ),
            operator,
            "payload.failed.converged",
        );
    }
}

fn declared_quality() -> Value {
    json!({"structural_quality_score":0.0,"structural_quality_ready":true,
           "structural_quality_grade":"excellent"})
}

#[test]
fn composite_objective_rejects_nonconvergence_in_envelope_or_quality_source() {
    let operator = "transform.compose_quality_objective";
    for mut summary in [
        declared_quality(),
        json!({
            "validation_contract":"kyuubiki.summary_tolerance_validation/v1",
            "validation_passed":true,"validation_checked_field_count":1,
            "validation_failed_field_count":0,"validation_missing_field_count":0
        }),
    ] {
        let accepted = run_transform_operator(
            operator,
            json!({"qualities":{"structure":summary}}),
            json!({}),
        )
        .unwrap();
        assert_eq!(accepted["composite_quality_ready"], true);
        summary["converged"] = json!(false);
        rejected(
            run_transform_operator(
                operator,
                json!({"qualities":{"structure":summary}}),
                json!({}),
            ),
            operator,
            "qualities.structure.converged",
        );
    }
    rejected(
        run_transform_operator(
            operator,
            json!({"converged":false,"qualities":{"structure":declared_quality()}}),
            json!({}),
        ),
        operator,
        "payload.converged",
    );
}

#[test]
fn quality_ranking_reports_rejected_partial_candidate_instead_of_selecting_it() {
    let mut failed = declared_quality();
    failed["converged"] = json!(false);
    let ranking = run_transform_operator(
        "transform.rank_quality_candidates",
        json!({"candidates":{
            "failed":{"qualities":{"structure":failed}},
            "healthy":{"qualities":{"structure":declared_quality()}}
        }}),
        json!({}),
    )
    .unwrap();
    assert_eq!(ranking["best_candidate_id"], "healthy");
    assert_eq!(ranking["ranking_complete"], false);
    assert_eq!(ranking["rejected_candidate_count"], 1);
    assert_eq!(ranking["rejected_candidates"][0]["candidate_id"], "failed");
    let next = run_transform_operator(
        "transform.prepare_quality_next_round_request",
        ranking,
        json!({}),
    )
    .unwrap();
    assert_eq!(next["action"], "replan");
    rejected(
        run_transform_operator(
            "transform.rank_quality_candidates",
            json!({"candidates":{"failed":{"qualities":{"structure":failed}}}}),
            json!({}),
        ),
        "transform.rank_quality_candidates",
        "qualities.structure.converged",
    );
}

#[test]
fn incomplete_raw_results_remain_exportable_for_diagnosis() {
    let result = partial_result(true);
    let exported = export_summary_json(result.clone()).unwrap();
    let restored: Value = serde_json::from_str(exported["content"].as_str().unwrap()).unwrap();
    assert_eq!(restored, result);
    assert_eq!(restored["achieved_load_factor"], 0.5);
}
