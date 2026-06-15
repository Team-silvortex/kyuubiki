use kyuubiki_operator_sdk::{
    JsonOperator, OperatorDescriptorBuilder, OperatorRegistry, OperatorSdkError,
    operator_port_with_dataset, operator_summary_result, partial_validation,
};
use kyuubiki_protocol::{OperatorKind, OperatorRunContext, OperatorRunRequest, OperatorRunResult};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct PeakTemperatureInput {
    temperatures: Vec<f64>,
}

struct PeakTemperatureOperator {
    descriptor: kyuubiki_protocol::OperatorDescriptor,
}

impl PeakTemperatureOperator {
    fn new() -> Self {
        Self {
            descriptor: OperatorDescriptorBuilder::new(
                "extract.peak_temperature",
                OperatorKind::Extract,
                "thermal",
                "peak_temperature",
            )
            .summary("Extract the peak temperature from a flat temperature collection.")
            .capability_tags(["thermal", "postprocess", "headless_safe"])
            .input_port(operator_port_with_dataset(
                "result",
                "artifact/json",
                "Temperature collection input",
                "temperature_collection",
            ))
            .output_port(operator_port_with_dataset(
                "summary",
                "artifact/json",
                "Peak temperature summary output",
                "peak_temperature_summary",
            ))
            .validation(partial_validation("peak_temperature_example"))
            .build(),
        }
    }
}

impl JsonOperator for PeakTemperatureOperator {
    type Input = PeakTemperatureInput;

    fn descriptor(&self) -> &kyuubiki_protocol::OperatorDescriptor {
        &self.descriptor
    }

    fn run_typed(
        &self,
        input: Self::Input,
        _context: &OperatorRunContext,
    ) -> Result<OperatorRunResult, OperatorSdkError> {
        let peak = input
            .temperatures
            .iter()
            .copied()
            .reduce(f64::max)
            .ok_or_else(|| OperatorSdkError::Handler {
                operator_id: self.descriptor.id.clone(),
                message: "peak_temperature expects at least one temperature value".to_string(),
            })?;
        Ok(operator_summary_result(
            self.descriptor.id.clone(),
            serde_json::json!({
                "peak_temperature": peak,
                "sample_count": input.temperatures.len(),
            }),
        ))
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut registry = OperatorRegistry::new();
    registry.register_json(PeakTemperatureOperator::new())?;

    let result = registry.run(OperatorRunRequest {
        operator_id: "extract.peak_temperature".to_string(),
        input: serde_json::json!({
            "temperatures": [42.0, 87.5, 63.25]
        }),
        context: OperatorRunContext::default(),
    })?;

    println!("{}", serde_json::to_string_pretty(&result.summary)?);
    Ok(())
}
