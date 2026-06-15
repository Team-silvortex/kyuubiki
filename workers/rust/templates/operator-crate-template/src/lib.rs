use kyuubiki_operator_sdk::{
    JsonOperator, OperatorDescriptorBuilder, OperatorRegistry, OperatorSdkError,
    operator_port_with_dataset, operator_summary_result, partial_validation,
};
use kyuubiki_protocol::{OperatorKind, OperatorRunContext, OperatorRunRequest, OperatorRunResult};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct TemplateInput {
    pub values: Vec<f64>,
}

pub struct TemplateSummaryOperator {
    descriptor: kyuubiki_protocol::OperatorDescriptor,
}

impl TemplateSummaryOperator {
    pub fn new() -> Self {
        Self {
            descriptor: OperatorDescriptorBuilder::new(
                "extract.template_summary",
                OperatorKind::Extract,
                "multi_domain",
                "template_summary",
            )
            .summary("Template operator that extracts basic summary statistics.")
            .capability_tags(["template", "example", "headless_safe"])
            .input_port(operator_port_with_dataset(
                "input",
                "artifact/json",
                "Template input payload",
                "template_input",
            ))
            .output_port(operator_port_with_dataset(
                "summary",
                "artifact/json",
                "Template summary payload",
                "template_summary",
            ))
            .validation(partial_validation("template_summary_example"))
            .build(),
        }
    }
}

impl Default for TemplateSummaryOperator {
    fn default() -> Self {
        Self::new()
    }
}

impl JsonOperator for TemplateSummaryOperator {
    type Input = TemplateInput;

    fn descriptor(&self) -> &kyuubiki_protocol::OperatorDescriptor {
        &self.descriptor
    }

    fn run_typed(
        &self,
        input: Self::Input,
        _context: &OperatorRunContext,
    ) -> Result<OperatorRunResult, OperatorSdkError> {
        if input.values.is_empty() {
            return Err(OperatorSdkError::Handler {
                operator_id: self.descriptor.id.clone(),
                message: "template_summary expects at least one numeric value".to_string(),
            });
        }

        let count = input.values.len();
        let sum = input.values.iter().sum::<f64>();
        let mean = sum / count as f64;
        let max = input
            .values
            .iter()
            .copied()
            .reduce(f64::max)
            .unwrap_or(mean);

        Ok(operator_summary_result(
            self.descriptor.id.clone(),
            serde_json::json!({
                "count": count,
                "sum": sum,
                "mean": mean,
                "max": max,
            }),
        ))
    }
}

pub fn install_template_operator(
    registry: &mut OperatorRegistry,
) -> Result<(), OperatorSdkError> {
    registry.register_json(TemplateSummaryOperator::new())
}

#[unsafe(no_mangle)]
/// # Safety
///
/// This symbol is loaded by the Kyuubiki runtime host from a trusted operator
/// package built against the same SDK/runtime contract.
pub unsafe fn register_template_operator(
    registry: &mut OperatorRegistry,
) -> Result<(), OperatorSdkError> {
    install_template_operator(registry)
}

pub fn run_template_operator(values: Vec<f64>) -> Result<OperatorRunResult, OperatorSdkError> {
    let mut registry = OperatorRegistry::new();
    install_template_operator(&mut registry)?;
    registry.run(OperatorRunRequest {
        operator_id: "extract.template_summary".to_string(),
        input: serde_json::json!({ "values": values }),
        context: OperatorRunContext::default(),
    })
}

#[cfg(test)]
mod tests {
    use super::run_template_operator;

    #[test]
    fn computes_template_summary() {
        let result = run_template_operator(vec![2.0, 4.0, 8.0]).expect("template operator");
        assert_eq!(result.summary["count"].as_u64(), Some(3));
        assert_eq!(result.summary["sum"].as_f64(), Some(14.0));
        assert_eq!(result.summary["mean"].as_f64(), Some(14.0 / 3.0));
        assert_eq!(result.summary["max"].as_f64(), Some(8.0));
    }
}
