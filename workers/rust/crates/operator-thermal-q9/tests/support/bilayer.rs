use kyuubiki_protocol::{
    HeatPlaneNodeInput, HeatPlaneQuadElementInput, SolveHeatPlaneQuad2dRequest,
    SolveThermalQ9RectRequest, THERMAL_Q9_RECT_SCHEMA, ThermalQ9Element, ThermalQ9Node,
};

pub struct Case {
    pub heat: SolveHeatPlaneQuad2dRequest,
    pub seed: SolveThermalQ9RectRequest,
    pub reference_temperature: f64,
    pub energy: f64,
    pub eigen: [[f64; 2]; 2],
    pub extension: f64,
    pub curvature: f64,
    pub temperature: [[f64; 2]; 2],
}

impl Case {
    pub fn new(
        nx: usize,
        ny: usize,
        conductivity: [f64; 2],
        alpha: [f64; 2],
        rise: [f64; 2],
    ) -> Self {
        assert!(ny >= 2 && ny % 2 == 0);
        let reference_temperature = 293.15;
        let flux = (rise[1] - rise[0]) / (0.125 / conductivity[0] + 0.125 / conductivity[1]);
        let slopes = conductivity.map(|k| flux / k);
        let temperature = [
            [rise[0], slopes[0]],
            [rise[0] + 0.125 * (slopes[0] - slopes[1]), slopes[1]],
        ];
        let eigen = std::array::from_fn(|i| temperature[i].map(|x| x * alpha[i]));
        let young = [70e9, 20e9];
        let (mut a, mut b, mut d, mut force, mut moment) = (0.0, 0.0, 0.0, 0.0, 0.0);
        for i in 0..2 {
            let (lo, hi) = (i as f64 * 0.125, (i + 1) as f64 * 0.125);
            let s = [
                hi - lo,
                (hi * hi - lo * lo) / 2.0,
                (hi.powi(3) - lo.powi(3)) / 3.0,
            ];
            a += young[i] * s[0];
            b += young[i] * s[1];
            d += young[i] * s[2];
            force += young[i] * (eigen[i][0] * s[0] + eigen[i][1] * s[1]);
            moment += young[i] * (eigen[i][0] * s[1] + eigen[i][1] * s[2]);
        }
        let extension = (force * d - moment * b) / (a * d - b * b);
        let curvature = (moment * a - force * b) / (a * d - b * b);
        let mut energy = 0.0;
        for i in 0..2 {
            let (lo, hi) = (i as f64 * 0.125, (i + 1) as f64 * 0.125);
            let (c, g) = (extension - eigen[i][0], curvature - eigen[i][1]);
            energy += 0.005
                * young[i]
                * (c * c * (hi - lo)
                    + c * g * (hi * hi - lo * lo)
                    + g * g * (hi.powi(3) - lo.powi(3)) / 3.0);
        }
        let width = 2 * nx + 1;
        let mut heat = SolveHeatPlaneQuad2dRequest {
            nodes: vec![],
            elements: vec![],
            contact_interfaces: vec![],
        };
        let mut seed = SolveThermalQ9RectRequest {
            schema_version: THERMAL_Q9_RECT_SCHEMA.into(),
            nodes: vec![],
            elements: vec![],
        };
        for j in 0..=2 * ny {
            for i in 0..=2 * nx {
                let (x, y) = (
                    i as f64 / (2 * nx) as f64,
                    0.25 * j as f64 / (2 * ny) as f64,
                );
                let id = format!("n{i}_{j}");
                heat.nodes.push(HeatPlaneNodeInput {
                    id: id.clone(),
                    x,
                    y,
                    fix_temperature: j == 0 || j == 2 * ny,
                    temperature: if j == 0 {
                        reference_temperature + rise[0]
                    } else if j == 2 * ny {
                        reference_temperature + rise[1]
                    } else {
                        0.0
                    },
                    heat_load: 0.0,
                });
                seed.nodes.push(ThermalQ9Node {
                    id,
                    x,
                    y,
                    fix_x: i == 0 && j == 0,
                    fix_y: j == 0 && (i == 0 || i == 2 * nx),
                    load_x: 0.0,
                    load_y: 0.0,
                    temperature_delta: 0.0,
                });
            }
        }
        for j in 0..2 * ny {
            for i in 0..2 * nx {
                let n = j * width + i;
                heat.elements.push(HeatPlaneQuadElementInput {
                    id: format!("h{i}_{j}"),
                    node_i: n,
                    node_j: n + 1,
                    node_k: n + 1 + width,
                    node_l: n + width,
                    thickness: 0.01,
                    conductivity: conductivity[usize::from(j >= ny)],
                });
            }
        }
        for j in 0..ny {
            let layer = usize::from(j >= ny / 2);
            for i in 0..nx {
                seed.elements.push(ThermalQ9Element {
                    id: format!("e{i}_{j}"),
                    nodes: std::array::from_fn(|s| (2 * j + s / 3) * width + 2 * i + s % 3),
                    thickness: 0.01,
                    youngs_modulus: young[layer],
                    poisson_ratio: [0.2, 0.35][layer],
                    thermal_expansion: alpha[layer],
                });
            }
            let dy = 0.25 / ny as f64;
            let stress = |y: f64| {
                young[layer] * (extension + curvature * y - eigen[layer][0] - eigen[layer][1] * y)
            };
            let (s0, s1) = (stress(j as f64 * dy), stress((j + 1) as f64 * dy));
            // Independent closed-form quadratic-edge traction integrals.
            for (col, sign) in [(0, -1.0), (2 * nx, 1.0)] {
                for (offset, f) in [s0 / 6.0, (s0 + s1) / 3.0, s1 / 6.0]
                    .into_iter()
                    .enumerate()
                {
                    seed.nodes[(2 * j + offset) * width + col].load_x += sign * f * dy * 0.01;
                }
            }
        }
        Self {
            heat,
            seed,
            reference_temperature,
            energy,
            eigen,
            extension,
            curvature,
            temperature,
        }
    }

    pub fn expected_delta(&self, y: f64) -> f64 {
        let t = self.temperature[usize::from(y > 0.125)];
        t[0] + t[1] * y
    }

    pub fn expected_displacement(&self, x: f64, y: f64) -> [f64; 2] {
        let ux = (self.extension + self.curvature * y) * x - self.curvature * y / 2.0;
        let mut uy = self.curvature * x * (1.0 - x) / 2.0;
        for (i, nu) in [0.2, 0.35].into_iter().enumerate() {
            let lo = i as f64 * 0.125;
            let hi = y.min(lo + 0.125);
            if hi > lo {
                uy += ((1.0 + nu) * self.eigen[i][0] - nu * self.extension) * (hi - lo)
                    + ((1.0 + nu) * self.eigen[i][1] - nu * self.curvature) * (hi * hi - lo * lo)
                        / 2.0;
            }
        }
        [ux, uy]
    }
}
