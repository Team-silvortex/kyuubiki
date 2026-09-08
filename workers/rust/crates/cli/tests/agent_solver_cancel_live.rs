use serde_json::{Value, json};
use std::error::Error;
use std::fs;
use std::io::Write;
use std::net::{Shutdown, TcpStream};
use std::thread;
use std::time::{Duration, Instant};

#[allow(dead_code)]
#[path = "support/agent_lifecycle.rs"]
mod support;
use support::*;

const HEAT: &str = "solve_heat_plane_quad_2d";

#[path = "support/solver_scaling_cancel.rs"]
mod scaling_cancel;

fn heat_grid(job: &str, n: usize) -> Value {
    let mut nodes = Vec::new();
    let mut elements = Vec::new();
    for y in 0..=n {
        for x in 0..=n {
            nodes.push(json!({"id":format!("n{x}-{y}"), "x":x as f64 / n as f64,
                "y":y as f64 / n as f64, "fix_temperature":x == 0 || x == n,
                "temperature":if x == 0 {100.0} else {20.0}, "heat_load":0.0}));
        }
    }
    for y in 0..n {
        for x in 0..n {
            let i = y * (n + 1) + x;
            elements.push(json!({"id":format!("e{x}-{y}"), "node_i":i,"node_j":i+1,
                "node_k":i+n+2,"node_l":i+n+1,"thickness":0.02,"conductivity":45.0}));
        }
    }
    json!({"job_id":job,"nodes":nodes,"elements":elements})
}

fn pending(
    agent: &LiveAgent,
    id: &str,
    method: &str,
    params: Value,
) -> Result<TcpStream, Box<dyn Error>> {
    let mut stream = TcpStream::connect(("127.0.0.1", agent.port))?;
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;
    let bytes =
        serde_json::to_vec(&json!({"rpc_version":1,"id":id,"method":method,"params":params}))?;
    stream.write_all(&u32::try_from(bytes.len())?.to_be_bytes())?;
    stream.write_all(&bytes)?;
    Ok(stream)
}

fn final_frame(stream: &mut TcpStream) -> Result<Value, Box<dyn Error>> {
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        let frame = read_json_frame(stream)?;
        if frame.get("ok").is_some() {
            return Ok(frame);
        }
    }
    Err("solver never returned a terminal receipt".into())
}

fn record(agent: &LiveAgent, name: &str, value: &Value) -> Result<(), Box<dyn Error>> {
    fs::write(
        agent.root.join(format!("{name}.json")),
        serde_json::to_vec_pretty(value)?,
    )?;
    Ok(())
}

fn wait_for_numerical_steps(
    agent: &LiveAgent,
    ids: &[&str],
    stage: &str,
) -> Result<Value, Box<dyn Error>> {
    let deadline = Instant::now() + Duration::from_secs(20);
    let mut last = Value::Null;
    while Instant::now() < deadline {
        last = agent.request("solver-progress", "describe_agent", json!({}))?;
        let control = &last["result"]["solver_control"];
        if let Some(active) = control["active"].as_array() {
            if ids.iter().all(|id| {
                active.iter().any(|entry| {
                    entry["request_id"] == *id
                        && entry["checkpoint"]["stage"] == stage
                        && entry["checkpoint"]["completed_steps"]
                            .as_u64()
                            .is_some_and(|n| n >= 3)
                })
            }) {
                assert_eq!(control["thread_preemption"], false);
                assert_eq!(
                    last["result"]["fault_injection"]["phase"],
                    "solver_safe_point"
                );
                record(agent, "observed-numerical-steps", &last)?;
                return Ok(last);
            }
        }
        thread::sleep(Duration::from_millis(20));
    }
    Err(format!("never observed completed {stage} steps for {ids:?}: {last}").into())
}

fn cancel(agent: &LiveAgent, job: &str) -> Result<(), Box<dyn Error>> {
    let receipt = agent.request("cancel-control", "cancel_job", json!({"job_id":job}))?;
    assert!(
        receipt["result"]["cancelled"] == true
            || receipt["error"]["details"]["cancel_registered"] == true
    );
    Ok(())
}

fn assert_cancelled(response: &Value, stage: &str) {
    assert_eq!(response["ok"], false, "{response}");
    assert_eq!(response["error"]["code"], "cancelled", "{response}");
    assert!(response.get("result").is_none_or(Value::is_null));
    let point = &response["error"]["details"]["solver_checkpoint"];
    assert_eq!(point["stage"], stage);
    assert!(point["completed_steps"].as_u64().unwrap() >= 3);
    assert_eq!(point["resumable"], false);
}

fn verify_heat(agent: &LiveAgent, job: &str, n: usize) -> Result<(), Box<dyn Error>> {
    let response = agent.request("healthy-next", HEAT, heat_grid(job, n))?;
    verify_heat_result(agent, n, n * n, &response)?;
    wait_for_lifecycle(agent, "accepting", 0)?;
    Ok(())
}

fn verify_heat_result(
    agent: &LiveAgent,
    n: usize,
    elements: usize,
    response: &Value,
) -> Result<(), Box<dyn Error>> {
    let result = successful_result(&response, "healthy-next");
    let nodes = result["nodes"].as_array().unwrap();
    assert_eq!(nodes.len(), (n + 1) * (n + 1));
    assert_eq!(result["elements"].as_array().unwrap().len(), elements);
    for (i, node) in nodes.iter().enumerate() {
        let expected = 100.0 - 80.0 * (i % (n + 1)) as f64 / n as f64;
        let value = node["temperature"].as_f64().unwrap();
        assert!(
            (value - expected).abs() < 1e-6,
            "node {i}: {value} != {expected}"
        );
    }
    record(agent, "healthy-next", &response)?;
    Ok(())
}

#[test]
fn cancellation_after_real_pcg_iterations_discards_partial_results_and_recovers()
-> Result<(), Box<dyn Error>> {
    let agent = LiveAgent::start_with_solver_hold("sparse_iteration", HEAT, "1")?;
    fs::write(&agent.hold_path, "pcg-held")?;
    let mut stream = pending(&agent, "pcg-request", HEAT, heat_grid("pcg-held", 40))?;
    wait_for_numerical_steps(&agent, &["pcg-request"], "sparse_iteration")?;
    cancel(&agent, "pcg-held")?;
    let response = final_frame(&mut stream)?;
    assert_cancelled(&response, "sparse_iteration");
    record(&agent, "cancelled", &response)?;
    wait_for_lifecycle(&agent, "accepting", 0)?;
    assert!(agent.hold_path.exists());
    verify_heat(&agent, "pcg-next", 40)
}

#[test]
fn cancellation_after_dense_pivots_preserves_healthy_heat_solution() -> Result<(), Box<dyn Error>> {
    let agent = LiveAgent::start_with_solver_hold("dense_factor", HEAT, "1")?;
    fs::write(&agent.hold_path, "dense-held")?;
    let mut stream = pending(&agent, "dense-request", HEAT, heat_grid("dense-held", 8))?;
    wait_for_numerical_steps(&agent, &["dense-request"], "dense_factor")?;
    cancel(&agent, "dense-held")?;
    let response = final_frame(&mut stream)?;
    assert_cancelled(&response, "dense_factor");
    record(&agent, "cancelled", &response)?;
    wait_for_lifecycle(&agent, "accepting", 0)?;
    assert!(agent.hold_path.exists());
    verify_heat(&agent, "dense-next", 8)
}

#[test]
fn transport_loss_inside_pcg_reaps_only_the_orphan_and_allows_replay() -> Result<(), Box<dyn Error>>
{
    let agent = LiveAgent::start_with_solver_hold("sparse_iteration", HEAT, "1")?;
    let job = "pcg-orphan";
    fs::write(&agent.hold_path, job)?;
    let stream = pending(&agent, "pcg-orphan-old", HEAT, heat_grid(job, 40))?;
    wait_for_numerical_steps(&agent, &["pcg-orphan-old"], "sparse_iteration")?;
    stream.shutdown(Shutdown::Both)?;
    drop(stream);
    wait_for_lifecycle(&agent, "accepting", 0)?;
    assert!(agent.hold_path.exists());
    let observed = agent.request("orphan-outcome", "describe_agent", json!({}))?;
    assert!(
        observed["result"]["watchdog"]["recent_failures"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["request_id"] == "pcg-orphan-old"
                && f["reason_code"] == "cancelled"
                && f["message"]
                    .as_str()
                    .unwrap_or("")
                    .contains("sparse_iteration"))
    );
    record(&agent, "orphan-outcome", &observed)?;
    fs::remove_file(&agent.hold_path)?;
    verify_heat(&agent, job, 40)
}

#[test]
fn one_job_cancel_stops_both_active_solvers_without_poisoning_later_execution()
-> Result<(), Box<dyn Error>> {
    let agent = LiveAgent::start_with_solver_hold("sparse_iteration", HEAT, "2")?;
    let job = "pcg-shared-job";
    fs::write(&agent.hold_path, job)?;
    let mut first = pending(&agent, "pcg-first", HEAT, heat_grid(job, 40))?;
    let mut second = pending(&agent, "pcg-second", HEAT, heat_grid(job, 40))?;
    wait_for_numerical_steps(&agent, &["pcg-first", "pcg-second"], "sparse_iteration")?;
    cancel(&agent, job)?;
    for (name, stream) in [
        ("first-cancelled", &mut first),
        ("second-cancelled", &mut second),
    ] {
        let response = final_frame(stream)?;
        assert_cancelled(&response, "sparse_iteration");
        record(&agent, name, &response)?;
    }
    wait_for_lifecycle(&agent, "accepting", 0)?;
    assert!(agent.hold_path.exists());
    fs::remove_file(&agent.hold_path)?;
    verify_heat(&agent, job, 40)
}

#[test]
fn invalid_numerical_stage_fails_before_listening() {
    let error = LiveAgent::start_with_solver_hold("linear_prepare", HEAT, "1")
        .err()
        .expect("invalid stage accepted");
    assert!(
        error
            .to_string()
            .contains("KYUUBIKI_AGENT_FAULT_INJECTION_SOLVER_STAGE")
    );
}

#[test]
fn cancellation_inside_heat_chain_preserves_the_next_analytical_solution()
-> Result<(), Box<dyn Error>> {
    let method = "solve_heat_bar_1d";
    let agent = LiveAgent::start_with_solver_hold("tridiagonal_factor", method, "1")?;
    let job = "chain-held";
    let n = 600;
    let nodes: Vec<_> = (0..=n).map(|i| json!({"id":format!("n{i}"), "x":i as f64 / n as f64,
        "fix_temperature":i == 0 || i == n,"temperature":if i == 0 {100.0} else {20.0},"heat_load":0.0})).collect();
    let elements: Vec<_> = (0..n)
        .map(|i| {
            json!({"id":format!("e{i}"),"node_i":i,"node_j":i+1,
        "area":0.02,"conductivity":45.0})
        })
        .collect();
    let params = json!({"job_id":job,"nodes":nodes,"elements":elements});
    fs::write(&agent.hold_path, job)?;
    let mut stream = pending(&agent, "chain-request", method, params.clone())?;
    wait_for_numerical_steps(&agent, &["chain-request"], "tridiagonal_factor")?;
    cancel(&agent, job)?;
    let response = final_frame(&mut stream)?;
    assert_cancelled(&response, "tridiagonal_factor");
    record(&agent, "cancelled", &response)?;
    wait_for_lifecycle(&agent, "accepting", 0)?;
    assert!(agent.hold_path.exists());
    fs::remove_file(&agent.hold_path)?;
    let next = agent.request("chain-next", method, params)?;
    let result = successful_result(&next, "chain-next");
    for (i, node) in result["nodes"].as_array().unwrap().iter().enumerate() {
        let expected = 100.0 - 80.0 * i as f64 / n as f64;
        assert!((node["temperature"].as_f64().unwrap() - expected).abs() < 1e-6);
    }
    record(&agent, "healthy-next", &next)?;
    Ok(())
}

fn cancel_preparation(stage: &str, method: &str, params: Value) -> Result<(), Box<dyn Error>> {
    cancel_pass(stage, method, params, 64)
}

fn cancel_pass(stage: &str, method: &str, params: Value, steps: u64) -> Result<(), Box<dyn Error>> {
    let agent = LiveAgent::start_with_solver_hold(stage, method, "1")?;
    let job = params["job_id"].as_str().unwrap();
    fs::write(&agent.hold_path, job)?;
    let mut stream = pending(&agent, "preparation-held", method, params.clone())?;
    let observed = wait_for_numerical_steps(&agent, &["preparation-held"], stage)?;
    let point = &observed["result"]["solver_control"]["active"][0]["checkpoint"];
    assert_eq!(point["completed_steps"], steps);
    cancel(&agent, job)?;
    let response = final_frame(&mut stream)?;
    assert_cancelled(&response, stage);
    assert_eq!(
        response["error"]["details"]["solver_checkpoint"]["completed_steps"],
        steps
    );
    record(&agent, "cancelled", &response)?;
    wait_for_lifecycle(&agent, "accepting", 0)?;
    assert!(agent.hold_path.exists());
    fs::remove_file(&agent.hold_path)?;
    let response = agent.request("healthy-next", method, params.clone())?;
    verify_heat_result(
        &agent,
        40,
        params["elements"].as_array().unwrap().len(),
        &response,
    )?;
    wait_for_lifecycle(&agent, "accepting", 0)?;
    Ok(())
}

#[test]
fn cancel_pcg_rhs_scale_then_reuse_the_job() -> Result<(), Box<dyn Error>> {
    cancel_pass("pcg_rhs_scale", HEAT, heat_grid("pcg-scale", 40), 1024)
}

#[test]
fn cancel_pcg_rhs_normalize_then_reuse_the_job() -> Result<(), Box<dyn Error>> {
    cancel_pass(
        "pcg_rhs_normalize",
        HEAT,
        heat_grid("pcg-normalize", 40),
        1024,
    )
}

#[test]
fn cancel_pcg_direction_copy_then_reuse_the_job() -> Result<(), Box<dyn Error>> {
    cancel_pass("pcg_direction_copy", HEAT, heat_grid("pcg-copy", 40), 1024)
}

#[test]
fn cancel_pcg_dot_then_reuse_the_job() -> Result<(), Box<dyn Error>> {
    cancel_pass("pcg_dot", HEAT, heat_grid("pcg-dot", 40), 1024)
}

#[test]
fn cancel_pcg_norm_then_reuse_the_job() -> Result<(), Box<dyn Error>> {
    cancel_pass("pcg_norm", HEAT, heat_grid("pcg-norm", 40), 1024)
}

#[test]
fn cancel_pcg_vector_update_then_reuse_the_job() -> Result<(), Box<dyn Error>> {
    cancel_pass("pcg_vector_update", HEAT, heat_grid("pcg-update", 40), 1024)
}

#[test]
fn cancel_pcg_residual_update_then_reuse_the_job() -> Result<(), Box<dyn Error>> {
    cancel_pass(
        "pcg_residual_update",
        HEAT,
        heat_grid("pcg-residual", 40),
        1024,
    )
}

#[test]
fn cancel_pcg_direction_update_then_reuse_the_job() -> Result<(), Box<dyn Error>> {
    cancel_pass(
        "pcg_direction_update",
        HEAT,
        heat_grid("pcg-direction", 40),
        1024,
    )
}

#[test]
fn cancel_pcg_solution_scale_then_reuse_the_job() -> Result<(), Box<dyn Error>> {
    cancel_pass(
        "pcg_solution_scale",
        HEAT,
        heat_grid("pcg-solution", 40),
        1024,
    )
}

#[test]
fn disconnected_pcg_update_releases_capacity_without_releasing_the_hold()
-> Result<(), Box<dyn Error>> {
    disconnected_pass("pcg_vector_update", 1024)
}

#[test]
fn disconnected_pcg_solution_scale_does_not_commit_a_partial_success() -> Result<(), Box<dyn Error>>
{
    disconnected_pass("pcg_solution_scale", 1024)
}

#[test]
fn cancel_heat_element_precompute_then_reuse_the_job() -> Result<(), Box<dyn Error>> {
    cancel_preparation("element_precompute", HEAT, heat_grid("precompute-held", 40))
}

#[test]
fn cancel_heat_element_assembly_then_reuse_the_job() -> Result<(), Box<dyn Error>> {
    cancel_preparation("element_assembly", HEAT, heat_grid("assembly-held", 40))
}

#[test]
fn cancel_heat_sparse_compression_then_reuse_the_job() -> Result<(), Box<dyn Error>> {
    cancel_preparation("sparse_compress", HEAT, heat_grid("compression-held", 40))
}

#[test]
fn cancel_heat_preconditioner_setup_then_reuse_the_job() -> Result<(), Box<dyn Error>> {
    cancel_preparation(
        "preconditioner_setup",
        HEAT,
        heat_grid("preconditioner-held", 40),
    )
}

fn heat_triangles(job: &str) -> Value {
    let mut grid = heat_grid(job, 40);
    let elements: Vec<_> = grid["elements"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|quad| {
            let mut first = quad.clone();
            first.as_object_mut().unwrap().remove("node_l");
            first["id"] = json!(format!("{}a", quad["id"].as_str().unwrap()));
            let mut second = first.clone();
            second["id"] = json!(format!("{}b", quad["id"].as_str().unwrap()));
            second["node_j"] = quad["node_k"].clone();
            second["node_k"] = quad["node_l"].clone();
            [first, second]
        })
        .collect();
    grid["elements"] = json!(elements);
    grid
}

#[test]
fn cancel_heat_constraint_index_then_reuse_the_job() -> Result<(), Box<dyn Error>> {
    cancel_preparation("constraint_index", HEAT, heat_grid("index-held", 40))
}

#[test]
fn cancel_heat_free_mapping_then_reuse_the_job() -> Result<(), Box<dyn Error>> {
    cancel_preparation("constraint_map", HEAT, heat_grid("mapping-held", 40))
}

#[test]
fn cancel_heat_constraint_reduction_then_reuse_the_job() -> Result<(), Box<dyn Error>> {
    cancel_preparation("constraint_reduce", HEAT, heat_grid("reduction-held", 40))
}

#[test]
fn cancel_heat_jacobi_application_then_reuse_the_job() -> Result<(), Box<dyn Error>> {
    cancel_preparation("preconditioner_jacobi", HEAT, heat_grid("jacobi-held", 40))
}

#[test]
fn cancel_heat_matvec_then_reuse_the_job() -> Result<(), Box<dyn Error>> {
    cancel_preparation("sparse_matvec", HEAT, heat_grid("matvec-held", 40))
}

#[test]
fn cancel_heat_residual_then_reuse_the_job() -> Result<(), Box<dyn Error>> {
    cancel_preparation("sparse_residual", HEAT, heat_grid("residual-held", 40))
}

#[test]
fn cancel_heat_validation_then_reuse_the_job() -> Result<(), Box<dyn Error>> {
    cancel_preparation("residual_validate", HEAT, heat_grid("validation-held", 40))
}

#[test]
fn disconnected_validation_releases_capacity_without_releasing_the_hold()
-> Result<(), Box<dyn Error>> {
    disconnected_sweep("residual_validate")
}

#[test]
fn cancel_triangle_precompute_then_reuse_the_job() -> Result<(), Box<dyn Error>> {
    cancel_preparation(
        "element_precompute",
        "solve_heat_plane_triangle_2d",
        heat_triangles("tri-precompute"),
    )
}

#[test]
fn cancel_triangle_assembly_then_reuse_the_job() -> Result<(), Box<dyn Error>> {
    cancel_preparation(
        "element_assembly",
        "solve_heat_plane_triangle_2d",
        heat_triangles("tri-assembly"),
    )
}

#[test]
fn disconnected_assembly_releases_capacity_without_releasing_the_hold() -> Result<(), Box<dyn Error>>
{
    disconnected_sweep("element_assembly")
}

#[test]
fn disconnected_reduction_releases_capacity_without_releasing_the_hold()
-> Result<(), Box<dyn Error>> {
    disconnected_sweep("constraint_reduce")
}

fn disconnected_sweep(stage: &str) -> Result<(), Box<dyn Error>> {
    disconnected_pass(stage, 64)
}

fn disconnected_pass(stage: &str, steps: u64) -> Result<(), Box<dyn Error>> {
    let agent = LiveAgent::start_with_solver_hold(stage, HEAT, "1")?;
    let job = "assembly-orphan";
    fs::write(&agent.hold_path, job)?;
    let stream = pending(&agent, "assembly-orphan-old", HEAT, heat_grid(job, 40))?;
    let held = wait_for_numerical_steps(&agent, &["assembly-orphan-old"], stage)?;
    assert_eq!(
        held["result"]["solver_control"]["active"][0]["checkpoint"]["completed_steps"],
        steps
    );
    stream.shutdown(Shutdown::Both)?;
    drop(stream);
    wait_for_lifecycle(&agent, "accepting", 0)?;
    assert!(agent.hold_path.exists());
    let observed = agent.request("orphan-outcome", "describe_agent", json!({}))?;
    assert!(
        observed["result"]["watchdog"]["recent_failures"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["request_id"] == "assembly-orphan-old"
                && f["reason_code"] == "cancelled"
                && f["message"]
                    .as_str()
                    .unwrap_or("")
                    .contains(&format!("{stage} after {steps} steps")))
    );
    record(&agent, "orphan-outcome", &observed)?;
    fs::remove_file(&agent.hold_path)?;
    verify_heat(&agent, job, 40)
}

#[test]
fn cancelling_one_assembly_does_not_cancel_another_job() -> Result<(), Box<dyn Error>> {
    let agent = LiveAgent::start_with_solver_hold("element_assembly", HEAT, "2")?;
    fs::write(&agent.hold_path, "assembly-isolated")?;
    let mut stream = pending(
        &agent,
        "isolated-held",
        HEAT,
        heat_grid("assembly-isolated", 40),
    )?;
    wait_for_numerical_steps(&agent, &["isolated-held"], "element_assembly")?;
    let healthy = agent.request(
        "healthy-next",
        HEAT,
        heat_grid("independent-before-cancel", 40),
    )?;
    verify_heat_result(&agent, 40, 1600, &healthy)?;
    wait_for_lifecycle(&agent, "accepting", 1)?;
    cancel(&agent, "assembly-isolated")?;
    let response = final_frame(&mut stream)?;
    assert_cancelled(&response, "element_assembly");
    record(&agent, "cancelled", &response)?;
    wait_for_lifecycle(&agent, "accepting", 0)?;
    verify_heat(&agent, "independent-after-cancel", 40)
}
