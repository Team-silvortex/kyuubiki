use crate::workflow_executor::run_transform_operator;
use serde_json::{Value, json};

#[test]
fn writes_final_array_elements_and_nested_array_cells() {
    let base = json!({"loads": [10, 20], "matrix": [[0, 1]]});
    let result = expand(
        base.clone(),
        json!([
            {"path": "loads.1", "values": [2, 4]},
            {"path": "matrix.0.0", "values": [6, 8]}
        ]),
    )
    .unwrap();
    assert_eq!(result["case_count"], 4);
    for (index, (load, cell)) in [(2, 6), (2, 8), (4, 6), (4, 8)].iter().enumerate() {
        let case = &result["cases"][index];
        assert_eq!(case["id"], format!("case_{index}"));
        assert_eq!(
            case["model"],
            json!({"loads": [10, load], "matrix": [[cell, 1]]})
        );
        assert_eq!(
            case["parameters"],
            json!({"loads.1": load, "matrix.0.0": cell})
        );
    }
    assert_eq!(base, json!({"loads": [10, 20], "matrix": [[0, 1]]}));
}

#[test]
fn root_arrays_support_replacing_values_without_appending() {
    let result = expand(
        json!([1, 2]),
        json!([
            {"path": "1", "values": [null, {"nested": true}, [3, 4]]}
        ]),
    )
    .unwrap();
    for (index, value) in [Value::Null, json!({"nested": true}), json!([3, 4])]
        .iter()
        .enumerate()
    {
        assert_eq!(result["cases"][index]["model"], json!([1, value]));
    }
    assert!(expand(json!([1, 2]), json!([{"path": "2", "values": [3]}])).is_err());
}

#[test]
fn numeric_object_keys_are_not_mistaken_for_array_indices() {
    let result = expand(
        json!({"regions": {"0": {"k": 1}, "00": {"k": 2}}}),
        json!([
            {"path": "regions.0.k", "values": [3, 4]},
            {"path": "regions.00.k", "values": [5, 6]}
        ]),
    )
    .unwrap();
    assert_eq!(result["case_count"], 4);
    assert_eq!(
        result["cases"][3]["model"],
        json!({"regions": {"0": {"k": 4}, "00": {"k": 6}}})
    );
    let huge_key = "999999999999999999999999999999999999999";
    let result = expand(json!({}), json!([{"path": huge_key, "values": [7]}])).unwrap();
    assert_eq!(result["cases"][0]["model"][huge_key], 7);
}

#[test]
fn duplicate_explicit_or_implicit_labels_cannot_overwrite_parameters() {
    for axes in [
        json!([{"label": "shared", "path": "x", "values": [1]},
               {"label": "shared", "path": "y", "values": [2]}]),
        json!([{"path": "x", "values": [1]},
               {"label": "x", "path": "y", "values": [2]}]),
    ] {
        let error = expand(json!({"x": 0, "y": 0}), axes).unwrap_err();
        assert!(
            error.contains("duplicate") && error.contains("label"),
            "{error}"
        );
        assert!(error.contains("axis 1"), "{error}");
    }
}

#[test]
fn malformed_or_blank_labels_do_not_silently_become_path_labels() {
    for label in [
        Value::Null,
        json!(false),
        json!(2),
        json!([]),
        json!({}),
        json!(""),
        json!(" \t "),
    ] {
        let error = expand(
            json!({}),
            json!([{"label": label, "path": "x", "values": [1]}]),
        )
        .unwrap_err();
        assert!(
            error.contains("axis 0") && error.contains("label"),
            "{error}"
        );
    }
}

#[test]
fn duplicate_targets_are_rejected_even_with_distinct_labels() {
    for path in ["x", " x "] {
        let error = expand(
            json!({}),
            json!([
                {"label": "first", "path": "x", "values": [1]},
                {"label": "second", "path": path, "values": [2]}
            ]),
        )
        .unwrap_err();
        assert!(error.contains("overlap") && error.contains("x"), "{error}");
    }
}

#[test]
fn ancestor_targets_cannot_invalidate_descendant_assignments() {
    for (base, parent, child, replacement) in [
        (
            json!({"material": {"density": 0}}),
            "material",
            "material.density",
            json!({"density": 1}),
        ),
        (json!({"loads": [0]}), "loads", "loads.0", json!([1])),
        (
            json!({"cells": [{"k": 0}]}),
            "cells.0",
            "cells.0.k",
            json!({"k": 1}),
        ),
    ] {
        for reverse in [false, true] {
            let mut axes = vec![
                json!({"path": parent, "values": [replacement]}),
                json!({"path": child, "values": [2]}),
            ];
            if reverse {
                axes.reverse();
            }
            let error = expand(base.clone(), Value::Array(axes)).unwrap_err();
            assert!(
                error.contains("overlap") && error.contains(parent) && error.contains(child),
                "{error}"
            );
        }
    }
}

#[test]
fn path_prefixes_that_are_not_ancestors_remain_independent() {
    let result = expand(
        json!({"m": {}}),
        json!([
            {"path": "m.a", "values": [1]}, {"path": "m.ab", "values": [2]},
            {"path": "m.a_b", "values": [3]}
        ]),
    )
    .unwrap();
    assert_eq!(
        result["cases"][0]["model"],
        json!({"m": {"a": 1, "ab": 2, "a_b": 3}})
    );
}

#[test]
fn decimal_array_aliases_resolve_to_one_target() {
    let error = expand(
        json!({"cells": [{"k": 0}]}),
        json!([
            {"path": "cells.0.k", "values": [1]}, {"path": "cells.00.k", "values": [2]}
        ]),
    )
    .unwrap_err();
    assert!(error.contains("overlap"), "{error}");
    let result = expand(
        json!({"cells": [0]}),
        json!([{"path": "cells.00", "values": [3]}]),
    )
    .unwrap();
    assert_eq!(result["cases"][0]["model"], json!({"cells": [3]}));
}

#[test]
fn empty_path_segments_are_errors_even_when_the_base_has_empty_keys() {
    let base = json!({"": {"x": 0}, "m": {"": {"x": 0}, "   ": {"x": 0}}});
    for path in ["", " ", ".x", "m.", "m..x", "m.   .x"] {
        let error = expand(base.clone(), json!([{"path": path, "values": [1]}])).unwrap_err();
        assert!(
            error.contains("axis 0") && error.contains("path"),
            "{error}"
        );
    }
}

#[test]
fn invalid_targets_report_axis_and_full_path_before_generation() {
    for path in [
        "missing.x",
        "scalar.x",
        "loads.4",
        "loads.-1",
        "loads.+0",
        "loads.-",
        "loads.18446744073709551616",
        "loads.0.x",
    ] {
        let error = expand(
            json!({"ok": 0, "scalar": 0, "loads": [0]}),
            json!([
                {"path": "ok", "values": [1, 2]}, {"path": path, "values": [3]}
            ]),
        )
        .unwrap_err();
        assert!(error.contains("axis 1") && error.contains(path), "{error}");
    }
}

#[test]
fn existing_parents_allow_new_object_fields_but_are_never_created_implicitly() {
    let result = expand(json!({"m": {}}), json!([{"path": "m.k", "values": [2]}])).unwrap();
    assert_eq!(result["cases"][0]["model"], json!({"m": {"k": 2}}));
    assert!(expand(json!({}), json!([{"path": "m.k", "values": [2]}])).is_err());
}

#[test]
fn active_materialization_rejects_ambiguous_or_unresolvable_targets() {
    for axes in [
        json!([{"path": "missing.x", "values": [1]}]),
        json!([{"label": "same", "path": "x", "values": [1]},
                        {"label": "same", "path": "y", "values": [2]}]),
        json!([{"path": "m", "values": [{"x": 1}]},
                        {"path": "m.x", "values": [2]}]),
    ] {
        let error = run_transform_operator(
            "transform.materialize_quality_sweep_expansion",
            json!({"sweep_enabled": true, "base": {"m": {"x": 0}}, "axes": axes}),
            Value::Null,
        )
        .unwrap_err();
        assert!(error.contains("axis"), "{error}");
    }
}

#[test]
fn quality_sweep_roundtrip_writes_the_same_array_values_it_reports() {
    let plan = run_transform_operator(
        "transform.build_quality_parameter_sweep_plan",
        json!({
            "request_payload": {"search_space": {"loads.0": [2, 4]}}, "base": {"loads": [0]}
        }),
        Value::Null,
    )
    .unwrap();
    let expansion = run_transform_operator(
        "transform.materialize_quality_sweep_expansion",
        plan,
        Value::Null,
    )
    .unwrap();
    let result =
        run_transform_operator("transform.expand_parameter_sweep", expansion, Value::Null).unwrap();
    assert_eq!(result["case_count"], 2);
    for case in result["cases"].as_array().unwrap() {
        assert_eq!(case["model"]["loads"][0], case["parameters"]["loads.0"]);
    }
}

#[test]
fn thousands_of_singleton_axes_expand_without_recursive_stack_growth() {
    let axes: Vec<Value> = (0..8192)
        .map(|index| {
            json!({
                "path": format!("p{index}"), "values": [index]
            })
        })
        .collect();
    let result = expand(json!({}), Value::Array(axes)).unwrap();
    assert_eq!(result["axis_count"], 8192);
    assert_eq!(result["case_count"], 1);
    assert_eq!(
        result["cases"][0]["parameters"].as_object().unwrap().len(),
        8192
    );
    assert_eq!(
        result["cases"][0]["model"],
        result["cases"][0]["parameters"]
    );
    assert_eq!(result["cases"][0]["model"]["p8191"], 8191);
}

#[test]
fn mixed_radix_expansion_preserves_each_choice_and_metadata() {
    let base = json!({"cells": [[0, 0]], "material": {"k": 0}, "tag": "unchanged"});
    let result = run_transform_operator(
        "transform.expand_parameter_sweep",
        json!({
            "base": base,
            "axes": [
                {"label": "z", "path": "cells.0.0", "values": [1, 2]},
                {"label": "a", "path": "material.k", "values": [3, 4, 5]},
                {"label": "b", "path": "cells.0.1", "values": [6, 7]}
            ], "case_metadata": {"source_candidate_id": "seed"}
        }),
        json!({"id_prefix": "chosen"}),
    )
    .unwrap();
    assert_eq!(result["case_count"], 12);
    for (index, case) in result["cases"].as_array().unwrap().iter().enumerate() {
        let (x, k, y) = (1 + index / 6, 3 + (index / 2) % 3, 6 + index % 2);
        assert_eq!(case["id"], format!("chosen_{index}"));
        assert_eq!(
            case["model"],
            json!({"cells": [[x, y]], "material": {"k": k}, "tag": "unchanged"})
        );
        assert_eq!(case["parameters"], json!({"z": x, "a": k, "b": y}));
        assert_eq!(case["label"], format!("a={k}, b={y}, z={x}"));
        assert_eq!(case["metadata"]["source_candidate_id"], "seed");
    }
}

#[test]
fn array_load_sweep_drives_real_cohesive_solver_results() {
    let base = cohesive_model();
    let result = expand(
        base.clone(),
        json!([
            {"label": "left_load", "path": "nodes.2.load.1", "values": [1.25, 2.5]},
            {"label": "right_load", "path": "nodes.3.load.1", "values": [1.25, 2.5]}
        ]),
    )
    .unwrap();
    for (index, (left, right)) in [(1.25, 1.25), (1.25, 2.5), (2.5, 1.25), (2.5, 2.5)]
        .iter()
        .enumerate()
    {
        let mut reference_model = base.clone();
        reference_model["nodes"][2]["load"][1] = json!(left);
        reference_model["nodes"][3]["load"][1] = json!(right);
        let reference =
            crate::run_solve_operator("solve.cohesive_interface_mesh_2d", reference_model).unwrap();
        let solved = crate::run_solve_operator(
            "solve.cohesive_interface_mesh_2d",
            result["cases"][index]["model"].clone(),
        )
        .unwrap();
        assert_eq!(solved["converged"], true);
        crate::verify_solver_result_provenance(&solved, "solve.cohesive_interface_mesh_2d")
            .unwrap();
        assert_eq!(solved["max_normal_damage"], 0.0);
        assert_eq!(solved["nodes"], reference["nodes"]);
        let displacement = solved["max_displacement"].as_f64().unwrap();
        assert!(displacement.is_finite() && displacement > 0.0);
        if left == right {
            // Unit-area interface: the two equal nodal forces give traction 2F.
            assert!((displacement - 2.0 * left / 1000.0).abs() < 1.0e-12);
        }
    }
}

fn cohesive_model() -> Value {
    json!({
        "id": "sweep.cohesive-loads",
        "nodes": [
            {"id": "lower-0", "x": 0.0, "y": 0.0, "fixed": [true, true], "load": [0.0, 0.0]},
            {"id": "lower-1", "x": 1.0, "y": 0.0, "fixed": [true, true], "load": [0.0, 0.0]},
            {"id": "upper-0", "x": 0.0, "y": 0.0, "fixed": [true, false], "load": [0.0, 0.0]},
            {"id": "upper-1", "x": 1.0, "y": 0.0, "fixed": [true, false], "load": [0.0, 0.0]}
        ],
        "materials": [{"id": "adhesive", "properties": {
            "normal_initial_stiffness": 1000.0, "normal_compression_stiffness": 1200.0,
            "normal_peak_traction": 10.0, "normal_failure_separation": 0.04,
            "shear_initial_stiffness": 800.0, "shear_peak_traction": 8.0, "shear_failure_separation": 0.05
        }}],
        "elements": [{"id": "interface", "lower_i": 0, "lower_j": 1, "upper_i": 2, "upper_j": 3,
                      "thickness": 1.0, "material_id": "adhesive"}],
        "load_steps": 4, "max_iterations": 12, "tolerance": 1.0e-11
    })
}

fn expand(base: Value, axes: Value) -> Result<Value, String> {
    run_transform_operator(
        "transform.expand_parameter_sweep",
        json!({"base": base, "axes": axes}),
        Value::Null,
    )
}
