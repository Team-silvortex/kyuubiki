use kyuubiki_operator_sdk::{
    JsonOperator, OperatorDescriptorBuilder, OperatorRegistry, OperatorSdkError,
    operator_port_with_dataset, operator_summary_result, partial_validation,
};
use kyuubiki_protocol::{
    OperatorKind, OperatorRunContext, OperatorRunRequest, OperatorRunResult,
    SolveElectrostaticPlaneQuad2dResult,
};

struct PeakFieldOperator {
    descriptor: kyuubiki_protocol::OperatorDescriptor,
}

impl PeakFieldOperator {
    fn new() -> Self {
        Self {
            descriptor: OperatorDescriptorBuilder::new(
                "extract.electrostatic_peak_field",
                OperatorKind::Extract,
                "electromagnetic",
                "electrostatic_peak_field",
            )
            .summary("Extract the peak electric-field and flux-density metrics from an electrostatic quad result.")
            .capability_tags(["electromagnetic", "electrostatic", "postprocess", "headless_safe"])
            .input_port(operator_port_with_dataset(
                "result",
                "result/electrostatic_plane_quad_2d",
                "Electrostatic plane quad solve result",
                "electrostatic_plane_quad_2d_result",
            ))
            .output_port(operator_port_with_dataset(
                "summary",
                "artifact/json",
                "Peak field summary",
                "electrostatic_peak_field_summary",
            ))
            .validation(partial_validation("electrostatic_peak_field_example"))
            .build(),
        }
    }
}

impl JsonOperator for PeakFieldOperator {
    type Input = SolveElectrostaticPlaneQuad2dResult;

    fn descriptor(&self) -> &kyuubiki_protocol::OperatorDescriptor {
        &self.descriptor
    }

    fn run_typed(
        &self,
        input: Self::Input,
        _context: &OperatorRunContext,
    ) -> Result<OperatorRunResult, OperatorSdkError> {
        let peak_element = input
            .elements
            .iter()
            .max_by(|left, right| {
                left.electric_field_magnitude
                    .partial_cmp(&right.electric_field_magnitude)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .ok_or_else(|| OperatorSdkError::Handler {
                operator_id: self.descriptor.id.clone(),
                message: "electrostatic_peak_field expects at least one element".to_string(),
            })?;

        Ok(operator_summary_result(
            self.descriptor.id.clone(),
            serde_json::json!({
                "peak_element_id": peak_element.id,
                "peak_electric_field": peak_element.electric_field_magnitude,
                "peak_flux_density": peak_element.electric_flux_density_magnitude,
                "max_potential": input.max_potential,
                "max_electric_field": input.max_electric_field,
                "max_flux_density": input.max_flux_density,
            }),
        ))
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut registry = OperatorRegistry::new();
    registry.register_json(PeakFieldOperator::new())?;

    let result = registry.run(OperatorRunRequest {
        operator_id: "extract.electrostatic_peak_field".to_string(),
        input: serde_json::json!({
            "input": {
                "nodes": [],
                "elements": []
            },
            "nodes": [
                { "index": 0, "id": "n0", "x": 0.0, "y": 0.0, "potential": 10.0, "charge_density": 0.0 },
                { "index": 1, "id": "n1", "x": 1.0, "y": 0.0, "potential": 0.0, "charge_density": 0.0 },
                { "index": 2, "id": "n2", "x": 1.0, "y": 1.0, "potential": 0.0, "charge_density": 0.0 },
                { "index": 3, "id": "n3", "x": 0.0, "y": 1.0, "potential": 5.0, "charge_density": 0.0 }
            ],
            "elements": [
                {
                    "index": 0,
                    "id": "eq0",
                    "node_i": 0,
                    "node_j": 1,
                    "node_k": 2,
                    "node_l": 3,
                    "area": 1.0,
                    "average_potential": 3.75,
                    "potential_gradient_x": -10.0,
                    "potential_gradient_y": 0.0,
                    "electric_field_x": 10.0,
                    "electric_field_y": 0.0,
                    "electric_field_magnitude": 10.0,
                    "electric_flux_density_x": 8.854e-11,
                    "electric_flux_density_y": 0.0,
                    "electric_flux_density_magnitude": 8.854e-11
                }
            ],
            "max_potential": 10.0,
            "max_electric_field": 10.0,
            "max_flux_density": 8.854e-11
        }),
        context: OperatorRunContext::default(),
    })?;

    println!("{}", serde_json::to_string_pretty(&result.summary)?);
    Ok(())
}
