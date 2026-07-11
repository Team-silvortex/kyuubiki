use crate::catalog::describe_built_in_operator;
use crate::operator_sdk_bridges::register_bridge_transform_operators;
use crate::operator_sdk_material_envelope::register_material_envelope_operator;
use crate::operator_sdk_material_margins::register_material_margin_operator;
use crate::operator_sdk_material_pareto::register_material_pareto_operator;
use crate::operator_sdk_parameter_sweep::register_parameter_sweep_operators;
use crate::operator_sdk_quality_objective::register_quality_objective_operators;
use crate::operator_sdk_workflow_extensions::{
    register_workflow_export_extensions, register_workflow_extract_extensions,
    register_workflow_transform_extensions,
};
use crate::workflow_reporting::{
    export_alert_markdown, export_summary_csv, export_summary_json, extract_field_hotspots,
    extract_field_statistics, extract_result_summary,
};
use crate::workflow_summary_transforms::{
    aggregate_summary_collection, compare_summary_pair, normalize_summary_fields,
    select_best_summary,
};
use crate::workflow_summary_validation::validate_summary_tolerance;
use kyuubiki_operator_sdk::{JsonOperator, OperatorRegistry, OperatorSdkError};
use kyuubiki_protocol::{
    OperatorDescriptor, OperatorRunContext, OperatorRunRequest, OperatorRunResult,
};
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltInOperatorRegistryKind {
    Extract,
    Export,
    Transform,
}

#[derive(Debug, Deserialize)]
pub(crate) struct WorkflowOperatorEnvelope {
    pub(crate) payload: Value,
    #[serde(default)]
    pub(crate) config: Value,
}

struct ResultSummaryOperator {
    descriptor: OperatorDescriptor,
}

struct FieldStatisticsOperator {
    descriptor: OperatorDescriptor,
}

struct FieldHotspotsOperator {
    descriptor: OperatorDescriptor,
}

struct SummaryJsonOperator {
    descriptor: OperatorDescriptor,
}

struct SummaryCsvOperator {
    descriptor: OperatorDescriptor,
}

struct AlertMarkdownOperator {
    descriptor: OperatorDescriptor,
}

struct FirstAvailableOperator {
    descriptor: OperatorDescriptor,
}

struct MergeSummaryPairOperator {
    descriptor: OperatorDescriptor,
}

struct CompareSummaryPairOperator {
    descriptor: OperatorDescriptor,
}

struct ValidateSummaryToleranceOperator {
    descriptor: OperatorDescriptor,
}

struct AggregateSummaryCollectionOperator {
    descriptor: OperatorDescriptor,
}

struct NormalizeSummaryFieldsOperator {
    descriptor: OperatorDescriptor,
}

struct SelectBestSummaryOperator {
    descriptor: OperatorDescriptor,
}

impl JsonOperator for ResultSummaryOperator {
    type Input = WorkflowOperatorEnvelope;

    fn descriptor(&self) -> &OperatorDescriptor {
        &self.descriptor
    }

    fn run_typed(
        &self,
        input: Self::Input,
        _context: &OperatorRunContext,
    ) -> Result<OperatorRunResult, OperatorSdkError> {
        run_summary_only(
            &self.descriptor.id,
            extract_result_summary(input.payload, input.config),
        )
    }
}

impl JsonOperator for FieldStatisticsOperator {
    type Input = WorkflowOperatorEnvelope;

    fn descriptor(&self) -> &OperatorDescriptor {
        &self.descriptor
    }

    fn run_typed(
        &self,
        input: Self::Input,
        _context: &OperatorRunContext,
    ) -> Result<OperatorRunResult, OperatorSdkError> {
        run_summary_only(
            &self.descriptor.id,
            extract_field_statistics(input.payload, input.config),
        )
    }
}

impl JsonOperator for FieldHotspotsOperator {
    type Input = WorkflowOperatorEnvelope;

    fn descriptor(&self) -> &OperatorDescriptor {
        &self.descriptor
    }

    fn run_typed(
        &self,
        input: Self::Input,
        _context: &OperatorRunContext,
    ) -> Result<OperatorRunResult, OperatorSdkError> {
        run_summary_only(
            &self.descriptor.id,
            extract_field_hotspots(input.payload, input.config),
        )
    }
}

impl JsonOperator for SummaryJsonOperator {
    type Input = WorkflowOperatorEnvelope;

    fn descriptor(&self) -> &OperatorDescriptor {
        &self.descriptor
    }

    fn run_typed(
        &self,
        input: Self::Input,
        _context: &OperatorRunContext,
    ) -> Result<OperatorRunResult, OperatorSdkError> {
        run_summary_only(&self.descriptor.id, export_summary_json(input.payload))
    }
}

impl JsonOperator for SummaryCsvOperator {
    type Input = WorkflowOperatorEnvelope;

    fn descriptor(&self) -> &OperatorDescriptor {
        &self.descriptor
    }

    fn run_typed(
        &self,
        input: Self::Input,
        _context: &OperatorRunContext,
    ) -> Result<OperatorRunResult, OperatorSdkError> {
        run_summary_only(
            &self.descriptor.id,
            export_summary_csv(input.payload, input.config),
        )
    }
}

impl JsonOperator for AlertMarkdownOperator {
    type Input = WorkflowOperatorEnvelope;

    fn descriptor(&self) -> &OperatorDescriptor {
        &self.descriptor
    }

    fn run_typed(
        &self,
        input: Self::Input,
        _context: &OperatorRunContext,
    ) -> Result<OperatorRunResult, OperatorSdkError> {
        run_summary_only(
            &self.descriptor.id,
            export_alert_markdown(input.payload, input.config),
        )
    }
}

impl JsonOperator for FirstAvailableOperator {
    type Input = WorkflowOperatorEnvelope;

    fn descriptor(&self) -> &OperatorDescriptor {
        &self.descriptor
    }

    fn run_typed(
        &self,
        input: Self::Input,
        _context: &OperatorRunContext,
    ) -> Result<OperatorRunResult, OperatorSdkError> {
        run_summary_only(&self.descriptor.id, Ok(input.payload))
    }
}

impl JsonOperator for MergeSummaryPairOperator {
    type Input = WorkflowOperatorEnvelope;

    fn descriptor(&self) -> &OperatorDescriptor {
        &self.descriptor
    }

    fn run_typed(
        &self,
        input: Self::Input,
        _context: &OperatorRunContext,
    ) -> Result<OperatorRunResult, OperatorSdkError> {
        run_summary_only(
            &self.descriptor.id,
            crate::workflow_reporting::merge_summary_pair(input.payload, input.config),
        )
    }
}

impl JsonOperator for CompareSummaryPairOperator {
    type Input = WorkflowOperatorEnvelope;

    fn descriptor(&self) -> &OperatorDescriptor {
        &self.descriptor
    }

    fn run_typed(
        &self,
        input: Self::Input,
        _context: &OperatorRunContext,
    ) -> Result<OperatorRunResult, OperatorSdkError> {
        run_summary_only(
            &self.descriptor.id,
            compare_summary_pair(input.payload, input.config),
        )
    }
}

impl JsonOperator for ValidateSummaryToleranceOperator {
    type Input = WorkflowOperatorEnvelope;

    fn descriptor(&self) -> &OperatorDescriptor {
        &self.descriptor
    }

    fn run_typed(
        &self,
        input: Self::Input,
        _context: &OperatorRunContext,
    ) -> Result<OperatorRunResult, OperatorSdkError> {
        run_summary_only(
            &self.descriptor.id,
            validate_summary_tolerance(input.payload, input.config),
        )
    }
}

impl JsonOperator for AggregateSummaryCollectionOperator {
    type Input = WorkflowOperatorEnvelope;

    fn descriptor(&self) -> &OperatorDescriptor {
        &self.descriptor
    }

    fn run_typed(
        &self,
        input: Self::Input,
        _context: &OperatorRunContext,
    ) -> Result<OperatorRunResult, OperatorSdkError> {
        run_summary_only(
            &self.descriptor.id,
            aggregate_summary_collection(input.payload, input.config),
        )
    }
}

impl JsonOperator for NormalizeSummaryFieldsOperator {
    type Input = WorkflowOperatorEnvelope;

    fn descriptor(&self) -> &OperatorDescriptor {
        &self.descriptor
    }

    fn run_typed(
        &self,
        input: Self::Input,
        _context: &OperatorRunContext,
    ) -> Result<OperatorRunResult, OperatorSdkError> {
        run_summary_only(
            &self.descriptor.id,
            normalize_summary_fields(input.payload, input.config),
        )
    }
}

impl JsonOperator for SelectBestSummaryOperator {
    type Input = WorkflowOperatorEnvelope;

    fn descriptor(&self) -> &OperatorDescriptor {
        &self.descriptor
    }

    fn run_typed(
        &self,
        input: Self::Input,
        _context: &OperatorRunContext,
    ) -> Result<OperatorRunResult, OperatorSdkError> {
        run_summary_only(
            &self.descriptor.id,
            select_best_summary(input.payload, input.config),
        )
    }
}

pub fn run_registered_extract_operator(
    operator_id: &str,
    payload: Value,
    config: Value,
) -> Result<Value, String> {
    run_with_registry(
        built_in_operator_registry(BuiltInOperatorRegistryKind::Extract),
        operator_id,
        payload,
        config,
    )
}

pub fn run_registered_export_operator(
    operator_id: &str,
    payload: Value,
    config: Value,
) -> Result<Value, String> {
    run_with_registry(
        built_in_operator_registry(BuiltInOperatorRegistryKind::Export),
        operator_id,
        payload,
        config,
    )
}

pub fn run_registered_transform_operator(
    operator_id: &str,
    payload: Value,
    config: Value,
) -> Result<Value, String> {
    run_with_registry(
        built_in_operator_registry(BuiltInOperatorRegistryKind::Transform),
        operator_id,
        payload,
        config,
    )
}

fn run_with_registry(
    registry: OperatorRegistry,
    operator_id: &str,
    payload: Value,
    config: Value,
) -> Result<Value, String> {
    let result = registry
        .run(OperatorRunRequest {
            operator_id: operator_id.to_string(),
            input: serde_json::json!({
                "payload": payload,
                "config": config,
            }),
            context: OperatorRunContext::default(),
        })
        .map_err(|error| error.to_string())?;
    Ok(result.summary)
}

pub fn built_in_operator_registry(kind: BuiltInOperatorRegistryKind) -> OperatorRegistry {
    let mut registry = OperatorRegistry::new();
    match kind {
        BuiltInOperatorRegistryKind::Extract => {
            registry
                .register_json(ResultSummaryOperator {
                    descriptor: descriptor("extract.result_summary"),
                })
                .expect("extract.result_summary should register");
            registry
                .register_json(FieldStatisticsOperator {
                    descriptor: descriptor("extract.field_statistics"),
                })
                .expect("extract.field_statistics should register");
            registry
                .register_json(FieldHotspotsOperator {
                    descriptor: descriptor("extract.field_hotspots"),
                })
                .expect("extract.field_hotspots should register");
            register_workflow_extract_extensions(&mut registry);
        }
        BuiltInOperatorRegistryKind::Export => {
            registry
                .register_json(SummaryJsonOperator {
                    descriptor: descriptor("export.summary_json"),
                })
                .expect("export.summary_json should register");
            registry
                .register_json(SummaryCsvOperator {
                    descriptor: descriptor("export.summary_csv"),
                })
                .expect("export.summary_csv should register");
            registry
                .register_json(AlertMarkdownOperator {
                    descriptor: descriptor("export.alert_markdown"),
                })
                .expect("export.alert_markdown should register");
            register_workflow_export_extensions(&mut registry);
        }
        BuiltInOperatorRegistryKind::Transform => {
            register_bridge_transform_operators(&mut registry, descriptor);
            registry
                .register_json(FirstAvailableOperator {
                    descriptor: descriptor("transform.first_available"),
                })
                .expect("transform.first_available should register");
            registry
                .register_json(MergeSummaryPairOperator {
                    descriptor: descriptor("transform.merge_summary_pair"),
                })
                .expect("transform.merge_summary_pair should register");
            registry
                .register_json(CompareSummaryPairOperator {
                    descriptor: descriptor("transform.compare_summary_pair"),
                })
                .expect("transform.compare_summary_pair should register");
            registry
                .register_json(ValidateSummaryToleranceOperator {
                    descriptor: descriptor("transform.validate_summary_tolerance"),
                })
                .expect("transform.validate_summary_tolerance should register");
            registry
                .register_json(AggregateSummaryCollectionOperator {
                    descriptor: descriptor("transform.aggregate_summary_collection"),
                })
                .expect("transform.aggregate_summary_collection should register");
            registry
                .register_json(NormalizeSummaryFieldsOperator {
                    descriptor: descriptor("transform.normalize_summary_fields"),
                })
                .expect("transform.normalize_summary_fields should register");
            registry
                .register_json(SelectBestSummaryOperator {
                    descriptor: descriptor("transform.select_best_summary"),
                })
                .expect("transform.select_best_summary should register");
            register_quality_objective_operators(&mut registry, descriptor);
            register_parameter_sweep_operators(&mut registry, descriptor);
            register_material_envelope_operator(&mut registry, descriptor);
            register_material_margin_operator(&mut registry, descriptor);
            register_material_pareto_operator(&mut registry, descriptor);
            register_workflow_transform_extensions(&mut registry);
        }
    }
    registry
}

fn descriptor(operator_id: &str) -> OperatorDescriptor {
    describe_built_in_operator(operator_id)
        .unwrap_or_else(|| panic!("missing built-in descriptor for {operator_id}"))
}

pub(crate) fn run_summary_only(
    operator_id: &str,
    result: Result<Value, String>,
) -> Result<OperatorRunResult, OperatorSdkError> {
    result
        .map(|summary| OperatorRunResult {
            operator_id: operator_id.to_string(),
            summary,
            artifacts: Vec::new(),
        })
        .map_err(|message| OperatorSdkError::Handler {
            operator_id: operator_id.to_string(),
            message,
        })
}
