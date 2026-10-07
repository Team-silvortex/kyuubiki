use super::*;
use crate::{build_template_document, normalize_workflow_document};
use serde_json::json;

fn batch(study: &MaterialStudyDescriptor) -> HeadlessExecutionBatch {
    normalize_workflow_document(&build_template_document(study.template_id, None).unwrap()).unwrap()
}

fn grouped(study: &MaterialStudyDescriptor, wait: bool) -> HeadlessExecutionBatch {
    let mut batch = batch(study);
    let original = std::mem::take(&mut batch.steps);
    let order: &[usize] = if wait {
        &[0, 3, 6, 1, 4, 7, 2, 5, 8]
    } else {
        &[0, 3, 6, 2, 5, 8]
    };
    batch.steps = order.iter().map(|index| original[*index].clone()).collect();
    for (position, step) in batch.steps.iter_mut().enumerate() {
        step.index = position + 1;
        if position >= 3 {
            step.payload["job_id"] = json!(format!(
                "{{{{steps.{}.result.job_id}}}}",
                (position - 3) % 3 + 1
            ));
        }
    }
    batch
}

#[test]
fn every_builtin_material_report_requires_complete_result_cardinality_before_execution() {
    for study in MATERIAL_STUDIES {
        let original = batch(study);
        for retained in [0, 2, 3, 6, 8] {
            let mut incomplete = original.clone();
            incomplete.steps.truncate(retained);
            assert!(
                validate_material_report_compatibility(study.id, &incomplete).is_err(),
                "study {} accepted only {retained} steps",
                study.id
            );
        }
        let mut extra = original.clone();
        let mut repeated = original.steps[8].clone();
        repeated.index = 10;
        extra.steps.push(repeated);
        assert!(
            validate_material_report_compatibility(study.id, &extra)
                .unwrap_err()
                .contains("result_fetch")
        );
    }
}

#[test]
fn canonical_and_grouped_candidate_chains_accept_aliases_without_mutating_inputs() {
    for study in MATERIAL_STUDIES {
        for candidate_batch in [batch(study), grouped(study, true)] {
            let original = serde_json::to_value(&candidate_batch).unwrap();
            for key in std::iter::once(study.id).chain(study.aliases.iter().copied()) {
                validate_material_report_compatibility(key, &candidate_batch).unwrap();
            }
            assert_eq!(serde_json::to_value(candidate_batch).unwrap(), original);
        }
    }
}

#[test]
fn material_result_order_and_unique_candidate_ownership_are_not_inferred_from_counts() {
    for study in MATERIAL_STUDIES {
        let mut swapped = grouped(study, true);
        swapped.steps.swap(6, 7);
        swapped.steps[6].index = 7;
        swapped.steps[7].index = 8;
        assert!(
            validate_material_report_compatibility(study.id, &swapped)
                .unwrap_err()
                .contains("candidate order")
        );
        let mut repeated = grouped(study, true);
        repeated.steps[7].payload["job_id"] = json!("{{steps.1.result.job_id}}");
        assert!(
            validate_material_report_compatibility(study.id, &repeated)
                .unwrap_err()
                .contains("candidate order")
        );
        let mut literal = grouped(study, true);
        literal.steps[6].payload["job_id"] = json!("retained-job-not-owned-by-this-plan");
        assert!(
            validate_material_report_compatibility(study.id, &literal)
                .unwrap_err()
                .contains("candidate source")
        );
    }
}

#[test]
fn every_candidate_must_have_the_matching_solver_and_unambiguous_research_identity() {
    for study in MATERIAL_STUDIES {
        for (path, value) in [
            ("/research/candidate_id", json!("unknown-candidate")),
            ("/research/study", json!("other-study")),
            ("/research/candidate_id", Value::Null),
        ] {
            let mut edited = batch(study);
            *edited.steps[0].payload.pointer_mut(path).unwrap() = value;
            assert!(
                validate_material_report_compatibility(study.id, &edited).is_err(),
                "accepted {path}"
            );
        }
        let mut duplicated = batch(study);
        duplicated.steps[3].payload["research"]["candidate_id"] =
            duplicated.steps[0].payload["research"]["candidate_id"].clone();
        assert!(
            validate_material_report_compatibility(study.id, &duplicated)
                .unwrap_err()
                .contains("duplicate candidate")
        );
        let mut wrong_solver = batch(study);
        wrong_solver.steps[0].action = "service_health".into();
        assert!(validate_material_report_compatibility(study.id, &wrong_solver).is_err());
    }
}

#[test]
fn a_completed_job_wait_gate_must_precede_each_report_result_fetch() {
    for study in MATERIAL_STUDIES {
        let no_wait = grouped(study, false);
        assert!(
            validate_material_report_compatibility(study.id, &no_wait)
                .unwrap_err()
                .contains("job_wait")
        );
        let mut wrong_wait = grouped(study, true);
        wrong_wait.steps[3].payload["job_id"] = json!("some-other-job");
        assert!(
            validate_material_report_compatibility(study.id, &wrong_wait)
                .unwrap_err()
                .contains("job_wait")
        );
    }
}

#[test]
fn readbacks_can_follow_wait_bindings_and_native_job_id_aliases() {
    for study in MATERIAL_STUDIES {
        let mut bridged = grouped(study, true);
        for (candidate, step) in bridged.steps[6..].iter_mut().enumerate() {
            let payload = step.payload.as_object_mut().unwrap();
            let binding = json!(format!(
                "  {{{{ steps.{}.result.job_id }}}}  ",
                candidate + 4
            ));
            payload.insert("job_id".into(), binding.clone());
            payload.insert("jobId".into(), binding);
        }
        validate_material_report_compatibility(study.id, &bridged).unwrap();
        bridged.steps[6].payload["job_id"] = json!("{{steps.2.result.job_id}}");
        assert!(
            validate_material_report_compatibility(study.id, &bridged)
                .unwrap_err()
                .contains("job_id aliases")
        );
        bridged.steps[6]
            .payload
            .as_object_mut()
            .unwrap()
            .remove("job_id");
        assert!(
            validate_material_report_compatibility(study.id, &bridged)
                .unwrap_err()
                .contains("missing required payload key job_id")
        );
    }
}

#[test]
fn dielectric_input_checks_cannot_be_satisfied_by_an_empty_element_list() {
    let study = find_material_study("dielectric-screening").unwrap();
    let mut edited = batch(study);
    edited.steps[0].payload["model"]["elements"] = json!([]);
    assert!(
        validate_material_report_compatibility(study.id, &edited)
            .unwrap_err()
            .contains("non-empty model.elements")
    );
}
