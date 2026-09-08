use kyuubiki_engine::{run_solve_operator, run_workflow_graph};
use serde_json::{Value, json};

pub fn run(shape: &str, heat: Value, config: Value) -> Result<Value, String> {
    let source = format!("result/heat_plane_{shape}_2d");
    let target = format!("study_model/thermal_plane_{shape}_2d");
    let request = serde_json::from_value(json!({"graph": {
        "schema_version":"kyuubiki.workflow-graph/v1", "id":"research.bridge-integrity",
        "name":"Bridge integrity", "version":"1.0.0", "entry_nodes":["input"],
        "output_nodes":["out"], "nodes":[
            {"id":"input","kind":"input","inputs":[],"outputs":[{"id":"result","artifact_type":source}]},
            {"id":"bridge","kind":"transform","operator_id":format!("bridge.temperature_field_to_thermo_{shape}_2d"),
                "config":config,"inputs":[{"id":"result","artifact_type":source}],
                "outputs":[{"id":"model","artifact_type":target}]},
            {"id":"out","kind":"output","inputs":[{"id":"model","artifact_type":target}],"outputs":[]}],
        "edges":[{"id":"map","from":{"node":"input","port":"result"},
            "to":{"node":"bridge","port":"result"},"artifact_type":source},
            {"id":"retain","from":{"node":"bridge","port":"model"},
            "to":{"node":"out","port":"model"},"artifact_type":target}]},
        "input_artifacts":{"input":heat}})).map_err(|error| error.to_string())?;
    run_workflow_graph(request)?
        .artifacts
        .remove("out.model")
        .ok_or_else(|| "bridge output missing".into())
}

pub fn fixture(shape: &str) -> (Value, Value) {
    let (coords, connectivity) = if shape == "triangle" {
        (
            vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [4.0, 0.0]],
            vec![vec![0, 1, 2], vec![1, 3, 2]],
        )
    } else {
        (
            vec![
                [0.0, 0.0],
                [1.0, 0.0],
                [4.0, 0.0],
                [0.0, 1.0],
                [1.0, 1.0],
                [4.0, 1.0],
            ],
            vec![vec![0, 1, 4, 3], vec![1, 2, 5, 4]],
        )
    };
    let nodes: Vec<_> = coords
        .iter()
        .enumerate()
        .map(|(i, [x, y])| {
            json!({
        "id":format!("n{i}"),"x":x,"y":y,"temperature":30.0,"heat_load":0.0,
        "fix_temperature":true,"fix_x":true,"fix_y":true,"load_x":0.0,"load_y":0.0,
        "temperature_delta":0.0})
        })
        .collect();
    let elements: Vec<_> = connectivity
        .iter()
        .enumerate()
        .map(|(i, indexes)| {
            let mut element = json!({"id":format!("e{i}"),"thickness":0.01,"conductivity":10.0,
            "youngs_modulus":70e9,"poisson_ratio":0.25,"thermal_expansion":10e-6});
            for (field, index) in ["node_i", "node_j", "node_k", "node_l"].iter().zip(indexes) {
                element[*field] = json!(index);
            }
            element
        })
        .collect();
    let seed = json!({"nodes":nodes,"elements":elements});
    let mut heat =
        run_solve_operator(&format!("solve.heat_plane_{shape}_2d"), seed.clone()).unwrap();
    // A controlled element field tests mapping arithmetic, not a solved temperature profile.
    heat["elements"][0]["average_temperature"] = json!(30.0);
    heat["elements"][1]["average_temperature"] = json!(50.0);
    (heat, seed)
}

pub fn config(shape: &str, seed: &Value, reduction: &str) -> Value {
    let indexes = if shape == "triangle" {
        vec!["node_i", "node_j", "node_k"]
    } else {
        vec!["node_i", "node_j", "node_k", "node_l"]
    };
    json!({"seed_model":seed,"contract":{
        "source":{"field":"average_temperature","distribution":"element_to_nodes","node_index_fields":indexes},
        "transform":{"reference_temperature":20.0,"scale":2.0,"reduction":reduction,"default_value":0.0}}})
}
