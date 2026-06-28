use super::*;

#[test]
fn handles_solver_rpc_requests() {
    let request = RpcRequest {
        rpc_version: RPC_VERSION,
        id: "rpc-1".to_string(),
        method: RpcMethod::SolveBar1d,
        params: serde_json::to_value(SolveBarRequest {
            length: 1.0,
            area: 0.01,
            youngs_modulus: 210.0e9,
            elements: 1,
            tip_force: 1000.0,
        })
        .expect("params"),
    };

    let response =
        handle_request_bytes(&serde_json::to_vec(&request).expect("request should serialize"));

    let AgentReply::Stream(progress_frames, final_response) = response;

    assert_eq!(progress_frames.len(), 4);
    assert_eq!(progress_frames[0].event, "progress");
    assert_eq!(progress_frames[2].progress.stage, JobStatus::Solving);
    assert!(final_response.ok);
    assert!(final_response.error.is_none());
    assert_eq!(final_response.id, "rpc-1");
    let result: kyuubiki_protocol::SolveBarResult =
        serde_json::from_value(final_response.result.expect("solver result")).expect("bar result");
    assert!((result.tip_displacement - 4.761904761904762e-7).abs() < 1.0e-12);
}

#[test]
fn handles_thermal_bar_1d_rpc_requests() {
    let request = RpcRequest {
        rpc_version: RPC_VERSION,
        id: "rpc-thermal-bar".to_string(),
        method: RpcMethod::SolveThermalBar1d,
        params: serde_json::to_value(SolveThermalBar1dRequest {
            nodes: vec![
                ThermalBar1dNodeInput {
                    id: "n0".to_string(),
                    x: 0.0,
                    fix_x: true,
                    load_x: 0.0,
                    temperature_delta: 40.0,
                },
                ThermalBar1dNodeInput {
                    id: "n1".to_string(),
                    x: 1.0,
                    fix_x: true,
                    load_x: 0.0,
                    temperature_delta: 40.0,
                },
            ],
            elements: vec![ThermalBar1dElementInput {
                id: "tb0".to_string(),
                node_i: 0,
                node_j: 1,
                area: 0.01,
                youngs_modulus: 210.0e9,
                thermal_expansion: 12.0e-6,
            }],
        })
        .expect("params"),
    };

    let response =
        handle_request_bytes(&serde_json::to_vec(&request).expect("request should serialize"));

    let AgentReply::Stream(progress_frames, final_response) = response;

    assert_eq!(progress_frames.len(), 4);
    assert!(final_response.ok);
    let result: kyuubiki_protocol::SolveThermalBar1dResult =
        serde_json::from_value(final_response.result.expect("solver result"))
            .expect("thermal bar result");
    assert!(result.max_stress > 1.0e8);
    assert_eq!(result.max_temperature_delta, 40.0);
}

#[test]
fn handles_acoustic_bar_1d_rpc_requests() {
    let request = RpcRequest {
        rpc_version: RPC_VERSION,
        id: "rpc-acoustic-bar".to_string(),
        method: RpcMethod::SolveAcousticBar1d,
        params: serde_json::to_value(SolveAcousticBar1dRequest {
            frequency_hz: 100.0,
            nodes: vec![
                AcousticBar1dNodeInput {
                    id: "a0".to_string(),
                    x: 0.0,
                    fix_pressure: true,
                    pressure: 1.0,
                    volume_velocity_source: 0.0,
                },
                AcousticBar1dNodeInput {
                    id: "a1".to_string(),
                    x: 1.0,
                    fix_pressure: false,
                    pressure: 0.0,
                    volume_velocity_source: 0.01,
                },
            ],
            elements: vec![AcousticBar1dElementInput {
                id: "ae0".to_string(),
                node_i: 0,
                node_j: 1,
                area: 0.1,
                density: 1.2,
                bulk_modulus: 142_000.0,
                damping_ratio: 0.02,
            }],
        })
        .expect("params"),
    };

    let response =
        handle_request_bytes(&serde_json::to_vec(&request).expect("request should serialize"));

    let AgentReply::Stream(progress_frames, final_response) = response;

    assert_eq!(progress_frames.len(), 4);
    assert!(final_response.ok);
    let result: kyuubiki_protocol::SolveAcousticBar1dResult =
        serde_json::from_value(final_response.result.expect("solver result"))
            .expect("acoustic bar result");
    assert!(result.max_pressure >= 1.0);
    assert!(result.max_sound_pressure_level_db > 90.0);
    assert_eq!(result.elements.len(), 1);
}

#[test]
fn handles_heat_bar_1d_rpc_requests() {
    let request = RpcRequest {
        rpc_version: RPC_VERSION,
        id: "rpc-heat-bar".to_string(),
        method: RpcMethod::SolveHeatBar1d,
        params: serde_json::to_value(SolveHeatBar1dRequest {
            nodes: vec![
                HeatBar1dNodeInput {
                    id: "n0".to_string(),
                    x: 0.0,
                    fix_temperature: true,
                    temperature: 100.0,
                    heat_load: 0.0,
                },
                HeatBar1dNodeInput {
                    id: "n1".to_string(),
                    x: 1.0,
                    fix_temperature: true,
                    temperature: 0.0,
                    heat_load: 0.0,
                },
            ],
            elements: vec![HeatBar1dElementInput {
                id: "hb0".to_string(),
                node_i: 0,
                node_j: 1,
                area: 0.02,
                conductivity: 50.0,
            }],
        })
        .expect("params"),
    };

    let response =
        handle_request_bytes(&serde_json::to_vec(&request).expect("request should serialize"));

    let AgentReply::Stream(progress_frames, final_response) = response;

    assert_eq!(progress_frames.len(), 4);
    assert!(final_response.ok);
    let result: kyuubiki_protocol::SolveHeatBar1dResult =
        serde_json::from_value(final_response.result.expect("solver result"))
            .expect("heat bar result");
    assert_eq!(result.max_temperature, 100.0);
    assert!((result.max_heat_flux - 5_000.0).abs() < 1.0e-6);
}

#[test]
fn handles_electrostatic_bar_1d_rpc_requests() {
    let request = RpcRequest {
        rpc_version: RPC_VERSION,
        id: "rpc-electrostatic-bar".to_string(),
        method: RpcMethod::SolveElectrostaticBar1d,
        params: serde_json::to_value(SolveElectrostaticBar1dRequest {
            nodes: vec![
                ElectrostaticBar1dNodeInput {
                    id: "n0".to_string(),
                    x: 0.0,
                    fix_potential: true,
                    potential: 10.0,
                    charge_density: 0.0,
                },
                ElectrostaticBar1dNodeInput {
                    id: "n1".to_string(),
                    x: 1.0,
                    fix_potential: true,
                    potential: 0.0,
                    charge_density: 0.0,
                },
            ],
            elements: vec![ElectrostaticBar1dElementInput {
                id: "eb0".to_string(),
                node_i: 0,
                node_j: 1,
                area: 0.02,
                permittivity: 2.0,
            }],
        })
        .expect("params"),
    };

    let response =
        handle_request_bytes(&serde_json::to_vec(&request).expect("request should serialize"));

    let AgentReply::Stream(progress_frames, final_response) = response;

    assert_eq!(progress_frames.len(), 4);
    assert!(final_response.ok);
    let result: kyuubiki_protocol::SolveElectrostaticBar1dResult =
        serde_json::from_value(final_response.result.expect("solver result"))
            .expect("electrostatic bar result");
    assert_eq!(result.max_potential, 10.0);
    assert!((result.max_electric_field - 10.0).abs() < 1.0e-6);
    assert!((result.max_flux_density - 20.0).abs() < 1.0e-6);
}

#[test]
fn handles_magnetostatic_bar_1d_rpc_requests() {
    let request = RpcRequest {
        rpc_version: RPC_VERSION,
        id: "rpc-magnetostatic-bar".to_string(),
        method: RpcMethod::SolveMagnetostaticBar1d,
        params: serde_json::to_value(SolveMagnetostaticBar1dRequest {
            nodes: vec![
                MagnetostaticBar1dNodeInput {
                    id: "n0".to_string(),
                    x: 0.0,
                    fix_magnetic_potential: true,
                    magnetic_potential: 10.0,
                    magnetomotive_source: 0.0,
                },
                MagnetostaticBar1dNodeInput {
                    id: "n1".to_string(),
                    x: 1.0,
                    fix_magnetic_potential: true,
                    magnetic_potential: 0.0,
                    magnetomotive_source: 0.0,
                },
            ],
            elements: vec![MagnetostaticBar1dElementInput {
                id: "mb0".to_string(),
                node_i: 0,
                node_j: 1,
                area: 0.02,
                permeability: 2.0,
            }],
        })
        .expect("params"),
    };

    let response =
        handle_request_bytes(&serde_json::to_vec(&request).expect("request should serialize"));

    let AgentReply::Stream(progress_frames, final_response) = response;

    assert_eq!(progress_frames.len(), 4);
    assert!(final_response.ok);
    let result: kyuubiki_protocol::SolveMagnetostaticBar1dResult =
        serde_json::from_value(final_response.result.expect("solver result"))
            .expect("magnetostatic bar result");
    assert_eq!(result.max_magnetic_potential, 10.0);
    assert!((result.max_magnetic_field_strength - 10.0).abs() < 1.0e-6);
    assert!((result.max_flux_density - 20.0).abs() < 1.0e-6);
}
