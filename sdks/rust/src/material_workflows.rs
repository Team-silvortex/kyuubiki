use serde_json::{Value, json};

pub const MATERIAL_ENVELOPE_CATALOG_WORKFLOW_ID: &str =
    "workflow.material-study-envelope-ranking-json";
pub const MATERIAL_STUDY_EXECUTION_PLAN_SCHEMA_VERSION: &str =
    "kyuubiki.material-study-execution-plan/v1";

pub fn material_study_envelope_input_artifacts() -> Value {
    json!({
        "material_rows": {
            "rows": [
                {
                    "case_id": "cool_stiff",
                    "summaries": {
                        "thermal": {"max_temperature": 82.0},
                        "structural": {"max_stress": 100.0}
                    }
                },
                {
                    "case_id": "warm_safe",
                    "summaries": {
                        "thermal": {"max_temperature": 90.0},
                        "structural": {"max_stress": 120.0}
                    }
                },
                {
                    "case_id": "hot_light",
                    "summaries": {
                        "thermal": {"max_temperature": 140.0},
                        "structural": {"max_stress": 110.0}
                    }
                }
            ]
        }
    })
}

pub fn material_study_envelope_catalog_request(input_artifacts: Option<Value>) -> Value {
    json!({
        "workflow_id": MATERIAL_ENVELOPE_CATALOG_WORKFLOW_ID,
        "input_artifacts": input_artifacts.unwrap_or_else(material_study_envelope_input_artifacts)
    })
}

pub fn material_workflow_catalog() -> Value {
    json!([
        {
            "id": "material_study_envelope_catalog",
            "title": "Material Study Envelope Catalog Job",
            "domain": "multi_physics_materials",
            "objective": "submit the built-in material envelope workflow from the Orchestra catalog",
            "template_id": "material_study_envelope_catalog",
            "workflow_kind": "orchestra_catalog_job",
            "required_actions": ["workflow_submit_catalog", "job_wait", "result_fetch"],
            "aliases": ["material-envelope-catalog", "material_envelope_catalog"]
        },
        {
            "id": "material_study_envelope_ranking",
            "title": "Material Study Envelope Ranking",
            "domain": "multi_physics_materials",
            "objective": "compose material envelopes, rank candidates, and extract a Pareto frontier",
            "template_id": "material_study_envelope_ranking",
            "workflow_kind": "operator_graph",
            "required_actions": ["workflow_submit_graph", "job_wait", "result_fetch"],
            "aliases": ["material-envelope", "material_envelope", "material.pareto_ranking.v1"]
        }
    ])
}

pub fn material_study_execution_plan_example() -> Value {
    serde_json::from_str(include_str!(
        "../../../schemas/examples.material-study-execution-plan.json"
    ))
    .expect("bundled material study execution plan example should be valid JSON")
}
