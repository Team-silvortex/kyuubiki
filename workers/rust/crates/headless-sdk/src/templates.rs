use crate::{
    HeadlessTemplateDescriptor, HeadlessWorkflowDocument, HeadlessWorkflowDraft,
    HeadlessWorkflowStep,
};
use serde_json::json;

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
        id: "direct_heat_quad",
        title: "Direct Heat Quad",
        description: "Submit a heat plane quad solve directly and fetch the final result.",
        runtime_style: crate::HeadlessRuntimeStyle::ServiceOnly,
        category: "thermal",
        tags: &["direct", "heat", "quad"],
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
        id: "direct_electrostatic_quad",
        title: "Direct Electrostatic Quad",
        description: "Submit an electrostatic plane quad solve directly and fetch the final result.",
        runtime_style: crate::HeadlessRuntimeStyle::ServiceOnly,
        category: "electromagnetic",
        tags: &["direct", "electrostatic", "quad"],
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

pub fn list_templates() -> &'static [HeadlessTemplateDescriptor] {
    TEMPLATES
}

pub fn find_template(id: &str) -> Option<&'static HeadlessTemplateDescriptor> {
    TEMPLATES.iter().find(|template| template.id == id)
}

pub fn build_template_document(
    template_id: &str,
    workflow_id: Option<&str>,
) -> Option<HeadlessWorkflowDocument> {
    let template = find_template(template_id)?;
    let workflow_id = workflow_id.unwrap_or(template.default_workflow_id());
    Some(HeadlessWorkflowDocument {
        schema_version: "kyuubiki.headless-workflow/v1".to_string(),
        exported_at: "1970-01-01T00:00:00.000Z".to_string(),
        language: "en".to_string(),
        workflow: build_template_workflow(template.id, workflow_id),
        template: Some(template.to_snapshot()),
    })
}

fn build_template_workflow(template_id: &str, workflow_id: &str) -> HeadlessWorkflowDraft {
    let steps = match template_id {
        "solve_wait_result" => vec![HeadlessWorkflowStep::new(
            "solve_and_wait_from_model_version",
            json!({
                "model_version_id": "ver_123",
                "endpoints": ["http://127.0.0.1:7001"],
                "timeout_ms": 60000
            }),
        )],
        "workflow_submit_monitor" => vec![
            HeadlessWorkflowStep::new(
                "workflow_submit_catalog",
                json!({ "workflow_id": "wf_demo", "input_artifacts": {} }),
            ),
            HeadlessWorkflowStep::new(
                "job_wait",
                json!({ "job_id": "{{steps.1.result.job_id}}", "interval_ms": 1000, "timeout_ms": 60000 }),
            ),
            HeadlessWorkflowStep::new(
                "result_fetch",
                json!({ "job_id": "{{steps.1.result.job_id}}" }),
            ),
        ],
        "direct_mesh_pipeline" => vec![
            HeadlessWorkflowStep::new(
                "direct_mesh_solve",
                json!({ "study_kind": "truss_3d", "input": { "nodes": [], "elements": [] }, "endpoints": ["http://127.0.0.1:7001"] }),
            ),
            HeadlessWorkflowStep::new(
                "job_wait",
                json!({ "job_id": "{{steps.1.result.job_id}}", "interval_ms": 1000, "timeout_ms": 60000 }),
            ),
            HeadlessWorkflowStep::new(
                "result_fetch",
                json!({ "job_id": "{{steps.1.result.job_id}}" }),
            ),
        ],
        "direct_plane_quad" => vec![
            HeadlessWorkflowStep::new(
                "solve_plane_quad_2d",
                json!({ "model": {
                    "nodes": [
                        { "id": "q0", "x": 0.0, "y": 0.0, "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0 },
                        { "id": "q1", "x": 1.0, "y": 0.0, "fix_x": false, "fix_y": true, "load_x": 0.0, "load_y": 0.0 },
                        { "id": "q2", "x": 1.0, "y": 1.0, "fix_x": false, "fix_y": false, "load_x": 0.0, "load_y": -1000.0 },
                        { "id": "q3", "x": 0.0, "y": 1.0, "fix_x": true, "fix_y": false, "load_x": 0.0, "load_y": 0.0 }
                    ],
                    "elements": [
                        { "id": "pq0", "node_i": 0, "node_j": 1, "node_k": 2, "node_l": 3, "thickness": 0.02, "youngs_modulus": 70000000000.0, "poisson_ratio": 0.33 }
                    ]
                }}),
            ),
            HeadlessWorkflowStep::new(
                "job_wait",
                json!({ "job_id": "{{steps.1.result.job_id}}", "interval_ms": 1000, "timeout_ms": 60000 }),
            ),
            HeadlessWorkflowStep::new("result_fetch", json!({ "job_id": "{{steps.1.result.job_id}}" })),
        ],
        "direct_heat_quad" => vec![
            HeadlessWorkflowStep::new(
                "solve_heat_plane_quad_2d",
                json!({ "model": {
                    "nodes": [
                        { "id": "h0", "x": 0.0, "y": 0.0, "fix_temperature": true, "temperature": 100.0, "heat_load": 0.0 },
                        { "id": "h1", "x": 1.0, "y": 0.0, "fix_temperature": false, "temperature": 0.0, "heat_load": 0.0 },
                        { "id": "h2", "x": 1.0, "y": 1.0, "fix_temperature": true, "temperature": 20.0, "heat_load": 0.0 },
                        { "id": "h3", "x": 0.0, "y": 1.0, "fix_temperature": true, "temperature": 20.0, "heat_load": 0.0 }
                    ],
                    "elements": [
                        { "id": "hq0", "node_i": 0, "node_j": 1, "node_k": 2, "node_l": 3, "thickness": 0.02, "conductivity": 45.0 }
                    ]
                }}),
            ),
            HeadlessWorkflowStep::new(
                "job_wait",
                json!({ "job_id": "{{steps.1.result.job_id}}", "interval_ms": 1000, "timeout_ms": 60000 }),
            ),
            HeadlessWorkflowStep::new("result_fetch", json!({ "job_id": "{{steps.1.result.job_id}}" })),
        ],
        "direct_thermal_quad" => vec![
            HeadlessWorkflowStep::new(
                "solve_thermal_plane_quad_2d",
                json!({ "model": {
                    "nodes": [
                        { "id": "t0", "x": 0.0, "y": 0.0, "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0, "temperature_delta": 30.0 },
                        { "id": "t1", "x": 1.0, "y": 0.0, "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0, "temperature_delta": 30.0 },
                        { "id": "t2", "x": 1.0, "y": 1.0, "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0, "temperature_delta": 30.0 },
                        { "id": "t3", "x": 0.0, "y": 1.0, "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0, "temperature_delta": 30.0 }
                    ],
                    "elements": [
                        { "id": "tq0", "node_i": 0, "node_j": 1, "node_k": 2, "node_l": 3, "thickness": 0.02, "youngs_modulus": 210000000000.0, "poisson_ratio": 0.3, "thermal_expansion": 0.000011 }
                    ]
                }}),
            ),
            HeadlessWorkflowStep::new(
                "job_wait",
                json!({ "job_id": "{{steps.1.result.job_id}}", "interval_ms": 1000, "timeout_ms": 60000 }),
            ),
            HeadlessWorkflowStep::new("result_fetch", json!({ "job_id": "{{steps.1.result.job_id}}" })),
        ],
        "direct_electrostatic_quad" => vec![
            HeadlessWorkflowStep::new(
                "solve_electrostatic_plane_quad_2d",
                json!({ "model": {
                    "nodes": [
                        { "id": "e0", "x": 0.0, "y": 0.0, "fix_potential": true, "potential": 10.0, "charge_density": 0.0 },
                        { "id": "e1", "x": 1.0, "y": 0.0, "fix_potential": false, "potential": 0.0, "charge_density": 0.0 },
                        { "id": "e2", "x": 1.0, "y": 1.0, "fix_potential": true, "potential": 0.0, "charge_density": 0.0 },
                        { "id": "e3", "x": 0.0, "y": 1.0, "fix_potential": true, "potential": 0.0, "charge_density": 0.0 }
                    ],
                    "elements": [
                        { "id": "eq0", "node_i": 0, "node_j": 1, "node_k": 2, "node_l": 3, "thickness": 0.05, "permittivity": 2.5 }
                    ]
                }}),
            ),
            HeadlessWorkflowStep::new(
                "job_wait",
                json!({ "job_id": "{{steps.1.result.job_id}}", "interval_ms": 1000, "timeout_ms": 60000 }),
            ),
            HeadlessWorkflowStep::new("result_fetch", json!({ "job_id": "{{steps.1.result.job_id}}" })),
        ],
        "browser_capture_review" => vec![
            HeadlessWorkflowStep::new(
                "open_page",
                json!({ "url": "https://example.com", "waitUntil": "domcontentloaded" }),
            ),
            HeadlessWorkflowStep::new("wait", json!({ "selector": "body", "timeout": 1500 })),
            HeadlessWorkflowStep::new(
                "snapshot",
                json!({ "file": "browser-review.png", "fullPage": true }),
            ),
        ],
        "browser_submit_then_poll" => vec![
            HeadlessWorkflowStep::new(
                "open_page",
                json!({ "url": "https://example.com/jobs", "waitUntil": "domcontentloaded" }),
            ),
            HeadlessWorkflowStep::new("click", json!({ "selector": "[data-run-job]" })),
            HeadlessWorkflowStep::new(
                "job_wait",
                json!({ "job_id": "job_123", "interval_ms": 1000, "timeout_ms": 60000 }),
            ),
            HeadlessWorkflowStep::new("result_fetch", json!({ "job_id": "job_123" })),
        ],
        _ => vec![],
    };
    HeadlessWorkflowDraft {
        id: workflow_id.to_string(),
        steps,
    }
}
