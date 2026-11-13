use serde_json::{Value, json};

pub const NAMES: [&str; 8] = [
    "free_hot",
    "free_cold",
    "tension",
    "tension_distorted",
    "shear",
    "shear_distorted",
    "mixed_hot",
    "mixed_auxetic",
];
pub const YOUNG: f64 = 70e9;
pub const ALPHA: f64 = 12e-6;
pub const HEIGHT: f64 = 0.25;
pub const THICKNESS: f64 = 0.01;

#[derive(Clone, Copy, PartialEq)]
pub struct Case {
    pub name: &'static str,
    pub refinement: usize,
    pub poisson: f64,
    pub rise: f64,
    pub stress: [f64; 3],
    pub distorted: bool,
}

impl Case {
    pub fn new(name: &str, refinement: usize) -> Result<Self, String> {
        let name = *NAMES
            .iter()
            .find(|n| **n == name)
            .ok_or("unknown planar patch")?;
        if ![1, 2, 4, 8].contains(&refinement) {
            return Err("unapproved planar refinement".into());
        }
        let mut c = Self {
            name,
            refinement,
            poisson: 0.3,
            rise: 0.0,
            stress: [0.0; 3],
            distorted: !matches!(name, "tension" | "shear"),
        };
        match name {
            "free_hot" => c.rise = 10.0,
            "free_cold" => c.rise = -10.0,
            "tension" | "tension_distorted" => c.stress[0] = 1e6,
            "shear" | "shear_distorted" => c.stress[2] = 0.5e6,
            "mixed_hot" | "mixed_auxetic" => {
                c.stress = [1.2e6, -0.4e6, 0.3e6];
                c.poisson = if name == "mixed_hot" { 0.49 } else { -0.2 };
                c.rise = if name == "mixed_hot" { 10.0 } else { -10.0 };
            }
            _ => unreachable!(),
        }
        Ok(c)
    }

    pub fn id(self) -> String {
        format!("planar-{}-r{}-v1", self.name, self.refinement)
    }

    pub fn strain(self) -> [f64; 3] {
        let [sx, sy, tau] = self.stress;
        [
            (sx - self.poisson * sy) / YOUNG + ALPHA * self.rise,
            (sy - self.poisson * sx) / YOUNG + ALPHA * self.rise,
            2.0 * (1.0 + self.poisson) * tau / YOUNG,
        ]
    }

    pub fn displacement(self, x: f64, y: f64) -> [f64; 2] {
        let [ex, ey, gamma] = self.strain();
        [ex * x + gamma * y, ey * y]
    }

    pub fn energy_density(self) -> f64 {
        let [ex, ey, gamma] = self.strain();
        let [sx, sy, tau] = self.stress;
        0.5 * (sx * (ex - ALPHA * self.rise) + sy * (ey - ALPHA * self.rise) + tau * gamma)
    }

    pub fn models(self) -> (Value, Value) {
        let n = 2 * self.refinement;
        let mut heat = vec![];
        let mut structure = vec![];
        for j in 0..=n {
            for i in 0..=n {
                let u = i as f64 / n as f64;
                let v = j as f64 / n as f64;
                let boundary = i == 0 || j == 0 || i == n || j == n;
                let warp = if self.distorted && !boundary {
                    (std::f64::consts::PI * u).sin() * (std::f64::consts::PI * v).sin()
                } else {
                    0.0
                };
                let x = u + 0.18 * warp / n as f64;
                let y = HEIGHT * (v - 0.16 * warp / n as f64);
                heat.push(json!({"id":format!("n{i}_{j}"),"x":x,"y":y,
                    "temperature":20.0+self.rise,"fix_temperature":boundary,"heat_load":0.0}));
                structure.push(json!({"id":format!("n{i}_{j}"),"x":x,"y":y,
                    "fix_x":i==0 && j==0,"fix_y":j==0 && (i==0 || i==n),
                    "temperature_delta":0.0,"load_x":0.0,"load_y":0.0}));
            }
        }
        let mut h_elements = vec![];
        let mut s_elements = vec![];
        for j in 0..n {
            for i in 0..n {
                let a = j * (n + 1) + i;
                let base = json!({"id":format!("e{i}_{j}"),"node_i":a,"node_j":a+1,
                    "node_k":a+n+2,"node_l":a+n+1,"thickness":THICKNESS});
                let mut h = base.clone();
                h["conductivity"] = json!(20.0);
                h_elements.push(h);
                let mut s = base;
                s["youngs_modulus"] = json!(YOUNG);
                s["poisson_ratio"] = json!(self.poisson);
                s["thermal_expansion"] = json!(ALPHA);
                s_elements.push(s);
            }
        }
        // CCW outer boundary: sigma*n integrated exactly on each straight edge.
        let boundary: Vec<_> = (0..n)
            .chain((0..n).map(|j| j * (n + 1) + n))
            .chain((1..=n).rev().map(|i| n * (n + 1) + i))
            .chain((1..=n).rev().map(|j| j * (n + 1)))
            .collect();
        for edge in 0..boundary.len() {
            let a = boundary[edge];
            let b = boundary[(edge + 1) % boundary.len()];
            let dx = structure[b]["x"].as_f64().unwrap() - structure[a]["x"].as_f64().unwrap();
            let dy = structure[b]["y"].as_f64().unwrap() - structure[a]["y"].as_f64().unwrap();
            let [sx, sy, tau] = self.stress;
            let force = [
                (sx * dy - tau * dx) * THICKNESS / 2.0,
                (tau * dy - sy * dx) * THICKNESS / 2.0,
            ];
            for index in [a, b] {
                for (field, f) in ["load_x", "load_y"].into_iter().zip(force) {
                    structure[index][field] = json!(structure[index][field].as_f64().unwrap() + f);
                }
            }
        }
        (
            json!({"nodes":heat,"elements":h_elements}),
            json!({"nodes":structure,"elements":s_elements}),
        )
    }

    pub fn workflow(self) -> (Value, Value) {
        let (heat, seed) = self.models();
        super::model::workflow_for(&self.id(), heat, seed, 20.0)
    }
}

pub fn catalog() -> Value {
    json!(
        NAMES
            .iter()
            .map(|name| {
                let c = Case::new(name, 1).unwrap();
                json!({"scenario":name,"youngs_modulus":YOUNG,"thermal_expansion":ALPHA,
            "poisson":c.poisson,"reference_temperature":20.0,"temperature":20.0+c.rise,
            "boundary_stress_pa":c.stress,"distorted":c.distorted,"fixed_dofs":3})
            })
            .collect::<Vec<_>>()
    )
}

pub fn observation(row: &Value) -> Value {
    json!({"scenario":row["scenario"],"case":row["case"],"refinement":row["refinement"],
        "job_id":row["job_id"],"physics_passed":row["physics_passed"],"readback_matches":row["readback_matches"],
        "reused_retained_evidence":row["reused_retained_evidence"],"planar_validation":row["planar_validation"]})
}

pub fn verify(rows: &[Value]) -> Result<Value, String> {
    let mut jobs = std::collections::BTreeSet::new();
    let mut cases = vec![];
    for (name, ux, uy, energy) in [
        ("free_hot", 120e-6, 30e-6, 0.0),
        ("free_cold", -120e-6, -30e-6, 0.0),
        ("tension", 1e-3 / 70.0, -75e-6 / 70.0, 0.01785714285714286),
        (
            "tension_distorted",
            1e-3 / 70.0,
            -75e-6 / 70.0,
            0.01785714285714286,
        ),
        ("shear", 325e-6 / 70.0, 0.0, 0.01160714285714286),
        ("shear_distorted", 325e-6 / 70.0, 0.0, 0.01160714285714286),
        (
            "mixed_hot",
            143.1357142857143e-6,
            26.47142857142857e-6,
            0.04176071428571429,
        ),
        (
            "mixed_auxetic",
            -102.2857142857143e-6,
            -30.57142857142857e-6,
            0.02771428571428572,
        ),
    ] {
        for r in [1, 2, 4, 8] {
            let c = Case::new(name, r)?;
            let row = rows
                .iter()
                .find(|v| v["case"] == c.id())
                .ok_or("missing planar matrix case")?;
            let job = row["job_id"]
                .as_str()
                .filter(|s| !s.is_empty())
                .ok_or("missing planar job")?;
            let v = &row["planar_validation"];
            if !jobs.insert(job)
                || row["physics_passed"] != true
                || row["readback_matches"] != true
                || row["reused_retained_evidence"] != false
                || v["passed"] != true
            {
                return Err("planar evidence mismatch".into());
            }
            for (k, expected, tol) in [
                ("corner_ux", ux, 1e-10),
                ("corner_uy", uy, 1e-10),
                ("observed_energy_j", energy, 1e-9),
            ] {
                let actual = v[k]
                    .as_f64()
                    .filter(|v| v.is_finite())
                    .ok_or("missing planar reference metric")?;
                if (actual - expected).abs() > tol {
                    return Err(format!("independent planar {name} {k} mismatch"));
                }
            }
            cases.push(observation(row));
        }
    }
    Ok(json!({"passed":true,"scope":"eight_affine_planar_patches_four_meshes","cases":cases}))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hand_references_separate_poisson_shear_and_thermal_displacement() {
        for (name, corner, energy) in [
            ("free_hot", [120e-6, 30e-6], 0.0),
            ("free_cold", [-120e-6, -30e-6], 0.0),
            ("tension", [1e-3 / 70.0, -75e-6 / 70.0], 0.01785714285714286),
            ("shear", [325e-6 / 70.0, 0.0], 0.01160714285714286),
            (
                "mixed_hot",
                [143.1357142857143e-6, 26.47142857142857e-6],
                0.04176071428571429,
            ),
            (
                "mixed_auxetic",
                [-102.2857142857143e-6, -30.57142857142857e-6],
                0.02771428571428572,
            ),
        ] {
            let c = Case::new(name, 1).unwrap();
            let actual = c.displacement(1.0, HEIGHT);
            for i in 0..2 {
                assert!((actual[i] - corner[i]).abs() < 1e-14, "{name}");
            }
            assert!(
                (c.energy_density() * HEIGHT * THICKNESS - energy).abs() < 1e-14,
                "{name}"
            );
        }
    }

    #[test]
    fn only_rigid_modes_are_pinned_and_boundary_loads_balance() {
        for name in NAMES {
            for r in [1, 2, 4, 8] {
                let c = Case::new(name, r).unwrap();
                let (_, s) = c.models();
                let nodes = s["nodes"].as_array().unwrap();
                assert_eq!(
                    nodes
                        .iter()
                        .map(|n| usize::from(n["fix_x"] == true) + usize::from(n["fix_y"] == true))
                        .sum::<usize>(),
                    3
                );
                let (mut fx, mut fy, mut m, mut right) = (0.0, 0.0, 0.0, 0.0);
                for p in nodes {
                    let x = p["x"].as_f64().unwrap();
                    let y = p["y"].as_f64().unwrap();
                    let a = p["load_x"].as_f64().unwrap();
                    let b = p["load_y"].as_f64().unwrap();
                    fx += a;
                    fy += b;
                    m += x * b - y * a;
                    if x == 1.0 {
                        right += a;
                    }
                }
                assert!(
                    fx.abs() < 1e-9 && fy.abs() < 1e-9 && m.abs() < 1e-9,
                    "{name} {r}"
                );
                if name == "tension" {
                    assert!((right - 2500.0).abs() < 1e-9);
                }
            }
        }
        assert!(Case::new("shell", 2).is_err());
        assert!(Case::new("free_hot", 200).is_err());
    }
}
