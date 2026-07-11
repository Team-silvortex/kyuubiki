use crate::workflow_executor::run_transform_operator;

#[test]
fn runs_summary_tolerance_validation_through_sdk_registry() {
    let summary = run_transform_operator(
        "transform.validate_summary_tolerance",
        serde_json::json!({
            "left": { "max_stress": 10.0, "max_displacement": 1.5 },
            "right": { "max_stress": 10.01, "max_displacement": 1.5005 }
        }),
        serde_json::json!({
            "fields": ["max_stress", "max_displacement"],
            "absolute_tolerance": 0.02,
            "relative_tolerance": 0.001
        }),
    )
    .expect("transform.validate_summary_tolerance should succeed");

    assert_eq!(
        summary["validation_contract"].as_str(),
        Some("kyuubiki.summary_tolerance_validation/v1")
    );
    assert_eq!(summary["validation_passed"].as_bool(), Some(true));
    assert_eq!(summary["validation_checked_field_count"].as_u64(), Some(2));
}
