use crate::{
    HeadlessEngine, HeadlessRisk, action_capability_manifest, all_action_contracts,
    all_direct_fem_routes, build_template_document, direct_fem_capability_manifest,
    direct_fem_submit_route, list_templates, normalize_workflow_document,
};
use std::collections::BTreeSet;

#[test]
fn every_solver_contract_has_direct_fem_route() {
    let solver_contracts = direct_solver_contract_ids();
    assert!(
        !solver_contracts.is_empty(),
        "solver contract catalog should not be empty"
    );

    for action in solver_contracts {
        let route = direct_fem_submit_route(action)
            .unwrap_or_else(|| panic!("solver action {action} is missing a direct FEM route"));
        assert!(
            route.starts_with("/api/v1/fem/") && route.ends_with("/jobs"),
            "solver action {action} has malformed route {route}"
        );
    }
}

#[test]
fn direct_fem_route_catalog_matches_solver_contract_catalog() {
    let solver_contracts = direct_solver_contract_ids()
        .into_iter()
        .collect::<BTreeSet<_>>();
    let route_actions = all_direct_fem_routes()
        .iter()
        .map(|entry| entry.action)
        .collect::<BTreeSet<_>>();

    assert_eq!(
        route_actions, solver_contracts,
        "direct FEM route catalog drifted from solver contracts"
    );
}

#[test]
fn direct_fem_manifest_matches_route_catalog() {
    let manifest = direct_fem_capability_manifest();
    let manifest_actions = manifest
        .iter()
        .map(|entry| entry.action.as_str())
        .collect::<BTreeSet<_>>();
    let route_actions = all_direct_fem_routes()
        .iter()
        .map(|entry| entry.action)
        .collect::<BTreeSet<_>>();

    assert_eq!(manifest.len(), all_direct_fem_routes().len());
    assert_eq!(manifest_actions, route_actions);
}

#[test]
fn action_manifest_contains_every_contract_and_direct_fem_subset() {
    let manifest = action_capability_manifest();
    let manifest_actions = manifest
        .iter()
        .map(|entry| entry.action.as_str())
        .collect::<BTreeSet<_>>();
    let contract_actions = all_action_contracts()
        .iter()
        .map(|contract| contract.id)
        .collect::<BTreeSet<_>>();
    let routed_actions = manifest
        .iter()
        .filter(|entry| entry.direct_fem_route.is_some())
        .map(|entry| entry.action.as_str())
        .collect::<BTreeSet<_>>();
    let direct_fem_actions = all_direct_fem_routes()
        .iter()
        .map(|entry| entry.action)
        .collect::<BTreeSet<_>>();

    assert_eq!(manifest.len(), all_action_contracts().len());
    assert_eq!(manifest_actions, contract_actions);
    assert_eq!(routed_actions, direct_fem_actions);
}

#[test]
fn solver_contracts_keep_service_safe_model_contract() {
    for contract in all_action_contracts()
        .iter()
        .filter(|contract| is_direct_solver_contract(contract))
    {
        assert_eq!(contract.engine, HeadlessEngine::Service, "{}", contract.id);
        assert_eq!(contract.risk, HeadlessRisk::Normal, "{}", contract.id);
        assert_eq!(
            contract.required_payload_keys,
            &["model"],
            "{}",
            contract.id
        );
        assert_eq!(
            contract.output_keys,
            &["job_id", "status", "progress", "job"],
            "{}",
            contract.id
        );
    }
}

#[test]
fn direct_solver_templates_start_with_routed_solver_action() {
    let direct_templates = list_templates()
        .iter()
        .filter(|template| template.tags.contains(&"direct"))
        .collect::<Vec<_>>();
    assert!(
        direct_templates.len() >= 20,
        "expected broad direct solver template coverage"
    );

    for template in direct_templates {
        let document = build_template_document(template.id, None)
            .unwrap_or_else(|| panic!("template {} should build", template.id));
        let batch = normalize_workflow_document(&document)
            .unwrap_or_else(|error| panic!("template {} should normalize: {error}", template.id));
        let first = batch
            .steps
            .first()
            .unwrap_or_else(|| panic!("template {} should contain steps", template.id));
        if first.action == "direct_mesh_solve" {
            continue;
        }

        assert!(
            first.action.starts_with("solve_"),
            "template {} should start with a solver action, got {}",
            template.id,
            first.action
        );
        assert!(
            first.payload.get("model").is_some(),
            "template {} first solver step should carry an inline model payload",
            template.id
        );
        assert!(
            direct_fem_submit_route(&first.action).is_some(),
            "template {} first solver action {} is missing a direct FEM route",
            template.id,
            first.action
        );
    }
}

#[test]
fn solver_contract_ids_are_unique_and_sorted_by_catalog_order() {
    let ids = direct_solver_contract_ids();
    let unique = ids.iter().copied().collect::<BTreeSet<_>>();
    assert_eq!(ids.len(), unique.len(), "duplicate solver contract ids");
    assert_eq!(ids.len(), 39, "unexpected solver contract coverage drift");
}

fn direct_solver_contract_ids() -> Vec<&'static str> {
    all_action_contracts()
        .iter()
        .filter(|contract| is_direct_solver_contract(contract))
        .map(|contract| contract.id)
        .collect()
}

fn is_direct_solver_contract(contract: &crate::HeadlessActionContract) -> bool {
    contract.category == "solve"
        && contract.id.starts_with("solve_")
        && contract.required_payload_keys == ["model"]
}
