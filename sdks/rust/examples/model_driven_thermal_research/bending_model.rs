use serde_json::{Value, json};

pub const HEIGHT: f64 = 0.25;
pub const THICKNESS: f64 = 0.01;
pub const ENERGY_NAMES: [&str; 4] = ["energy_isotropic", "energy_x", "energy_y", "energy_uniform"];
pub const NAMES: [&str; 6] = [
    "gradient_free",
    "gradient_free_reverse",
    "bilayer_gradient",
    "bilayer_gradient_reverse",
    "bilayer_uniform",
    "bilayer_shift",
];

#[derive(Clone, Copy, PartialEq)]
pub struct Case {
    pub name: &'static str,
    pub refinement: usize,
    pub young: [f64; 2],
    pub poisson: [f64; 2],
    pub alpha: [f64; 2],
    pub conductivity: [f64; 2],
    pub reference: f64,
    pub bottom: f64,
    pub top: f64,
}

impl Case {
    pub fn new(name: &str, refinement: usize) -> Result<Self, String> {
        let name = *NAMES
            .iter()
            .chain(ENERGY_NAMES.iter())
            .find(|n| **n == name)
            .ok_or("unknown bending case")?;
        if ![1, 2, 4, 8].contains(&refinement) || (ENERGY_NAMES.contains(&name) && refinement > 4) {
            return Err("unapproved bending refinement".into());
        }
        let mut c = Self {
            name,
            refinement,
            young: [70e9, 20e9],
            poisson: [0.2, 0.35],
            alpha: [10e-6, 30e-6],
            conductivity: [10.0, 40.0],
            reference: 20.0,
            bottom: 20.0,
            top: 40.0,
        };
        if name.starts_with("gradient_free") {
            c.young = [70e9; 2];
            c.poisson = [0.3; 2];
            c.alpha = [12e-6; 2];
            c.conductivity = [20.0; 2];
        }
        if name.ends_with("reverse") {
            c.top = 0.0;
        }
        if name == "bilayer_uniform" || name == "energy_uniform" {
            c.bottom = 30.0;
            c.top = 30.0;
        }
        if name == "bilayer_shift" {
            c.reference = 293.15;
            c.bottom = 293.15;
            c.top = 313.15;
        }
        Ok(c)
    }

    pub fn id(self) -> String {
        format!("bending-{}-r{}-v1", self.name, self.refinement)
    }
    pub fn dimensions(self) -> (usize, usize) {
        let r = self.refinement;
        match self.name {
            "energy_isotropic" | "energy_uniform" => (16 * r, 8 * r),
            "energy_x" => (16 * r, 16),
            "energy_y" => (32, 8 * r),
            _ => (4 * r, 2 * r),
        }
    }
    pub fn flux(self) -> f64 {
        (self.bottom - self.top)
            / (HEIGHT / 2.0 * (1.0 / self.conductivity[0] + 1.0 / self.conductivity[1]))
    }
    pub fn temperature(self, y: f64) -> f64 {
        self.bottom
            - self.flux()
                * (y.min(HEIGHT / 2.0) / self.conductivity[0]
                    + (y - HEIGHT / 2.0).max(0.0) / self.conductivity[1])
    }
    // Thermal eigenstrain = a + b*y within each layer.
    pub fn eigenstrain(self, layer: usize) -> [f64; 2] {
        let y = layer as f64 * HEIGHT / 2.0;
        let b = -self.alpha[layer] * self.flux() / self.conductivity[layer];
        [
            self.alpha[layer] * (self.temperature(y) - self.reference) - b * y,
            b,
        ]
    }
    pub fn axial(self) -> [f64; 2] {
        let (mut a, mut b, mut d, mut n, mut m) = (0.0, 0.0, 0.0, 0.0, 0.0);
        for i in 0..2 {
            let lo = i as f64 * HEIGHT / 2.0;
            let hi = lo + HEIGHT / 2.0;
            let i0 = hi - lo;
            let i1 = (hi * hi - lo * lo) / 2.0;
            let i2 = (hi.powi(3) - lo.powi(3)) / 3.0;
            let [t, s] = self.eigenstrain(i);
            let e = self.young[i];
            a += e * i0;
            b += e * i1;
            d += e * i2;
            n += e * (t * i0 + s * i1);
            m += e * (t * i1 + s * i2);
        }
        let det = a * d - b * b;
        [(n * d - m * b) / det, (m * a - n * b) / det]
    }
    pub fn strain(self, y: f64, layer: usize) -> [f64; 3] {
        let [e, k] = self.axial();
        let [a, b] = self.eigenstrain(layer);
        let ex = e + k * y;
        let thermal = a + b * y;
        [
            ex,
            (1.0 + self.poisson[layer]) * thermal - self.poisson[layer] * ex,
            0.0,
        ]
    }
    pub fn stress(self, y: f64, layer: usize) -> f64 {
        let [e, k] = self.axial();
        let [a, b] = self.eigenstrain(layer);
        self.young[layer] * (e + k * y - a - b * y)
    }
    pub fn displacement(self, x: f64, y: f64) -> [f64; 2] {
        let [e, k] = self.axial();
        let mut v = 0.0;
        for i in 0..2 {
            let lo = i as f64 * HEIGHT / 2.0;
            let hi = y.min(lo + HEIGHT / 2.0);
            if hi <= lo {
                continue;
            }
            let [a, b] = self.eigenstrain(i);
            let nu = self.poisson[i];
            v += ((1.0 + nu) * a - nu * e) * (hi - lo)
                + ((1.0 + nu) * b - nu * k) * (hi * hi - lo * lo) / 2.0;
        }
        [(e + k * y) * x - k * y / 2.0, v + k * x * (1.0 - x) / 2.0]
    }
    pub fn energy(self) -> f64 {
        let [e, k] = self.axial();
        (0..2)
            .map(|i| {
                let lo = i as f64 * HEIGHT / 2.0;
                let hi = lo + HEIGHT / 2.0;
                let [a, b] = self.eigenstrain(i);
                let c = e - a;
                let d = k - b;
                0.5 * self.young[i]
                    * THICKNESS
                    * (c * c * (hi - lo)
                        + c * d * (hi * hi - lo * lo)
                        + d * d * (hi.powi(3) - lo.powi(3)) / 3.0)
            })
            .sum()
    }
    pub fn models(self) -> (Value, Value) {
        let (nx, ny) = self.dimensions();
        let mut heat = vec![];
        let mut nodes = vec![];
        for j in 0..=ny {
            for i in 0..=nx {
                let x = i as f64 / nx as f64;
                let y = HEIGHT * j as f64 / ny as f64;
                heat.push(json!({"id":format!("n{i}_{j}"),"x":x,"y":y,
                "temperature":if j==0 || j==ny {self.temperature(y)}else{self.reference},
                "fix_temperature":j==0 || j==ny,"heat_load":0.0}));
                nodes.push(json!({"id":format!("n{i}_{j}"),"x":x,"y":y,
                "fix_x":i==0 && j==0,"fix_y":j==0 && (i==0 || i==nx),
                "temperature_delta":0.0,"load_x":0.0,"load_y":0.0}));
            }
        }
        let mut hs = vec![];
        let mut ss = vec![];
        for j in 0..ny {
            for i in 0..nx {
                let layer = usize::from(j >= ny / 2);
                let a = j * (nx + 1) + i;
                let base = json!({"id":format!("e{i}_{j}"),"node_i":a,"node_j":a+1,
                "node_k":a+nx+2,"node_l":a+nx+1,"thickness":THICKNESS});
                let mut h = base.clone();
                h["conductivity"] = json!(self.conductivity[layer]);
                hs.push(h);
                let mut s = base;
                s["youngs_modulus"] = json!(self.young[layer]);
                s["poisson_ratio"] = json!(self.poisson[layer]);
                s["thermal_expansion"] = json!(self.alpha[layer]);
                ss.push(s);
            }
        }
        // Consistent nodal integrals of the layerwise-linear end traction.
        if !self.name.starts_with("gradient_free") {
            for j in 0..ny {
                let layer = usize::from(j >= ny / 2);
                let dy = HEIGHT / ny as f64;
                let a = self.stress(j as f64 * dy, layer);
                let b = self.stress((j + 1) as f64 * dy, layer);
                for (i, sign) in [(0, -1.0), (nx, 1.0)] {
                    for (row, force) in [
                        (j, (2.0 * a + b) * dy * THICKNESS / 6.0),
                        (j + 1, (a + 2.0 * b) * dy * THICKNESS / 6.0),
                    ] {
                        let n = &mut nodes[row * (nx + 1) + i];
                        n["load_x"] = json!(n["load_x"].as_f64().unwrap() + sign * force);
                    }
                }
            }
        }
        (
            json!({"nodes":heat,"elements":hs}),
            json!({"nodes":nodes,"elements":ss}),
        )
    }
    pub fn workflow(self) -> (Value, Value) {
        let (heat, seed) = self.models();
        super::model::workflow_for(&self.id(), heat, seed, self.reference)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn energy_diagnostics_use_bounded_directional_meshes() {
        for (name, dimensions) in [
            ("energy_isotropic", (64, 32)),
            ("energy_x", (64, 16)),
            ("energy_y", (32, 32)),
            ("energy_uniform", (64, 32)),
        ] {
            let c = Case::new(name, 4).unwrap();
            assert_eq!(c.dimensions(), dimensions);
            assert!(Case::new(name, 8).is_err());
            let base = Case::new(
                if name == "energy_uniform" {
                    "bilayer_uniform"
                } else {
                    "bilayer_gradient"
                },
                1,
            )
            .unwrap();
            assert_eq!(c.energy(), base.energy());
            assert_eq!(c.displacement(0.5, 0.0), base.displacement(0.5, 0.0));
        }
        assert!(Case::new("bilayer_gradient", 16).is_err());
    }
    #[test]
    fn thermal_gradient_requires_curvature_and_only_rigid_restraints() {
        let c = Case::new("gradient_free", 1).unwrap();
        assert!((c.axial()[1] - 0.00096).abs() < 1e-14);
        assert!((c.displacement(0.5, 0.0)[1] - 120e-6).abs() < 1e-14);
        assert!(c.energy().abs() < 1e-12);
        assert!(c.displacement(0.0, 0.0).iter().all(|v| v.abs() < 1e-14));
        assert!(c.displacement(1.0, 0.0)[1].abs() < 1e-14);
        let (_, s) = c.models();
        let ns = s["nodes"].as_array().unwrap();
        assert_eq!(
            ns.iter()
                .map(|n| usize::from(n["fix_x"] == true) + usize::from(n["fix_y"] == true))
                .sum::<usize>(),
            3
        );
        assert!(ns.iter().all(|n| n["load_x"].as_f64() == Some(0.0)));
    }

    #[test]
    fn interior_heat_nodes_are_not_prefilled_with_the_reference_solution() {
        for name in [
            "gradient_free",
            "bilayer_gradient",
            "bilayer_uniform",
            "bilayer_shift",
        ] {
            let c = Case::new(name, 2).unwrap();
            let (h, _) = c.models();
            let mut different = 0;
            for n in h["nodes"].as_array().unwrap() {
                let t = n["temperature"].as_f64().unwrap();
                let exact = c.temperature(n["y"].as_f64().unwrap());
                if n["fix_temperature"] == true {
                    assert_eq!(t, exact);
                } else {
                    assert_eq!(t, c.reference, "free heat node must not carry the answer");
                    different += usize::from((t - exact).abs() > 1e-6);
                }
            }
            assert!(different > 0);
        }
    }
    #[test]
    fn layered_reference_has_continuous_displacement_and_balanced_end_loads() {
        for name in NAMES {
            for r in [1, 2, 4, 8] {
                let c = Case::new(name, r).unwrap();
                let (_, s) = c.models();
                let mut force = 0.0;
                let mut moment = 0.0;
                for n in s["nodes"].as_array().unwrap() {
                    if n["x"].as_f64() == Some(1.0) {
                        let f = n["load_x"].as_f64().unwrap();
                        force += f;
                        moment += f * n["y"].as_f64().unwrap();
                    }
                }
                assert!(
                    force.abs() < 1e-8 && moment.abs() < 1e-8,
                    "{name} {r}: {force} {moment}"
                );
                let left = c.displacement(0.4, HEIGHT / 2.0 - 1e-9);
                let right = c.displacement(0.4, HEIGHT / 2.0 + 1e-9);
                assert!((left[0] - right[0]).abs() < 1e-10 && (left[1] - right[1]).abs() < 1e-10);
            }
        }
        let c = Case::new("bilayer_gradient", 1).unwrap();
        assert!((c.axial()[0] + 7.742_971_887_550_2e-5).abs() < 1e-14);
        assert!((c.axial()[1] - 0.002876144578313253).abs() < 1e-14);
        assert!((c.flux() + 1280.0).abs() < 1e-10);
        let shifted = Case::new("bilayer_shift", 1).unwrap();
        for y in [0.0, 0.125, 0.25] {
            assert!((c.temperature(y) - 20.0 - (shifted.temperature(y) - 293.15)).abs() < 1e-12);
            assert!((c.displacement(0.4, y)[1] - shifted.displacement(0.4, y)[1]).abs() < 1e-14);
        }
        assert!(Case::new("arbitrary", 1).is_err());
        assert!(Case::new("gradient_free", 500).is_err());
    }
}
