use crate::{describe_built_in_operator, workflow_executor::is_supported_workflow_operator};

#[test]
fn quality_round_descriptors_expose_coupled_readiness_tags() {
    for operator_id in [
        "transform.prepare_quality_next_round_request",
        "transform.build_quality_parameter_sweep_plan",
        "transform.compose_quality_lineage_report",
    ] {
        let descriptor = describe_built_in_operator(operator_id).expect("descriptor should exist");
        assert!(
            descriptor
                .capability_tags
                .iter()
                .any(|tag| tag == "coupled"),
            "{operator_id} should expose coupled tag"
        );
        assert!(
            descriptor
                .capability_tags
                .iter()
                .any(|tag| tag == "readiness"),
            "{operator_id} should expose readiness tag"
        );
        assert!(
            descriptor.summary.contains("readiness")
                || descriptor.summary.contains("coupled-readiness"),
            "{operator_id} summary should mention readiness"
        );
    }
}

#[test]
fn coupled_readiness_descriptors_match_runtime_support() {
    for operator_id in [
        "transform.evaluate_coupled_readiness",
        "transform.prepare_quality_next_round_request",
        "transform.build_quality_parameter_sweep_plan",
        "transform.compose_quality_lineage_report",
    ] {
        assert!(
            describe_built_in_operator(operator_id).is_some(),
            "{operator_id} should have a built-in descriptor"
        );
        assert!(
            is_supported_workflow_operator(operator_id),
            "{operator_id} should be executable by the workflow runtime"
        );
    }
}
