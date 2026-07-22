use kyuubiki_engine::run_solve_operator;
use serde_json::json;

#[test]
fn workflow_route_executes_buckling_beam_solver() {
    let result = run_solve_operator(
        "solve.buckling_beam_1d",
        json!({
            "nodes": [
                {"id": "n0", "x": 0.0, "fix_y": true, "fix_rz": false},
                {"id": "n1", "x": 1.0, "fix_y": false, "fix_rz": false},
                {"id": "n2", "x": 2.0, "fix_y": true, "fix_rz": false}
            ],
            "elements": [
                {"id": "e0", "node_i": 0, "node_j": 1, "youngs_modulus": 210000000000.0, "moment_of_inertia": 0.000008, "reference_compressive_force": 100000.0},
                {"id": "e1", "node_i": 1, "node_j": 2, "youngs_modulus": 210000000000.0, "moment_of_inertia": 0.000008, "reference_compressive_force": 100000.0}
            ],
            "mode_count": 1
        }),
    )
    .expect("workflow buckling route should solve");

    assert!(result["minimum_load_factor"].as_f64().unwrap() > 0.0);
    assert_eq!(result["modes"][0]["direction_assessment"], "unassessed");
    assert_eq!(result["mode_cluster_relative_tolerance"], 1.0e-4);
    assert_eq!(result["modes"].as_array().unwrap().len(), 1);
    assert_eq!(
        result["_solver_provenance"]["operator_id"],
        "solve.buckling_beam_1d"
    );
}
