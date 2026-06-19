use crate::template_search::{
    matches_search_tokens, normalize_search_token, score_search_tokens, tokenize_search_query,
};
use crate::template_workflows::build_template_workflow;
use crate::{HeadlessRuntimeStyle, HeadlessTemplateDescriptor, HeadlessWorkflowDocument};
use serde::Serialize;
use std::collections::BTreeSet;

const TEMPLATES: &[HeadlessTemplateDescriptor] = &[
    HeadlessTemplateDescriptor {
        id: "solve_wait_result",
        title: "Solve From Version",
        description: "Start from a saved model version and go straight to final result.",
        runtime_style: crate::HeadlessRuntimeStyle::ServiceOnly,
        category: "solver",
        tags: &["solve", "result", "version"],
    },
    HeadlessTemplateDescriptor {
        id: "workflow_submit_monitor",
        title: "Workflow Submit",
        description: "Submit a workflow job and keep follow-up polling and result fetch explicit.",
        runtime_style: crate::HeadlessRuntimeStyle::ServiceOnly,
        category: "orchestration",
        tags: &["workflow", "job", "polling"],
    },
    HeadlessTemplateDescriptor {
        id: "direct_mesh_pipeline",
        title: "Direct Mesh Solve",
        description: "Resolve from a raw mesh payload and keep the job follow-up explicit.",
        runtime_style: crate::HeadlessRuntimeStyle::ServiceOnly,
        category: "mesh",
        tags: &["mesh", "direct", "solve"],
    },
    HeadlessTemplateDescriptor {
        id: "direct_plane_quad",
        title: "Direct Plane Quad",
        description: "Submit a plane quad structural solve directly and fetch the final result.",
        runtime_style: crate::HeadlessRuntimeStyle::ServiceOnly,
        category: "mechanical",
        tags: &["direct", "mechanical", "quad"],
    },
    HeadlessTemplateDescriptor {
        id: "direct_plane_triangle",
        title: "Direct Plane Triangle",
        description: "Submit a plane triangle structural solve directly and fetch the final result.",
        runtime_style: crate::HeadlessRuntimeStyle::ServiceOnly,
        category: "mechanical",
        tags: &["direct", "mechanical", "triangle"],
    },
    HeadlessTemplateDescriptor {
        id: "direct_bar_1d",
        title: "Direct Bar 1D",
        description: "Submit a bar 1D structural solve directly and fetch the final result.",
        runtime_style: crate::HeadlessRuntimeStyle::ServiceOnly,
        category: "mechanical",
        tags: &["direct", "mechanical", "bar", "1d"],
    },
    HeadlessTemplateDescriptor {
        id: "direct_truss_3d",
        title: "Direct Truss 3D",
        description: "Submit a truss 3D structural solve directly and fetch the final result.",
        runtime_style: crate::HeadlessRuntimeStyle::ServiceOnly,
        category: "mechanical",
        tags: &["direct", "mechanical", "truss", "3d"],
    },
    HeadlessTemplateDescriptor {
        id: "direct_frame_2d",
        title: "Direct Frame 2D",
        description: "Submit a frame 2D structural solve directly and fetch the final result.",
        runtime_style: crate::HeadlessRuntimeStyle::ServiceOnly,
        category: "mechanical",
        tags: &["direct", "mechanical", "frame", "2d"],
    },
    HeadlessTemplateDescriptor {
        id: "direct_beam_1d",
        title: "Direct Beam 1D",
        description: "Submit a beam 1D structural solve directly and fetch the final result.",
        runtime_style: crate::HeadlessRuntimeStyle::ServiceOnly,
        category: "mechanical",
        tags: &["direct", "mechanical", "beam", "1d"],
    },
    HeadlessTemplateDescriptor {
        id: "direct_truss_2d",
        title: "Direct Truss 2D",
        description: "Submit a truss 2D structural solve directly and fetch the final result.",
        runtime_style: crate::HeadlessRuntimeStyle::ServiceOnly,
        category: "mechanical",
        tags: &["direct", "mechanical", "truss", "2d"],
    },
    HeadlessTemplateDescriptor {
        id: "direct_spring_2d",
        title: "Direct Spring 2D",
        description: "Submit a spring 2D structural solve directly and fetch the final result.",
        runtime_style: crate::HeadlessRuntimeStyle::ServiceOnly,
        category: "mechanical",
        tags: &["direct", "mechanical", "spring", "2d"],
    },
    HeadlessTemplateDescriptor {
        id: "direct_torsion_1d",
        title: "Direct Torsion 1D",
        description: "Submit a torsion 1D structural solve directly and fetch the final result.",
        runtime_style: crate::HeadlessRuntimeStyle::ServiceOnly,
        category: "mechanical",
        tags: &["direct", "mechanical", "torsion", "1d"],
    },
    HeadlessTemplateDescriptor {
        id: "direct_spring_3d",
        title: "Direct Spring 3D",
        description: "Submit a spring 3D structural solve directly and fetch the final result.",
        runtime_style: crate::HeadlessRuntimeStyle::ServiceOnly,
        category: "mechanical",
        tags: &["direct", "mechanical", "spring", "3d"],
    },
    HeadlessTemplateDescriptor {
        id: "direct_frame_3d",
        title: "Direct Frame 3D",
        description: "Submit a frame 3D structural solve directly and fetch the final result.",
        runtime_style: crate::HeadlessRuntimeStyle::ServiceOnly,
        category: "mechanical",
        tags: &["direct", "mechanical", "frame", "3d"],
    },
    HeadlessTemplateDescriptor {
        id: "direct_spring_1d",
        title: "Direct Spring 1D",
        description: "Submit a spring 1D structural solve directly and fetch the final result.",
        runtime_style: crate::HeadlessRuntimeStyle::ServiceOnly,
        category: "mechanical",
        tags: &["direct", "mechanical", "spring", "1d"],
    },
    HeadlessTemplateDescriptor {
        id: "direct_heat_quad",
        title: "Direct Heat Quad",
        description: "Submit a heat plane quad solve directly and fetch the final result.",
        runtime_style: crate::HeadlessRuntimeStyle::ServiceOnly,
        category: "thermal",
        tags: &["direct", "heat", "quad"],
    },
    HeadlessTemplateDescriptor {
        id: "direct_heat_triangle",
        title: "Direct Heat Triangle",
        description: "Submit a heat plane triangle solve directly and fetch the final result.",
        runtime_style: crate::HeadlessRuntimeStyle::ServiceOnly,
        category: "thermal",
        tags: &["direct", "heat", "triangle"],
    },
    HeadlessTemplateDescriptor {
        id: "direct_heat_bar_1d",
        title: "Direct Heat Bar 1D",
        description: "Submit a heat bar 1D solve directly and fetch the final result.",
        runtime_style: crate::HeadlessRuntimeStyle::ServiceOnly,
        category: "thermal",
        tags: &["direct", "heat", "bar", "1d"],
    },
    HeadlessTemplateDescriptor {
        id: "direct_thermal_quad",
        title: "Direct Thermo Quad",
        description: "Submit a thermo-mechanical plane quad solve directly and fetch the final result.",
        runtime_style: crate::HeadlessRuntimeStyle::ServiceOnly,
        category: "thermo_mechanical",
        tags: &["direct", "thermo", "quad"],
    },
    HeadlessTemplateDescriptor {
        id: "direct_thermal_triangle",
        title: "Direct Thermo Triangle",
        description: "Submit a thermo-mechanical plane triangle solve directly and fetch the final result.",
        runtime_style: crate::HeadlessRuntimeStyle::ServiceOnly,
        category: "thermo_mechanical",
        tags: &["direct", "thermo", "triangle"],
    },
    HeadlessTemplateDescriptor {
        id: "direct_thermal_truss_2d",
        title: "Direct Thermo Truss 2D",
        description: "Submit a thermo-mechanical truss 2D solve directly and fetch the final result.",
        runtime_style: crate::HeadlessRuntimeStyle::ServiceOnly,
        category: "thermo_mechanical",
        tags: &["direct", "thermo", "truss", "2d"],
    },
    HeadlessTemplateDescriptor {
        id: "direct_thermal_frame_2d",
        title: "Direct Thermo Frame 2D",
        description: "Submit a thermo-mechanical frame 2D solve directly and fetch the final result.",
        runtime_style: crate::HeadlessRuntimeStyle::ServiceOnly,
        category: "thermo_mechanical",
        tags: &["direct", "thermo", "frame", "2d"],
    },
    HeadlessTemplateDescriptor {
        id: "direct_thermal_beam_1d",
        title: "Direct Thermo Beam 1D",
        description: "Submit a thermo-mechanical beam 1D solve directly and fetch the final result.",
        runtime_style: crate::HeadlessRuntimeStyle::ServiceOnly,
        category: "thermo_mechanical",
        tags: &["direct", "thermo", "beam", "1d"],
    },
    HeadlessTemplateDescriptor {
        id: "direct_thermal_truss_3d",
        title: "Direct Thermo Truss 3D",
        description: "Submit a thermo-mechanical truss 3D solve directly and fetch the final result.",
        runtime_style: crate::HeadlessRuntimeStyle::ServiceOnly,
        category: "thermo_mechanical",
        tags: &["direct", "thermo", "truss", "3d"],
    },
    HeadlessTemplateDescriptor {
        id: "direct_thermal_frame_3d",
        title: "Direct Thermo Frame 3D",
        description: "Submit a thermo-mechanical frame 3D solve directly and fetch the final result.",
        runtime_style: crate::HeadlessRuntimeStyle::ServiceOnly,
        category: "thermo_mechanical",
        tags: &["direct", "thermo", "frame", "3d"],
    },
    HeadlessTemplateDescriptor {
        id: "direct_electrostatic_quad",
        title: "Direct Electrostatic Quad",
        description: "Submit an electrostatic plane quad solve directly and fetch the final result.",
        runtime_style: crate::HeadlessRuntimeStyle::ServiceOnly,
        category: "electromagnetic",
        tags: &["direct", "electrostatic", "quad"],
    },
    HeadlessTemplateDescriptor {
        id: "direct_electrostatic_triangle",
        title: "Direct Electrostatic Triangle",
        description: "Submit an electrostatic plane triangle solve directly and fetch the final result.",
        runtime_style: crate::HeadlessRuntimeStyle::ServiceOnly,
        category: "electromagnetic",
        tags: &["direct", "electrostatic", "triangle"],
    },
    HeadlessTemplateDescriptor {
        id: "browser_capture_review",
        title: "Browser Capture Review",
        description: "Open a page, wait for a stable target, then capture a review snapshot.",
        runtime_style: crate::HeadlessRuntimeStyle::BrowserOnly,
        category: "browser",
        tags: &["browser", "snapshot", "review"],
    },
    HeadlessTemplateDescriptor {
        id: "browser_submit_then_poll",
        title: "Browser Submit And Poll",
        description: "Drive a browser-side submit action, then switch to service-side job polling and result fetch.",
        runtime_style: crate::HeadlessRuntimeStyle::Hybrid,
        category: "hybrid",
        tags: &["browser", "service", "job"],
    },
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HeadlessTemplateSuggestion {
    pub id: String,
    pub title: String,
    pub category: String,
    pub runtime_style: HeadlessRuntimeStyle,
    pub tags: Vec<String>,
    pub score: usize,
    pub matched_terms: Vec<String>,
}

pub fn list_templates() -> &'static [HeadlessTemplateDescriptor] {
    TEMPLATES
}

pub fn find_template(id: &str) -> Option<&'static HeadlessTemplateDescriptor> {
    TEMPLATES.iter().find(|template| template.id == id)
}

pub fn list_template_categories() -> Vec<&'static str> {
    TEMPLATES
        .iter()
        .map(|template| template.category)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

pub fn search_templates(
    runtime: Option<HeadlessRuntimeStyle>,
    category: Option<&str>,
    tag: Option<&str>,
    query: Option<&str>,
) -> Vec<&'static HeadlessTemplateDescriptor> {
    let category = category.map(normalize_search_token);
    let tag = tag.map(normalize_search_token);
    let tokens = tokenize_search_query(query);
    let mut templates = TEMPLATES
        .iter()
        .filter(|template| runtime.is_none_or(|runtime| template.runtime_style == runtime))
        .filter(|template| {
            category
                .as_ref()
                .is_none_or(|category| normalize_search_token(template.category) == *category)
        })
        .filter(|template| {
            tag.as_ref().is_none_or(|tag| {
                template
                    .tags
                    .iter()
                    .any(|candidate| normalize_search_token(candidate) == *tag)
            })
        })
        .filter_map(|template| {
            let metadata = template_search_metadata(template);
            matches_search_tokens(&metadata.fields, &tokens).then_some((
                template,
                score_search_tokens(&metadata.weighted_fields, &tokens),
            ))
        })
        .collect::<Vec<_>>();
    if !tokens.is_empty() {
        templates.sort_by(
            |(left_template, left_score), (right_template, right_score)| {
                right_score
                    .cmp(left_score)
                    .then_with(|| left_template.id.cmp(right_template.id))
            },
        );
    }
    templates
        .into_iter()
        .map(|(template, _)| template)
        .collect()
}

pub fn suggest_templates(query: &str, limit: usize) -> Vec<&'static HeadlessTemplateDescriptor> {
    let tokens = tokenize_search_query(Some(query));
    let mut templates = TEMPLATES
        .iter()
        .map(|template| {
            let metadata = template_search_metadata(template);
            (
                template,
                score_search_tokens(&metadata.weighted_fields, &tokens),
            )
        })
        .filter(|(_, score)| *score > 0)
        .collect::<Vec<_>>();
    templates.sort_by(
        |(left_template, left_score), (right_template, right_score)| {
            right_score
                .cmp(left_score)
                .then_with(|| left_template.id.cmp(right_template.id))
        },
    );
    templates
        .into_iter()
        .take(limit)
        .map(|(template, _)| template)
        .collect()
}

pub fn suggest_template_details(query: &str, limit: usize) -> Vec<HeadlessTemplateSuggestion> {
    let tokens = tokenize_search_query(Some(query));
    let mut templates = TEMPLATES
        .iter()
        .map(|template| {
            let metadata = template_search_metadata(template);
            let score = score_search_tokens(&metadata.weighted_fields, &tokens);
            let matched_terms = tokens
                .iter()
                .filter(|token| {
                    metadata
                        .fields
                        .iter()
                        .any(|field| field.contains(token.as_str()))
                })
                .cloned()
                .collect::<Vec<_>>();
            (template, score, matched_terms)
        })
        .filter(|(_, score, _)| *score > 0)
        .collect::<Vec<_>>();
    templates.sort_by(
        |(left_template, left_score, _), (right_template, right_score, _)| {
            right_score
                .cmp(left_score)
                .then_with(|| left_template.id.cmp(right_template.id))
        },
    );
    templates
        .into_iter()
        .take(limit)
        .map(
            |(template, score, matched_terms)| HeadlessTemplateSuggestion {
                id: template.id.to_string(),
                title: template.title.to_string(),
                category: template.category.to_string(),
                runtime_style: template.runtime_style,
                tags: template.tags.iter().map(|tag| (*tag).to_string()).collect(),
                score,
                matched_terms,
            },
        )
        .collect()
}

pub fn build_template_document(
    template_id: &str,
    workflow_id: Option<&str>,
) -> Option<HeadlessWorkflowDocument> {
    let template = find_template(template_id)?;
    let workflow_id = workflow_id
        .map(|value| value.to_string())
        .unwrap_or_else(|| template.default_workflow_id());
    Some(HeadlessWorkflowDocument {
        schema_version: "kyuubiki.headless-workflow/v1".to_string(),
        exported_at: "1970-01-01T00:00:00.000Z".to_string(),
        language: "en".to_string(),
        workflow: build_template_workflow(template.id, &workflow_id),
        template: Some(template.to_snapshot()),
    })
}

fn template_search_metadata(template: &HeadlessTemplateDescriptor) -> TemplateSearchMetadata {
    let actions = build_template_workflow(template.id, template.id)
        .steps
        .into_iter()
        .map(|step| step.action)
        .collect::<Vec<_>>();
    let tags = template
        .tags
        .iter()
        .map(|tag| normalize_search_token(tag))
        .collect::<Vec<_>>();
    let action_tokens = actions
        .iter()
        .map(|action| normalize_search_token(action))
        .collect::<Vec<_>>();
    let id = normalize_search_token(template.id);
    let title = normalize_search_token(template.title);
    let description = normalize_search_token(template.description);
    let category = normalize_search_token(template.category);
    let mut fields = vec![
        id.clone(),
        title.clone(),
        description.clone(),
        category.clone(),
    ];
    fields.extend(tags.iter().cloned());
    fields.extend(action_tokens.iter().cloned());
    let mut weighted_fields = vec![(id, 5usize), (title, 4), (category, 3), (description, 1)];
    weighted_fields.extend(tags.into_iter().map(|tag| (tag, 4)));
    weighted_fields.extend(action_tokens.into_iter().map(|action| (action, 2)));
    TemplateSearchMetadata {
        fields,
        weighted_fields,
    }
}

struct TemplateSearchMetadata {
    fields: Vec<String>,
    weighted_fields: Vec<(String, usize)>,
}
