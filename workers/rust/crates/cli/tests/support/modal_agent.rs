use kyuubiki_protocol::compute_operator_task_digest;
use serde_json::{Value, json};

#[path = "../../../solver/tests/support/modal_published_reference.rs"]
mod published_reference;

pub(crate) fn operator(space: bool) -> &'static str {
    if space {
        "solve.modal_frame_3d"
    } else {
        "solve.modal_frame_2d"
    }
}

pub(crate) fn method(space: bool) -> &'static str {
    if space {
        "solve_modal_frame_3d"
    } else {
        "solve_modal_frame_2d"
    }
}

pub(crate) fn model(segments: usize, space: bool) -> Value {
    let nodes: Vec<_> = (0..=segments)
        .map(|i| {
            let mut node = json!({"id":format!("node-{i}"), "x":i as f64, "y":0.0,
                "fix_x":true, "fix_y":i==0, "fix_rz":i==0,
                "load_x":0.0, "load_y":0.0, "moment_z":0.0});
            if space {
                node["z"] = json!(0.0);
                node["fix_z"] = json!(i == 0);
                node["fix_ry"] = json!(i == 0);
                node["fix_rx"] = json!(true);
                for key in ["load_z", "moment_x", "moment_y"] {
                    node[key] = json!(0.0);
                }
            }
            node
        })
        .collect();
    let elements: Vec<_> = (0..segments)
        .map(|i| {
            let mut element = json!({"id":format!("beam-{i}"), "node_i":i, "node_j":i+1,
                "area":1.0, "youngs_modulus":1.0, "density":1.0});
            if space {
                element["shear_modulus"] = json!(0.4);
                for key in [
                    "moment_of_inertia_y",
                    "moment_of_inertia_z",
                    "torsion_constant",
                ] {
                    element[key] = json!(1.0);
                }
            } else {
                element["moment_of_inertia"] = json!(1.0);
                element["section_modulus"] = json!(1.0);
            }
            element
        })
        .collect();
    json!({"nodes":nodes, "elements":elements, "mode_count":if space {6} else {20}})
}

pub(crate) fn seal(task: &mut Value) {
    task["integrity"] = json!({"task_digest":compute_operator_task_digest(task).unwrap()});
}

pub(crate) fn task(id: &str, space: bool, model: Value) -> Value {
    let operator = operator(space);
    let mut task = json!({
        "schema_version":"kyuubiki.operator-task-ir/v1", "task_id":id,
        "operator":{"id":operator,"family":"mechanical","kind":"solver"},
        "descriptor_authoring":{
            "schema_version":"kyuubiki.operator-descriptor-authoring/v1",
            "mode":"rust_native","runtime":"rust","source":"modal_agent_regression",
            "hot_reloadable":false,"execution_language":"language_neutral"
        },
        "node":{}, "input_artifact":model,
        // This metadata exercises digest readback, not extreme physical materials.
        "config":{"numeric_readback_probe":[1.00952715248558e-308, -0.0, 1.0/3.0]},
        "execution_program":{
            "schema_version":"kyuubiki.operator-execution-program/v1",
            "program_id":operator,"program_family":"mechanical","program_kind":"solver",
            "package_ref":null,"package_version":"library-managed","package_integrity":null,
            "runtime_protocol":"kyuubiki.solver-rpc/v1",
            "abi":{"kind":"solver_rpc","input_encoding":"json","output_encoding":"json"},
            "entrypoint":{"kind":"solver_method","name":method(space),"operator_kind":"solver"},
            "bindings":{"input_artifact":"task.input_artifact","config":"task.config",
                "output_artifact":"task.output_artifact"},
            "node_binding":{"node_id":null,"input_ports":[],"output_ports":[]}
        },
        "dataset_contract":{},"orchestration_context":{},
        "runtime_hints":{"authority_mode":"agent_local","execution_mode":"agent_native",
            "cache_scope":"none","agent_fetchable":false,"operator_kind":"solver"}
    });
    seal(&mut task);
    task
}

pub(crate) fn assert_bits(expected: &Value, actual: &Value) {
    match (expected, actual) {
        (Value::Number(a), Value::Number(b)) => {
            assert_eq!(a.as_f64().unwrap().to_bits(), b.as_f64().unwrap().to_bits());
        }
        (Value::Array(a), Value::Array(b)) => {
            assert_eq!(a.len(), b.len());
            for (a, b) in a.iter().zip(b) {
                assert_bits(a, b);
            }
        }
        (Value::Object(a), Value::Object(b)) => {
            assert_eq!(a.len(), b.len());
            for (key, a) in a {
                assert_bits(
                    a,
                    b.get(key)
                        .expect("result field must survive Agent transport"),
                );
            }
        }
        _ => assert_eq!(expected, actual),
    }
}

pub(crate) fn check_result(result: &Value, segments: usize, space: bool) {
    let roots = match segments {
        96 => [
            1.4553085447431156e-7,
            5.712730198911023e-6,
            4.47606463734338e-5,
        ],
        100 => [
            1.236075027143164e-7,
            4.852330686025576e-6,
            3.8021072825043554e-5,
        ],
        _ => panic!("no independent roots for this fixture"),
    };
    let stride = if space { 6 } else { 3 };
    let modes = result["modes"].as_array().unwrap();
    assert_eq!(modes.len(), if space { 6 } else { 20 });
    assert_eq!(
        result["free_dofs"].as_array().unwrap().len(),
        segments * if space { 4 } else { 2 }
    );
    for (i, mode) in modes.iter().enumerate() {
        let eigenvalue = mode["eigenvalue_rad_s_squared"].as_f64().unwrap();
        let root_index = if space { i / 2 } else { i };
        if let Some(root) = roots.get(root_index) {
            assert!((eigenvalue / root - 1.0).abs() < 2e-8);
        }
        assert_eq!(mode["index"], json!(i));
        let shape: Vec<_> = mode["shape"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_f64().unwrap())
            .collect();
        assert_eq!(shape.len(), stride * (segments + 1));
        for (dof, &v) in shape.iter().enumerate() {
            assert!(v.is_finite());
            if dof < stride || dof % stride == 0 || (space && dof % stride == 3) {
                assert_eq!(v, 0.0);
            }
        }
        let norm: f64 = shape.iter().map(|v| v * v).sum();
        assert!((norm - 1.0).abs() < 1e-10);
        let residual =
            published_reference::unit_bending_residual(segments, eigenvalue, &shape, space);
        assert!(
            residual <= 1e-8,
            "Agent published mode {i}: relative={residual:e}"
        );
        assert!(
            (mode["natural_frequency_hz"].as_f64().unwrap() * mode["period_s"].as_f64().unwrap()
                - 1.0)
                .abs()
                < 1e-12
        );
    }
}
