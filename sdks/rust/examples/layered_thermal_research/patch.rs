use serde_json::{Value, json};

const ALPHA: f64 = 12e-6;

#[derive(Clone, Copy)]
pub struct Case {
    pub shape: &'static str,
    pub poisson: f64,
    pub rise: f64,
    pub reference: f64,
    pub clamped: bool,
    pub element_mapping: bool,
    pub stiffness_contrast: bool,
}

impl Case {
    pub fn id(self) -> String {
        format!(
            "patch-{}-nu{}-ref{}-rise{}-clamped{}-element{}-contrast{}",
            self.shape,
            self.poisson,
            self.reference,
            self.rise,
            self.clamped,
            self.element_mapping,
            self.stiffness_contrast
        )
    }

    fn modulus(self, index: usize) -> f64 {
        if self.stiffness_contrast && index % 2 == 0 {
            3e9
        } else {
            70e9
        }
    }

    pub fn workflow(self) -> (Value, Value) {
        let mut nodes = vec![];
        for j in 0..=2 {
            for i in 0..=2 {
                nodes.push(
                    json!({"id":format!("n{i}_{j}"),"x":i as f64 * 0.5,"y":j as f64 * 0.125,
                    "temperature":self.reference+self.rise,"fix_temperature":true,"heat_load":0.0,
                    "temperature_delta":0.0,"fix_x":self.clamped || i==0 && j==0,
                    "fix_y":self.clamped || j==0,"load_x":0.0,"load_y":0.0}),
                );
            }
        }
        let mut cells = vec![];
        for j in 0..2 {
            for i in 0..2 {
                let a = 3 * j + i;
                if self.shape == "triangle" {
                    cells.extend([vec![a, a + 1, a + 4], vec![a, a + 4, a + 3]]);
                } else {
                    cells.push(vec![a, a + 1, a + 4, a + 3]);
                }
            }
        }
        let elements: Vec<_> = cells.iter().enumerate().map(|(i, indexes)| {
            let mut element = json!({"id":format!("e{i}"),"thickness":0.01,"conductivity":10.0,
                "youngs_modulus":self.modulus(i),"poisson_ratio":self.poisson,"thermal_expansion":ALPHA});
            for (field, index) in ["node_i","node_j","node_k","node_l"].iter().zip(indexes) {
                element[*field] = json!(index);
            }
            element
        }).collect();
        let model = json!({"nodes":nodes,"elements":elements});
        let source = format!("result/heat_plane_{}_2d", self.shape);
        let thermal_model = format!("study_model/thermal_plane_{}_2d", self.shape);
        let thermal_result = format!("result/thermal_plane_{}_2d", self.shape);
        let heat_model = format!("study_model/heat_plane_{}_2d", self.shape);
        let port = |id: &str, kind: &str| json!({"id":id,"artifact_type":kind});
        let edge = |a: &str, ap: &str, b: &str, bp: &str, kind: &str| {
            json!({"id":format!("{a}-{b}"),
            "from":{"node":a,"port":ap},"to":{"node":b,"port":bp},"artifact_type":kind})
        };
        let nodes = vec![
            json!({"id":"input","kind":"input","inputs":[],"outputs":[port("model",&heat_model)]}),
            json!({"id":"heat","kind":"solve","operator_id":format!("solve.heat_plane_{}_2d",self.shape),
                "inputs":[port("model",&heat_model)],"outputs":[port("result",&source)]}),
            json!({"id":"bridge","kind":"transform","operator_id":format!("bridge.temperature_field_to_thermo_{}_2d",self.shape),
                "inputs":[port("result",&source)],"outputs":[port("model",&thermal_model)],
                "config":{"seed_model":model,"contract":{
                    "source":{"field":if self.element_mapping {"average_temperature"} else {"temperature"},
                        "distribution":if self.element_mapping {"element_to_nodes"} else {"node_to_node"}},
                    "transform":{"reference_temperature":self.reference}}}}),
            json!({"id":"structure","kind":"solve","operator_id":format!("solve.thermal_plane_{}_2d",self.shape),
                "inputs":[port("model",&thermal_model)],"outputs":[port("result",&thermal_result)]}),
            json!({"id":"heat_out","kind":"output","inputs":[port("result",&source)],"outputs":[]}),
            json!({"id":"bridge_out","kind":"output","inputs":[port("model",&thermal_model)],"outputs":[]}),
            json!({"id":"structure_out","kind":"output","inputs":[port("result",&thermal_result)],"outputs":[]}),
        ];
        (
            json!({"schema_version":"kyuubiki.workflow-graph/v1","id":format!("research.{}",self.id()),
            "name":"Thermal patch reference","version":"1.0.0","entry_nodes":["input"],
            "output_nodes":["heat_out","bridge_out","structure_out"],"nodes":nodes,
            "edges":[edge("input","model","heat","model",&heat_model),
                edge("heat","result","bridge","result",&source),edge("bridge","model","structure","model",&thermal_model),
                edge("heat","result","heat_out","result",&source),edge("bridge","model","bridge_out","model",&thermal_model),
                edge("structure","result","structure_out","result",&thermal_result)]}),
            json!({"input":model}),
        )
    }

    pub fn validate(self, result: &Value) -> Result<Value, String> {
        let payload = if result.get("artifacts").is_some() {
            result
        } else {
            &result["result"]
        };
        if payload["workflow_id"] != format!("research.{}", self.id()) {
            return Err("wrong patch workflow identity".into());
        }
        let artifacts = &payload["artifacts"];
        let mut errors = [0.0_f64; 5];
        for (key, field, reference) in [
            ("heat_out.result", "temperature", self.reference + self.rise),
            ("bridge_out.model", "temperature_delta", self.rise),
            ("structure_out.result", "ux", 0.0),
        ] {
            let nodes = artifacts[key]["nodes"]
                .as_array()
                .filter(|nodes| nodes.len() == 9)
                .ok_or("patch node count mismatch")?;
            for (i, node) in nodes.iter().enumerate() {
                let x = (i % 3) as f64 * 0.5;
                let y = (i / 3) as f64 * 0.125;
                if node["id"] != format!("n{}_{}", i % 3, i / 3)
                    || (number(node, "x")? - x).abs() > 1e-12
                    || (number(node, "y")? - y).abs() > 1e-12
                {
                    return Err("patch node identity or geometry mismatch".into());
                }
                if key == "structure_out.result" {
                    for (field, coordinate) in [("ux", x), ("uy", y)] {
                        let expected = if self.clamped {
                            0.0
                        } else {
                            ALPHA * self.rise * coordinate
                        };
                        errors[2] = errors[2].max((number(node, field)? - expected).abs());
                    }
                } else {
                    let index = usize::from(key == "bridge_out.model");
                    errors[index] = errors[index].max((number(node, field)? - reference).abs());
                }
            }
        }
        let count = if self.shape == "triangle" { 8 } else { 4 };
        for key in ["heat_out.result", "structure_out.result"] {
            let elements = artifacts[key]["elements"]
                .as_array()
                .filter(|elements| elements.len() == count)
                .ok_or("patch element count mismatch")?;
            for (i, element) in elements.iter().enumerate() {
                if element["id"] != format!("e{i}") {
                    return Err("patch element identity mismatch".into());
                }
                if key == "heat_out.result" {
                    errors[3] = errors[3]
                        .max(number(element, "heat_flux_x")?.abs())
                        .max(number(element, "heat_flux_y")?.abs());
                } else {
                    let expected = if self.clamped {
                        -self.modulus(i) * ALPHA * self.rise / (1.0 - self.poisson)
                    } else {
                        0.0
                    };
                    let scale = (self.modulus(i) * ALPHA * self.rise.abs()).max(1.0);
                    errors[4] = errors[4]
                        .max((number(element, "stress_x")? - expected).abs() / scale)
                        .max((number(element, "stress_y")? - expected).abs() / scale)
                        .max(number(element, "tau_xy")?.abs() / scale);
                }
            }
        }
        let gates: Vec<_> = ["uniform_temperature","temperature_reference","free_or_clamped_displacement","zero_heat_flux","plane_stress_response"]
            .into_iter().zip(errors).zip([1e-8,1e-8,1e-10,1e-8,1e-8])
            .map(|((id,error),tolerance)| json!({"id":id,"error":error,"tolerance":tolerance,"passed":error<=tolerance})).collect();
        Ok(
            json!({"case":self.id(),"passed":gates.iter().all(|gate|gate["passed"]==true),"gates":gates}),
        )
    }
}

fn number(value: &Value, key: &str) -> Result<f64, String> {
    value[key]
        .as_f64()
        .filter(|v| v.is_finite())
        .ok_or_else(|| format!("missing/nonfinite patch value: {key}"))
}

pub fn cases() -> Vec<Case> {
    let mut result = vec![];
    for shape in ["triangle", "quad"] {
        for poisson in [0.0, 0.25, 0.45] {
            for rise in [-20.0, 0.0, 20.0] {
                for reference in [20.0, 293.15] {
                    for clamped in [false, true] {
                        for element_mapping in [false, true] {
                            for stiffness_contrast in [false, true] {
                                result.push(Case {
                                    shape,
                                    poisson,
                                    rise,
                                    reference,
                                    clamped,
                                    element_mapping,
                                    stiffness_contrast,
                                });
                            }
                        }
                    }
                }
            }
        }
    }
    result
}
