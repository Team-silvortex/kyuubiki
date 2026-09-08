use super::model::Case;
use serde_json::{Value, json};

fn number(value: &Value, field: &str) -> Result<f64, String> {
    value[field]
        .as_f64()
        .filter(|n| n.is_finite())
        .ok_or_else(|| format!("missing/nonfinite result field: {field}"))
}

pub fn validate(case: Case, result: &Value) -> Result<Value, String> {
    let payload = if result.get("artifacts").is_some() {
        result
    } else {
        &result["result"]
    };
    if payload["workflow_id"] != format!("research.{}", case.id()) {
        return Err("result belongs to a different research workflow".into());
    }
    let artifacts = payload
        .get("artifacts")
        .ok_or("missing retained workflow artifacts")?;
    let heat = &artifacts["heat_out.result"];
    let bridge = &artifacts["bridge_out.model"];
    let structure = &artifacts["structure_out.result"];
    let expected_count = (2 * case.refinement + 1) * (case.refinement + 1);
    let (expected_model, _) = case.models();
    for (name, artifact) in [("heat", heat), ("bridge", bridge), ("structure", structure)] {
        let nodes = artifact["nodes"]
            .as_array()
            .ok_or_else(|| format!("{name}: missing nodes"))?;
        if nodes.len() != expected_count {
            return Err(format!("{name}: node count mismatch"));
        }
        for (actual, expected) in nodes
            .iter()
            .zip(expected_model["nodes"].as_array().unwrap())
        {
            if actual["id"] != expected["id"]
                || (number(actual, "x")? - number(expected, "x")?).abs() > 1e-12
                || (number(actual, "y")? - number(expected, "y")?).abs() > 1e-12
            {
                return Err(format!("{name}: node identity or geometry drift"));
            }
        }
    }
    let mut gates = vec![];
    for (name, artifact, field, tolerance, expected) in [
        (
            "heat_temperature",
            heat,
            "temperature",
            1e-7,
            Case::temperature as fn(Case, f64) -> f64,
        ),
        (
            "thermal_displacement",
            structure,
            "ux",
            1e-9,
            Case::displacement,
        ),
    ] {
        let nodes = artifact["nodes"]
            .as_array()
            .ok_or_else(|| format!("{name}: missing nodes"))?;
        if nodes.len() != expected_count {
            return Err(format!("{name}: node count mismatch"));
        }
        let mut max_error = 0.0_f64;
        for node in nodes {
            max_error =
                max_error.max((number(node, field)? - expected(case, number(node, "x")?)).abs());
        }
        gates.push(
            json!({"id": name, "max_abs_error": max_error, "tolerance": tolerance,
            "passed": max_error <= tolerance}),
        );
    }
    let nodes = bridge["nodes"].as_array().ok_or("bridge: missing nodes")?;
    if nodes.len() != expected_count {
        return Err("bridge: node count mismatch".into());
    }
    let mut bridge_error = 0.0_f64;
    for node in nodes {
        let expected = case.temperature(number(node, "x")?) - case.reference;
        bridge_error = bridge_error.max((number(node, "temperature_delta")? - expected).abs());
    }
    gates.push(
        json!({"id": "temperature_reference", "max_abs_error": bridge_error,
        "tolerance": 1e-7, "passed": bridge_error <= 1e-7}),
    );
    let elements = heat["elements"]
        .as_array()
        .ok_or("heat: missing elements")?;
    if elements.len() != 2 * case.refinement * case.refinement {
        return Err("heat: element count mismatch".into());
    }
    let mut flux_error = 0.0_f64;
    for element in elements {
        flux_error = flux_error.max((number(element, "heat_flux_x")? - case.flux()).abs());
        flux_error = flux_error.max(number(element, "heat_flux_y")?.abs());
    }
    let tolerance = 1e-6 * case.flux().abs().max(1.0);
    gates.push(
        json!({"id": "heat_flux_continuity", "max_abs_error": flux_error,
        "tolerance": tolerance, "passed": flux_error <= tolerance}),
    );
    Ok(
        json!({"case": case.id(), "material": case.material, "refinement": case.refinement,
        "reference_temperature": case.reference, "temperature_rise": case.rise,
        "node_count": expected_count, "expected_flux": case.flux(),
        "expected_tip_displacement": case.displacement(1.0), "gates": gates,
        "passed": gates.iter().all(|g| g["passed"] == true)}),
    )
}
