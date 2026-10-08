use kyuubiki_protocol::compute_operator_task_digest;
use serde_json::{Value, json};

#[derive(Clone, Copy)]
pub(crate) struct Domain {
    pub name: &'static str,
    pub solver: &'static str,
    pub diagnostic: &'static str,
    pub quality: &'static str,
    pub term: &'static str,
    pub scalar: &'static str,
}

pub(crate) const DOMAINS: [Domain; 5] = [
    Domain {
        name: "thermal",
        solver: "solve.heat_plane_quad_2d",
        diagnostic: "extract.thermal_result_diagnostics",
        quality: "transform.score_thermal_quality",
        term: "thermal_temperature_max",
        scalar: "temperature",
    },
    Domain {
        name: "electrostatic",
        solver: "solve.electrostatic_plane_quad_2d",
        diagnostic: "extract.electrostatic_result_diagnostics",
        quality: "transform.score_electrostatic_quality",
        term: "electrostatic_field_peak_magnitude",
        scalar: "potential",
    },
    Domain {
        name: "magnetostatic",
        solver: "solve.magnetostatic_plane_quad_2d",
        diagnostic: "extract.magnetostatic_result_diagnostics",
        quality: "transform.score_magnetostatic_quality",
        term: "magnetostatic_field_peak_magnitude",
        scalar: "vector_potential",
    },
    Domain {
        name: "thermo",
        solver: "solve.thermal_plane_quad_2d",
        diagnostic: "extract.thermo_result_diagnostics",
        quality: "transform.score_thermal_quality",
        term: "thermo_temperature_delta_max",
        scalar: "temperature_delta",
    },
    Domain {
        name: "stokes_flow",
        solver: "solve.stokes_flow_quad_2d",
        diagnostic: "extract.stokes_flow_result_diagnostics",
        quality: "transform.score_cfd_quality",
        term: "cfd_velocity_span",
        scalar: "pressure",
    },
];

impl Domain {
    pub fn model(self) -> Value {
        let nodes = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)]
            .into_iter()
            .enumerate()
            .map(|(i, (x, y))| match self.name {
                "thermo" => json!({"id":format!("n{i}"),"x":x,"y":y,
                    "fix_x":true,"fix_y":true,"load_x":0.0,"load_y":0.0,"temperature_delta":40.0}),
                "stokes_flow" => json!({"id":format!("n{i}"),"x":x,"y":y,
                    "fix_velocity_x":true,"fix_velocity_y":true,"velocity_x":y,"velocity_y":0.0,
                    "fix_pressure":true,"pressure":0.0}),
                _ => {
                    let (fix, source) = match self.name {
                        "thermal" => ("fix_temperature", "heat_load"),
                        "electrostatic" => ("fix_potential", "charge_density"),
                        _ => ("fix_vector_potential", "current_density"),
                    };
                    json!({"id":format!("n{i}"),"x":x,"y":y,fix:i<2,self.scalar:i as f64,source:0.25})
                }
            })
            .collect::<Vec<_>>();
        let mut element =
            json!({"id":"e0","node_i":0,"node_j":1,"node_k":2,"node_l":3,"thickness":1.0});
        let material = match self.name {
            "thermal" => json!({"conductivity":1.0}),
            "electrostatic" => json!({"permittivity":1.0}),
            "magnetostatic" => json!({"permeability":1.0}),
            "thermo" => {
                json!({"youngs_modulus":1000.0,"poisson_ratio":0.25,"thermal_expansion":1e-5})
            }
            _ => json!({"viscosity":1.0,"density":1.0}),
        };
        element
            .as_object_mut()
            .unwrap()
            .extend(material.as_object().unwrap().clone());
        json!({"nodes":nodes,"elements":[element]})
    }

    pub fn raw(self) -> Value {
        kyuubiki_engine::run_solve_operator(self.solver, self.model()).unwrap()
    }

    pub fn quality_config(self) -> Value {
        json!({"enabled_terms":[self.term],"targets":{self.term:100.0}})
    }

    pub fn corrupt(self, mut raw: Value) -> Value {
        raw["nodes"][0][self.scalar] = Value::Null;
        raw
    }

    pub fn task(self, task_id: &str, id: &str, kind: &str, input: Value, config: Value) -> Value {
        let mut task: Value = serde_json::from_str(include_str!(
            "../../../../../../schemas/examples.operator-task-ir.json"
        ))
        .unwrap();
        task["task_id"] = json!(task_id);
        task["operator"] = json!({"id":id,"family":self.name,"kind":kind});
        task["input_artifact"] = input;
        task["config"] = config;
        task["execution_program"]["program_id"] = json!(id);
        task["execution_program"]["program_family"] = json!(self.name);
        task["execution_program"]["program_kind"] = json!(kind);
        task["execution_program"]["package_ref"] = Value::Null;
        task["execution_program"]["package_version"] = json!("built-in");
        task["execution_program"]["entrypoint"] =
            json!({"kind":"operator_id","name":id,"operator_kind":kind});
        task["runtime_hints"] = json!({"authority_mode":"agent_local","execution_mode":"agent_native",
            "source_ref":format!("builtin://engine/{id}"),"package_ref":null,"package_version":"built-in",
            "agent_fetchable":false,"cache_scope":"none","operator_kind":kind});
        task["integrity"]["task_digest"] = json!(compute_operator_task_digest(&task).unwrap());
        task
    }
}
