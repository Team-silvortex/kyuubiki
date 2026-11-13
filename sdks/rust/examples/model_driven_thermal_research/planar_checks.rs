use super::planar_patch::{ALPHA, Case, HEIGHT, THICKNESS, YOUNG};
use serde_json::{Value, json};

fn number(v: &Value, k: &str) -> Result<f64, String> {
    v[k].as_f64()
        .filter(|v| v.is_finite())
        .ok_or_else(|| format!("missing/nonfinite planar field: {k}"))
}

fn array<'a>(v: &'a Value, k: &str, n: usize) -> Result<&'a Vec<Value>, String> {
    v[k].as_array()
        .filter(|v| v.len() == n)
        .ok_or_else(|| format!("planar {k} cardinality mismatch"))
}

pub fn validate(c: Case, result: &Value) -> Result<Value, String> {
    let root = if result.get("artifacts").is_some() {
        result
    } else {
        &result["result"]
    };
    if root["workflow_id"] != format!("research.{}", c.id()) {
        return Err("wrong planar workflow identity".into());
    }
    let a = &root["artifacts"];
    let h = &a["heat_out.result"];
    let b = &a["bridge_out.model"];
    let s = &a["structure_out.result"];
    let (_, seed) = c.models();
    let expected = seed["nodes"].as_array().unwrap();
    let elements = seed["elements"].as_array().unwrap();
    let nodes = array(s, "nodes", expected.len())?;
    let mut temperature_error = 0.0_f64;
    let mut displacement_error = 0.0_f64;
    for (artifact, kind) in [(h, "heat"), (b, "bridge"), (s, "structure")] {
        for (actual, e) in array(artifact, "nodes", expected.len())?
            .iter()
            .zip(expected)
        {
            let x = number(e, "x")?;
            let y = number(e, "y")?;
            if actual["id"] != e["id"]
                || (number(actual, "x")? - x).abs() > 1e-12
                || (number(actual, "y")? - y).abs() > 1e-12
            {
                return Err("planar node identity/geometry drift".into());
            }
            let (key, temperature) = if kind == "heat" {
                ("temperature", 20.0 + c.rise)
            } else {
                ("temperature_delta", c.rise)
            };
            temperature_error = temperature_error.max((number(actual, key)? - temperature).abs());
            if kind == "structure" {
                for (key, u) in ["ux", "uy"].into_iter().zip(c.displacement(x, y)) {
                    displacement_error = displacement_error.max((number(actual, key)? - u).abs());
                }
            }
            if kind == "bridge" {
                for k in ["fix_x", "fix_y"] {
                    if actual[k] != e[k] {
                        return Err(format!("planar bridge restraint drift: {k}"));
                    }
                }
                // JSON services may encode integral-valued floats as integers.
                for k in ["load_x", "load_y"] {
                    if number(actual, k)? != number(e, k)? {
                        return Err(format!("planar bridge load drift: {k}"));
                    }
                }
            }
        }
    }
    let heat_elements = array(h, "elements", elements.len())?;
    let bridge_elements = array(b, "elements", elements.len())?;
    let actual_elements = array(s, "elements", elements.len())?;
    let mut forces = vec![[0.0; 2]; expected.len()];
    let (
        mut heat_error,
        mut stress_error,
        mut strain_error,
        mut energy_density_error,
        mut area_error,
    ) = (0.0_f64, 0.0_f64, 0.0_f64, 0.0_f64, 0.0_f64);
    let (mut area_sum, mut reported_energy, mut quadrature_energy) = (0.0, 0.0, 0.0);
    let (mut min_det, mut max_det) = (f64::INFINITY, 0.0_f64);
    let [sx, sy, tau] = c.stress;
    let [ex, ey, gamma] = c.strain();
    let radius = ((sx - sy).powi(2) / 4.0 + tau * tau).sqrt();
    let stress_fields = [
        ("stress_x", sx),
        ("stress_y", sy),
        ("tau_xy", tau),
        ("principal_stress_1", (sx + sy) / 2.0 + radius),
        ("principal_stress_2", (sx + sy) / 2.0 - radius),
        ("max_in_plane_shear", radius),
        (
            "von_mises",
            (sx * sx - sx * sy + sy * sy + 3.0 * tau * tau).sqrt(),
        ),
    ];
    let mut stress_ranges = [[f64::INFINITY, f64::NEG_INFINITY, 0.0]; 7];
    let stress_scale = sx
        .abs()
        .max(sy.abs())
        .max(tau.abs())
        .max(YOUNG * ALPHA * c.rise.abs())
        .max(1.0);
    for (index, e) in elements.iter().enumerate() {
        let actual = &actual_elements[index];
        for value in [&heat_elements[index], &bridge_elements[index], actual] {
            for k in ["id", "node_i", "node_j", "node_k", "node_l"] {
                if value[k] != e[k] {
                    return Err("planar connectivity drift".into());
                }
            }
        }
        for k in [
            "youngs_modulus",
            "poisson_ratio",
            "thermal_expansion",
            "thickness",
        ] {
            if number(&bridge_elements[index], k)? != number(e, k)? {
                return Err(format!("planar bridge material drift: {k}"));
            }
        }
        for k in ["heat_flux_x", "heat_flux_y"] {
            heat_error = heat_error.max(number(&heat_elements[index], k)?.abs());
        }
        for ((k, value), range) in stress_fields.iter().zip(&mut stress_ranges) {
            let observed = number(actual, k)?;
            range[0] = range[0].min(observed);
            range[1] = range[1].max(observed);
            range[2] = range[2].max((observed - value).abs());
            stress_error = stress_error.max(range[2]);
        }
        for (k, value) in [
            ("total_strain_x", ex),
            ("total_strain_y", ey),
            ("gamma_xy", gamma),
            ("mechanical_strain_x", ex - ALPHA * c.rise),
            ("mechanical_strain_y", ey - ALPHA * c.rise),
            ("thermal_strain", ALPHA * c.rise),
        ] {
            strain_error = strain_error.max((number(actual, k)? - value).abs());
        }
        temperature_error =
            temperature_error.max((number(actual, "average_temperature_delta")? - c.rise).abs());
        energy_density_error = energy_density_error
            .max((number(actual, "strain_energy_density")? - c.energy_density()).abs());
        let ids: [usize; 4] =
            ["node_i", "node_j", "node_k", "node_l"].map(|k| e[k].as_u64().unwrap() as usize);
        let points = ids.map(|i| {
            [
                expected[i]["x"].as_f64().unwrap(),
                expected[i]["y"].as_f64().unwrap(),
            ]
        });
        let mut area = 0.0;
        let q = 1.0 / 3.0_f64.sqrt();
        for xi in [-q, q] {
            for eta in [-q, q] {
                let signs = [[-1.0, -1.0], [1.0, -1.0], [1.0, 1.0], [-1.0, 1.0]];
                let dxi = signs.map(|[a, b]| a * (1.0 + b * eta) / 4.0);
                let deta = signs.map(|[a, b]| b * (1.0 + a * xi) / 4.0);
                let xx = (0..4).map(|i| dxi[i] * points[i][0]).sum::<f64>();
                let xy = (0..4).map(|i| dxi[i] * points[i][1]).sum::<f64>();
                let yx = (0..4).map(|i| deta[i] * points[i][0]).sum::<f64>();
                let yy = (0..4).map(|i| deta[i] * points[i][1]).sum::<f64>();
                let det = xx * yy - xy * yx;
                if !det.is_finite() || det <= 0.0 {
                    return Err("invalid independent planar Jacobian".into());
                }
                min_det = min_det.min(det);
                max_det = max_det.max(det);
                area += det;
                let dx: [f64; 4] = std::array::from_fn(|i| (yy * dxi[i] - xy * deta[i]) / det);
                let dy: [f64; 4] = std::array::from_fn(|i| (-yx * dxi[i] + xx * deta[i]) / det);
                let (mut ax, mut ay, mut shear) = (0.0, 0.0, 0.0);
                for i in 0..4 {
                    let u = number(&nodes[ids[i]], "ux")?;
                    let v = number(&nodes[ids[i]], "uy")?;
                    ax += dx[i] * u;
                    ay += dy[i] * v;
                    shear += dy[i] * u + dx[i] * v;
                }
                let mx = ax - ALPHA * c.rise;
                let my = ay - ALPHA * c.rise;
                let stress = [
                    YOUNG / (1.0 - c.poisson * c.poisson) * (mx + c.poisson * my),
                    YOUNG / (1.0 - c.poisson * c.poisson) * (my + c.poisson * mx),
                    YOUNG / (2.0 * (1.0 + c.poisson)) * shear,
                ];
                if stress.iter().any(|v| !v.is_finite()) {
                    return Err("planar reconstruction overflow".into());
                }
                for (actual, expected) in stress.iter().zip(c.stress) {
                    stress_error = stress_error.max((actual - expected).abs());
                }
                quadrature_energy +=
                    0.5 * (stress[0] * mx + stress[1] * my + stress[2] * shear) * det * THICKNESS;
                for i in 0..4 {
                    forces[ids[i]][0] += (dx[i] * stress[0] + dy[i] * stress[2]) * det * THICKNESS;
                    forces[ids[i]][1] += (dy[i] * stress[1] + dx[i] * stress[2]) * det * THICKNESS;
                }
            }
        }
        area_sum += area;
        area_error = area_error.max((number(actual, "area")? - area).abs());
        reported_energy += number(actual, "strain_energy_density")? * area * THICKNESS;
    }
    let (mut residual, mut elastic_pairing, mut total_pairing, mut thermal_pairing) =
        (0.0_f64, 0.0, 0.0, 0.0);
    for (i, e) in expected.iter().enumerate() {
        for (axis, field, u, coordinate) in [(0, "load_x", "ux", "x"), (1, "load_y", "uy", "y")] {
            let load = number(e, field)?;
            if !forces[i][axis].is_finite() {
                return Err("planar force overflow".into());
            }
            residual = residual.max((forces[i][axis] - load).abs());
            let thermal_u = ALPHA * c.rise * number(e, coordinate)?;
            let actual_u = number(&nodes[i], u)?;
            total_pairing += load * actual_u;
            thermal_pairing += load * thermal_u;
            elastic_pairing += load * (actual_u - thermal_u);
        }
    }
    let exact_energy = c.energy_density() * HEIGHT * THICKNESS;
    let energy_error = (number(s, "total_strain_energy")? - exact_energy)
        .abs()
        .max((reported_energy - exact_energy).abs())
        .max((quadrature_energy - exact_energy).abs())
        .max((elastic_pairing / 2.0 - exact_energy).abs());
    // Final traction/displacement pairing is not path-integrated work.
    let expected_thermal_pairing = HEIGHT * THICKNESS * ALPHA * c.rise * (sx + sy);
    let pairing_error = (thermal_pairing - expected_thermal_pairing)
        .abs()
        .max((total_pairing - 2.0 * exact_energy - expected_thermal_pairing).abs());
    let observed_stress_fields: serde_json::Map<String, Value> = stress_fields
        .iter()
        .zip(stress_ranges)
        .map(|((key, value), range)| {
            (
                key.to_string(),
                json!({"expected":value,
            "min":range[0],"max":range[1],"max_abs_error":range[2]}),
            )
        })
        .collect();
    let force_tolerance = 1e-8 * stress_scale * THICKNESS;
    let passed = temperature_error <= 1e-7
        && displacement_error <= 1e-10
        && heat_error <= 1e-6
        && stress_error <= 1e-8 * stress_scale
        && strain_error <= 1e-10
        && area_error <= 1e-12
        && (area_sum - HEIGHT).abs() <= 1e-12
        && energy_density_error <= 1e-8 * c.energy_density().abs().max(1.0)
        && energy_error <= 1e-9
        && pairing_error <= 1e-9
        && residual <= force_tolerance;
    let corner = nodes.last().ok_or("empty planar nodes")?;
    Ok(
        json!({"passed":passed,"scope":"homogeneous_affine_plane_stress_patch_not_general_field_qualification",
        "node_count":nodes.len(),"element_count":elements.len(),"fixed_dofs":3,"free_y_dofs":nodes.len()-2,
        "temperature_error":temperature_error,"displacement_error_m":displacement_error,"heat_flux_error":heat_error,
        "stress_error_pa":stress_error,"stress_tolerance_pa":1e-8*stress_scale,"strain_error":strain_error,
        "energy_density_error":energy_density_error,"energy_error_j":energy_error,"expected_energy_j":exact_energy,
        "observed_energy_j":number(s,"total_strain_energy")?,"quadrature_energy_j":quadrature_energy,
        "traction_displacement_pairing_j":total_pairing,"thermal_traction_pairing_j":thermal_pairing,
        "elastic_traction_pairing_j":elastic_pairing,"traction_pairing_error_j":pairing_error,
        "path_integrated_work_evaluated":false,"observed_stress_fields_pa":observed_stress_fields,
        "all_dof_force_residual_n":residual,"force_tolerance_n":force_tolerance,
        "min_jacobian":min_det,"max_jacobian":max_det,"area_m2":area_sum,
        "corner_ux":number(corner,"ux")?,"corner_uy":number(corner,"uy")?,
        "expected_stress_pa":c.stress,"expected_strain":c.strain(),"poisson":c.poisson,"temperature_rise":c.rise,
        "distorted":c.distorted,"reactions_are_reconstructed":true,"general_peak_stress_qualification":false}),
    )
}

#[cfg(test)]
#[path = "planar_tests.rs"]
mod tests;
