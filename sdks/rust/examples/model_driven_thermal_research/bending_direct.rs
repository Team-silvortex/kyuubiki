//! Bounded rectangular-Q4 reference, independent of production solver code.
use super::bending_model::{Case, HEIGHT, THICKNESS};
use serde_json::{Value, json};

pub(super) struct Band {
    n: usize,
    width: usize,
    data: Vec<f64>,
}

impl Band {
    pub(super) fn new(n: usize, width: usize) -> Self {
        Self {
            n,
            width,
            data: vec![0.0; n * (width + 1)],
        }
    }
    pub(super) fn add(&mut self, row: usize, col: usize, value: f64) {
        assert!(row >= col && row - col <= self.width);
        self.data[row * (self.width + 1) + row - col] += value;
    }
    fn get(&self, row: usize, col: usize) -> f64 {
        if row - col > self.width {
            0.0
        } else {
            self.data[row * (self.width + 1) + row - col]
        }
    }
    pub(super) fn solve(mut self, mut rhs: Vec<f64>) -> Result<Vec<f64>, String> {
        if rhs.len() != self.n || rhs.iter().chain(&self.data).any(|v| !v.is_finite()) {
            return Err("invalid band reference system".into());
        }
        for i in 0..self.n {
            let start = i.saturating_sub(self.width);
            for j in start..=i {
                let mut v = self.get(i, j);
                for k in start..j {
                    v -= self.get(i, k) * self.get(j, k);
                }
                let v = if i == j {
                    if v <= 0.0 || !v.is_finite() {
                        return Err("band reference is not SPD".into());
                    }
                    v.sqrt()
                } else {
                    v / self.get(j, j)
                };
                self.data[i * (self.width + 1) + i - j] = v;
            }
        }
        for i in 0..self.n {
            for j in i.saturating_sub(self.width)..i {
                rhs[i] -= self.get(i, j) * rhs[j];
            }
            rhs[i] /= self.get(i, i);
        }
        for i in (0..self.n).rev() {
            for j in i + 1..self.n.min(i + self.width + 1) {
                rhs[i] -= self.get(j, i) * rhs[j];
            }
            rhs[i] /= self.get(i, i);
        }
        if rhs.iter().any(|v| !v.is_finite()) {
            return Err("nonfinite band reference solution".into());
        }
        Ok(rhs)
    }
}

fn points(c: Case, row: usize, mut visit: impl FnMut([[f64; 8]; 3], [[f64; 3]; 3], f64, f64)) {
    let (nx, ny) = c.dimensions();
    let (dx, dy) = (1.0 / nx as f64, HEIGHT / ny as f64);
    let layer = usize::from(row >= ny / 2);
    let e = c.young[layer];
    let nu = c.poisson[layer];
    let d = e / (1.0 - nu * nu);
    let material = [
        [d, nu * d, 0.0],
        [nu * d, d, 0.0],
        [0.0, 0.0, e / (2.0 * (1.0 + nu))],
    ];
    let q = 1.0 / 3.0_f64.sqrt();
    for xi in [-q, q] {
        for eta in [-q, q] {
            let mut b = [[0.0; 8]; 3];
            for (i, (sx, sy)) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)]
                .into_iter()
                .enumerate()
            {
                let gx = sx * (1.0 + sy * eta) / (2.0 * dx);
                let gy = sy * (1.0 + sx * xi) / (2.0 * dy);
                b[0][2 * i] = gx;
                b[1][2 * i + 1] = gy;
                b[2][2 * i] = gy;
                b[2][2 * i + 1] = gx;
            }
            let y = (row as f64 + (eta + 1.0) / 2.0) * dy;
            let eigen = c.alpha[layer] * (c.temperature(y) - c.reference);
            visit(b, material, eigen, dx * dy * THICKNESS / 4.0);
        }
    }
}

fn indices(nx: usize, row: usize, col: usize) -> [usize; 8] {
    let a = row * (nx + 1) + col;
    let nodes = [a, a + 1, a + nx + 2, a + nx + 1];
    std::array::from_fn(|i| 2 * nodes[i / 2] + i % 2)
}

fn solve(c: Case) -> Result<Vec<f64>, String> {
    let (nx, ny) = c.dimensions();
    if nx > 64 || ny > 32 {
        return Err("direct reference mesh exceeds bounded scope".into());
    }
    let n = 2 * (nx + 1) * (ny + 1);
    let mut a = Band::new(n, 2 * nx + 5);
    let (_, seed) = c.models();
    let mut rhs: Vec<_> = seed["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|v| [v["load_x"].as_f64().unwrap(), v["load_y"].as_f64().unwrap()])
        .collect();
    let fixed = [0, 1, 2 * nx + 1];
    for row in 0..ny {
        for col in 0..nx {
            let ids = indices(nx, row, col);
            points(c, row, |b, d, t, w| {
                let db: [[f64; 8]; 3] = std::array::from_fn(|i| {
                    std::array::from_fn(|j| (0..3).map(|k| d[i][k] * b[k][j]).sum())
                });
                for i in 0..8 {
                    if fixed.contains(&ids[i]) {
                        continue;
                    }
                    rhs[ids[i]] += (0..3)
                        .map(|k| b[k][i] * (d[k][0] + d[k][1]) * t)
                        .sum::<f64>()
                        * w;
                    for j in 0..8 {
                        if ids[i] >= ids[j] && !fixed.contains(&ids[j]) {
                            a.add(
                                ids[i],
                                ids[j],
                                (0..3).map(|k| b[k][i] * db[k][j]).sum::<f64>() * w,
                            );
                        }
                    }
                }
            });
        }
    }
    for i in fixed {
        a.add(i, i, 1.0);
        rhs[i] = 0.0;
    }
    a.solve(rhs)
}

fn energy(c: Case, u: &[f64]) -> f64 {
    let (nx, ny) = c.dimensions();
    let mut energy = 0.0;
    for row in 0..ny {
        for col in 0..nx {
            let ids = indices(nx, row, col);
            points(c, row, |b, d, t, w| {
                let e: [f64; 3] = std::array::from_fn(|k| {
                    (0..8).map(|i| b[k][i] * u[ids[i]]).sum::<f64>() - if k < 2 { t } else { 0.0 }
                });
                energy += (0..3)
                    .map(|i| (0..3).map(|j| e[i] * d[i][j] * e[j]).sum::<f64>())
                    .sum::<f64>()
                    * w
                    / 2.0;
            });
        }
    }
    energy
}

pub fn local_summary(c: Case) -> Result<Value, String> {
    let u = solve(c)?;
    let actual = energy(c, &u);
    let exact = c.energy();
    if !actual.is_finite() || !exact.is_finite() || exact <= 1e-12 {
        return Err("Q4 order comparison requires finite nonzero reference energy".into());
    }
    let (nx, ny) = c.dimensions();
    Ok(
        json!({"mesh":[nx,ny],"nodes":u.len()/2,"mechanical_free_dofs":u.len()-3,
        "energy_j":actual,"reference_energy_j":exact,"energy_error_j":(actual-exact).abs(),
        "ordinary_relative_energy_error":(actual-exact).abs()/exact,
        "scope":"local_independent_q4_reference_not_agent_execution",
        "temperature_input":"analytical_layerwise_linear_reference",
        "heat_solved":false,"production_operator":false}),
    )
}

pub fn validate(c: Case, result: &Value) -> Result<Value, String> {
    let u = solve(c)?;
    let root = if result.get("artifacts").is_some() {
        result
    } else {
        &result["result"]
    };
    let s = &root["artifacts"]["structure_out.result"];
    let nodes = s["nodes"]
        .as_array()
        .ok_or("direct comparison nodes absent")?;
    if nodes.len() * 2 != u.len() {
        return Err("direct comparison node count".into());
    }
    let mut delta2 = 0.0;
    let mut norm2 = 0.0;
    let mut max = 0.0_f64;
    for (i, node) in nodes.iter().enumerate() {
        for (axis, k) in ["ux", "uy"].into_iter().enumerate() {
            let x = node[k]
                .as_f64()
                .filter(|x| x.is_finite())
                .ok_or("direct comparison displacement invalid")?;
            let e = x - u[2 * i + axis];
            delta2 += e * e;
            norm2 += u[2 * i + axis].powi(2);
            max = max.max(e.abs());
        }
    }
    let direct_energy = energy(c, &u);
    let actual = s["total_strain_energy"]
        .as_f64()
        .filter(|x| x.is_finite())
        .ok_or("direct comparison energy invalid")?;
    let relative = (delta2 / norm2).sqrt();
    let energy_gap = (actual - direct_energy).abs();
    Ok(
        json!({"passed":relative.is_finite() && relative<=1e-5 && energy_gap<=1e-6,
        "method":"independent_rectangular_q4_banded_cholesky_no_regularization",
        "relative_displacement_difference":relative,"max_displacement_difference_m":max,
        "direct_energy_j":direct_energy,"energy_difference_j":energy_gap,
        "relative_displacement_limit":1e-5,"energy_difference_limit_j":1e-6,
        "continuum_reference_energy_j":c.energy(),"same_discretization_not_continuum_exact":true}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn interpolant_error_components(c: Case) -> [f64; 2] {
        let (nx, ny) = c.dimensions();
        let (hx, hy) = (1.0 / nx as f64, HEIGHT / ny as f64);
        let k = c.axial()[1];
        let volume_per_layer = HEIGHT * THICKNESS / 2.0;
        let mut terms = [0.0; 2];
        for i in 0..2 {
            let nu = c.poisson[i];
            let ey_slope = (1.0 + nu) * c.eigenstrain(i)[1] - nu * k;
            // Iu has exact ex, ey error -ey_slope*(y-y_mid), and gamma=k*(x-x_mid).
            terms[0] +=
                c.young[i] / (1.0 - nu * nu) * ey_slope.powi(2) * hy * hy * volume_per_layer / 24.0;
            terms[1] += c.young[i] / (2.0 * (1.0 + nu)) * k * k * hx * hx * volume_per_layer / 24.0;
        }
        terms
    }

    fn interpolant(c: Case) -> Vec<f64> {
        let (nx, ny) = c.dimensions();
        (0..=ny)
            .flat_map(|j| {
                (0..=nx).flat_map(move |i| {
                    c.displacement(i as f64 / nx as f64, j as f64 * HEIGHT / ny as f64)
                })
            })
            .collect()
    }

    #[test]
    fn interpolant_error_matches_closed_form_and_directional_scaling() {
        let c = Case::new("gradient_free", 1).unwrap();
        let [normal, shear] = interpolant_error_components(c);
        assert!((normal - 0.11538461538461539).abs() < 1e-12);
        assert!((shear - 0.16153846153846155).abs() < 1e-12);
        for name in super::super::bending_model::ENERGY_NAMES {
            let mut previous: Option<[f64; 2]> = None;
            for r in [1, 2, 4] {
                let c = Case::new(name, r).unwrap();
                let components = interpolant_error_components(c);
                let total: f64 = components.iter().sum();
                assert!((energy(c, &interpolant(c)) - c.energy() - total).abs() < 1e-11);
                if let Some([n, s]) = previous {
                    let want = [
                        if name == "energy_x" { n } else { n / 4.0 },
                        if name == "energy_y" { s } else { s / 4.0 },
                    ];
                    for (value, target) in components.iter().zip(want) {
                        assert!((value - target).abs() < 1e-12);
                    }
                }
                previous = Some(components);
            }
        }
    }

    #[test]
    #[ignore = "explicit offline audit of owned retained results; no API or new solves"]
    fn bending_interpolation_audit_retained_results() {
        use sha2::{Digest, Sha256};
        let root = std::path::PathBuf::from(
            std::env::var("KYUUBIKI_BENDING_REPLAY_DIR")
                .expect("set retained energy-run directory"),
        );
        let mut rows = vec![];
        let mut jobs = std::collections::BTreeSet::new();
        for name in super::super::bending_model::ENERGY_NAMES {
            for r in [1, 2, 4] {
                let c = Case::new(name, r).unwrap();
                let id = c.id();
                let read = |suffix| {
                    let path = root.join(format!("{id}-{suffix}.json"));
                    let m = std::fs::symlink_metadata(&path).unwrap();
                    assert!(m.is_file() && m.len() < 64 * 1024 * 1024);
                    std::fs::read(path).unwrap()
                };
                let evidence: Value = serde_json::from_slice(&read("evidence")).unwrap();
                assert_eq!(evidence["case"], id);
                let job = evidence["job_id"]
                    .as_str()
                    .filter(|s| !s.is_empty())
                    .unwrap();
                assert!(jobs.insert(job.to_owned()));
                for kind in ["request", "result"] {
                    assert_eq!(
                        format!("{:x}", Sha256::digest(read(kind))),
                        evidence[format!("{kind}_sha256")].as_str().unwrap()
                    );
                }
                let terminal: Value = serde_json::from_slice(&read("terminal")).unwrap();
                assert_eq!(terminal["job"]["job_id"], job);
                assert_eq!(terminal["job"]["status"], "completed");
                let result: Value = serde_json::from_slice(&read("result")).unwrap();
                let checks = super::super::bending_checks::validate(c, &result).unwrap();
                assert_eq!(checks["passed"], true);
                let payload = if result.get("artifacts").is_some() {
                    &result
                } else {
                    &result["result"]
                };
                let u: Vec<_> = payload["artifacts"]["structure_out.result"]["nodes"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .flat_map(|n| [n["ux"].as_f64().unwrap(), n["uy"].as_f64().unwrap()])
                    .collect();
                let sampled = interpolant(c);
                let delta: Vec<_> = sampled.iter().zip(u).map(|(a, b)| a - b).collect();
                let [normal, shear] = interpolant_error_components(c);
                let exact_interpolant_error = normal + shear;
                let quadrature_error =
                    (energy(c, &sampled) - c.energy() - exact_interpolant_error).abs();
                let mut no_eigenstrain = c;
                no_eigenstrain.alpha = [0.0; 2];
                let redistribution = energy(no_eigenstrain, &delta);
                let fe_error = checks["error_energy_j"].as_f64().unwrap();
                let identity_error = (exact_interpolant_error - fe_error - redistribution).abs();
                assert!(
                    [
                        normal,
                        shear,
                        redistribution,
                        identity_error,
                        quadrature_error
                    ]
                    .iter()
                    .all(|x| x.is_finite() && *x >= 0.0)
                );
                assert!(
                    quadrature_error <= 1e-11 && identity_error <= 1e-9,
                    "{id}: quadrature={quadrature_error}, orthogonality={identity_error}"
                );
                rows.push(json!({"case":id,"job_id":job,"passed":true,
                    "mesh":[c.dimensions().0,c.dimensions().1],
                    "interpolant_normal_error_energy_j":normal,"interpolant_shear_error_energy_j":shear,
                    "interpolant_error_energy_j":exact_interpolant_error,
                    "equilibrated_error_energy_j":fe_error,"redistribution_energy_j":redistribution,
                    "galerkin_identity_error_j":identity_error,"interpolant_quadrature_error_j":quadrature_error,
                    "production_shear_error_energy_j":checks["shear_error_energy_j"],
                    "result_sha256":evidence["result_sha256"]}));
            }
        }
        let diagnostic = json!({"passed":true,"cases":rows,"new_workflows":0,"new_model_calls":0,
            "source_sha256":format!("{:x}",Sha256::digest(include_str!("bending_direct.rs"))),
            "scope":"offline_interpolant_vs_equilibrated_rectangular_q4_diagnostic",
            "galerkin_identity_tolerance_j":1e-9,"quadrature_tolerance_j":1e-11,
            "model_reviewed":false,"production_acceptance_gates_changed":false});
        super::super::provider::retain(&root, "interpolation-audit-20261111.json", &diagnostic)
            .unwrap();
        println!("offline interpolation audit: 12 retained jobs; no new model calls or workflows");
    }

    #[test]
    fn band_cholesky_solves_known_system_and_rejects_non_spd() {
        let mut a = Band::new(4, 1);
        for i in 0..4 {
            a.add(i, i, 4.0);
            if i > 0 {
                a.add(i, i - 1, -1.0);
            }
        }
        let x = a.solve(vec![2.0, 4.0, 6.0, 13.0]).unwrap();
        for (got, want) in x.iter().zip([1.0, 2.0, 3.0, 4.0]) {
            assert!((got - want).abs() < 1e-13);
        }
        assert!(Band::new(2, 1).solve(vec![1.0, 1.0]).is_err());
    }
    #[test]
    fn independent_q4_recovers_affine_thermal_patch() {
        let mut c = Case::new("gradient_free", 1).unwrap();
        c.bottom = 30.0;
        c.top = 30.0;
        let u = solve(c).unwrap();
        let (_, model) = c.models();
        for (i, n) in model["nodes"].as_array().unwrap().iter().enumerate() {
            let want = c.displacement(n["x"].as_f64().unwrap(), n["y"].as_f64().unwrap());
            assert!((u[2 * i] - want[0]).abs() < 1e-14);
            assert!((u[2 * i + 1] - want[1]).abs() < 1e-14);
        }
        assert!(energy(c, &u) < 1e-20);
    }
    #[test]
    fn comparison_rejects_changed_displacements_and_reported_energy() {
        let c = Case::new("bilayer_gradient", 1).unwrap();
        let u = solve(c).unwrap();
        let nodes: Vec<_> = u
            .chunks_exact(2)
            .map(|a| json!({"ux":a[0],"uy":a[1]}))
            .collect();
        let v = json!({"artifacts":{"structure_out.result":{"nodes":nodes,"total_strain_energy":energy(c,&u)}}});
        assert_eq!(validate(c, &v).unwrap()["passed"], true);
        for path in [
            "/artifacts/structure_out.result/nodes/1/ux",
            "/artifacts/structure_out.result/total_strain_energy",
        ] {
            let mut bad = v.clone();
            *bad.pointer_mut(path).unwrap() = json!(1.0);
            assert_eq!(validate(c, &bad).unwrap()["passed"], false);
        }
    }
}
