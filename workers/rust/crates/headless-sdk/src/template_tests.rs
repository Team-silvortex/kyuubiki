use crate::{
    HeadlessRuntimeStyle, build_template_document, find_action_contract, list_templates,
    normalize_workflow_document, validate_batch,
};
use std::collections::BTreeSet;

#[test]
fn template_catalog_uses_unique_ids() {
    let ids = list_templates()
        .iter()
        .map(|template| template.id)
        .collect::<Vec<_>>();
    let unique = ids.iter().copied().collect::<BTreeSet<_>>();
    assert_eq!(ids.len(), unique.len(), "duplicate template ids detected");
}

#[test]
fn every_template_builds_normalizes_and_validates() {
    for template in list_templates() {
        let document = build_template_document(template.id, None)
            .unwrap_or_else(|| panic!("failed to build template {}", template.id));
        assert_eq!(document.workflow.id, format!("template.{}", template.id));
        assert!(
            !document.workflow.steps.is_empty(),
            "template {} has no workflow steps",
            template.id
        );

        let batch = normalize_workflow_document(&document)
            .unwrap_or_else(|error| panic!("failed to normalize template {}: {}", template.id, error));
        let report = validate_batch(&batch);
        assert!(report.ok, "template {} validation issues: {:?}", template.id, report.issues);
    }
}

#[test]
fn template_runtime_style_matches_policy_summary() {
    for template in list_templates() {
        let document = build_template_document(template.id, None).unwrap();
        let batch = normalize_workflow_document(&document).unwrap();
        let report = validate_batch(&batch);
        let policy = report
            .policy
            .unwrap_or_else(|| panic!("missing policy for template {}", template.id));
        assert_eq!(
            policy.recommended_runtime,
            template.runtime_style,
            "template {} runtime style drifted from policy",
            template.id
        );
    }
}

#[test]
fn direct_service_templates_keep_job_follow_up_chain() {
    for template in list_templates()
        .iter()
        .filter(|template| {
            template.runtime_style == HeadlessRuntimeStyle::ServiceOnly
                && template.tags.contains(&"direct")
        })
    {
        let document = build_template_document(template.id, None).unwrap();
        let actions = document
            .workflow
            .steps
            .iter()
            .map(|step| step.action.as_str())
            .collect::<Vec<_>>();
        assert!(
            actions.len() >= 3,
            "template {} should keep solve/wait/result structure",
            template.id
        );
        assert_eq!(actions[actions.len() - 2], "job_wait", "template {}", template.id);
        assert_eq!(
            actions[actions.len() - 1],
            "result_fetch",
            "template {}",
            template.id
        );
    }
}

#[test]
fn every_template_step_uses_registered_action_contracts() {
    for template in list_templates() {
        let document = build_template_document(template.id, None).unwrap();
        for step in &document.workflow.steps {
            assert!(
                find_action_contract(&step.action).is_some(),
                "template {} references unsupported action {}",
                template.id,
                step.action
            );
        }
    }
}
