use kyuubiki_protocol::{CoupledWorkflowKind, coupled_workflow_descriptors};
use serde::Serialize;

/// Serializable, SDK-facing view of a protocol-owned coupled workflow route.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CoupledWorkflowCatalogEntry {
    pub kind: CoupledWorkflowKind,
    pub id: String,
    pub source_artifact_type: String,
    pub result_artifact_type: String,
    pub domains: Vec<String>,
    pub bridge_operator_ids: Vec<String>,
}

/// Lists all built-in coupled routes in protocol catalog order.
pub fn coupled_workflow_catalog() -> Vec<CoupledWorkflowCatalogEntry> {
    coupled_workflow_descriptors()
        .iter()
        .map(|descriptor| CoupledWorkflowCatalogEntry {
            kind: descriptor.kind,
            id: descriptor.id.to_string(),
            source_artifact_type: descriptor.source_artifact_type.to_string(),
            result_artifact_type: descriptor.result_artifact_type.to_string(),
            domains: descriptor
                .domains
                .iter()
                .map(|domain| (*domain).to_string())
                .collect(),
            bridge_operator_ids: descriptor
                .bridge_operator_ids
                .iter()
                .map(|operator_id| (*operator_id).to_string())
                .collect(),
        })
        .collect()
}

/// Finds a coupled route by its stable workflow identifier.
pub fn find_coupled_workflow(workflow_id: &str) -> Option<CoupledWorkflowCatalogEntry> {
    let normalized = normalize_workflow_id(workflow_id);
    coupled_workflow_catalog()
        .into_iter()
        .find(|entry| normalize_workflow_id(&entry.id) == normalized)
}

/// Searches coupled routes across workflow identifiers, domains, and bridge IDs.
pub fn search_coupled_workflows(query: &str) -> Vec<CoupledWorkflowCatalogEntry> {
    let terms = query
        .split_whitespace()
        .map(normalize_workflow_id)
        .filter(|term| !term.is_empty())
        .collect::<Vec<_>>();
    coupled_workflow_catalog()
        .into_iter()
        .filter(|entry| {
            let haystack = format!(
                "{} {} {}",
                entry.id,
                entry.domains.join(" "),
                entry.bridge_operator_ids.join(" ")
            );
            let haystack = normalize_workflow_id(&haystack);
            terms.iter().all(|term| haystack.contains(term))
        })
        .collect()
}

fn normalize_workflow_id(value: &str) -> String {
    value.trim().replace(['-', '.'], "_").to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::{coupled_workflow_catalog, find_coupled_workflow, search_coupled_workflows};
    use kyuubiki_protocol::CoupledWorkflowKind;

    #[test]
    fn catalog_exposes_protocol_owned_magnetostatic_route() {
        let catalog = coupled_workflow_catalog();

        assert_eq!(catalog.len(), 4);
        let route = catalog
            .iter()
            .find(|entry| entry.kind == CoupledWorkflowKind::MagnetostaticHeatToThermoPlaneQuad2d)
            .expect("magnetostatic route");
        assert_eq!(route.domains, ["magnetostatic", "thermal", "thermo"]);
        assert_eq!(route.bridge_operator_ids.len(), 2);
    }

    #[test]
    fn lookup_and_search_resolve_coupled_routes() {
        let route = find_coupled_workflow("workflow.magnetostatic-heat-to-thermo-quad-2d")
            .expect("route lookup");
        assert_eq!(
            route.source_artifact_type,
            "study_model/magnetostatic_plane_quad_2d"
        );

        let matches = search_coupled_workflows("magnetostatic heat");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].id, route.id);
    }

    #[test]
    fn triangle_route_lists_both_registered_bridge_operators() {
        let route = find_coupled_workflow("workflow.electrostatic-heat-to-thermo-triangle-2d")
            .expect("triangle route lookup");

        assert_eq!(route.bridge_operator_ids.len(), 2);
        assert!(
            route
                .bridge_operator_ids
                .contains(&"bridge.temperature_field_to_thermo_triangle_2d".to_string())
        );
    }
}
