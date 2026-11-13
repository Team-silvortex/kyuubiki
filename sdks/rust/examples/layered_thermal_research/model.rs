use serde_json::{Value, json};

#[derive(Clone, Copy)]
pub struct Case {
    pub material: &'static str,
    pub conductivity: [f64; 2],
    pub refinement: usize,
    pub reference: f64,
    pub rise: f64,
    pub cold_offset: f64,
    pub expansion: [f64; 2],
    pub poisson: [f64; 2],
}

pub const ALPHA: [f64; 2] = [10e-6, 20e-6];

impl Case {
    pub fn id(self) -> String {
        let rounded = format!("{:.2}", self.reference);
        let reference = if rounded.parse::<f64>().ok() == Some(self.reference) {
            rounded
        } else {
            self.reference.to_string()
        };
        let base = format!(
            "{}-r{}-ref{}-rise{}",
            self.material, self.refinement, reference, self.rise
        );
        let base = if self.cold_offset == 0.0 {
            base
        } else {
            format!("{base}-offset{}", self.cold_offset)
        };
        if self.expansion == ALPHA && self.poisson == [0.0; 2] {
            base
        } else {
            format!(
                "{base}-a{}-{}-nu{}-{}",
                self.expansion[0], self.expansion[1], self.poisson[0], self.poisson[1]
            )
        }
    }

    pub fn flux(self) -> f64 {
        self.rise / (0.5 / self.conductivity[0] + 0.5 / self.conductivity[1])
    }

    pub fn temperature(self, x: f64) -> f64 {
        self.reference + self.cold_offset + self.rise
            - self.flux()
                * (x.min(0.5) / self.conductivity[0] + (x - 0.5).max(0.0) / self.conductivity[1])
    }

    pub fn displacement(self, x: f64) -> f64 {
        let left = x.min(0.5);
        let right = (x - 0.5).max(0.0);
        self.expansion[0]
            * (1.0 + self.poisson[0])
            * left
            * (self.temperature(0.0) + self.temperature(left) - 2.0 * self.reference)
            / 2.0
            + self.expansion[1]
                * (1.0 + self.poisson[1])
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
                    "temperature": self.reference + self.cold_offset + if i == 0 { self.rise } else { 0.0 },
                    "heat_load": 0.0}));
                // Transverse restraint gives axial strain (1 + nu) * alpha * temperature delta.
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
                common["poisson_ratio"] = json!(self.poisson[layer]);
                common["thermal_expansion"] = json!(self.expansion[layer]);
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
        workflow_for(&self.id(), heat, seed, self.reference)
    }
}

pub fn workflow_for(id: &str, heat: Value, seed: Value, reference: f64) -> (Value, Value) {
    let heat_type = "result/heat_plane_quad_2d";
    let thermo_type = "result/thermal_plane_quad_2d";
    let model_type = "study_model/thermal_plane_quad_2d";
    let nodes = vec![
        json!({"id": "input", "kind": "input", "inputs": [],
                "outputs": [port("model", "study_model/heat_plane_quad_2d")]}),
        json!({"id": "seed", "kind": "input", "inputs": [], "outputs": [port("model", model_type)]}),
        json!({"id": "heat", "kind": "solve", "operator_id": "solve.heat_plane_quad_2d",
                "inputs": [port("model", "study_model/heat_plane_quad_2d")], "outputs": [port("result", heat_type)]}),
        json!({"id": "bridge", "kind": "transform", "operator_id": "bridge.temperature_field_to_thermo_quad_2d",
                "config": {"contract": {
                    "source": {"field": "temperature", "distribution": "node_to_node"},
                    "transform": {"scale": 1.0, "reference_temperature": reference},
                    "target": {"field": "temperature_delta"}}},
                "inputs": [port("heat_result", heat_type), port("seed_model", model_type)], "outputs": [port("model", model_type)]}),
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
        edge("seed", "model", "bridge", "seed_model", model_type),
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
        json!({"schema_version": "kyuubiki.workflow-graph/v1", "id": format!("research.{id}"),
            "name": "Layered thermal reference study", "version": "1.0.0",
            "entry_nodes": ["input", "seed"], "output_nodes": ["heat_out", "bridge_out", "structure_out"],
            "nodes": nodes, "edges": edges}),
        json!({"input": heat, "seed": seed}),
    )
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
                cold_offset: 0.0,
                expansion: ALPHA,
                poisson: [0.0; 2],
            });
        }
        for (reference, rise) in [(293.15, 20.0), (20.0, 0.0), (20.0, 40.0)] {
            cases.push(Case {
                material,
                conductivity,
                refinement: 8,
                reference,
                rise,
                cold_offset: 0.0,
                expansion: ALPHA,
                poisson: [0.0; 2],
            });
        }
    }
    cases
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn structural_mesh_is_an_input_artifact_not_node_configuration() {
        let (heat, seed) = base().models();
        let (graph, inputs) = workflow_for("input-contract", heat.clone(), seed.clone(), 20.0);
        assert_eq!(inputs["input"], heat);
        assert_eq!(inputs["seed"], seed);
        assert_eq!(graph["entry_nodes"], json!(["input", "seed"]));
        let bridge = graph["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|n| n["id"] == "bridge")
            .unwrap();
        assert!(bridge["config"].get("seed_model").is_none());
        assert_eq!(bridge["inputs"][1]["id"], "seed_model");
        assert!(
            graph["edges"]
                .as_array()
                .unwrap()
                .iter()
                .any(|edge| edge["from"]["node"] == "seed"
                    && edge["to"]["node"] == "bridge"
                    && edge["to"]["port"] == "seed_model")
        );
    }

    fn base() -> Case {
        Case {
            material: "reference",
            conductivity: [100.0, 200.0],
            refinement: 2,
            reference: 20.0,
            rise: 0.0,
            cold_offset: 0.0,
            expansion: ALPHA,
            poisson: [0.0; 2],
        }
    }

    #[test]
    fn uniform_temperature_is_independent_of_strain_reference() {
        let c = Case {
            cold_offset: 10.0,
            ..base()
        };
        assert_eq!(c.flux(), 0.0);
        assert_eq!(c.temperature(0.4), 30.0);
        assert!((c.displacement(1.0) - 150e-6).abs() < 1e-14);
        let (heat, _) = c.models();
        assert_eq!(heat["nodes"][0]["temperature"], 30.0);
        assert_eq!(heat["nodes"][4]["temperature"], 30.0);
    }

    #[test]
    fn nearby_references_and_offsets_have_distinct_execution_ids() {
        let ids: std::collections::BTreeSet<_> = [
            base(),
            Case {
                cold_offset: 10.0,
                ..base()
            },
            Case {
                reference: 20.001,
                ..base()
            },
            Case {
                reference: 20.002,
                ..base()
            },
        ]
        .into_iter()
        .map(Case::id)
        .collect();
        assert_eq!(ids.len(), 4);
    }

    #[test]
    fn poisson_and_unequal_expansion_are_part_of_model_and_identity() {
        let c = Case {
            expansion: [10e-6, 35e-6],
            poisson: [0.2, 0.4],
            rise: 20.0,
            ..base()
        };
        let zero = Case {
            poisson: [0.0; 2],
            ..c
        };
        assert_ne!(c.id(), zero.id());
        assert!((c.displacement(0.5) - 80e-6).abs() < 1e-14);
        assert!((c.displacement(1.0) - (485e-6 / 3.0)).abs() < 1e-14);
        let (_, model) = c.models();
        assert_eq!(model["elements"][0]["poisson_ratio"], 0.2);
        assert_eq!(model["elements"][2]["poisson_ratio"], 0.4);
        assert_eq!(model["elements"][2]["thermal_expansion"], 35e-6);
    }
}
