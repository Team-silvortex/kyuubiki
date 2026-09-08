#[path = "../../../../../sdks/rust/examples/layered_thermal_research/checks.rs"]
mod checks;
#[path = "../../../../../sdks/rust/examples/layered_thermal_research/model.rs"]
mod model;

use kyuubiki_engine::run_workflow_graph;
use serde_json::{Value, json};

fn run_transform_operator(operator: &str, payload: Value, config: Value) -> Result<Value, String> {
    let shape = if operator.contains("triangle") {
        "triangle"
    } else {
        "quad"
    };
    let source_type = format!("result/heat_plane_{shape}_2d");
    let target_type = format!("study_model/thermal_plane_{shape}_2d");
    let request = serde_json::from_value(json!({"graph": {
        "schema_version":"kyuubiki.workflow-graph/v1", "id":"research.reference-conversion",
        "name":"Reference conversion", "version":"1.0.0", "entry_nodes":["input"],
        "output_nodes":["out"], "nodes":[
            {"id":"input","kind":"input","inputs":[],"outputs":[{"id":"result","artifact_type":source_type}]},
            {"id":"bridge","kind":"transform","operator_id":operator,"config":config,
                "inputs":[{"id":"result","artifact_type":source_type}],
                "outputs":[{"id":"model","artifact_type":target_type}]},
            {"id":"out","kind":"output","inputs":[{"id":"model","artifact_type":target_type}],
                "outputs":[{"id":"model","artifact_type":target_type}]}],
        "edges":[{"id":"map","from":{"node":"input","port":"result"},
            "to":{"node":"bridge","port":"result"},"artifact_type":source_type},
            {"id":"retain","from":{"node":"bridge","port":"model"},
            "to":{"node":"out","port":"model"},"artifact_type":target_type}]},
        "input_artifacts":{"input":payload}})).map_err(|error| error.to_string())?;
    let result = run_workflow_graph(request)?;
    result
        .artifacts
        .get("bridge.model")
        .cloned()
        .ok_or("bridge did not produce a model".into())
}

#[test]
fn layered_research_matches_independent_temperature_flux_and_expansion_references() {
    for case in model::cases() {
        let (graph, input_artifacts) = case.workflow();
        let request =
            serde_json::from_value(json!({"graph": graph, "input_artifacts": input_artifacts}))
                .unwrap();
        let result = serde_json::to_value(run_workflow_graph(request).unwrap()).unwrap();
        let report = checks::validate(case, &result).unwrap();
        assert_eq!(report["passed"], true, "{report}");
    }
}

#[test]
fn research_gates_reject_wrong_identity_missing_data_and_numerical_corruption() {
    let case = model::cases()[0];
    let (graph, input_artifacts) = case.workflow();
    let request =
        serde_json::from_value(json!({"graph":graph,"input_artifacts":input_artifacts})).unwrap();
    let good = serde_json::to_value(run_workflow_graph(request).unwrap()).unwrap();
    for path in [
        "/workflow_id",
        "/artifacts/heat_out.result/nodes/0/temperature",
        "/artifacts/heat_out.result/elements/0/heat_flux_x",
        "/artifacts/bridge_out.model/nodes/0/id",
        "/artifacts/structure_out.result/nodes/0/x",
    ] {
        let mut bad = good.clone();
        *bad.pointer_mut(path).unwrap() = Value::Null;
        assert!(
            checks::validate(case, &bad).is_err(),
            "unchecked corruption: {path}"
        );
    }
    let mut empty = good.clone();
    empty["artifacts"]["heat_out.result"]["nodes"] = json!([]);
    assert!(checks::validate(case, &empty).is_err());
    let mut shifted = good;
    shifted["artifacts"]["bridge_out.model"]["nodes"][0]["temperature_delta"] = json!(40.0);
    assert_eq!(checks::validate(case, &shifted).unwrap()["passed"], false);
}

#[test]
fn reference_temperature_is_applied_before_scaling_for_triangles_and_quads() {
    for shape in ["triangle", "quad"] {
        let mut nodes = json!([
            {"id":"a","x":0.0,"y":0.0,"fix_temperature":true,"temperature":30.0,"heat_load":0.0},
            {"id":"b","x":1.0,"y":0.0,"fix_temperature":true,"temperature":30.0,"heat_load":0.0},
            {"id":"c","x":0.0,"y":1.0,"fix_temperature":true,"temperature":30.0,"heat_load":0.0}
        ]);
        let mut element =
            json!({"id":"e","node_i":0,"node_j":1,"node_k":2,"thickness":0.01,"conductivity":10.0});
        if shape == "quad" {
            nodes[2]["x"] = json!(1.0);
            nodes.as_array_mut().unwrap().push(json!({"id":"d","x":0.0,"y":1.0,"fix_temperature":true,"temperature":30.0,"heat_load":0.0}));
            element["node_l"] = json!(3);
        }
        let input = json!({"nodes": nodes, "elements": [element]});
        let heat =
            kyuubiki_engine::run_solve_operator(&format!("solve.heat_plane_{shape}_2d"), input)
                .unwrap();
        let mut seed_nodes = nodes.clone();
        for node in seed_nodes.as_array_mut().unwrap() {
            for (field, value) in [
                ("fix_x", json!(true)),
                ("fix_y", json!(true)),
                ("load_x", json!(0.0)),
                ("load_y", json!(0.0)),
                ("temperature_delta", json!(0.0)),
            ] {
                node[field] = value;
            }
        }
        let mut seed_element = element;
        seed_element["youngs_modulus"] = json!(70e9);
        seed_element["poisson_ratio"] = json!(0.0);
        seed_element["thermal_expansion"] = json!(10e-6);
        let seed = json!({"nodes":seed_nodes,"elements":[seed_element]});
        let operator = format!("bridge.temperature_field_to_thermo_{shape}_2d");
        for distribution in ["node_to_node", "element_to_nodes"] {
            let source = if distribution == "node_to_node" {
                "temperature"
            } else {
                "average_temperature"
            };
            let indices = if shape == "quad" {
                vec!["node_i", "node_j", "node_k", "node_l"]
            } else {
                vec!["node_i", "node_j", "node_k"]
            };
            let config = json!({"seed_model": seed, "contract": {
                "source":{"field":source,"distribution":distribution,"node_index_fields":indices},
                "transform":{"reference_temperature":20.0,"scale":2.0}}});
            let result = run_transform_operator(&operator, heat.clone(), config).unwrap();
            for node in result["nodes"].as_array().unwrap() {
                assert_eq!(node["temperature_delta"], 20.0);
            }
        }
        for invalid in [Value::Null, json!("20"), json!(true)] {
            assert!(
                run_transform_operator(
                    &operator,
                    heat.clone(),
                    json!({"seed_model":seed,
                "contract":{"transform":{"reference_temperature":invalid}}})
                )
                .is_err()
            );
        }
        assert!(
            run_transform_operator(
                &operator,
                heat.clone(),
                json!({"seed_model":seed,
            "contract":{"source":{"field":"heat_load"},"transform":{"reference_temperature":20.0}}})
            )
            .is_err()
        );
        let recovered = run_transform_operator(
            &operator,
            heat,
            json!({"seed_model":seed,
            "contract":{"transform":{"reference_temperature":30.0}}}),
        )
        .unwrap();
        for node in recovered["nodes"].as_array().unwrap() {
            assert_eq!(node["temperature_delta"], 0.0);
        }
    }
}
