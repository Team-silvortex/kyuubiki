use crate::{
    HeadlessActionContract, HeadlessEngine, HeadlessRisk, HeadlessRuntimeStyle,
    all_action_contracts, direct_fem_submit_route, find_action_contract,
};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HeadlessActionCapability {
    pub action: String,
    pub category: String,
    pub engine: HeadlessEngine,
    pub risk: HeadlessRisk,
    pub runtime_style: HeadlessRuntimeStyle,
    pub required_payload_keys: Vec<String>,
    pub output_keys: Vec<String>,
    pub direct_fem_route: Option<String>,
}

pub fn action_capability_manifest() -> Vec<HeadlessActionCapability> {
    all_action_contracts()
        .iter()
        .map(HeadlessActionCapability::from_contract)
        .collect()
}

pub fn find_action_capability(action: &str) -> Option<HeadlessActionCapability> {
    find_action_contract(action).map(HeadlessActionCapability::from_contract)
}

impl HeadlessActionCapability {
    fn from_contract(contract: &HeadlessActionContract) -> Self {
        Self {
            action: contract.id.to_string(),
            category: contract.category.to_string(),
            engine: contract.engine,
            risk: contract.risk,
            runtime_style: runtime_style_for_engine(contract.engine),
            required_payload_keys: to_strings(contract.required_payload_keys),
            output_keys: to_strings(contract.output_keys),
            direct_fem_route: direct_fem_submit_route(contract.id).map(str::to_string),
        }
    }
}

fn runtime_style_for_engine(engine: HeadlessEngine) -> HeadlessRuntimeStyle {
    match engine {
        HeadlessEngine::Browser => HeadlessRuntimeStyle::BrowserOnly,
        HeadlessEngine::Service => HeadlessRuntimeStyle::ServiceOnly,
    }
}

fn to_strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

#[cfg(test)]
mod tests {
    use super::{action_capability_manifest, find_action_capability};
    use crate::{HeadlessRisk, HeadlessRuntimeStyle, all_action_contracts, all_direct_fem_routes};
    use std::collections::BTreeSet;

    #[test]
    fn action_manifest_covers_every_contract_once() {
        let manifest = action_capability_manifest();
        let actions = manifest
            .iter()
            .map(|entry| entry.action.as_str())
            .collect::<BTreeSet<_>>();

        assert_eq!(manifest.len(), all_action_contracts().len());
        assert_eq!(actions.len(), manifest.len());
        assert!(actions.contains("service_health"));
        assert!(actions.contains("snapshot"));
        assert!(actions.contains("solve_thermal_frame_3d"));
        serde_json::to_value(&manifest).expect("manifest should serialize");
    }

    #[test]
    fn action_manifest_marks_direct_fem_subset_with_routes() {
        let manifest = action_capability_manifest();
        let routed = manifest
            .iter()
            .filter(|entry| entry.direct_fem_route.is_some())
            .collect::<Vec<_>>();

        assert_eq!(routed.len(), all_direct_fem_routes().len());
        for entry in routed {
            assert!(entry.action.starts_with("solve_"));
            assert!(
                entry
                    .direct_fem_route
                    .as_deref()
                    .unwrap()
                    .starts_with("/api/v1/fem/")
            );
        }
    }

    #[test]
    fn action_manifest_preserves_runtime_and_risk_metadata() {
        let snapshot = find_action_capability("snapshot").expect("snapshot capability");
        assert_eq!(snapshot.runtime_style, HeadlessRuntimeStyle::BrowserOnly);
        assert_eq!(snapshot.risk, HeadlessRisk::Sensitive);
        assert_eq!(snapshot.direct_fem_route, None);

        let delete = find_action_capability("project_delete").expect("delete capability");
        assert_eq!(delete.runtime_style, HeadlessRuntimeStyle::ServiceOnly);
        assert_eq!(delete.risk, HeadlessRisk::Destructive);
        assert_eq!(delete.required_payload_keys, vec!["project_id".to_string()]);
    }
}
