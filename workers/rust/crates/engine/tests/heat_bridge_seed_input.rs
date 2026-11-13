#[path = "support/heat_bridge.rs"]
mod bridge;
use kyuubiki_engine::{run_transform_operator, run_workflow_graph};
use serde_json::{Value, json};

#[test]
fn separate_seed_input_matches_legacy_and_preserves_both_dependencies() {
    for shape in ["quad", "triangle"] {
        let (heat, seed) = bridge::fixture(shape);
        let config = bridge::config(shape, &seed, "mean");
        let expected = bridge::run(shape, heat.clone(), config.clone()).unwrap();
        let input = json!({"heat_result":heat,"seed_model":seed});
        let small = json!({"contract":config["contract"]});
        let id = format!("bridge.temperature_field_to_thermo_{shape}_2d");
        assert_eq!(
            run_transform_operator(&id, input.clone(), small.clone()).unwrap(),
            expected
        );
        assert!(run_transform_operator(&id, input.clone(), config).is_err());
        let mut extra = input.clone();
        extra["unexpected"] = json!({});
        assert!(run_transform_operator(&id, extra, small.clone()).is_err());
        let mut invalid = input.clone();
        invalid["seed_model"] = Value::Null;
        assert!(run_transform_operator(&id, invalid, small.clone()).is_err());
        assert!(run_transform_operator(&id, input.clone(), seed.clone()).is_err());
        assert!(run_transform_operator(&id, input.clone(), Value::Null).is_err());
        for field in ["heat_result", "seed_model"] {
            let mut missing = input.clone();
            missing.as_object_mut().unwrap().remove(field);
            assert!(run_transform_operator(&id, missing, small.clone()).is_err());
        }
        let request=serde_json::from_value(json!({"graph":{
            "schema_version":"kyuubiki.workflow-graph/v1","id":"research.seed-input","name":"Seed input","version":"1.0.0",
            "entry_nodes":["heat","seed"],"output_nodes":["out"],"nodes":[
                node("heat","input",vec![],vec!["payload"]),node("seed","input",vec![],vec!["payload"]),
                {"id":"bridge","kind":"transform","operator_id":id,"config":small,
                    "inputs":[port("heat_result"),port("seed_model")],"outputs":[port("model")]},
                node("out","output",vec!["model"],vec![])],
            "edges":[edge("heat","payload","bridge","heat_result"),edge("seed","payload","bridge","seed_model"),edge("bridge","model","out","model")]},
            "input_artifacts":{"heat":input["heat_result"],"seed":input["seed_model"]}})).unwrap();
        let result = run_workflow_graph(request).unwrap();
        assert_eq!(result.artifacts["out.model"], expected);
        let value = serde_json::to_value(result).unwrap();
        let lineage = value["artifact_lineage"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["artifact_key"] == "bridge.model")
            .unwrap();
        assert_eq!(
            lineage["source_artifacts"],
            json!(["heat.payload", "seed.payload"])
        );
    }
}

fn port(id: &str) -> Value {
    json!({"id":id,"artifact_type":"artifact/json"})
}
fn node(id: &str, kind: &str, inputs: Vec<&str>, outputs: Vec<&str>) -> Value {
    json!({"id":id,"kind":kind,"inputs":inputs.into_iter().map(port).collect::<Vec<_>>(),"outputs":outputs.into_iter().map(port).collect::<Vec<_>>()})
}
fn edge(from: &str, fp: &str, to: &str, tp: &str) -> Value {
    json!({"id":format!("{from}-{to}"),"artifact_type":"artifact/json","from":{"node":from,"port":fp},"to":{"node":to,"port":tp}})
}
