use serde_json::{Value, json};

#[derive(Clone, Copy)]
pub struct Case {
    pub material: &'static str,
    pub conductivity: [f64; 2],
    pub refinement: usize,
    pub reference: f64,
    pub rise: f64,
}

pub const ALPHA: [f64; 2] = [10e-6, 20e-6];

impl Case {
    pub fn id(self) -> String {
        format!(
            "{}-r{}-ref{:.2}-rise{:.0}",
            self.material, self.refinement, self.reference, self.rise
        )
    }

    pub fn flux(self) -> f64 {
        self.rise / (0.5 / self.conductivity[0] + 0.5 / self.conductivity[1])
    }

    pub fn temperature(self, x: f64) -> f64 {
        self.reference + self.rise
            - self.flux()
                * (x.min(0.5) / self.conductivity[0] + (x - 0.5).max(0.0) / self.conductivity[1])
    }

    pub fn displacement(self, x: f64) -> f64 {
        let left = x.min(0.5);
        let right = (x - 0.5).max(0.0);
        ALPHA[0] * left * (self.rise + self.temperature(left) - self.reference) / 2.0
            + ALPHA[1]
                * right
                * (self.temperature(0.5) + self.temperature(x.max(0.5)) - 2.0 * self.reference)
                / 2.0
    }

    pub fn models(self) -> (Value, Value) {
        let nx = 2 * self.refinement;
        let ny = self.refinement;
        let mut heat_nodes = vec![];
        let mut thermo_nodes = vec![];
        for j in 0..=ny {
            for i in 0..=nx {
                let x = i as f64 / nx as f64;
                let y = 0.1 * j as f64 / ny as f64;
                let id = format!("n{i}_{j}");
                heat_nodes.push(json!({"id": id, "x": x, "y": y,
                    "fix_temperature": i == 0 || i == nx,
                    "temperature": if i == 0 { self.reference + self.rise } else { self.reference },
                    "heat_load": 0.0}));
                // Zero Poisson ratio and transverse restraint isolate the exact axial reference.
                thermo_nodes.push(json!({"id": id, "x": x, "y": y,
                    "fix_x": i == 0, "fix_y": true, "load_x": 0.0, "load_y": 0.0,
                    "temperature_delta": 0.0}));
            }
        }
        let mut heat_elements = vec![];
        let mut thermo_elements = vec![];
        for j in 0..ny {
            for i in 0..nx {
                let layer = usize::from(i >= self.refinement);
                let a = j * (nx + 1) + i;
                let mut common = json!({"id": format!("e{i}_{j}"), "node_i": a,
                    "node_j": a + 1, "node_k": a + nx + 2, "node_l": a + nx + 1,
                    "thickness": 0.01});
                let mut heat = common.clone();
                heat["conductivity"] = json!(self.conductivity[layer]);
                heat_elements.push(heat);
                common["youngs_modulus"] = json!([70e9, 3e9][layer]);
                common["poisson_ratio"] = json!(0.0);
                common["thermal_expansion"] = json!(ALPHA[layer]);
                thermo_elements.push(common);
            }
        }
        (
            json!({"nodes": heat_nodes, "elements": heat_elements}),
            json!({"nodes": thermo_nodes, "elements": thermo_elements}),
        )
    }

    pub fn workflow(self) -> (Value, Value) {
        let (heat, seed) = self.models();
        let heat_type = "result/heat_plane_quad_2d";
        let thermo_type = "result/thermal_plane_quad_2d";
        let model_type = "study_model/thermal_plane_quad_2d";
        let nodes = vec![
            json!({"id": "input", "kind": "input", "inputs": [],
                "outputs": [port("model", "study_model/heat_plane_quad_2d")]}),
            json!({"id": "heat", "kind": "solve", "operator_id": "solve.heat_plane_quad_2d",
                "inputs": [port("model", "study_model/heat_plane_quad_2d")], "outputs": [port("result", heat_type)]}),
            json!({"id": "bridge", "kind": "transform", "operator_id": "bridge.temperature_field_to_thermo_quad_2d",
                "config": {"seed_model": seed, "contract": {
                    "source": {"field": "temperature", "distribution": "node_to_node"},
                    "transform": {"scale": 1.0, "reference_temperature": self.reference},
                    "target": {"field": "temperature_delta"}}},
                "inputs": [port("heat_result", heat_type)], "outputs": [port("model", model_type)]}),
            json!({"id": "structure", "kind": "solve", "operator_id": "solve.thermal_plane_quad_2d",
                "inputs": [port("model", model_type)], "outputs": [port("result", thermo_type)]}),
            json!({"id": "heat_out", "kind": "output", "inputs": [port("result", heat_type)], "outputs": []}),
            json!({"id": "bridge_out", "kind": "output", "inputs": [port("model", model_type)], "outputs": []}),
            json!({"id": "structure_out", "kind": "output", "inputs": [port("result", thermo_type)], "outputs": []}),
        ];
        let edges = vec![
            edge(
                "input",
                "model",
                "heat",
                "model",
                "study_model/heat_plane_quad_2d",
            ),
            edge("heat", "result", "bridge", "heat_result", heat_type),
            edge("bridge", "model", "structure", "model", model_type),
            edge("heat", "result", "heat_out", "result", heat_type),
            edge("bridge", "model", "bridge_out", "model", model_type),
            edge(
                "structure",
                "result",
                "structure_out",
                "result",
                thermo_type,
            ),
        ];
        (
            json!({"schema_version": "kyuubiki.workflow-graph/v1", "id": format!("research.{}", self.id()),
            "name": "Layered thermal reference study", "version": "1.0.0",
            "entry_nodes": ["input"], "output_nodes": ["heat_out", "bridge_out", "structure_out"],
            "nodes": nodes, "edges": edges}),
            json!({"input": heat}),
        )
    }
}

fn port(id: &str, kind: &str) -> Value {
    json!({"id": id, "artifact_type": kind})
}

fn edge(from: &str, from_port: &str, to: &str, to_port: &str, kind: &str) -> Value {
    json!({"id": format!("{from}-{to}"), "from": {"node": from, "port": from_port},
        "to": {"node": to, "port": to_port}, "artifact_type": kind})
}

pub fn cases() -> Vec<Case> {
    let mut cases = vec![];
    for (material, conductivity) in [
        ("moderate-contrast", [10.0, 100.0]),
        ("high-contrast", [2.0, 200.0]),
    ] {
        for refinement in [1, 2, 4, 8] {
            cases.push(Case {
                material,
                conductivity,
                refinement,
                reference: 20.0,
                rise: 20.0,
            });
        }
        for (reference, rise) in [(293.15, 20.0), (20.0, 0.0), (20.0, 40.0)] {
            cases.push(Case {
                material,
                conductivity,
                refinement: 8,
                reference,
                rise,
            });
        }
    }
    cases
}
