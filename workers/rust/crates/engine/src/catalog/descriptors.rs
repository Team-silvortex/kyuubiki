use kyuubiki_protocol::{
    OperatorDescriptor, OperatorKind, OperatorOrigin, OperatorPortDescriptor, OperatorSchemaRef,
    OperatorValidationProfile, OperatorValidationStatus,
};

pub fn built_in_solver_descriptor(
    id: &str,
    domain: &str,
    family: &str,
    summary: &str,
    capability_tags: &[&str],
) -> OperatorDescriptor {
    OperatorDescriptor {
        id: id.to_string(),
        version: "1.0.0".to_string(),
        domain: domain.to_string(),
        family: family.to_string(),
        kind: OperatorKind::Solver,
        summary: summary.to_string(),
        capability_tags: capability_tags
            .iter()
            .map(|tag| (*tag).to_string())
            .collect(),
        origin: OperatorOrigin::BuiltIn,
        input_schema: OperatorSchemaRef {
            schema: format!("kyuubiki.operator.{family}.input"),
            version: "1".to_string(),
        },
        output_schema: OperatorSchemaRef {
            schema: format!("kyuubiki.operator.{family}.output"),
            version: "1".to_string(),
        },
        inputs: vec![operator_port_descriptor(
            "model",
            &format!("model/{family}"),
            "Primary operator model input",
            Some("model"),
            Some(&format!("kyuubiki.operator.{family}.input")),
        )],
        outputs: vec![operator_port_descriptor(
            "result",
            &format!("result/{family}"),
            "Primary operator result output",
            Some("result"),
            Some(&format!("kyuubiki.operator.{family}.output")),
        )],
        validation: verified_operator_validation_profile(
            family,
            &["workflow_graph", "orchestrated_api"],
        ),
    }
}

pub fn built_in_bridge_descriptor(
    id: &str,
    domain: &str,
    family: &str,
    summary: &str,
    capability_tags: &[&str],
) -> OperatorDescriptor {
    OperatorDescriptor {
        id: id.to_string(),
        version: "1.0.0".to_string(),
        domain: domain.to_string(),
        family: family.to_string(),
        kind: OperatorKind::WorkflowBridge,
        summary: summary.to_string(),
        capability_tags: capability_tags
            .iter()
            .map(|tag| (*tag).to_string())
            .collect(),
        origin: OperatorOrigin::BuiltIn,
        input_schema: OperatorSchemaRef {
            schema: format!("kyuubiki.operator.{family}.bridge_input"),
            version: "1".to_string(),
        },
        output_schema: OperatorSchemaRef {
            schema: format!("kyuubiki.operator.{family}.bridge_output"),
            version: "1".to_string(),
        },
        inputs: vec![operator_port_descriptor(
            "source",
            &format!("result/{family}_bridge_source"),
            "Upstream workflow bridge payload",
            Some("upstream_result"),
            Some(&format!("kyuubiki.operator.{family}.bridge_input")),
        )],
        outputs: vec![operator_port_descriptor(
            "bridged_model",
            &format!("model/{family}"),
            "Downstream bridged model payload",
            Some("bridged_model"),
            Some(&format!("kyuubiki.operator.{family}.bridge_output")),
        )],
        validation: verified_operator_validation_profile(
            family,
            &["workflow_graph", "catalog_job"],
        ),
    }
}

pub fn built_in_transform_descriptor(
    id: &str,
    domain: &str,
    family: &str,
    summary: &str,
    capability_tags: &[&str],
) -> OperatorDescriptor {
    OperatorDescriptor {
        id: id.to_string(),
        version: "1.0.0".to_string(),
        domain: domain.to_string(),
        family: family.to_string(),
        kind: OperatorKind::Transform,
        summary: summary.to_string(),
        capability_tags: capability_tags
            .iter()
            .map(|tag| (*tag).to_string())
            .collect(),
        origin: OperatorOrigin::BuiltIn,
        input_schema: OperatorSchemaRef {
            schema: format!("kyuubiki.operator.{family}.transform_input"),
            version: "1".to_string(),
        },
        output_schema: OperatorSchemaRef {
            schema: format!("kyuubiki.operator.{family}.transform_output"),
            version: "1".to_string(),
        },
        inputs: vec![
            operator_port_descriptor(
                "left",
                "artifact/json",
                "Primary branch input payload",
                Some("left"),
                Some(&format!("kyuubiki.operator.{family}.transform_input")),
            ),
            operator_port_descriptor(
                "right",
                "artifact/json",
                "Secondary branch input payload",
                Some("right"),
                Some(&format!("kyuubiki.operator.{family}.transform_input")),
            ),
        ],
        outputs: vec![operator_port_descriptor(
            "merged",
            "artifact/json",
            "Merged branch payload",
            Some("merged"),
            Some(&format!("kyuubiki.operator.{family}.transform_output")),
        )],
        validation: verified_operator_validation_profile(
            family,
            &["workflow_graph", "draft_builder"],
        ),
    }
}

pub fn built_in_extract_descriptor(
    id: &str,
    domain: &str,
    family: &str,
    summary: &str,
    capability_tags: &[&str],
) -> OperatorDescriptor {
    OperatorDescriptor {
        id: id.to_string(),
        version: "1.0.0".to_string(),
        domain: domain.to_string(),
        family: family.to_string(),
        kind: OperatorKind::Extract,
        summary: summary.to_string(),
        capability_tags: capability_tags
            .iter()
            .map(|tag| (*tag).to_string())
            .collect(),
        origin: OperatorOrigin::BuiltIn,
        input_schema: OperatorSchemaRef {
            schema: format!("kyuubiki.operator.{family}.extract_input"),
            version: "1".to_string(),
        },
        output_schema: OperatorSchemaRef {
            schema: format!("kyuubiki.operator.{family}.extract_output"),
            version: "1".to_string(),
        },
        inputs: vec![operator_port_descriptor(
            "result",
            "result/any",
            "Result payload to extract from",
            Some("result"),
            Some(&format!("kyuubiki.operator.{family}.extract_input")),
        )],
        outputs: vec![operator_port_descriptor(
            "summary",
            &format!("extract/{family}"),
            "Extracted summary payload",
            Some("summary"),
            Some(&format!("kyuubiki.operator.{family}.extract_output")),
        )],
        validation: verified_operator_validation_profile(
            family,
            &["workflow_graph", "draft_builder"],
        ),
    }
}

pub fn built_in_export_descriptor(
    id: &str,
    domain: &str,
    family: &str,
    summary: &str,
    capability_tags: &[&str],
) -> OperatorDescriptor {
    OperatorDescriptor {
        id: id.to_string(),
        version: "1.0.0".to_string(),
        domain: domain.to_string(),
        family: family.to_string(),
        kind: OperatorKind::Export,
        summary: summary.to_string(),
        capability_tags: capability_tags
            .iter()
            .map(|tag| (*tag).to_string())
            .collect(),
        origin: OperatorOrigin::BuiltIn,
        input_schema: OperatorSchemaRef {
            schema: format!("kyuubiki.operator.{family}.export_input"),
            version: "1".to_string(),
        },
        output_schema: OperatorSchemaRef {
            schema: format!("kyuubiki.operator.{family}.export_output"),
            version: "1".to_string(),
        },
        inputs: vec![operator_port_descriptor(
            "summary",
            "extract/result_summary",
            "Summary payload to export",
            Some("summary"),
            Some(&format!("kyuubiki.operator.{family}.export_input")),
        )],
        outputs: vec![operator_port_descriptor(
            "export_artifact",
            &format!("export/{family}"),
            "Exported delivery artifact",
            Some("export_artifact"),
            Some(&format!("kyuubiki.operator.{family}.export_output")),
        )],
        validation: verified_operator_validation_profile(
            family,
            &["workflow_graph", "draft_builder"],
        ),
    }
}

fn operator_port_descriptor(
    id: &str,
    artifact_type: &str,
    description: &str,
    dataset_value: Option<&str>,
    schema: Option<&str>,
) -> OperatorPortDescriptor {
    OperatorPortDescriptor {
        id: id.to_string(),
        artifact_type: artifact_type.to_string(),
        description: description.to_string(),
        dataset_value: dataset_value.map(|value| value.to_string()),
        schema_ref: schema.map(|schema| OperatorSchemaRef {
            schema: schema.to_string(),
            version: "1".to_string(),
        }),
    }
}

fn verified_operator_validation_profile(
    family: &str,
    smoke_paths: &[&str],
) -> OperatorValidationProfile {
    OperatorValidationProfile {
        baseline_status: OperatorValidationStatus::Verified,
        baseline_cases: vec![format!("{family}_baseline")],
        smoke_paths: smoke_paths.iter().map(|path| (*path).to_string()).collect(),
    }
}
