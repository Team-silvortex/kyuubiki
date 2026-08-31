use kyuubiki_operator_sdk::{
    JsonOperator, OperatorDescriptorBuilder, OperatorJsonAbiBuffer, OperatorRegistry,
    OperatorSdkError, execute_operator_json_abi, free_operator_json_abi_buffer,
    operator_port_with_dataset, operator_summary_result, partial_validation,
};
use kyuubiki_protocol::{
    OperatorKind, OperatorRunContext, OperatorRunRequest, OperatorRunResult,
    OperatorTaskInputEnvelope,
};
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
    type Input = OperatorTaskInputEnvelope<TemplateInput>;

    fn descriptor(&self) -> &kyuubiki_protocol::OperatorDescriptor {
        &self.descriptor
    }

    fn run_typed(
        &self,
        input: Self::Input,
        _context: &OperatorRunContext,
    ) -> Result<OperatorRunResult, OperatorSdkError> {
        if input.payload.values.is_empty() {
            return Err(OperatorSdkError::Handler {
                operator_id: self.descriptor.id.clone(),
                message: "template_summary expects at least one numeric value".to_string(),
            });
        }

        let count = input.payload.values.len();
        let sum = input.payload.values.iter().sum::<f64>();
        let mean = sum / count as f64;
        let max = input
            .payload
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

pub fn install_template_operator(registry: &mut OperatorRegistry) -> Result<(), OperatorSdkError> {
    registry.register_json(TemplateSummaryOperator::new())
}

/// # Safety
///
/// The request pointer and output buffer must satisfy the stable JSON ABI
/// contract declared by `kyuubiki-operator-sdk`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn run_template_operator_json(
    request_ptr: *const u8,
    request_len: usize,
    output: *mut OperatorJsonAbiBuffer,
) -> i32 {
    unsafe {
        execute_operator_json_abi(request_ptr, request_len, output, |request| {
            let mut registry = OperatorRegistry::new();
            install_template_operator(&mut registry)?;
            registry.run(request)
        })
    }
}

/// # Safety
///
/// The buffer must have been allocated by `run_template_operator_json` and
/// must be returned exactly once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kyuubiki_operator_json_free(buffer: OperatorJsonAbiBuffer) {
    unsafe { free_operator_json_abi_buffer(buffer) }
}

pub fn run_template_operator(values: Vec<f64>) -> Result<OperatorRunResult, OperatorSdkError> {
    let mut registry = OperatorRegistry::new();
    install_template_operator(&mut registry)?;
    registry.run(OperatorRunRequest {
        operator_id: "extract.template_summary".to_string(),
        input: serde_json::json!({ "payload": { "values": values }, "config": {} }),
        context: OperatorRunContext::default(),
    })
}

#[cfg(test)]
mod tests {
    use super::{
        TemplateSummaryOperator, kyuubiki_operator_json_free, run_template_operator,
        run_template_operator_json,
    };
    use kyuubiki_operator_sdk::{
        OperatorJsonAbiBuffer, decode_operator_json_abi_response, operator_descriptor_readiness,
    };
    use kyuubiki_protocol::{OperatorRunContext, OperatorRunRequest};

    #[test]
    fn computes_template_summary() {
        let result = run_template_operator(vec![2.0, 4.0, 8.0]).expect("template operator");
        assert_eq!(result.summary["count"].as_u64(), Some(3));
        assert_eq!(result.summary["sum"].as_f64(), Some(14.0));
        assert_eq!(result.summary["mean"].as_f64(), Some(14.0 / 3.0));
        assert_eq!(result.summary["max"].as_f64(), Some(8.0));
    }

    #[test]
    fn descriptor_passes_operator_sdk_readiness() {
        let operator = TemplateSummaryOperator::new();
        let report = operator_descriptor_readiness(&operator.descriptor);
        assert!(report.ok, "{:?}", report.issues);
    }

    #[test]
    fn stable_json_abi_never_shares_rust_values_with_the_host() {
        let request = serde_json::to_vec(&OperatorRunRequest {
            operator_id: "extract.template_summary".to_string(),
            input: serde_json::json!({
                "payload": { "values": [2.0, 4.0, 8.0] },
                "config": {}
            }),
            context: OperatorRunContext::default(),
        })
        .expect("request JSON");
        let mut output = OperatorJsonAbiBuffer::empty();
        let status = unsafe {
            run_template_operator_json(request.as_ptr(), request.len(), &mut output)
        };
        let bytes = unsafe { std::slice::from_raw_parts(output.ptr, output.len) }.to_vec();
        unsafe { kyuubiki_operator_json_free(output) };
        let result = decode_operator_json_abi_response(status, &bytes).expect("ABI result");
        assert_eq!(result.summary["count"], 3);
        assert_eq!(result.summary["sum"], 14.0);
    }
}
