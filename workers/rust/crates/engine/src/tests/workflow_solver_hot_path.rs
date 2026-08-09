use super::workflow_large_graphs::{heat_model_input, thermo_seed_model};
use crate::heat_bridge::bridge_heat_result_to_thermal_plane_quad_model;
use crate::operator_sdk_runtime::{BuiltInOperatorRegistryKind, built_in_operator_registry_ref};
use crate::workflow_executor::{run_solve_operator, run_transform_operator};
use kyuubiki_protocol::{SolveHeatPlaneQuad2dResult, SolveThermalPlaneQuad2dRequest};
use serde_json::Value;
use std::collections::BTreeMap;
use std::hint::black_box;
use std::thread;
use std::time::{Duration, Instant};

const SAMPLE_COUNT: usize = 64;
const MATERIALIZATION_COUNT: usize = 1024;

#[test]
fn shares_transform_registry_across_concurrent_bridge_runs() {
    let handles = (0..8)
        .map(|_| {
            thread::spawn(|| {
                let registry =
                    built_in_operator_registry_ref(BuiltInOperatorRegistryKind::Transform);
                let heat_result =
                    run_solve_operator("solve.heat_plane_quad_2d", heat_model_input())
                        .expect("concurrent heat solve should run");
                let bridged = run_transform_operator(
                    "bridge.temperature_field_to_thermo_quad_2d",
                    heat_result,
                    bridge_config(thermo_seed_model()),
                )
                .expect("concurrent bridge should run");
                assert_eq!(bridged["nodes"].as_array().map(Vec::len), Some(4));
                registry as *const _ as usize
            })
        })
        .collect::<Vec<_>>();
    let registry_addresses = handles
        .into_iter()
        .map(|handle| handle.join().expect("bridge worker should not panic"))
        .collect::<Vec<_>>();

    assert!(registry_addresses.windows(2).all(|pair| pair[0] == pair[1]));
}

#[test]
fn profiles_heat_to_thermo_solver_pipeline_segments() {
    let heat_model = heat_model_input();
    let thermo_seed = thermo_seed_model();
    let bridge_config = bridge_config(thermo_seed.clone());
    let heat_result = run_solve_operator("solve.heat_plane_quad_2d", heat_model.clone())
        .expect("heat solve should warm the benchmark");
    let thermo_model = run_transform_operator(
        "bridge.temperature_field_to_thermo_quad_2d",
        heat_result.clone(),
        bridge_config.clone(),
    )
    .expect("heat-to-thermo bridge should warm the benchmark");
    run_solve_operator("solve.thermal_plane_quad_2d", thermo_model.clone())
        .expect("thermo solve should warm the benchmark");

    let registry_lookup = measure(SAMPLE_COUNT, || {
        black_box(built_in_operator_registry_ref(
            BuiltInOperatorRegistryKind::Transform,
        ));
    });
    let heat_solve = measure(SAMPLE_COUNT, || {
        let payload = heat_model.clone();
        black_box(run_solve_operator("solve.heat_plane_quad_2d", payload))
            .expect("heat solve sample should run");
    });
    let bridge_api = measure(SAMPLE_COUNT, || {
        let payload = heat_result.clone();
        let config = bridge_config.clone();
        black_box(run_transform_operator(
            "bridge.temperature_field_to_thermo_quad_2d",
            payload,
            config,
        ))
        .expect("bridge API sample should run");
    });
    let bridge_core = measure_bridge_core(&heat_result, &thermo_seed);
    let thermo_solve = measure(SAMPLE_COUNT, || {
        let payload = thermo_model.clone();
        black_box(run_solve_operator("solve.thermal_plane_quad_2d", payload))
            .expect("thermo solve sample should run");
    });
    let materialization = measure_materialization(&heat_result);

    eprintln!(
        "workflow_solver_hot_path[rust]: samples={SAMPLE_COUNT} registry_lookup_avg_us={:.3} heat_solve_avg_us={:.3} bridge_api_avg_us={:.3} bridge_core_avg_us={:.3} thermo_solve_avg_us={:.3} materialize_count={MATERIALIZATION_COUNT} materialize_total_ms={:.3}",
        average_micros(registry_lookup),
        average_micros(heat_solve),
        average_micros(bridge_api),
        average_micros(bridge_core),
        average_micros(thermo_solve),
        materialization.as_secs_f64() * 1000.0,
    );

    assert!(heat_result["max_temperature"].as_f64().unwrap_or_default() > 0.0);
    assert_eq!(thermo_model["nodes"].as_array().map(Vec::len), Some(4));
    assert!(
        registry_lookup + heat_solve + bridge_api + bridge_core + thermo_solve + materialization
            < Duration::from_secs(30)
    );
}

fn measure(iterations: usize, mut sample: impl FnMut()) -> Duration {
    let started_at = Instant::now();
    for _ in 0..iterations {
        sample();
    }
    started_at.elapsed()
}

fn measure_bridge_core(heat_result: &Value, thermo_seed: &Value) -> Duration {
    let typed_heat: SolveHeatPlaneQuad2dResult = serde_json::from_value(heat_result.clone())
        .expect("heat result should decode for direct bridge measurement");
    let typed_seed: SolveThermalPlaneQuad2dRequest = serde_json::from_value(thermo_seed.clone())
        .expect("thermo seed should decode for direct bridge measurement");
    measure(SAMPLE_COUNT, || {
        black_box(bridge_heat_result_to_thermal_plane_quad_model(
            &typed_heat,
            &typed_seed,
        ))
        .expect("direct bridge sample should run");
    })
}

fn measure_materialization(value: &Value) -> Duration {
    let started_at = Instant::now();
    let artifacts = (0..MATERIALIZATION_COUNT)
        .map(|index| (format!("pass_{index:04}.result"), value.clone()))
        .collect::<BTreeMap<_, _>>();
    black_box(artifacts);
    started_at.elapsed()
}

fn average_micros(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1_000_000.0 / SAMPLE_COUNT as f64
}

fn bridge_config(seed_model: Value) -> Value {
    serde_json::json!({
        "seed_model": seed_model,
        "contract": {
            "version": "kyuubiki.bridge-contract/v1",
            "source": { "field": "temperature" },
            "transform": { "scale": 1.0, "default_value": 0.0 },
            "target": { "field": "temperature_delta" }
        }
    })
}
