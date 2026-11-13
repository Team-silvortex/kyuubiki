#[path = "../../operator-thermal-q9/tests/support/bilayer.rs"]
mod bilayer;
mod support;
use serde_json::{Value, json};
use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};
use support::operator_package::{
    LiveAgent, external_operator_task, refresh_task_digest, sha256_file,
};

struct Package(PathBuf);
impl Drop for Package {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn package() -> (Package, String) {
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let library = workspace.join("target/debug").join(format!(
        "{}kyuubiki_operator_thermal_q9{}",
        std::env::consts::DLL_PREFIX,
        std::env::consts::DLL_SUFFIX
    ));
    let digest = sha256_file(&library).expect("build -p kyuubiki-operator-thermal-q9 first");
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let package = Package(
        std::env::temp_dir().join(format!("kyuubiki-q9-live-{}-{suffix}", std::process::id())),
    );
    let dir = package.0.join("q9");
    fs::create_dir_all(&dir).unwrap();
    fs::copy(&library, dir.join(library.file_name().unwrap())).unwrap();
    fs::copy(
        workspace.join("crates/operator-thermal-q9/kyuubiki-operator.json"),
        dir.join("kyuubiki-operator.json"),
    )
    .unwrap();
    (package, digest)
}

fn task(digest: &str, id: &str, kind: &str, input: Value) -> Value {
    let mut task = external_operator_task(digest);
    task["task_id"] = json!(id);
    task["operator"] = json!({"id":id,"family":"thermal_q9_rect","kind":kind});
    let p = &mut task["execution_program"];
    p["program_id"] = json!(id);
    p["program_family"] = json!("thermal_q9_rect");
    p["program_kind"] = json!(kind);
    p["package_ref"] = json!("bundle://operator.thermal-q9-rect");
    p["entrypoint"] = json!({"kind":"operator_id","name":id,"operator_kind":kind});
    task["runtime_hints"]["operator_kind"] = json!(kind);
    task["runtime_hints"]["package_ref"] = json!("bundle://operator.thermal-q9-rect");
    task["input_artifact"] = input;
    task["config"] = json!({});
    refresh_task_digest(&mut task);
    task
}

fn send(agent: &LiveAgent, task: Value) -> Value {
    agent
        .request(
            "q9",
            "run_operator_task_ir",
            json!({"mode":"execute","task_ir":task}),
        )
        .unwrap()
}

#[test]
#[ignore = "requires prebuilt Q9 cdylib and permission for local Agent test ports"]
fn live_agent_executes_q9_task_and_recovers_after_invalid_mesh_and_digest() {
    let (package, digest) = package();
    let agent = LiveAgent::start(&package.0).unwrap();
    let nodes: Vec<_> = (0..9)
        .map(|i| {
            json!({"id":format!("n{i}"),"x":(i%3) as f64/2.0,"y":(i/3) as f64/2.0,
        "load_x":0.0,"load_y":0.0,"fix_x":i==0,"fix_y":i==0||i==2,"temperature_delta":10.0})
        })
        .collect();
    let task = task(
        &digest,
        "solve.thermal_plane_q9_rect_2d",
        "solver",
        json!({"schema_version":kyuubiki_protocol::THERMAL_Q9_RECT_SCHEMA,"nodes":nodes,
        "elements":[{"id":"e0","nodes":[0,1,2,3,4,5,6,7,8],"thickness":0.01,"youngs_modulus":70e9,"poisson_ratio":0.3,"thermal_expansion":12e-6}]}),
    );
    let send = |t| send(&agent, t);
    for _ in 0..2 {
        let result = send(task.clone());
        assert_eq!(result["ok"], true, "{result}");
        assert_eq!(
            result["result"]["operator_package_execution"]["integrity_verified"],
            true
        );
        let value = &result["result"]["result"]["summary"];
        assert!((value["nodes"][8]["ux"].as_f64().unwrap() - 120e-6).abs() < 1e-12);
        let mut invalid = task.clone();
        invalid["input_artifact"]["nodes"][4]["x"] = json!(0.6);
        refresh_task_digest(&mut invalid);
        let rejected = send(invalid);
        assert_eq!(rejected["ok"], false);
        assert_eq!(
            rejected["error"]["code"], "operator_package_dispatch_failed",
            "{rejected}"
        );
        let mut invalid = task.clone();
        invalid["execution_program"]["package_integrity"]["digest"] = json!("0".repeat(64));
        refresh_task_digest(&mut invalid);
        let rejected = send(invalid);
        assert_eq!(
            rejected["error"]["code"], "operator_package_identity_mismatch",
            "{rejected}"
        );
    }
    assert_eq!(send(task)["ok"], true);
}

#[test]
#[ignore = "requires prebuilt Q9 cdylib and permission for local Agent test ports"]
fn live_agent_heat_bridge_q9_chain_checks_bilayers_and_failed_transfer_recovery() {
    let (package, digest) = package();
    let agent = LiveAgent::start(&package.0).unwrap();
    let cases = [
        (2, 2, [10.0, 40.0], [10e-6, 30e-6], [0.0, 20.0]),
        (8, 4, [10.0, 40.0], [10e-6, 30e-6], [0.0, 20.0]),
        (16, 8, [10.0, 40.0], [10e-6, 30e-6], [0.0, 20.0]),
        (2, 2, [10.0, 40.0], [10e-6, 30e-6], [10.0, 10.0]),
        (8, 4, [10.0, 40.0], [10e-6, 30e-6], [10.0, 10.0]),
        (16, 8, [10.0, 40.0], [10e-6, 30e-6], [10.0, 10.0]),
        (8, 4, [40.0, 10.0], [30e-6, 10e-6], [20.0, 0.0]),
        (8, 4, [1.0, 100.0], [5e-6, 40e-6], [-10.0, 20.0]),
    ];
    for (nx, ny, k, alpha, rise) in cases {
        let case = bilayer::Case::new(nx, ny, k, alpha, rise);
        let heat = agent
            .request(
                "heat",
                "solve_heat_plane_quad_2d",
                serde_json::to_value(&case.heat).unwrap(),
            )
            .unwrap();
        assert_eq!(heat["ok"], true, "{heat}");
        let payload = json!({"schema_version":"kyuubiki.heat-to-thermal-q9-nodal/v1", "temperature_unit":"kelvin",
            "reference_temperature":case.reference_temperature, "heat_result":heat["result"], "seed_model":case.seed});
        for (pointer, value) in [
            ("/temperature_unit", json!("celsius")),
            ("/heat_result/nodes/0/id", json!("stale-node")),
        ] {
            let mut invalid = payload.clone();
            *invalid.pointer_mut(pointer).unwrap() = value;
            let response = send(
                &agent,
                task(
                    &digest,
                    "workflow_bridge.heat_to_thermal_q9_rect_2d",
                    "workflow_bridge",
                    invalid,
                ),
            );
            assert_eq!(response["ok"], false, "{response}");
            assert_eq!(
                response["error"]["code"], "operator_package_dispatch_failed",
                "{response}"
            );
        }
        let bridged = send(
            &agent,
            task(
                &digest,
                "workflow_bridge.heat_to_thermal_q9_rect_2d",
                "workflow_bridge",
                payload,
            ),
        );
        assert_eq!(bridged["ok"], true, "{bridged}");
        assert_eq!(
            bridged["result"]["operator_package_execution"]["integrity_verified"],
            true
        );
        let model = bridged["result"]["result"]["summary"].clone();
        let mut temperature_error = 0.0_f64;
        for n in model["nodes"].as_array().unwrap() {
            temperature_error = temperature_error.max(
                (n["temperature_delta"].as_f64().unwrap()
                    - case.expected_delta(n["y"].as_f64().unwrap()))
                .abs(),
            );
        }
        assert!(temperature_error < 1e-8, "{temperature_error}");
        let solved = send(
            &agent,
            task(&digest, "solve.thermal_plane_q9_rect_2d", "solver", model),
        );
        assert_eq!(solved["ok"], true, "{solved}");
        assert_eq!(
            solved["result"]["operator_package_execution"]["integrity_verified"],
            true
        );
        let result = &solved["result"]["result"]["summary"];
        let energy_error = (result["total_strain_energy_j"].as_f64().unwrap() - case.energy).abs();
        assert!(energy_error < 1e-8, "{energy_error}");
        let mut displacement_error = 0.0_f64;
        for n in result["nodes"].as_array().unwrap() {
            let [ux, uy] =
                case.expected_displacement(n["x"].as_f64().unwrap(), n["y"].as_f64().unwrap());
            displacement_error = displacement_error
                .max((n["ux"].as_f64().unwrap() - ux).abs())
                .max((n["uy"].as_f64().unwrap() - uy).abs());
        }
        assert!(displacement_error < 1e-10, "{displacement_error}");
        println!(
            "q9-chain mesh={nx}x{ny} k={k:?} alpha={alpha:?} rise={rise:?} reference_energy={:.12e} temperature_abs={temperature_error:.6e} displacement_abs={displacement_error:.6e} energy_abs={energy_error:.6e}",
            case.energy
        );
    }
}
