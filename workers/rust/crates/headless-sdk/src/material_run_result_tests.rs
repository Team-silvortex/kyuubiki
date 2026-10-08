use super::*;
use crate::{
    MockHeadlessExecutor, build_template_document, execute_batch_with_executor,
    normalize_workflow_document,
};
use serde_json::json;

fn fixture(study: &MaterialStudyDescriptor) -> HeadlessRunReport {
    let batch = normalize_workflow_document(
        &build_template_document(study.template_id, Some("custom.material-run")).unwrap(),
    )
    .unwrap();
    let mut run = execute_batch_with_executor(&batch, &mut MockHeadlessExecutor, false, false);
    // Synthetic receipts exercise report contracts, not physical execution authority.
    run.mode = "execute:report-fixture".into();
    for step in &mut run.steps {
        step.result_preview
            .as_object_mut()
            .unwrap()
            .remove("preview_only");
    }
    for step in run
        .steps
        .iter_mut()
        .filter(|step| step.action == "result_fetch")
    {
        step.result_preview["result"] = json!({
            "max_temperature": 64.0, "max_heat_flux": 900.0,
            "max_electric_field": 42e6, "max_flux_density": 1.2e-3,
            "max_stress": 90e6, "max_displacement": 0.00022,
            "max_temperature_delta": 110.0
        });
    }
    run
}

fn heat() -> HeadlessRunReport {
    fixture(find_material_study("heat-spreader").unwrap())
}

fn rejected(run: &HeadlessRunReport) {
    let before = serde_json::to_value(run).unwrap();
    assert!(
        extract_result_payloads_from_run(run).is_err(),
        "accepted typed run: {before}"
    );
    assert!(
        extract_material_result_payloads(&before).is_err(),
        "accepted JSON run: {before}"
    );
    assert_eq!(serde_json::to_value(run).unwrap(), before);
}

#[test]
fn failed_or_unfinished_runs_cannot_publish_partial_material_results() {
    for status in [
        "failed",
        "invalid",
        "blocked",
        "cancelled",
        "queued",
        "unknown",
    ] {
        let mut run = heat();
        run.status = status.into();
        rejected(&run);
    }
}

#[test]
fn only_executed_steps_and_consistent_run_headers_admit_retained_results() {
    for status in [
        "failed",
        "blocked",
        "cancelled",
        "dry_run",
        "completed",
        "",
        "future",
    ] {
        let mut run = heat();
        run.steps[2].status = status.into();
        rejected(&run);
    }
    for edit in 0..8 {
        let mut run = heat();
        match edit {
            0 => run.mode = "dry_run".into(),
            1 => run.mode = "execute:".into(),
            2 => run.schema_version = "kyuubiki.headless-execution-run/v99".into(),
            3 => run.validation.ok = false,
            4 => run.executed_step_count -= 1,
            5 => run.steps[2].index = 2,
            6 => run.validation.issues.push("retained contradiction".into()),
            _ => {
                run.blocked_by_confirmation = Some(crate::HeadlessBlockedConfirmation {
                    index: 1,
                    risk: crate::HeadlessRisk::Sensitive,
                })
            }
        }
        rejected(&run);
    }
}

#[test]
fn result_receipts_require_matching_job_identity_and_explicit_object_results() {
    for edit in 0..10 {
        let mut run = heat();
        let step = &mut run.steps[2];
        match edit {
            0 => step.result_preview["job_id"] = json!("unrelated-job"),
            1 => {
                step.result_preview
                    .as_object_mut()
                    .unwrap()
                    .remove("job_id");
            }
            2 => step.payload["jobId"] = json!("contradictory-job"),
            3 => {
                step.result_preview
                    .as_object_mut()
                    .unwrap()
                    .remove("result");
            }
            4 => step.result_preview["result"] = Value::Null,
            5 => step.result_preview["result"] = json!([1, 2, 3]),
            6 => step.result_preview["status"] = json!("solving"),
            7 => {
                step.result_preview["job"] = json!({"job_id":"unrelated-job","status":"completed"})
            }
            8 => step.result_preview["preview_only"] = json!(true),
            _ => {
                step.result_preview["result"] =
                    json!({"$kyuubiki_report_summary":"array","item_count":1000})
            }
        }
        rejected(&run);
    }
}

#[test]
fn every_fixed_study_rejects_wrong_candidate_or_unowned_retained_results() {
    for study in material_study_descriptors() {
        let baseline = fixture(study);
        for edit in 0..7 {
            let mut run = baseline.clone();
            match edit {
                0 => run.steps[0].payload["research"]["candidate_id"] = json!("ghost"),
                1 => run.steps[0].payload["research"]["study"] = json!("unknown-study"),
                2 => {
                    let payload = run.steps[0].payload.clone();
                    run.steps[0].payload = run.steps[3].payload.clone();
                    run.steps[3].payload = payload;
                }
                3 => run.steps[1].result_preview["status"] = json!("solving"),
                4 => run.steps[1].payload["job_id"] = json!("unrelated-job"),
                5 => {
                    let id = run.steps[3].result_preview["job_id"].clone();
                    run.steps[0].result_preview["job_id"] = id;
                }
                _ => run.steps[0].action = "service_health".into(),
            }
            let before = serde_json::to_value(&run).unwrap();
            assert!(
                build_material_report_from_run(study.id, &run).is_err(),
                "study {} accepted edit {edit}",
                study.id
            );
            assert!(build_material_report_from_input(study.id, &before, None).is_err());
            assert_eq!(serde_json::to_value(run).unwrap(), before);
        }
    }
}

#[test]
fn valid_custom_workflow_runs_and_explicit_mock_previews_keep_their_contract() {
    let mut run = heat();
    let expected = extract_result_payloads_from_run(&run).unwrap();
    assert_eq!(expected.len(), 3);
    assert_eq!(
        extract_material_result_payloads(&serde_json::to_value(&run).unwrap()).unwrap(),
        expected
    );
    assert!(build_material_report_from_run("heat-spreader", &run).is_ok());
    run.mode = "execute:mock".into();
    assert!(build_material_report_from_run("heat-spreader", &run).is_ok());
}

#[test]
fn run_shaped_json_cannot_downgrade_to_raw_result_arrays() {
    let original = serde_json::to_value(heat()).unwrap();
    for key in [
        "status",
        "mode",
        "executed_step_count",
        "validation",
        "steps",
    ] {
        let mut value = original.clone();
        value.as_object_mut().unwrap().remove(key);
        assert!(
            extract_material_result_payloads(&value).is_err(),
            "missing {key}"
        );
    }
    let mut value = original.clone();
    value["schema_version"] = json!("kyuubiki.headless-execution-run/v99");
    value["results"] = json!([{ "max_temperature": 1.0 }]);
    assert!(extract_material_result_payloads(&value).is_err());
    value.as_object_mut().unwrap().remove("schema_version");
    assert!(extract_material_result_payloads(&value).is_err());
}

#[test]
fn grouped_receipts_and_study_aliases_preserve_candidate_order() {
    for study in material_study_descriptors() {
        let mut run = fixture(study);
        let original = build_material_report_from_run(study.id, &run).unwrap();
        let ordered = [0, 3, 6, 1, 4, 7, 2, 5, 8];
        run.steps = ordered
            .into_iter()
            .map(|index| run.steps[index].clone())
            .collect();
        for (position, step) in run.steps.iter_mut().enumerate() {
            step.index = position + 1;
            if step.payload.get("research").is_some() {
                step.payload["research"]["study"] = json!(study.aliases[0]);
            }
        }
        assert_eq!(
            build_material_report_from_run(study.aliases[0], &run).unwrap(),
            original
        );
        assert_eq!(
            build_material_report_from_input(study.id, &serde_json::to_value(run).unwrap(), None)
                .unwrap(),
            original
        );
    }
}

#[test]
fn raw_result_reports_remain_caller_owned_and_run_optimization_is_preserved() {
    let run = heat();
    let payloads = extract_result_payloads_from_run(&run).unwrap();
    let base = build_material_report("heat-spreader", &payloads).unwrap();
    let profile: MaterialOptimizationProfile =
        serde_json::from_value(base["optimization"].clone()).unwrap();
    let original = serde_json::to_value(&run).unwrap();
    for value in [
        json!(payloads),
        json!({"results":payloads}),
        json!({"result_payloads":payloads}),
        original.clone(),
    ] {
        assert_eq!(
            build_material_report_from_input("heat-spreader", &value, Some(profile.clone()))
                .unwrap(),
            base
        );
    }
    assert_eq!(serde_json::to_value(run).unwrap(), original);
}

#[test]
fn explicit_returned_material_identity_cannot_contradict_its_candidate_job() {
    for study in material_study_descriptors() {
        let mut run = fixture(study);
        run.steps[2].result_preview["result"]["research"] = json!({
            "candidate_id": run.steps[0].payload["research"]["candidate_id"],
            "study": study.aliases[0]
        });
        assert!(build_material_report_from_run(study.id, &run).is_ok());
        for research in [
            json!({"candidate_id":"ghost"}),
            json!({"candidate_id":null}),
            json!({"study":"unknown-study"}),
            json!([]),
        ] {
            run.steps[2].result_preview["result"]["research"] = research;
            let error = build_material_report_from_run(study.id, &run).unwrap_err();
            assert!(error.contains("returned research metadata contradicts"));
            assert!(
                build_material_report_from_input(
                    study.id,
                    &serde_json::to_value(&run).unwrap(),
                    None
                )
                .is_err()
            );
        }
    }
}
