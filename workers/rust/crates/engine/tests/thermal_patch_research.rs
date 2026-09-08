#[path = "support/research_evidence.rs"]
mod evidence;
#[path = "../../../../../sdks/rust/examples/layered_thermal_research/patch.rs"]
mod patch;
use kyuubiki_engine::run_workflow_graph;
use serde_json::json;

#[test]
fn thermal_patch_matrix_covers_free_clamped_poisson_cooling_and_mapping_modes() {
    let cases = patch::cases();
    assert_eq!(cases.len(), 288);
    let mut evidence = evidence::Evidence::new("thermal-patch", cases.len());
    for case in cases {
        let (graph, input_artifacts) = case.workflow();
        let request_value = json!({"graph":graph,"input_artifacts":input_artifacts});
        let request = serde_json::from_value(request_value.clone()).unwrap();
        let result = serde_json::to_value(
            run_workflow_graph(request).unwrap_or_else(|error| panic!("{}: {error}", case.id())),
        )
        .unwrap();
        let report = case.validate(&result).unwrap();
        evidence.record(&request_value, &result, &report);
        assert_eq!(report["passed"], true, "{report}");
    }
    evidence.finish();
}

#[test]
fn thermal_patch_checks_reject_missing_or_wrong_signed_stress() {
    let case = patch::cases()
        .into_iter()
        .find(|case| case.clamped && case.rise > 0.0)
        .unwrap();
    let (graph, input_artifacts) = case.workflow();
    let request =
        serde_json::from_value(json!({"graph":graph,"input_artifacts":input_artifacts})).unwrap();
    let mut result = serde_json::to_value(run_workflow_graph(request).unwrap()).unwrap();
    result["artifacts"]["structure_out.result"]["elements"][0]["stress_x"] = json!(1e6);
    assert_eq!(case.validate(&result).unwrap()["passed"], false);
    result["artifacts"]["structure_out.result"]["elements"][0]["stress_x"] =
        serde_json::Value::Null;
    assert!(case.validate(&result).is_err());
}
