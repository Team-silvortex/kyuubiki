use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveCompositeThermoElectricPanelRequest {
    #[serde(default)]
    pub research: Value,
    pub electrostatic_model: Value,
    pub electric_conduction_model: Value,
    pub heat_model: Value,
    pub thermal_model: Value,
    pub electrothermal_loss: Value,
    pub electrothermal_feedback: Value,
    pub electric_conduction_feedback: Value,
    pub thermal_expansion_feedback: Value,
}

impl SolveCompositeThermoElectricPanelRequest {
    pub fn estimated_node_count(&self) -> usize {
        [
            &self.electrostatic_model,
            &self.electric_conduction_model,
            &self.heat_model,
            &self.thermal_model,
        ]
        .into_iter()
        .filter_map(|model| model.get("nodes").and_then(Value::as_array))
        .map(Vec::len)
        .sum()
    }
}
