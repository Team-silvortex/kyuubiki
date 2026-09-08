use serde_json::{Map, Value};

#[derive(Debug, Clone)]
pub(crate) struct HeatToThermoBridgeContract {
    pub source_field: String,
    pub distribution: String,
    pub node_index_fields: Vec<String>,
    pub reduction: String,
    pub target_field: String,
    pub scale: f64,
    pub reference_temperature: f64,
    pub default_value: f64,
}

impl HeatToThermoBridgeContract {
    pub fn for_shape(&self, fields: &[&str]) -> Result<Self, String> {
        let mut result = self.clone();
        if result.node_index_fields.is_empty() {
            result.node_index_fields = fields.iter().map(|field| (*field).to_string()).collect();
        }
        if result
            .node_index_fields
            .iter()
            .any(|field| !fields.contains(&field.as_str()))
        {
            return Err(
                "heat bridge node index field is not supported by this element shape".into(),
            );
        }
        Ok(result)
    }
}

pub(crate) fn resolve_heat_to_thermo_bridge_contract(
    config: &Value,
) -> Result<HeatToThermoBridgeContract, String> {
    if !config.is_object() {
        return Err("heat-to-thermo config must be an object".into());
    }
    let contract = config.get("contract").unwrap_or(config);
    let contract = contract
        .as_object()
        .ok_or("heat-to-thermo contract must be an object")?;
    let empty = Map::new();
    let source = section(contract, "source", &empty)?;
    let transform = section(contract, "transform", &empty)?;
    let target = section(contract, "target", &empty)?;
    let mut source_field = string(source, "field", "temperature")?;
    if source_field == "heat_flux" {
        source_field = "heat_flux_magnitude".into();
    }
    let distribution = string(source, "distribution", "node_to_node")?;
    let reduction = string(
        transform,
        "reduction",
        if distribution == "node_to_node" {
            "copy"
        } else {
            "mean"
        },
    )?;
    let target_field = string(target, "field", "temperature_delta")?;
    let node_index_fields = match source.get("node_index_fields") {
        None => vec![],
        Some(value) => {
            let items = value
                .as_array()
                .filter(|items| !items.is_empty())
                .ok_or("heat bridge node_index_fields must be a nonempty array")?;
            let mut fields = Vec::new();
            for value in items {
                let field = value
                    .as_str()
                    .filter(|field| ["node_i", "node_j", "node_k", "node_l"].contains(field))
                    .ok_or("invalid heat bridge node index field")?
                    .to_string();
                if fields.contains(&field) {
                    return Err("duplicate heat bridge node index field".into());
                }
                fields.push(field);
            }
            fields
        }
    };
    let scale = number(transform, "scale", 1.0)?;
    let default_value = number(transform, "default_value", 0.0)?;
    let reference_temperature = number(transform, "reference_temperature", 0.0)?;
    let valid_source = match distribution.as_str() {
        "node_to_node" => ["temperature", "heat_load"].contains(&source_field.as_str()),
        "element_to_nodes" => [
            "average_temperature",
            "heat_flux_x",
            "heat_flux_y",
            "heat_flux_magnitude",
        ]
        .contains(&source_field.as_str()),
        _ => false,
    };
    if !valid_source {
        return Err("unsupported heat-to-thermo source field or distribution".into());
    }
    if reference_temperature != 0.0
        && !["temperature", "average_temperature"].contains(&source_field.as_str())
    {
        return Err(
            "heat-to-thermo reference_temperature requires a temperature source field".into(),
        );
    }
    if !["copy", "mean", "sum", "area_weighted_mean", "min", "max"].contains(&reduction.as_str()) {
        return Err("unsupported heat-to-thermo reduction".into());
    }
    if target_field != "temperature_delta" {
        return Err("unsupported heat-to-thermo target field".into());
    }
    Ok(HeatToThermoBridgeContract {
        source_field,
        distribution,
        node_index_fields,
        reduction,
        target_field,
        scale,
        reference_temperature,
        default_value,
    })
}

fn section<'a>(
    parent: &'a Map<String, Value>,
    name: &str,
    empty: &'a Map<String, Value>,
) -> Result<&'a Map<String, Value>, String> {
    match parent.get(name) {
        None | Some(Value::Null) => Ok(empty),
        Some(value) => value
            .as_object()
            .ok_or_else(|| format!("heat bridge {name} must be an object")),
    }
}

fn string(map: &Map<String, Value>, name: &str, default: &str) -> Result<String, String> {
    match map.get(name) {
        None => Ok(default.into()),
        Some(value) => value
            .as_str()
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .ok_or_else(|| format!("heat bridge {name} must be a nonempty string")),
    }
}

fn number(map: &Map<String, Value>, name: &str, default: f64) -> Result<f64, String> {
    match map.get(name) {
        None => Ok(default),
        Some(value) => value
            .as_f64()
            .filter(|value| value.is_finite())
            .ok_or_else(|| format!("heat bridge {name} must be finite and numeric")),
    }
}
