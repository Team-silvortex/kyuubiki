use super::bending_model::{Case, HEIGHT, THICKNESS};
use serde_json::{Value, json};

fn number(v: &Value, k: &str) -> Result<f64, String> {
    v[k].as_f64()
        .filter(|v| v.is_finite())
        .ok_or_else(|| format!("invalid bending field {k}"))
}
fn array<'a>(v: &'a Value, k: &str, n: usize) -> Result<&'a Vec<Value>, String> {
    v[k].as_array()
        .filter(|v| v.len() == n)
        .ok_or_else(|| format!("bending {k} count mismatch"))
}
fn stress(e: f64, nu: f64, strain: [f64; 3]) -> [f64; 3] {
    [
        e / (1.0 - nu * nu) * (strain[0] + nu * strain[1]),
        e / (1.0 - nu * nu) * (strain[1] + nu * strain[0]),
        e / (2.0 * (1.0 + nu)) * strain[2],
    ]
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    (0..3).map(|i| a[i] * b[i]).sum()
}

fn constitutive_balance_error(energy: f64, traction_pairing: f64, thermal_pairing: f64) -> f64 {
    (2.0 * energy - traction_pairing + thermal_pairing).abs()
}

pub fn validate(c: Case, result: &Value) -> Result<Value, String> {
    let root = if result.get("artifacts").is_some() {
        result
    } else {
        &result["result"]
    };
    if root["workflow_id"] != format!("research.{}", c.id()) {
        return Err("bending workflow identity mismatch".into());
    }
    let artifacts = &root["artifacts"];
    let h = &artifacts["heat_out.result"];
    let b = &artifacts["bridge_out.model"];
    let s = &artifacts["structure_out.result"];
    let (_, seed) = c.models();
    let expected = seed["nodes"].as_array().unwrap();
    let elems = seed["elements"].as_array().unwrap();
    let nodes = array(s, "nodes", expected.len())?;
    let (nx, ny) = c.dimensions();
    let hx = 1.0 / nx as f64;
    let hy = HEIGHT / ny as f64;
    let (mut temperature_error, mut error2, mut scale2, mut max_displacement_error) =
        (0.0_f64, 0.0, 0.0, 0.0_f64);
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
                return Err("bending geometry/identity drift".into());
            }
            let (key, t) = if kind == "heat" {
                ("temperature", c.temperature(y))
            } else {
                ("temperature_delta", c.temperature(y) - c.reference)
            };
            temperature_error = temperature_error.max((number(actual, key)? - t).abs());
            if kind == "bridge" {
                for k in ["fix_x", "fix_y"] {
                    if actual[k] != e[k] {
                        return Err("bending restraint drift".into());
                    }
                }
                for k in ["load_x", "load_y"] {
                    if number(actual, k)? != number(e, k)? {
                        return Err("bending load drift".into());
                    }
                }
            }
            if kind == "structure" {
                for (k, u) in ["ux", "uy"].into_iter().zip(c.displacement(x, y)) {
                    let err = number(actual, k)? - u;
                    error2 += err * err;
                    scale2 += u * u;
                    max_displacement_error = max_displacement_error.max(err.abs());
                    if e[if k == "ux" { "fix_x" } else { "fix_y" }] == true
                        && number(actual, k)?.abs() > 1e-12
                    {
                        return Err("bending rigid-mode restraint violated".into());
                    }
                }
            }
        }
    }
    let hs = array(h, "elements", elems.len())?;
    let bs = array(b, "elements", elems.len())?;
    let ss = array(s, "elements", elems.len())?;
    let mut forces = vec![[0.0; 2]; expected.len()];
    let (mut flux_error, mut post_error, mut strain_error, mut density_error) =
        (0.0_f64, 0.0_f64, 0.0_f64, 0.0_f64);
    let (mut energy, mut error_energy, mut thermal_pairing, mut reported_energy) =
        (0.0, 0.0, 0.0, 0.0);
    let mut stress_l2 = 0.0;
    let (mut normal_error_energy, mut shear_error_energy, mut shear_strain2) = (0.0, 0.0, 0.0);
    for (index, e) in elems.iter().enumerate() {
        let layer = usize::from(index / nx >= ny / 2);
        let young = c.young[layer];
        let nu = c.poisson[layer];
        let ids: [usize; 4] =
            ["node_i", "node_j", "node_k", "node_l"].map(|k| e[k].as_u64().unwrap() as usize);
        for actual in [&hs[index], &bs[index], &ss[index]] {
            for k in ["id", "node_i", "node_j", "node_k", "node_l"] {
                if actual[k] != e[k] {
                    return Err("bending connectivity drift".into());
                }
            }
        }
        for k in [
            "youngs_modulus",
            "poisson_ratio",
            "thermal_expansion",
            "thickness",
        ] {
            if number(&bs[index], k)? != number(e, k)? {
                return Err(format!("bending material drift: {k}"));
            }
        }
        // Heat material assignment is independently checked in the returned flux.
        flux_error = flux_error
            .max(number(&hs[index], "heat_flux_x")?.abs())
            .max((number(&hs[index], "heat_flux_y")? - c.flux()).abs());
        let (mut mean_strain, mut mean_stress, mut mean_thermal, mut mean_energy) =
            ([0.0; 3], [0.0; 3], 0.0, 0.0);
        let q = 1.0 / 3.0_f64.sqrt();
        for xi in [-q, q] {
            for eta in [-q, q] {
                let signs = [[-1.0, -1.0], [1.0, -1.0], [1.0, 1.0], [-1.0, 1.0]];
                let shapes = signs.map(|[a, b]| (1.0 + a * xi) * (1.0 + b * eta) / 4.0);
                let dx = signs.map(|[a, b]| a * (1.0 + b * eta) / (2.0 * hx));
                let dy = signs.map(|[a, b]| b * (1.0 + a * xi) / (2.0 * hy));
                let mut strain = [0.0; 3];
                let mut theta = 0.0;
                for i in 0..4 {
                    let n = &nodes[ids[i]];
                    let u = number(n, "ux")?;
                    let v = number(n, "uy")?;
                    strain[0] += dx[i] * u;
                    strain[1] += dy[i] * v;
                    strain[2] += dy[i] * u + dx[i] * v;
                    theta += shapes[i] * number(n, "temperature_delta")?;
                }
                let y = number(&expected[ids[0]], "y")? + (eta + 1.0) * hy / 2.0;
                let thermal = c.alpha[layer] * theta;
                let mechanical = [strain[0] - thermal, strain[1] - thermal, strain[2]];
                let st = stress(young, nu, mechanical);
                let exact = c.strain(y, layer);
                let difference = std::array::from_fn(|i| strain[i] - exact[i]);
                let weight = hx * hy * THICKNESS / 4.0;
                let density = dot(st, mechanical) / 2.0;
                energy += density * weight;
                mean_energy += density / 4.0;
                let error_stress = stress(young, nu, difference);
                error_energy += dot(error_stress, difference) * weight / 2.0;
                normal_error_energy +=
                    (error_stress[0] * difference[0] + error_stress[1] * difference[1]) * weight
                        / 2.0;
                shear_error_energy += error_stress[2] * difference[2] * weight / 2.0;
                shear_strain2 += strain[2] * strain[2] * weight;
                thermal_pairing += (st[0] + st[1]) * thermal * weight;
                stress_l2 +=
                    ((st[0] - c.stress(y, layer)).powi(2) + st[1] * st[1] + 2.0 * st[2] * st[2])
                        * weight;
                mean_thermal += thermal / 4.0;
                for i in 0..3 {
                    mean_strain[i] += strain[i] / 4.0;
                    mean_stress[i] += st[i] / 4.0;
                }
                for i in 0..4 {
                    forces[ids[i]][0] += (dx[i] * st[0] + dy[i] * st[2]) * weight;
                    forces[ids[i]][1] += (dy[i] * st[1] + dx[i] * st[2]) * weight;
                }
            }
        }
        let actual = &ss[index];
        for (k, v) in ["total_strain_x", "total_strain_y", "gamma_xy"]
            .into_iter()
            .zip(mean_strain)
        {
            strain_error = strain_error.max((number(actual, k)? - v).abs());
        }
        for (k, v) in [
            ("mechanical_strain_x", mean_strain[0] - mean_thermal),
            ("mechanical_strain_y", mean_strain[1] - mean_thermal),
            ("thermal_strain", mean_thermal),
        ] {
            strain_error = strain_error.max((number(actual, k)? - v).abs());
        }
        let [sx, sy, tau] = mean_stress;
        let radius = ((sx - sy).powi(2) / 4.0 + tau * tau).sqrt();
        for (k, v) in [
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
        ] {
            post_error = post_error.max((number(actual, k)? - v).abs());
        }
        let area = number(actual, "area")?;
        if (area - hx * hy).abs() > 1e-12 {
            return Err("bending element area drift".into());
        }
        density_error =
            density_error.max((number(actual, "strain_energy_density")? - mean_energy).abs());
        reported_energy += number(actual, "strain_energy_density")? * area * THICKNESS;
    }
    let (mut residual, mut pairing, mut exact_pairing) = (0.0_f64, 0.0, 0.0);
    let (mut free_residual, mut fixed_residual) = (0.0_f64, 0.0_f64);
    for (i, e) in expected.iter().enumerate() {
        let exact = c.displacement(number(e, "x")?, number(e, "y")?);
        for (axis, k, u) in [(0, "load_x", "ux"), (1, "load_y", "uy")] {
            let load = number(e, k)?;
            let force_error = (forces[i][axis] - load).abs();
            residual = residual.max(force_error);
            if e[if axis == 0 { "fix_x" } else { "fix_y" }] == true {
                fixed_residual = fixed_residual.max(force_error);
            } else {
                free_residual = free_residual.max(force_error);
            }
            pairing += load * number(&nodes[i], u)?;
            exact_pairing += load * exact[axis];
        }
    }
    let stress_scale = c.young.into_iter().fold(0.0_f64, f64::max)
        * c.alpha.into_iter().fold(0.0_f64, f64::max)
        * (c.top - c.reference)
            .abs()
            .max((c.bottom - c.reference).abs())
            .max(1.0);
    let energy_scale = stress_scale.powi(2) / c.young.into_iter().fold(f64::INFINITY, f64::min)
        * HEIGHT
        * THICKNESS;
    let energy_error = (number(s, "total_strain_energy")? - energy)
        .abs()
        .max((reported_energy - energy).abs());
    let identity_error = (energy - c.energy() - (pairing - exact_pairing) - error_energy).abs();
    let balance_error = constitutive_balance_error(energy, pairing, thermal_pairing);
    let relative_rms = (error2 / scale2).sqrt();
    let finite = [
        energy,
        error_energy,
        stress_l2,
        temperature_error,
        relative_rms,
        residual,
        post_error,
        strain_error,
        density_error,
        identity_error,
        balance_error,
    ]
    .iter()
    .all(|v| v.is_finite());
    let passed = finite
        && temperature_error <= 1e-7
        && flux_error <= 1e-6 * c.flux().abs().max(1.0)
        && strain_error <= 1e-10
        && post_error <= 1e-8 * stress_scale
        && density_error <= 1e-8 * energy_scale / (HEIGHT * THICKNESS)
        && energy_error <= 1e-8 * energy_scale.max(1.0)
        && identity_error <= 1e-8 * energy_scale.max(1.0)
        && balance_error <= 1e-8 * energy_scale.max(1.0)
        && residual <= 1e-8 * stress_scale * HEIGHT * THICKNESS
        && relative_rms < 1.0;
    let mid = nx / 2;
    let continuum_energy = c.energy();
    let continuum_difference = number(s, "total_strain_energy")? - continuum_energy;
    let relative_energy = if continuum_energy > 1e-20 {
        Some(continuum_difference.abs() / continuum_energy)
    } else {
        None
    };
    Ok(
        json!({"passed":passed,"scope":"rectangular_nonaffine_plane_stress_reference_with_declared_end_tractions",
        "acceptance_stage":"per_case_checks_matrix_convergence_still_required",
        "normalization":{"stress_scale_pa":stress_scale,"energy_scale_j":energy_scale,
            "reference_nodal_l2_m":scale2.sqrt(),"volume_m3":HEIGHT*THICKNESS,
            "stress_scale_definition":"max(E)*max(alpha)*max(abs(theta),1)",
            "energy_scale_definition":"stress_scale_pa^2*volume/min(E)",
            "nodal_error_definition":"sqrt(sum(node_displacement_error^2)/sum(exact_node_displacement^2))",
            "stress_error_definition":"sqrt(integral(dsx^2+dsy^2+2*dtau^2)/volume)/stress_scale_pa",
            "error_energy_definition":"0.5*integral((strain_h-strain_exact):C:(strain_h-strain_exact))"},
        "node_count":nodes.len(),"element_count":elems.len(),"mesh_nx":nx,"mesh_ny":ny,
        "fixed_dofs":3,"temperature_error_k":temperature_error,
        "heat_flux_error":flux_error,"expected_heat_flux_y":c.flux(),"relative_nodal_rms_error":relative_rms,
        "max_displacement_error_m":max_displacement_error,"mid_bottom_uy":number(&nodes[mid],"uy")?,
        "expected_mid_bottom_uy":c.displacement(0.5,0.0)[1],"expected_curvature_coefficient":c.axial()[1],
        "expected_elastic_energy_j":c.energy(),"observed_elastic_energy_j":number(s,"total_strain_energy")?,
        "continuum_elastic_energy_difference_j":continuum_difference,
        "relative_continuum_elastic_energy_error":relative_energy,
        "error_energy_j":error_energy,"normalized_error_energy":error_energy/energy_scale,
        "normal_error_energy_j":normal_error_energy,"shear_error_energy_j":shear_error_energy,
        "shear_strain_rms":(shear_strain2/(HEIGHT*THICKNESS)).sqrt(),
        "normalized_stress_l2_error":(stress_l2/(HEIGHT*THICKNESS)).sqrt()/stress_scale,
        "energy_reconstruction_error_j":energy_error,"variational_identity_error_j":identity_error,
        "constitutive_balance_error_j":balance_error,"stress_postprocess_error_pa":post_error,
        "strain_postprocess_error":strain_error,"all_dof_force_residual_n":residual,
        "free_dof_force_residual_n":free_residual,"constrained_dof_force_residual_n":fixed_residual,
        "force_tolerance_n":1e-8*stress_scale*HEIGHT*THICKNESS,
        "traction_displacement_pairing_j":pairing,"thermal_stress_pairing_j":thermal_pairing,
        "continuum_exact_on_q4":false,"solver_returned_reactions":false,"path_integrated_work_evaluated":false}),
    )
}

#[cfg(test)]
#[path = "bending_tests.rs"]
mod tests;
