use crate::all_direct_fem_routes;
use serde::Serialize;

pub const ENGINE_SOLVER_HEADLESS_BRIDGE_SCHEMA_VERSION: &str =
    "kyuubiki.headless-engine-solver-bridge/v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EngineSolverHeadlessBridgeManifest {
    pub schema_version: &'static str,
    pub bridge_owner: &'static str,
    pub route_count: usize,
    pub routes: Vec<EngineSolverHeadlessBridgeRoute>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EngineSolverHeadlessBridgeRoute {
    pub action: String,
    pub direct_fem_route: String,
    pub engine_operator_id: String,
    pub result_type: String,
    pub provenance_field: &'static str,
}

pub fn engine_solver_headless_bridge_manifest() -> EngineSolverHeadlessBridgeManifest {
    let routes = all_direct_fem_routes()
        .iter()
        .map(|route| {
            let engine_operator_id = engine_operator_id_for_action(route.action);
            EngineSolverHeadlessBridgeRoute {
                action: route.action.to_string(),
                direct_fem_route: route.route.to_string(),
                result_type: result_type_for_engine_operator(&engine_operator_id),
                engine_operator_id,
                provenance_field: "_solver_provenance",
            }
        })
        .collect::<Vec<_>>();

    EngineSolverHeadlessBridgeManifest {
        schema_version: ENGINE_SOLVER_HEADLESS_BRIDGE_SCHEMA_VERSION,
        bridge_owner: "headless_sdk",
        route_count: routes.len(),
        routes,
    }
}

fn engine_operator_id_for_action(action: &str) -> String {
    let suffix = action.strip_prefix("solve_").unwrap_or(action);
    let engine_suffix = match suffix {
        "stokes_flow_plane_triangle_2d" => "stokes_flow_triangle_2d",
        "stokes_flow_plane_quad_2d" => "stokes_flow_quad_2d",
        other => other,
    };
    format!("solve.{engine_suffix}")
}

fn result_type_for_engine_operator(operator_id: &str) -> String {
    operator_id
        .strip_prefix("solve.")
        .map(|suffix| format!("result/{suffix}"))
        .unwrap_or_else(|| "result/unknown".to_string())
}

#[cfg(test)]
mod tests {
    use super::{
        ENGINE_SOLVER_HEADLESS_BRIDGE_SCHEMA_VERSION, engine_solver_headless_bridge_manifest,
    };
    use crate::all_direct_fem_routes;

    #[test]
    fn bridge_manifest_maps_direct_fem_routes_to_engine_solver_ids() {
        let manifest = engine_solver_headless_bridge_manifest();

        assert_eq!(
            manifest.schema_version,
            ENGINE_SOLVER_HEADLESS_BRIDGE_SCHEMA_VERSION
        );
        assert_eq!(manifest.route_count, all_direct_fem_routes().len());
        assert!(manifest.routes.iter().all(|route| {
            route.engine_operator_id.starts_with("solve.")
                && route.provenance_field == "_solver_provenance"
        }));
        assert!(manifest.routes.iter().any(|route| {
            route.action == "solve_stokes_flow_plane_quad_2d"
                && route.engine_operator_id == "solve.stokes_flow_quad_2d"
                && route.result_type == "result/stokes_flow_quad_2d"
        }));
    }
}
