use kyuubiki_headless_sdk::{
    MATERIAL_ENVELOPE_CATALOG_WORKFLOW_ID, MATERIAL_STUDY_EXECUTION_PLAN_SCHEMA_VERSION,
    material_study_envelope_catalog_request, material_study_envelope_input_artifacts,
    material_study_execution_plan_example, material_workflow_catalog,
};
use serde_json::json;

fn material_envelope_fixture() -> serde_json::Value {
    let mut fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../schemas/examples.material-envelope-catalog-request.json"
    ))
    .expect("material envelope catalog request fixture");
    fixture
        .as_object_mut()
        .expect("fixture object")
        .remove("$schema");
    fixture
}

#[test]
fn catalog_prefers_orchestra_catalog_path() {
    let catalog = material_workflow_catalog();

    assert_eq!(catalog[0]["id"], "material_study_envelope_catalog");
    assert_eq!(catalog[0]["workflow_kind"], "orchestra_catalog_job");
    assert_eq!(catalog[0]["required_actions"][0], "workflow_submit_catalog");
    assert_eq!(catalog[1]["workflow_kind"], "operator_graph");
}

#[test]
fn catalog_request_uses_builtin_workflow_id() {
    let request = material_study_envelope_catalog_request(None);

    assert_eq!(
        request["workflow_id"],
        json!(MATERIAL_ENVELOPE_CATALOG_WORKFLOW_ID)
    );
    assert_eq!(request, material_envelope_fixture());
    assert_eq!(
        request["input_artifacts"]["material_rows"]["rows"][0]["case_id"],
        "cool_stiff"
    );
}

#[test]
fn catalog_request_accepts_explicit_input_artifacts() {
    let input_artifacts = json!({
        "material_rows": {
            "rows": [{"case_id": "candidate-a", "summaries": {}}]
        }
    });
    let request = material_study_envelope_catalog_request(Some(input_artifacts));

    assert_eq!(
        request["input_artifacts"]["material_rows"]["rows"][0]["case_id"],
        "candidate-a"
    );
}

#[test]
fn input_artifacts_can_be_mutated_without_global_state() {
    let mut first = material_study_envelope_input_artifacts();
    first["material_rows"]["rows"][0]["case_id"] = json!("mutated");
    let second = material_study_envelope_input_artifacts();

    assert_eq!(second["material_rows"]["rows"][0]["case_id"], "cool_stiff");
}

#[test]
fn material_study_execution_plan_example_matches_shared_contract() {
    let plan = material_study_execution_plan_example();

    assert_eq!(
        plan["schema_version"],
        json!(MATERIAL_STUDY_EXECUTION_PLAN_SCHEMA_VERSION)
    );
    assert_eq!(plan["study_id"], "material_heat_spreader_screening");
    assert_eq!(plan["step_count"], plan["steps"].as_array().unwrap().len());
    assert_eq!(plan["solve_step_count"], 3);
    assert_eq!(plan["candidate_count"], 3);
    assert_eq!(plan["material_card_contract_required"], true);
    assert_eq!(
        plan["material_card_schema_version"],
        "kyuubiki.material-card/v1"
    );
    assert_eq!(plan["material_card_ref_count"], 3);
    assert!(
        plan["candidate_ids"]
            .as_array()
            .unwrap()
            .iter()
            .any(|candidate| candidate == "copper_c110")
    );
    assert!(
        plan["recommended_command"]
            .as_str()
            .unwrap()
            .contains("heat-spreader")
    );
}
