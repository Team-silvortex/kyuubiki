use crate::bridge::{
    bridge_electrostatic_result_to_heat_plane_quad_model,
    bridge_electrostatic_result_to_heat_plane_triangle_model,
    resolve_electrostatic_to_heat_bridge_contract,
};
use crate::heat_bridge::{
    bridge_heat_result_to_thermal_plane_quad_model,
    bridge_heat_result_to_thermal_plane_triangle_model,
};
use crate::{EngineSolveRequest, solve};
use kyuubiki_protocol::{
    AnalysisResult, ElectrostaticHeatToThermoPlaneQuad2dWorkflowRequest,
    ElectrostaticHeatToThermoPlaneQuad2dWorkflowResult,
    ElectrostaticHeatToThermoPlaneTriangle2dWorkflowRequest,
    ElectrostaticHeatToThermoPlaneTriangle2dWorkflowResult, HeatToThermoPlaneQuad2dWorkflowRequest,
    HeatToThermoPlaneQuad2dWorkflowResult, HeatToThermoPlaneTriangle2dWorkflowRequest,
    HeatToThermoPlaneTriangle2dWorkflowResult,
};
use serde_json::{Value, json};

pub fn run_heat_to_thermo_plane_quad_2d_workflow(
    request: HeatToThermoPlaneQuad2dWorkflowRequest,
) -> Result<HeatToThermoPlaneQuad2dWorkflowResult, String> {
    let heat_result = match solve(EngineSolveRequest::HeatPlaneQuad2d(request.heat_model))? {
        AnalysisResult::HeatPlaneQuad2d(result) => result,
        _ => return Err("heat-plane-quad workflow produced an unexpected heat result".to_string()),
    };

    let (bridged_model, _) =
        bridge_heat_result_to_thermal_plane_quad_model(&heat_result, &request.thermo_seed_model)?;
    let thermo_result = match solve(EngineSolveRequest::ThermalPlaneQuad2d(
        bridged_model.clone(),
    ))? {
        AnalysisResult::ThermalPlaneQuad2d(result) => result,
        _ => {
            return Err("heat-to-thermo workflow produced an unexpected thermo result".to_string());
        }
    };

    Ok(HeatToThermoPlaneQuad2dWorkflowResult {
        workflow_id: "workflow.heat-to-thermo-quad-2d".to_string(),
        heat_result,
        bridged_model,
        thermo_result,
    })
}

pub fn run_heat_to_thermo_plane_triangle_2d_workflow(
    request: HeatToThermoPlaneTriangle2dWorkflowRequest,
) -> Result<HeatToThermoPlaneTriangle2dWorkflowResult, String> {
    let heat_result = match solve(EngineSolveRequest::HeatPlaneTriangle2d(request.heat_model))? {
        AnalysisResult::HeatPlaneTriangle2d(result) => result,
        _ => {
            return Err(
                "heat-plane-triangle workflow produced an unexpected heat result".to_string(),
            );
        }
    };

    let (bridged_model, _) = bridge_heat_result_to_thermal_plane_triangle_model(
        &heat_result,
        &request.thermo_seed_model,
    )?;
    let thermo_result = match solve(EngineSolveRequest::ThermalPlaneTriangle2d(
        bridged_model.clone(),
    ))? {
        AnalysisResult::ThermalPlaneTriangle2d(result) => result,
        _ => {
            return Err(
                "heat-to-thermo triangle workflow produced an unexpected thermo result".to_string(),
            );
        }
    };

    Ok(HeatToThermoPlaneTriangle2dWorkflowResult {
        workflow_id: "workflow.heat-to-thermo-triangle-2d".to_string(),
        heat_result,
        bridged_model,
        thermo_result,
    })
}

pub fn run_electrostatic_to_heat_to_thermo_plane_quad_2d_workflow(
    request: ElectrostaticHeatToThermoPlaneQuad2dWorkflowRequest,
) -> Result<ElectrostaticHeatToThermoPlaneQuad2dWorkflowResult, String> {
    let electrostatic_result = match solve(EngineSolveRequest::ElectrostaticPlaneQuad2d(
        request.electrostatic_model,
    ))? {
        AnalysisResult::ElectrostaticPlaneQuad2d(result) => result,
        _ => {
            return Err(
                "electrostatic-plane-quad workflow produced an unexpected electrostatic result"
                    .to_string(),
            );
        }
    };

    let electrostatic_contract = resolve_electrostatic_to_heat_bridge_contract(&Value::Null)?;
    let (bridged_heat_model, _) = bridge_electrostatic_result_to_heat_plane_quad_model(
        &electrostatic_result,
        &request.heat_seed_model,
        &electrostatic_contract,
    )?;
    let heat_result = match solve(EngineSolveRequest::HeatPlaneQuad2d(
        bridged_heat_model.clone(),
    ))? {
        AnalysisResult::HeatPlaneQuad2d(result) => result,
        _ => {
            return Err(
                "electrostatic-to-heat quad workflow produced an unexpected heat result"
                    .to_string(),
            );
        }
    };

    let (bridged_thermo_model, _) =
        bridge_heat_result_to_thermal_plane_quad_model(&heat_result, &request.thermo_seed_model)?;
    let thermo_result = match solve(EngineSolveRequest::ThermalPlaneQuad2d(
        bridged_thermo_model.clone(),
    ))? {
        AnalysisResult::ThermalPlaneQuad2d(result) => result,
        _ => {
            return Err(
                "electrostatic-heat-to-thermo quad workflow produced an unexpected thermo result"
                    .to_string(),
            );
        }
    };

    Ok(ElectrostaticHeatToThermoPlaneQuad2dWorkflowResult {
        workflow_id: "workflow.electrostatic-heat-to-thermo-quad-2d".to_string(),
        electrostatic_result,
        bridged_heat_model,
        heat_result,
        bridged_thermo_model,
        thermo_result,
    })
}

pub fn run_electrostatic_to_heat_to_thermo_plane_triangle_2d_workflow(
    request: ElectrostaticHeatToThermoPlaneTriangle2dWorkflowRequest,
) -> Result<ElectrostaticHeatToThermoPlaneTriangle2dWorkflowResult, String> {
    let electrostatic_result = match solve(EngineSolveRequest::ElectrostaticPlaneTriangle2d(
        request.electrostatic_model,
    ))? {
        AnalysisResult::ElectrostaticPlaneTriangle2d(result) => result,
        _ => {
            return Err(
                "electrostatic-plane-triangle workflow produced an unexpected electrostatic result"
                    .to_string(),
            );
        }
    };

    let electrostatic_contract = resolve_electrostatic_to_heat_bridge_contract(&json!({
        "source": {
            "node_index_fields": ["node_i", "node_j", "node_k"]
        }
    }))?;
    let (bridged_heat_model, _) = bridge_electrostatic_result_to_heat_plane_triangle_model(
        &electrostatic_result,
        &request.heat_seed_model,
        &electrostatic_contract,
    )?;
    let heat_result = match solve(EngineSolveRequest::HeatPlaneTriangle2d(
        bridged_heat_model.clone(),
    ))? {
        AnalysisResult::HeatPlaneTriangle2d(result) => result,
        _ => {
            return Err(
                "electrostatic-to-heat triangle workflow produced an unexpected heat result"
                    .to_string(),
            );
        }
    };

    let (bridged_thermo_model, _) = bridge_heat_result_to_thermal_plane_triangle_model(
        &heat_result,
        &request.thermo_seed_model,
    )?;
    let thermo_result = match solve(EngineSolveRequest::ThermalPlaneTriangle2d(
        bridged_thermo_model.clone(),
    ))? {
        AnalysisResult::ThermalPlaneTriangle2d(result) => result,
        _ => {
            return Err(
                "electrostatic-heat-to-thermo triangle workflow produced an unexpected thermo result"
                    .to_string(),
            );
        }
    };

    Ok(ElectrostaticHeatToThermoPlaneTriangle2dWorkflowResult {
        workflow_id: "workflow.electrostatic-heat-to-thermo-triangle-2d".to_string(),
        electrostatic_result,
        bridged_heat_model,
        heat_result,
        bridged_thermo_model,
        thermo_result,
    })
}
