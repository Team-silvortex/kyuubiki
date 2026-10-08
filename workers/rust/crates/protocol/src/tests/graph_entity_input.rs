use crate::*;
use serde_json::{Value, json};
use std::io::{Cursor, Read};

fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../../../../schemas/examples.graph-entity-input-normalization.json"
    ))
    .unwrap()
}

fn patched(base: &Value, case: &Value) -> Value {
    let mut input = base.clone();
    if case["append_element"] == true {
        let first = input["elements"][0].clone();
        input["elements"].as_array_mut().unwrap().push(first);
    }
    for edit in case["edits"].as_array().unwrap() {
        let entity = &mut input[edit["collection"].as_str().unwrap()]
            [edit["index"].as_u64().unwrap() as usize];
        if let Some(replacement) = edit.get("replacement") {
            *entity = replacement.clone();
        } else {
            entity["id"] = edit["id"].clone();
        }
    }
    input
}

fn normalize(action: &str, input: Value) -> Result<Value, serde_json::Error> {
    macro_rules! decode {
        ($type:ty) => {
            serde_json::to_value(serde_json::from_value::<$type>(input)?)
        };
    }
    match action {
        "solve_thermal_bar_1d" => decode!(SolveThermalBar1dRequest),
        "solve_heat_bar_1d" => decode!(SolveHeatBar1dRequest),
        "solve_transient_heat_bar_1d" => decode!(SolveTransientHeatBar1dRequest),
        "solve_electrostatic_bar_1d" => decode!(SolveElectrostaticBar1dRequest),
        "solve_magnetostatic_bar_1d" => decode!(SolveMagnetostaticBar1dRequest),
        "solve_advection_diffusion_bar_1d" => decode!(SolveAdvectionDiffusionBar1dRequest),
        "solve_heat_plane_triangle_2d" => decode!(SolveHeatPlaneTriangle2dRequest),
        "solve_heat_plane_quad_2d" => decode!(SolveHeatPlaneQuad2dRequest),
        "solve_electrostatic_plane_triangle_2d" => {
            decode!(SolveElectrostaticPlaneTriangle2dRequest)
        }
        "solve_electrostatic_plane_quad_2d" => decode!(SolveElectrostaticPlaneQuad2dRequest),
        _ => panic!("uncovered model {action}"),
    }
}

#[test]
fn graph_entity_shared_accepted_inputs_preserve_or_generate_unique_ids() {
    let cases = fixture();
    assert_eq!(cases["models"].as_object().unwrap().len(), 10);
    assert_eq!(cases["accepted"].as_array().unwrap().len(), 6);
    for (action, base) in cases["models"].as_object().unwrap() {
        for case in cases["accepted"].as_array().unwrap() {
            let input = patched(base, case);
            let normalized = normalize(action, input.clone())
                .unwrap_or_else(|error| panic!("{action}/{}: {error}", case["id"]));
            for (collection, prefix) in [("nodes", "n"), ("elements", "e")] {
                for (index, entity) in input[collection].as_array().unwrap().iter().enumerate() {
                    let expected = entity["id"]
                        .as_str()
                        .filter(|id| !id.is_empty())
                        .map(str::to_owned)
                        .unwrap_or_else(|| format!("{prefix}{index}"));
                    assert_eq!(normalized[collection][index]["id"], expected);
                    for (field, value) in entity.as_object().unwrap() {
                        if field != "id" {
                            assert_eq!(normalized[collection][index][field], *value);
                        }
                    }
                }
            }
            assert_eq!(normalize(action, normalized.clone()).unwrap(), normalized);
        }
    }
}

#[test]
fn graph_entity_shared_invalid_ids_fail_without_silent_repair() {
    let cases = fixture();
    assert_eq!(cases["rejected"].as_array().unwrap().len(), 18);
    for (action, base) in cases["models"].as_object().unwrap() {
        for case in cases["rejected"].as_array().unwrap() {
            assert!(
                normalize(action, patched(base, case)).is_err(),
                "{action}/{}",
                case["id"]
            );
        }
    }
}

#[test]
fn graph_entity_reader_normalizes_after_skipping_large_metadata() {
    let model = serde_json::to_vec(&fixture()["models"]["solve_heat_bar_1d"]).unwrap();
    let prefix = Cursor::new(br#"{"transport_padding":""#);
    let padding = std::io::repeat(b'x').take(8_000_000);
    let suffix = Cursor::new(b"\",\"nodes\":".as_slice());
    let nodes = serde_json::to_vec(&fixture()["models"]["solve_heat_bar_1d"]["nodes"]).unwrap();
    let elements =
        serde_json::to_vec(&fixture()["models"]["solve_heat_bar_1d"]["elements"]).unwrap();
    let reader = prefix
        .chain(padding)
        .chain(suffix)
        .chain(Cursor::new(nodes))
        .chain(Cursor::new(b",\"elements\":"))
        .chain(Cursor::new(elements))
        .chain(Cursor::new(b"}"));
    let streamed: SolveHeatBar1dRequest = serde_json::from_reader(reader).unwrap();
    assert_eq!(streamed, serde_json::from_slice(&model).unwrap());
    assert_eq!(streamed.nodes[1].id, "n1");
    assert_eq!(streamed.elements[1].id, "e1");
}

#[test]
fn graph_entity_defaults_do_not_make_physics_or_graph_arrays_optional() {
    let fixture = fixture();
    let base = &fixture["models"]["solve_heat_bar_1d"];
    for field in ["nodes", "elements"] {
        let mut input = base.clone();
        input.as_object_mut().unwrap().remove(field);
        assert!(normalize("solve_heat_bar_1d", input).is_err());
    }
    for field in ["x", "fix_temperature"] {
        let mut input = base.clone();
        input["nodes"][0].as_object_mut().unwrap().remove(field);
        assert!(normalize("solve_heat_bar_1d", input).is_err());
    }
    for field in ["node_i", "node_j", "area", "conductivity"] {
        let mut input = base.clone();
        input["elements"][0].as_object_mut().unwrap().remove(field);
        assert!(normalize("solve_heat_bar_1d", input).is_err());
    }
}

#[test]
fn graph_entity_reader_rejects_duplicate_known_fields() {
    let duplicate =
        r#"{"nodes":[{"id":"left","id":"right","x":0,"fix_temperature":true}],"elements":[]}"#;
    assert!(serde_json::from_str::<SolveHeatBar1dRequest>(duplicate).is_err());
    let input = json!({"nodes":[],"elements":[]});
    assert!(serde_json::from_value::<SolveHeatBar1dRequest>(input).is_ok());
    // Empty arrays remain a solver-geometry validation concern, not an ID default.
}
