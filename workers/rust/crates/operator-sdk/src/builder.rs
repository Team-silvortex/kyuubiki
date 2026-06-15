use kyuubiki_protocol::{
    OperatorDescriptor, OperatorKind, OperatorOrigin, OperatorPortDescriptor, OperatorRunResult,
    OperatorSchemaRef, OperatorValidationProfile, OperatorValidationStatus,
};
use serde_json::Value;

pub struct OperatorDescriptorBuilder {
    descriptor: OperatorDescriptor,
}

impl OperatorDescriptorBuilder {
    pub fn new(
        id: impl Into<String>,
        kind: OperatorKind,
        domain: impl Into<String>,
        family: impl Into<String>,
    ) -> Self {
        let family = family.into();
        let schema_prefix = format!("kyuubiki.operator.{family}");
        Self {
            descriptor: OperatorDescriptor {
                id: id.into(),
                version: "1.0.0".to_string(),
                domain: domain.into(),
                family,
                kind,
                summary: String::new(),
                capability_tags: Vec::new(),
                origin: OperatorOrigin::ExternalLocal,
                input_schema: OperatorSchemaRef {
                    schema: format!("{schema_prefix}.input"),
                    version: "1".to_string(),
                },
                output_schema: OperatorSchemaRef {
                    schema: format!("{schema_prefix}.output"),
                    version: "1".to_string(),
                },
                inputs: Vec::new(),
                outputs: Vec::new(),
                validation: partial_validation("authoring_pending"),
            },
        }
    }

    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.descriptor.version = version.into();
        self
    }

    pub fn summary(mut self, summary: impl Into<String>) -> Self {
        self.descriptor.summary = summary.into();
        self
    }

    pub fn origin(mut self, origin: OperatorOrigin) -> Self {
        self.descriptor.origin = origin;
        self
    }

    pub fn capability_tag(mut self, tag: impl Into<String>) -> Self {
        self.descriptor.capability_tags.push(tag.into());
        self
    }

    pub fn capability_tags<I, S>(mut self, tags: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.descriptor
            .capability_tags
            .extend(tags.into_iter().map(Into::into));
        self
    }

    pub fn input_schema(mut self, schema: impl Into<String>, version: impl Into<String>) -> Self {
        self.descriptor.input_schema = OperatorSchemaRef {
            schema: schema.into(),
            version: version.into(),
        };
        self
    }

    pub fn output_schema(mut self, schema: impl Into<String>, version: impl Into<String>) -> Self {
        self.descriptor.output_schema = OperatorSchemaRef {
            schema: schema.into(),
            version: version.into(),
        };
        self
    }

    pub fn input_port(mut self, port: OperatorPortDescriptor) -> Self {
        self.descriptor.inputs.push(port);
        self
    }

    pub fn output_port(mut self, port: OperatorPortDescriptor) -> Self {
        self.descriptor.outputs.push(port);
        self
    }

    pub fn validation(mut self, validation: OperatorValidationProfile) -> Self {
        self.descriptor.validation = validation;
        self
    }

    pub fn build(self) -> OperatorDescriptor {
        self.descriptor
    }
}

pub fn operator_port(
    id: impl Into<String>,
    artifact_type: impl Into<String>,
    description: impl Into<String>,
) -> OperatorPortDescriptor {
    OperatorPortDescriptor {
        id: id.into(),
        artifact_type: artifact_type.into(),
        description: description.into(),
        dataset_value: None,
        schema_ref: None,
    }
}

pub fn operator_port_with_dataset(
    id: impl Into<String>,
    artifact_type: impl Into<String>,
    description: impl Into<String>,
    dataset_value: impl Into<String>,
) -> OperatorPortDescriptor {
    OperatorPortDescriptor {
        id: id.into(),
        artifact_type: artifact_type.into(),
        description: description.into(),
        dataset_value: Some(dataset_value.into()),
        schema_ref: None,
    }
}

pub fn verified_validation(case_id: impl Into<String>) -> OperatorValidationProfile {
    OperatorValidationProfile {
        baseline_status: OperatorValidationStatus::Verified,
        baseline_cases: vec![case_id.into()],
        smoke_paths: vec!["unit_test".to_string()],
    }
}

pub fn partial_validation(case_id: impl Into<String>) -> OperatorValidationProfile {
    OperatorValidationProfile {
        baseline_status: OperatorValidationStatus::Partial,
        baseline_cases: vec![case_id.into()],
        smoke_paths: vec!["unit_test".to_string()],
    }
}

pub fn operator_summary_result(
    operator_id: impl Into<String>,
    summary: Value,
) -> OperatorRunResult {
    OperatorRunResult {
        operator_id: operator_id.into(),
        summary,
        artifacts: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        OperatorDescriptorBuilder, operator_port_with_dataset, operator_summary_result,
        verified_validation,
    };
    use kyuubiki_protocol::{OperatorKind, OperatorOrigin, OperatorValidationStatus};

    #[test]
    fn builds_descriptor_with_ports_and_validation() {
        let descriptor = OperatorDescriptorBuilder::new(
            "extract.temperature_peak",
            OperatorKind::Extract,
            "thermal",
            "temperature_peak",
        )
        .version("1.2.0")
        .summary("Extract the peak temperature from a thermal field.")
        .origin(OperatorOrigin::ExternalLocal)
        .capability_tags(["thermal", "postprocess", "headless_safe"])
        .input_port(operator_port_with_dataset(
            "result",
            "result/heat_plane_quad_2d",
            "Heat solve result",
            "heat_plane_quad_2d_result",
        ))
        .output_port(operator_port_with_dataset(
            "summary",
            "artifact/json",
            "Peak temperature summary",
            "peak_temperature_summary",
        ))
        .validation(verified_validation("temperature_peak_baseline"))
        .build();

        assert_eq!(descriptor.version, "1.2.0");
        assert_eq!(descriptor.inputs.len(), 1);
        assert_eq!(descriptor.outputs.len(), 1);
        assert_eq!(descriptor.origin, OperatorOrigin::ExternalLocal);
        assert_eq!(
            descriptor.validation.baseline_status,
            OperatorValidationStatus::Verified
        );
    }

    #[test]
    fn builds_summary_result_without_artifacts() {
        let result = operator_summary_result(
            "extract.temperature_peak",
            serde_json::json!({ "peak_temperature": 125.0 }),
        );

        assert_eq!(result.operator_id, "extract.temperature_peak");
        assert_eq!(result.summary["peak_temperature"].as_f64(), Some(125.0));
        assert!(result.artifacts.is_empty());
    }
}
