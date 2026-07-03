use crate::{HeadlessRuntimeStyle, search_templates};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MaterialWorkflowDescriptor {
    pub id: &'static str,
    pub title: &'static str,
    pub domain: &'static str,
    pub objective: &'static str,
    pub template_id: &'static str,
    pub workflow_kind: &'static str,
    pub required_actions: &'static [&'static str],
    pub aliases: &'static [&'static str],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MaterialWorkflowCatalogEntry {
    pub id: String,
    pub title: String,
    pub domain: String,
    pub objective: String,
    pub template_id: String,
    pub workflow_kind: String,
    pub required_actions: Vec<String>,
    pub aliases: Vec<String>,
}

const MATERIAL_WORKFLOWS: &[MaterialWorkflowDescriptor] = &[MaterialWorkflowDescriptor {
    id: "material_study_envelope_ranking",
    title: "Material Study Envelope Ranking",
    domain: "multi_physics_materials",
    objective: "compose material envelopes, rank candidates, and extract a Pareto frontier",
    template_id: "material_study_envelope_ranking",
    workflow_kind: "operator_graph",
    required_actions: &["workflow_submit_graph", "job_wait", "result_fetch"],
    aliases: &[
        "material-envelope",
        "material_envelope",
        "material.pareto_ranking.v1",
    ],
}];

pub fn material_workflow_descriptors() -> &'static [MaterialWorkflowDescriptor] {
    MATERIAL_WORKFLOWS
}

pub fn material_workflow_catalog() -> Vec<MaterialWorkflowCatalogEntry> {
    MATERIAL_WORKFLOWS
        .iter()
        .map(MaterialWorkflowCatalogEntry::from_descriptor)
        .collect()
}

pub fn find_material_workflow(workflow: &str) -> Option<MaterialWorkflowCatalogEntry> {
    let normalized = normalize_workflow_key(workflow);
    MATERIAL_WORKFLOWS
        .iter()
        .find(|descriptor| {
            normalize_workflow_key(descriptor.id) == normalized
                || normalize_workflow_key(descriptor.template_id) == normalized
                || descriptor
                    .aliases
                    .iter()
                    .any(|alias| normalize_workflow_key(alias) == normalized)
        })
        .map(MaterialWorkflowCatalogEntry::from_descriptor)
}

pub fn search_material_workflow_templates(query: &str) -> Vec<MaterialWorkflowCatalogEntry> {
    search_templates(
        Some(HeadlessRuntimeStyle::ServiceOnly),
        Some("materials"),
        None,
        Some(query),
    )
    .into_iter()
    .filter_map(|template| find_material_workflow(template.id))
    .collect()
}

fn normalize_workflow_key(value: &str) -> String {
    value.trim().replace(['-', '.'], "_").to_lowercase()
}

impl MaterialWorkflowCatalogEntry {
    fn from_descriptor(descriptor: &MaterialWorkflowDescriptor) -> Self {
        Self {
            id: descriptor.id.to_string(),
            title: descriptor.title.to_string(),
            domain: descriptor.domain.to_string(),
            objective: descriptor.objective.to_string(),
            template_id: descriptor.template_id.to_string(),
            workflow_kind: descriptor.workflow_kind.to_string(),
            required_actions: descriptor
                .required_actions
                .iter()
                .map(|action| (*action).to_string())
                .collect(),
            aliases: descriptor
                .aliases
                .iter()
                .map(|alias| (*alias).to_string())
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        find_material_workflow, material_workflow_catalog, search_material_workflow_templates,
    };
    use crate::find_template;

    #[test]
    fn material_workflow_catalog_exposes_envelope_graph_entry() {
        let catalog = material_workflow_catalog();

        assert_eq!(catalog.len(), 1);
        assert_eq!(catalog[0].id, "material_study_envelope_ranking");
        assert_eq!(catalog[0].workflow_kind, "operator_graph");
        assert_eq!(
            catalog[0].required_actions,
            vec!["workflow_submit_graph", "job_wait", "result_fetch"]
        );
    }

    #[test]
    fn material_workflow_lookup_resolves_aliases() {
        let workflow = find_material_workflow("material-envelope").expect("workflow");

        assert_eq!(workflow.template_id, "material_study_envelope_ranking");
        assert_eq!(workflow.domain, "multi_physics_materials");
    }

    #[test]
    fn material_workflow_search_finds_envelope_ranking_template() {
        let matches = search_material_workflow_templates("material envelope pareto");

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].id, "material_study_envelope_ranking");
    }

    #[test]
    fn material_workflow_template_exists() {
        let workflow = find_material_workflow("material_study_envelope_ranking").unwrap();
        let template = find_template(&workflow.template_id).expect("template");

        assert_eq!(template.category, "materials");
    }
}
