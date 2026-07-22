use kyuubiki_engine::run_solve_operator;
use serde_json::json;

#[test]
fn workflow_route_executes_statically_preloaded_frame_buckling() {
    let result = run_solve_operator(
        "solve.buckling_frame_2d",
        json!({
            "frame": {
                "nodes": [
                    {"id": "n0", "x": 0.0, "y": 0.0, "fix_x": true, "fix_y": true, "fix_rz": false, "load_x": 0.0, "load_y": 0.0, "moment_z": 0.0},
                    {"id": "n1", "x": 0.0, "y": 1.0, "fix_x": false, "fix_y": false, "fix_rz": false, "load_x": 0.0, "load_y": 0.0, "moment_z": 0.0},
                    {"id": "n2", "x": 0.0, "y": 2.0, "fix_x": true, "fix_y": false, "fix_rz": false, "load_x": 0.0, "load_y": -100000.0, "moment_z": 0.0}
                ],
                "elements": [
                    {"id": "e0", "node_i": 0, "node_j": 1, "area": 0.01, "youngs_modulus": 210000000000.0, "moment_of_inertia": 0.000008, "section_modulus": 0.0001},
                    {"id": "e1", "node_i": 1, "node_j": 2, "area": 0.01, "youngs_modulus": 210000000000.0, "moment_of_inertia": 0.000008, "section_modulus": 0.0001}
                ]
            },
            "mode_count": 1
        }),
    )
    .expect("workflow frame buckling route should solve");

    assert!(result["minimum_load_factor"].as_f64().unwrap() > 0.0);
    assert_eq!(result["modes"].as_array().unwrap().len(), 1);
    assert_eq!(result["element_preloads"].as_array().unwrap().len(), 2);
    assert_eq!(
        result["_solver_provenance"]["operator_id"],
        "solve.buckling_frame_2d"
    );
}
