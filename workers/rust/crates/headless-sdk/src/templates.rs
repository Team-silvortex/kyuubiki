use crate::{
    HeadlessTemplateDescriptor, HeadlessWorkflowDocument,
};
use crate::template_workflows::build_template_workflow;

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
