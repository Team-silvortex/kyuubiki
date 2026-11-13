use super::model::Case;
use serde_json::{Value, json};

fn number(v: &Value, key: &str) -> Result<f64, String> {
    v[key]
        .as_f64()
        .filter(|v| v.is_finite())
        .ok_or_else(|| format!("missing/nonfinite mechanical field: {key}"))
}

pub fn validate(c: Case, result: &Value) -> Result<Value, String> {
    let root = if result.get("artifacts").is_some() {
        result
    } else {
        &result["result"]
    };
    let s = &root["artifacts"]["structure_out.result"];
    let nodes = s["nodes"].as_array().ok_or("missing mechanical nodes")?;
    let elements = s["elements"]
        .as_array()
        .ok_or("missing mechanical elements")?;
    let (_, expected) = c.models();
    let reference_nodes = expected["nodes"]
        .as_array()
        .ok_or("missing reference nodes")?;
    let reference_elements = expected["elements"]
        .as_array()
        .ok_or("missing reference elements")?;
    if nodes.len() != reference_nodes.len() || elements.len() != reference_elements.len() {
        return Err("mechanical result cardinality mismatch".into());
    }
    let mut constraints = 0.0_f64;
    let mut temperature_error = 0.0_f64;
    for (node, reference) in nodes.iter().zip(reference_nodes) {
        let x = number(reference, "x")?;
        if node["id"] != reference["id"]
            || (number(node, "x")? - x).abs() > 1e-12
            || (number(node, "y")? - number(reference, "y")?).abs() > 1e-12
        {
            return Err("mechanical node mapping mismatch".into());
        }
        constraints = constraints.max(number(node, "uy")?.abs());
        if x == 0.0 {
            constraints = constraints.max(number(node, "ux")?.abs());
        }
        temperature_error = temperature_error
            .max((number(node, "temperature_delta")? - (c.temperature(x) - c.reference)).abs());
    }
    let hx = 0.5 / c.refinement as f64;
    let hy = 0.1 / c.refinement as f64;
    let mut forces = vec![[0.0_f64; 2]; nodes.len()];
    let mut energy = 0.0;
    let mut continuum_energy = 0.0;
    let mut density_error = 0.0_f64;
    let mut density_scale = 1.0_f64;
    let mut expected_top = 0.0;
    let mut reaction_scale = 0.0;
    let q = 1.0 / 3.0_f64.sqrt();
    for (e, actual) in reference_elements.iter().zip(elements) {
        let ids: [usize; 4] =
            ["node_i", "node_j", "node_k", "node_l"].map(|k| e[k].as_u64().unwrap() as usize);
        for k in ["id", "node_i", "node_j", "node_k", "node_l"] {
            if actual[k] != e[k] {
                return Err("mechanical element mapping mismatch".into());
            }
        }
        let x0 = number(&reference_nodes[ids[0]], "x")?;
        let x = x0 + hx / 2.0;
        let layer = usize::from(x > 0.5);
        let young = [70e9, 3e9][layer];
        let alpha = c.expansion[layer];
        let nu = c.poisson[layer];
        let mean_delta = c.temperature(x) - c.reference;
        let variance = (c.flux() / c.conductivity[layer] * hx).powi(2) / 12.0;
        // Plane stress with ey=0: axial Q4 strain is (1+nu)*alpha*mean(delta T).
        let density =
            0.5 * young * alpha.powi(2) * (mean_delta.powi(2) + 2.0 * variance / (1.0 - nu));
        let volume = hx * hy * 0.01;
        energy += density * volume;
        continuum_energy += 0.5 * young * alpha.powi(2) * (mean_delta.powi(2) + variance) * volume;
        density_error =
            density_error.max((number(actual, "strain_energy_density")? - density).abs());
        density_scale = density_scale.max(density.abs());
        if number(&reference_nodes[ids[2]], "y")? == 0.1 {
            expected_top += -young * alpha * mean_delta * hx * 0.01;
            reaction_scale += young * alpha * mean_delta.abs() * hx * 0.01;
        }
        // Independent rectangular Q4 quadrature, using returned displacement but analytic temperature.
        for xi in [-q, q] {
            for eta in [-q, q] {
                let signs = [[-1.0, -1.0], [1.0, -1.0], [1.0, 1.0], [-1.0, 1.0]];
                let dx = signs.map(|[sx, sy]| sx * (1.0 + sy * eta) / (2.0 * hx));
                let dy = signs.map(|[sx, sy]| sy * (1.0 + sx * xi) / (2.0 * hy));
                let (mut ex, mut ey, mut gamma) = (0.0, 0.0, 0.0);
                for i in 0..4 {
                    let ux = number(&nodes[ids[i]], "ux")?;
                    let uy = number(&nodes[ids[i]], "uy")?;
                    ex += dx[i] * ux;
                    ey += dy[i] * uy;
                    gamma += dy[i] * ux + dx[i] * uy;
                }
                let thermal = alpha * (c.temperature(x + xi * hx / 2.0) - c.reference);
                let stress = [
                    young / (1.0 - nu * nu) * (ex + nu * ey - (1.0 + nu) * thermal),
                    young / (1.0 - nu * nu) * (ey + nu * ex - (1.0 + nu) * thermal),
                    young / (2.0 * (1.0 + nu)) * gamma,
                ];
                if stress.iter().any(|v| !v.is_finite()) {
                    return Err("mechanical reconstruction overflow".into());
                }
                for i in 0..4 {
                    forces[ids[i]][0] += (dx[i] * stress[0] + dy[i] * stress[2]) * volume / 4.0;
                    forces[ids[i]][1] += (dy[i] * stress[1] + dx[i] * stress[2]) * volume / 4.0;
                }
            }
        }
    }
    let mut free_residual = 0.0_f64;
    let (mut top, mut bottom, mut net_x, mut net_y, mut moment) = (0.0, 0.0, 0.0, 0.0, 0.0);
    for (n, force) in reference_nodes.iter().zip(forces) {
        if force.iter().any(|v| !v.is_finite()) {
            return Err("mechanical force accumulation overflow".into());
        }
        let x = number(n, "x")?;
        let y = number(n, "y")?;
        if x != 0.0 {
            free_residual = free_residual.max(force[0].abs());
        }
        if y == 0.1 {
            top += force[1];
        }
        if y == 0.0 {
            bottom += force[1];
        }
        net_x += force[0];
        net_y += force[1];
        moment += x * force[1] - y * force[0];
    }
    let force_tolerance = 1e-8 * reaction_scale.max(1.0);
    let moment_tolerance = force_tolerance * 1.0; // Fixed one-metre reference length.
    let energy_error = (number(s, "total_strain_energy")? - energy).abs();
    let force_error = (top - expected_top)
        .abs()
        .max((bottom + expected_top).abs())
        .max(net_x.abs())
        .max(net_y.abs())
        .max(free_residual);
    let passed = constraints <= 1e-12
        && temperature_error <= 1e-7
        && density_error <= 1e-8 * density_scale
        && energy_error <= 1e-8 * energy.abs().max(1.0)
        && force_error <= force_tolerance
        && moment.abs() <= moment_tolerance;
    Ok(
        json!({"passed":passed,"scope":"independent_rectangular_q4_reconstruction_not_solver_reported_reactions",
        "constraint_error_m":constraints,"temperature_error":temperature_error,
        "free_dof_residual_n":free_residual,"force_error_n":force_error,"force_tolerance_n":force_tolerance,
        "net_moment_nm":moment,"moment_tolerance_nm":moment_tolerance,
        "reconstructed_top_reaction_n":top,"reconstructed_bottom_reaction_n":bottom,
        "expected_top_reaction_n":expected_top,"expected_discrete_energy_j":energy,
        "continuum_energy_j":continuum_energy,"energy_error_j":energy_error,
        "max_energy_density_error":density_error,"pointwise_stress_qualification":false}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn uniform_fixture() -> (Case, Value) {
        let c = Case {
            material: "uniform",
            conductivity: [100.0, 200.0],
            refinement: 1,
            reference: 20.0,
            rise: 0.0,
            cold_offset: 10.0,
            expansion: super::super::model::ALPHA,
            poisson: [0.0; 2],
        };
        let (_, mut s) = c.models();
        for n in s["nodes"].as_array_mut().unwrap() {
            n["ux"] = json!(c.displacement(n["x"].as_f64().unwrap()));
            n["uy"] = json!(0.0);
            n["temperature_delta"] = json!(10.0);
        }
        for (i, e) in s["elements"].as_array_mut().unwrap().iter_mut().enumerate() {
            e["strain_energy_density"] = json!([350.0, 60.0][i]);
        }
        s["total_strain_energy"] = json!(0.205);
        (c, json!({"artifacts":{"structure_out.result":s}}))
    }

    #[test]
    fn uniform_heating_has_transverse_reactions_and_nonzero_energy() {
        let (c, v) = uniform_fixture();
        let r = validate(c, &v).unwrap();
        assert_eq!(r["passed"], true);
        assert!((r["expected_discrete_energy_j"].as_f64().unwrap() - 0.205).abs() < 1e-12);
        assert!((r["reconstructed_top_reaction_n"].as_f64().unwrap() + 38000.0).abs() < 1e-7);
    }

    #[test]
    fn rejects_hidden_constraint_temperature_and_energy_corruption() {
        let (c, v) = uniform_fixture();
        for (path, value) in [
            ("/artifacts/structure_out.result/nodes/2/uy", json!(1e-4)),
            ("/artifacts/structure_out.result/nodes/0/ux", json!(1e-4)),
            (
                "/artifacts/structure_out.result/nodes/1/temperature_delta",
                json!(0.0),
            ),
            (
                "/artifacts/structure_out.result/elements/0/strain_energy_density",
                json!(0.0),
            ),
            (
                "/artifacts/structure_out.result/total_strain_energy",
                json!(0.0),
            ),
            (
                "/artifacts/structure_out.result/total_strain_energy",
                Value::Null,
            ),
        ] {
            let mut bad = v.clone();
            *bad.pointer_mut(path).unwrap() = value;
            assert!(
                validate(c, &bad).map_or(true, |r| r["passed"] != true),
                "accepted {path}"
            );
        }
    }

    #[test]
    fn nonzero_poisson_uniform_patch_keeps_energy_but_changes_axial_strain() {
        let (base, _) = uniform_fixture();
        let c = Case {
            expansion: [10e-6, 35e-6],
            poisson: [0.2, 0.4],
            ..base
        };
        let (_, mut s) = c.models();
        for n in s["nodes"].as_array_mut().unwrap() {
            let x = n["x"].as_f64().unwrap();
            n["ux"] = json!(120e-6 * x.min(0.5) + 490e-6 * (x - 0.5).max(0.0));
            n["uy"] = json!(0.0);
            n["temperature_delta"] = json!(10.0);
        }
        for (i, e) in s["elements"].as_array_mut().unwrap().iter_mut().enumerate() {
            e["strain_energy_density"] = json!([350.0, 183.75][i]);
        }
        s["total_strain_energy"] = json!(0.266875);
        let v = json!({"artifacts":{"structure_out.result":s}});
        let r = validate(c, &v).unwrap();
        assert_eq!(r["passed"], true, "{r}");
        assert!((r["reconstructed_top_reaction_n"].as_f64().unwrap() + 40250.0).abs() < 1e-7);
        let mut ignored_poisson = v;
        for n in ignored_poisson["artifacts"]["structure_out.result"]["nodes"]
            .as_array_mut()
            .unwrap()
        {
            let x = n["x"].as_f64().unwrap();
            n["ux"] = json!(100e-6 * x.min(0.5) + 350e-6 * (x - 0.5).max(0.0));
        }
        assert_eq!(validate(c, &ignored_poisson).unwrap()["passed"], false);
    }
}
