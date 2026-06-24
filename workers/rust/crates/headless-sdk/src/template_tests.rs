use crate::{
    HeadlessRuntimeStyle, build_execution_plan, build_template_document,
    dielectric_screening_candidates, find_action_contract, heat_spreader_screening_candidates,
    list_template_categories, list_templates, normalize_workflow_document, search_templates,
    structural_panel_screening_candidates, suggest_template_details, suggest_templates,
    validate_batch,
};
use std::collections::{BTreeMap, BTreeSet};

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
fn template_document_exports_expected_header_and_snapshot() {
    let template = list_templates()
        .iter()
        .find(|template| template.id == "direct_plane_quad")
        .expect("direct_plane_quad template should exist");
    let document = build_template_document(template.id, None).unwrap();
    let snapshot = document
        .template
        .as_ref()
        .expect("template document should include snapshot metadata");

    assert_eq!(document.schema_version, "kyuubiki.headless-workflow/v1");
    assert_eq!(document.exported_at, "1970-01-01T00:00:00.000Z");
    assert_eq!(document.language, "en");
    assert_eq!(document.workflow.id, "template.direct_plane_quad");
    assert_eq!(snapshot.id, template.id);
    assert_eq!(snapshot.title, template.title);
    assert_eq!(snapshot.description, template.description);
    assert_eq!(snapshot.runtime_style, template.runtime_style);
    assert_eq!(snapshot.category, template.category);
    assert_eq!(snapshot.tags, template.tags);
}

#[test]
fn template_document_accepts_custom_workflow_id_override() {
    let document = build_template_document("workflow_submit_monitor", Some("custom.workflow"))
        .expect("workflow_submit_monitor template should build");
    assert_eq!(document.workflow.id, "custom.workflow");
}

#[test]
fn template_category_index_is_sorted_and_unique() {
    assert_eq!(
        list_template_categories(),
        vec![
            "browser",
            "electromagnetic",
            "hybrid",
            "materials",
            "mechanical",
            "mesh",
            "orchestration",
            "solver",
            "thermal",
            "thermo_mechanical",
        ]
    );
}

#[test]
fn search_templates_supports_runtime_category_and_tag_filters() {
    let mechanical = search_templates(None, Some("mechanical"), None, None);
    assert_eq!(mechanical.len(), 12);

    let browser = search_templates(
        Some(HeadlessRuntimeStyle::BrowserOnly),
        Some("browser"),
        Some("snapshot"),
        None,
    );
    assert_eq!(browser.len(), 1);
    assert_eq!(browser[0].id, "browser_capture_review");
}

#[test]
fn search_templates_matches_query_against_actions_and_metadata() {
    let matches = search_templates(None, None, None, Some("electrostatic result_fetch"));
    let ids = matches
        .iter()
        .map(|template| template.id)
        .collect::<Vec<_>>();
    assert_eq!(
        ids,
        vec![
            "direct_electrostatic_quad",
            "direct_electrostatic_triangle",
            "material_dielectric_screening"
        ]
    );
}

#[test]
fn search_templates_ranks_closest_query_first() {
    let matches = search_templates(None, None, None, Some("spring 3d"));
    let ids = matches
        .iter()
        .map(|template| template.id)
        .collect::<Vec<_>>();
    assert_eq!(ids.first().copied(), Some("direct_spring_3d"));
}

#[test]
fn suggest_templates_returns_best_fuzzy_candidates() {
    let suggestions = suggest_templates("browser poll", 3);
    let ids = suggestions
        .iter()
        .map(|template| template.id)
        .collect::<Vec<_>>();
    assert_eq!(ids.first().copied(), Some("browser_submit_then_poll"));
    assert!(ids.contains(&"browser_capture_review"));

    let underscore_suggestions = suggest_templates("browser_poll", 3);
    assert_eq!(
        underscore_suggestions.first().map(|template| template.id),
        Some("browser_submit_then_poll")
    );
}

#[test]
fn suggest_template_details_exposes_scores_and_matched_terms() {
    let suggestions = suggest_template_details("spring 3d", 3);
    let first = suggestions
        .first()
        .expect("expected at least one suggestion");
    assert_eq!(first.id, "direct_spring_3d");
    assert!(first.score > 0);
    assert!(first.matched_terms.iter().any(|term| term == "spring"));
    assert!(first.matched_terms.iter().any(|term| term == "3d"));
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

        let batch = normalize_workflow_document(&document).unwrap_or_else(|error| {
            panic!("failed to normalize template {}: {}", template.id, error)
        });
        let report = validate_batch(&batch);
        assert!(
            report.ok,
            "template {} validation issues: {:?}",
            template.id, report.issues
        );
    }
}

#[test]
fn template_category_distribution_matches_current_catalog() {
    let distribution =
        list_templates()
            .iter()
            .fold(BTreeMap::<&str, usize>::new(), |mut counts, template| {
                *counts.entry(template.category).or_default() += 1;
                counts
            });
    let expected = BTreeMap::from([
        ("browser", 1usize),
        ("electromagnetic", 2),
        ("hybrid", 1),
        ("materials", 4),
        ("mechanical", 12),
        ("mesh", 1),
        ("orchestration", 1),
        ("solver", 1),
        ("thermal", 3),
        ("thermo_mechanical", 7),
    ]);
    assert_eq!(distribution, expected);
}

#[test]
fn material_screening_template_expands_candidate_solve_chains() {
    let candidates = heat_spreader_screening_candidates();
    assert_eq!(candidates.len(), 3);

    let document = build_template_document("material_heat_spreader_screening", None)
        .expect("material screening template should build");
    let steps = &document.workflow.steps;
    assert_eq!(steps.len(), candidates.len() * 3);

    for (candidate_index, candidate) in candidates.iter().enumerate() {
        let base = candidate_index * 3;
        let solve_step_number = base + 1;
        assert_eq!(steps[base].action, "solve_heat_plane_quad_2d");
        assert_eq!(steps[base + 1].action, "job_wait");
        assert_eq!(steps[base + 2].action, "result_fetch");
        assert_eq!(
            steps[base].payload["research"]["candidate_id"],
            candidate.id
        );
        assert_eq!(
            steps[base].payload["model"]["elements"][0]["conductivity"],
            candidate.thermal_conductivity_w_mk
        );
        assert_eq!(
            steps[base + 1].payload["job_id"],
            format!("{{{{steps.{solve_step_number}.result.job_id}}}}")
        );
    }

    let batch = normalize_workflow_document(&document).unwrap();
    let plan = build_execution_plan(&batch);
    assert!(plan.ok);
    assert!(plan.compatibility.service_only_ok);
    let policy = plan
        .policy
        .as_ref()
        .expect("material screening plan should include policy");
    assert_eq!(
        policy.recommended_runtime,
        HeadlessRuntimeStyle::ServiceOnly
    );
    assert_eq!(plan.steps.len(), 9);
    assert!(
        plan.executor_matrix
            .iter()
            .any(|entry| entry.executor == "service" && entry.compatible)
    );
}

#[test]
fn dielectric_material_template_expands_candidate_solve_chains() {
    let candidates = dielectric_screening_candidates();
    assert_eq!(candidates.len(), 3);

    let document = build_template_document("material_dielectric_screening", None)
        .expect("dielectric material template should build");
    let steps = &document.workflow.steps;
    assert_eq!(steps.len(), candidates.len() * 3);

    for (candidate_index, candidate) in candidates.iter().enumerate() {
        let base = candidate_index * 3;
        let solve_step_number = base + 1;
        assert_eq!(steps[base].action, "solve_electrostatic_plane_quad_2d");
        assert_eq!(steps[base + 1].action, "job_wait");
        assert_eq!(steps[base + 2].action, "result_fetch");
        assert_eq!(
            steps[base].payload["research"]["candidate_id"],
            candidate.id
        );
        assert_eq!(
            steps[base].payload["research"]["relative_permittivity"],
            candidate.relative_permittivity
        );
        assert_eq!(
            steps[base + 1].payload["job_id"],
            format!("{{{{steps.{solve_step_number}.result.job_id}}}}")
        );
    }

    let batch = normalize_workflow_document(&document).unwrap();
    let plan = build_execution_plan(&batch);
    assert!(plan.ok);
    assert!(plan.compatibility.service_only_ok);
    assert_eq!(plan.steps.len(), 9);
}

#[test]
fn structural_material_template_expands_candidate_solve_chains() {
    let candidates = structural_panel_screening_candidates();
    assert_eq!(candidates.len(), 3);

    let document = build_template_document("material_structural_panel_screening", None)
        .expect("structural material template should build");
    let steps = &document.workflow.steps;
    assert_eq!(steps.len(), candidates.len() * 3);

    for (candidate_index, candidate) in candidates.iter().enumerate() {
        let base = candidate_index * 3;
        let solve_step_number = base + 1;
        assert_eq!(steps[base].action, "solve_plane_quad_2d");
        assert_eq!(steps[base + 1].action, "job_wait");
        assert_eq!(steps[base + 2].action, "result_fetch");
        assert_eq!(
            steps[base].payload["research"]["candidate_id"],
            candidate.id
        );
        assert_eq!(
            steps[base].payload["model"]["elements"][0]["youngs_modulus"],
            candidate.youngs_modulus_pa
        );
        assert_eq!(
            steps[base + 1].payload["job_id"],
            format!("{{{{steps.{solve_step_number}.result.job_id}}}}")
        );
    }

    let batch = normalize_workflow_document(&document).unwrap();
    let plan = build_execution_plan(&batch);
    assert!(plan.ok);
    assert!(plan.compatibility.service_only_ok);
    assert_eq!(plan.steps.len(), 9);
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
            policy.recommended_runtime, template.runtime_style,
            "template {} runtime style drifted from policy",
            template.id
        );
    }
}

#[test]
fn execution_plan_reports_runtime_and_confirmation_gates() {
    let service_document = build_template_document("workflow_submit_monitor", None).unwrap();
    let service_batch = normalize_workflow_document(&service_document).unwrap();
    let service_plan = build_execution_plan(&service_batch);
    assert_eq!(service_plan.schema_version, "kyuubiki.headless-plan/v1");
    assert!(service_plan.ok);
    assert!(service_plan.compatibility.service_only_ok);
    assert_eq!(service_plan.confirmation_count, 0);
    assert_eq!(service_plan.steps[1].bindings[0].source_step, 1);
    assert_eq!(service_plan.steps[1].bindings[0].output, "job_id");

    let browser_document = build_template_document("browser_capture_review", None).unwrap();
    let browser_batch = normalize_workflow_document(&browser_document).unwrap();
    let browser_plan = build_execution_plan(&browser_batch);
    assert!(browser_plan.compatibility.browser_session_required);
    assert_eq!(browser_plan.confirmation_count, 1);
    assert_eq!(browser_plan.confirmations[0].flag, "--allow-sensitive");
    let service_executor = browser_plan
        .executor_matrix
        .iter()
        .find(|entry| entry.executor == "service")
        .expect("service executor plan");
    assert!(!service_executor.compatible);
    assert_eq!(service_executor.issue_count, 3);
}

#[test]
fn direct_service_templates_keep_job_follow_up_chain() {
    for template in list_templates().iter().filter(|template| {
        template.runtime_style == HeadlessRuntimeStyle::ServiceOnly
            && template.tags.contains(&"direct")
    }) {
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
        assert_eq!(
            actions[actions.len() - 2],
            "job_wait",
            "template {}",
            template.id
        );
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
