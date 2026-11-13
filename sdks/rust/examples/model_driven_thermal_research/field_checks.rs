use super::model::Case;
use serde_json::{Value, json};

fn number(value: &Value, field: &str) -> Result<f64, String> {
    value[field]
        .as_f64()
        .filter(|v| v.is_finite())
        .ok_or_else(|| format!("missing/nonfinite field: {field}"))
}

pub fn validate(c: Case, result: &Value) -> Result<Value, String> {
    let root = if result.get("artifacts").is_some() {
        result
    } else {
        &result["result"]
    };
    let heat = root["artifacts"]["heat_out.result"]["elements"]
        .as_array()
        .ok_or("missing heat elements")?;
    let structure = &root["artifacts"]["structure_out.result"];
    let nodes = structure["nodes"]
        .as_array()
        .ok_or("missing structural nodes")?;
    let elements = structure["elements"]
        .as_array()
        .ok_or("missing structural elements")?;
    let (_, expected_model) = c.models();
    let expected = expected_model["elements"]
        .as_array()
        .ok_or("missing reference elements")?;
    if heat.len() != expected.len() || elements.len() != expected.len() {
        return Err("field-check element count mismatch".into());
    }
    let mut midpoint_error = 0.0_f64;
    let mut layer_midpoint_error = [0.0_f64; 2];
    let mut interpolation_error = 0.0_f64;
    let mut stress_error = 0.0_f64;
    let mut strain_error = 0.0_f64;
    let mut heat_in = 0.0;
    let mut heat_out = 0.0;
    let hx = 1.0 / (2 * c.refinement) as f64;
    for (index, e) in expected.iter().enumerate() {
        for actual in [&heat[index], &elements[index]] {
            for field in ["id", "node_i", "node_j", "node_k", "node_l"] {
                if actual[field] != e[field] {
                    return Err("field-check connectivity mismatch".into());
                }
            }
        }
        let mut ux = 0.0;
        let mut x = 0.0;
        for field in ["node_i", "node_j", "node_k", "node_l"] {
            let n = nodes
                .get(e[field].as_u64().ok_or("missing reference index")? as usize)
                .ok_or("missing reference node")?;
            ux += number(n, "ux")? / 4.0;
            x += number(n, "x")? / 4.0;
        }
        let layer = usize::from(x > 0.5);
        let error = (ux - c.displacement(x)).abs();
        // Q4 interpolates a quadratic axial reference linearly; midpoint error is O(h^2).
        let expected_error = ((1.0 + c.poisson[layer]) * c.expansion[layer] * c.flux()
            / c.conductivity[layer])
            .abs()
            * hx
            * hx
            / 8.0;
        midpoint_error = midpoint_error.max(error);
        layer_midpoint_error[layer] = layer_midpoint_error[layer].max(error);
        interpolation_error = interpolation_error.max((error - expected_error).abs());
        let thermal = c.expansion[layer] * (c.temperature(x) - c.reference);
        let strain = (1.0 + c.poisson[layer]) * thermal;
        let stress_y = -[70e9, 3e9][layer] * thermal;
        stress_error = stress_error
            .max(number(&elements[index], "stress_x")?.abs())
            .max(number(&elements[index], "tau_xy")?.abs())
            .max((number(&elements[index], "stress_y")? - stress_y).abs());
        strain_error =
            strain_error.max((number(&elements[index], "total_strain_x")? - strain).abs());
        let heat_rate = number(&heat[index], "heat_flux_x")? * (0.1 / c.refinement as f64) * 0.01;
        if index % (2 * c.refinement) == 0 {
            heat_in += heat_rate;
        }
        if index % (2 * c.refinement) == 2 * c.refinement - 1 {
            heat_out += heat_rate;
        }
    }
    let expected_rate = c.flux() * 0.1 * 0.01;
    let heat_error = (heat_in - heat_out)
        .abs()
        .max((heat_in - expected_rate).abs())
        .max((heat_out - expected_rate).abs());
    let heat_tolerance = 1e-6 * expected_rate.abs().max(1.0);
    let delta_scale = (c.temperature(0.0) - c.reference)
        .abs()
        .max((c.temperature(1.0) - c.reference).abs());
    let stress_tolerance =
        1e-8 * ((70e9 * c.expansion[0]).max(3e9 * c.expansion[1]) * delta_scale).max(1.0);
    Ok(json!({"max_midpoint_displacement_error":midpoint_error,
        "layer_midpoint_displacement_error_m":layer_midpoint_error,
        "midpoint_error_law_residual":interpolation_error,"midpoint_error_law_tolerance":1e-9,
        "max_element_mean_stress_error_pa":stress_error,"stress_tolerance_pa":stress_tolerance,
        "max_element_mean_axial_strain_error":strain_error,"strain_tolerance":1e-9,
        "heat_rate_in_w":heat_in,"heat_rate_out_w":heat_out,"heat_rate_error_w":heat_error,
        "heat_rate_tolerance_w":heat_tolerance,"interior_error_is_not_zero":midpoint_error > 1e-9,
        "passed":interpolation_error <= 1e-9 && stress_error <= stress_tolerance && strain_error <= 1e-9 && heat_error <= heat_tolerance}))
}

#[cfg(test)]
mod tests {
    use super::super::{policy::Design, study::case};
    use super::*;
    use serde_json::json;

    fn fixture(r: usize) -> (Case, Value) {
        let c = case(
            Design {
                left: 100.0,
                right: 200.0,
                refinement: r,
            },
            20.0,
            20.0,
        )
        .unwrap();
        fixture_for(c)
    }

    fn fixture_for(c: Case) -> (Case, Value) {
        let (mut heat, mut structure) = c.models();
        for n in structure["nodes"].as_array_mut().unwrap() {
            n["ux"] = json!(c.displacement(n["x"].as_f64().unwrap()));
        }
        for e in heat["elements"].as_array_mut().unwrap() {
            e["heat_flux_x"] = json!(c.flux());
        }
        let nodes = structure["nodes"].clone();
        for e in structure["elements"].as_array_mut().unwrap() {
            let x = (nodes[e["node_i"].as_u64().unwrap() as usize]["x"]
                .as_f64()
                .unwrap()
                + nodes[e["node_j"].as_u64().unwrap() as usize]["x"]
                    .as_f64()
                    .unwrap())
                / 2.0;
            let layer = usize::from(x > 0.5);
            let thermal = c.expansion[layer] * (c.temperature(x) - c.reference);
            let strain = (1.0 + c.poisson[layer]) * thermal;
            e["stress_x"] = json!(0.0);
            e["tau_xy"] = json!(0.0);
            e["stress_y"] = json!(-[70e9, 3e9][layer] * thermal);
            e["total_strain_x"] = json!(strain);
        }
        (
            c,
            json!({"artifacts":{"heat_out.result":heat,"structure_out.result":structure}}),
        )
    }

    #[test]
    fn exact_nodes_do_not_claim_exact_interior_fields() {
        let mut previous: Option<f64> = None;
        for r in [1, 2, 4, 8] {
            let (c, v) = fixture(r);
            let checked = validate(c, &v).unwrap();
            assert_eq!(checked["passed"], true);
            let error = checked["max_midpoint_displacement_error"].as_f64().unwrap();
            assert!(error > 1e-9);
            if let Some(old) = previous {
                assert!((old / error - 4.0_f64).abs() < 1e-8);
            }
            previous = Some(error);
            assert!((checked["heat_rate_in_w"].as_f64().unwrap() - 8.0 / 3.0).abs() < 1e-10);
        }
    }

    #[test]
    fn rejects_wrong_stress_flux_and_element_connectivity() {
        let (c, fixture) = fixture(2);
        for (artifact, field, value) in [
            ("structure_out.result", "stress_y", json!(0.0)),
            ("heat_out.result", "heat_flux_x", json!(0.0)),
            ("structure_out.result", "node_j", json!(0)),
        ] {
            let mut bad = fixture.clone();
            bad["artifacts"][artifact]["elements"][0][field] = value;
            assert!(validate(c, &bad).map_or(true, |v| v["passed"] != true));
        }
    }

    #[test]
    fn nonzero_poisson_and_unequal_curvature_cannot_use_the_old_reference() {
        let (base, _) = fixture(2);
        let c = Case {
            expansion: [10e-6, 35e-6],
            poisson: [0.2, 0.4],
            ..base
        };
        let (_, v) = fixture_for(c);
        assert_eq!(validate(c, &v).unwrap()["passed"], true);
        let (_, ignored_poisson) = fixture_for(Case {
            poisson: [0.0; 2],
            ..c
        });
        assert_eq!(validate(c, &ignored_poisson).unwrap()["passed"], false);
        let (_, repeated_layer) = fixture_for(Case {
            expansion: [10e-6; 2],
            ..c
        });
        assert_eq!(validate(c, &repeated_layer).unwrap()["passed"], false);
    }
}
