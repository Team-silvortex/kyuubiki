use crate::{HeadlessActionContract, HeadlessEngine, HeadlessRisk, find_action_contract};
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DirectFemRoute {
    pub action: &'static str,
    pub route: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DirectFemCapability {
    pub action: String,
    pub route: String,
    pub engine: HeadlessEngine,
    pub risk: HeadlessRisk,
    pub required_payload_keys: Vec<String>,
    pub output_keys: Vec<String>,
}

const DIRECT_FEM_ROUTES: &[DirectFemRoute] = &[
    DirectFemRoute {
        action: "solve_bar_1d",
        route: "/api/v1/fem/axial-bar/jobs",
    },
    DirectFemRoute {
        action: "solve_thermal_bar_1d",
        route: "/api/v1/fem/thermal-bar-1d/jobs",
    },
    DirectFemRoute {
        action: "solve_heat_bar_1d",
        route: "/api/v1/fem/heat-bar-1d/jobs",
    },
    DirectFemRoute {
        action: "solve_electrostatic_bar_1d",
        route: "/api/v1/fem/electrostatic-bar-1d/jobs",
    },
    DirectFemRoute {
        action: "solve_electrostatic_plane_triangle_2d",
        route: "/api/v1/fem/electrostatic-plane-triangle-2d/jobs",
    },
    DirectFemRoute {
        action: "solve_electrostatic_plane_quad_2d",
        route: "/api/v1/fem/electrostatic-plane-quad-2d/jobs",
    },
    DirectFemRoute {
        action: "solve_heat_plane_triangle_2d",
        route: "/api/v1/fem/heat-plane-triangle-2d/jobs",
    },
    DirectFemRoute {
        action: "solve_heat_plane_quad_2d",
        route: "/api/v1/fem/heat-plane-quad-2d/jobs",
    },
    DirectFemRoute {
        action: "solve_thermal_truss_2d",
        route: "/api/v1/fem/thermal-truss-2d/jobs",
    },
    DirectFemRoute {
        action: "solve_thermal_truss_3d",
        route: "/api/v1/fem/thermal-truss-3d/jobs",
    },
    DirectFemRoute {
        action: "solve_beam_1d",
        route: "/api/v1/fem/beam-1d/jobs",
    },
    DirectFemRoute {
        action: "solve_thermal_plane_triangle_2d",
        route: "/api/v1/fem/thermal-plane-triangle-2d/jobs",
    },
    DirectFemRoute {
        action: "solve_thermal_plane_quad_2d",
        route: "/api/v1/fem/thermal-plane-quad-2d/jobs",
    },
    DirectFemRoute {
        action: "solve_thermal_beam_1d",
        route: "/api/v1/fem/thermal-beam-1d/jobs",
    },
    DirectFemRoute {
        action: "solve_thermal_frame_2d",
        route: "/api/v1/fem/thermal-frame-2d/jobs",
    },
    DirectFemRoute {
        action: "solve_thermal_frame_3d",
        route: "/api/v1/fem/thermal-frame-3d/jobs",
    },
    DirectFemRoute {
        action: "solve_torsion_1d",
        route: "/api/v1/fem/torsion-1d/jobs",
    },
    DirectFemRoute {
        action: "solve_spring_1d",
        route: "/api/v1/fem/spring-1d/jobs",
    },
    DirectFemRoute {
        action: "solve_spring_2d",
        route: "/api/v1/fem/spring-2d/jobs",
    },
    DirectFemRoute {
        action: "solve_spring_3d",
        route: "/api/v1/fem/spring-3d/jobs",
    },
    DirectFemRoute {
        action: "solve_truss_2d",
        route: "/api/v1/fem/truss-2d/jobs",
    },
    DirectFemRoute {
        action: "solve_truss_3d",
        route: "/api/v1/fem/truss-3d/jobs",
    },
    DirectFemRoute {
        action: "solve_plane_triangle_2d",
        route: "/api/v1/fem/plane-triangle-2d/jobs",
    },
    DirectFemRoute {
        action: "solve_plane_quad_2d",
        route: "/api/v1/fem/plane-quad-2d/jobs",
    },
    DirectFemRoute {
        action: "solve_frame_2d",
        route: "/api/v1/fem/frame-2d/jobs",
    },
    DirectFemRoute {
        action: "solve_frame_3d",
        route: "/api/v1/fem/frame-3d/jobs",
    },
];

pub fn direct_fem_submit_route(action: &str) -> Option<&'static str> {
    DIRECT_FEM_ROUTES
        .iter()
        .find(|entry| entry.action == action)
        .map(|entry| entry.route)
}

pub fn all_direct_fem_routes() -> &'static [DirectFemRoute] {
    DIRECT_FEM_ROUTES
}

pub fn direct_fem_capability_manifest() -> Vec<DirectFemCapability> {
    DIRECT_FEM_ROUTES
        .iter()
        .filter_map(|route| {
            find_action_contract(route.action)
                .map(|contract| DirectFemCapability::from_route_and_contract(*route, contract))
        })
        .collect()
}

impl DirectFemCapability {
    fn from_route_and_contract(route: DirectFemRoute, contract: &HeadlessActionContract) -> Self {
        Self {
            action: route.action.to_string(),
            route: route.route.to_string(),
            engine: contract.engine,
            risk: contract.risk,
            required_payload_keys: contract
                .required_payload_keys
                .iter()
                .map(|key| (*key).to_string())
                .collect(),
            output_keys: contract
                .output_keys
                .iter()
                .map(|key| (*key).to_string())
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{all_direct_fem_routes, direct_fem_capability_manifest, direct_fem_submit_route};
    use crate::{HeadlessEngine, HeadlessRisk};
    use std::collections::BTreeSet;

    #[test]
    fn maps_core_fem_actions_to_routes() {
        assert_eq!(
            direct_fem_submit_route("solve_plane_quad_2d"),
            Some("/api/v1/fem/plane-quad-2d/jobs")
        );
        assert_eq!(
            direct_fem_submit_route("solve_heat_plane_quad_2d"),
            Some("/api/v1/fem/heat-plane-quad-2d/jobs")
        );
        assert_eq!(
            direct_fem_submit_route("solve_thermal_plane_quad_2d"),
            Some("/api/v1/fem/thermal-plane-quad-2d/jobs")
        );
        assert_eq!(
            direct_fem_submit_route("solve_electrostatic_plane_quad_2d"),
            Some("/api/v1/fem/electrostatic-plane-quad-2d/jobs")
        );
    }

    #[test]
    fn maps_extended_fem_actions_to_routes() {
        assert_eq!(
            direct_fem_submit_route("solve_bar_1d"),
            Some("/api/v1/fem/axial-bar/jobs")
        );
        assert_eq!(
            direct_fem_submit_route("solve_truss_3d"),
            Some("/api/v1/fem/truss-3d/jobs")
        );
        assert_eq!(
            direct_fem_submit_route("solve_thermal_frame_3d"),
            Some("/api/v1/fem/thermal-frame-3d/jobs")
        );
        assert_eq!(direct_fem_submit_route("solve_unknown"), None);
    }

    #[test]
    fn exposes_discoverable_unique_direct_fem_routes() {
        let routes = all_direct_fem_routes();
        let actions = routes
            .iter()
            .map(|entry| entry.action)
            .collect::<BTreeSet<_>>();
        let paths = routes
            .iter()
            .map(|entry| entry.route)
            .collect::<BTreeSet<_>>();

        assert_eq!(routes.len(), 26);
        assert_eq!(actions.len(), routes.len(), "duplicate direct FEM actions");
        assert_eq!(paths.len(), routes.len(), "duplicate direct FEM routes");

        for entry in routes {
            assert_eq!(direct_fem_submit_route(entry.action), Some(entry.route));
            assert!(entry.action.starts_with("solve_"));
            assert!(entry.route.starts_with("/api/v1/fem/"));
            assert!(entry.route.ends_with("/jobs"));
        }
    }

    #[test]
    fn exposes_serializable_capability_manifest() {
        let manifest = direct_fem_capability_manifest();
        let actions = manifest
            .iter()
            .map(|entry| entry.action.as_str())
            .collect::<BTreeSet<_>>();

        assert_eq!(manifest.len(), all_direct_fem_routes().len());
        assert_eq!(actions.len(), manifest.len());
        assert!(actions.contains("solve_thermal_frame_3d"));

        for entry in &manifest {
            assert_eq!(entry.engine, HeadlessEngine::Service);
            assert_eq!(entry.risk, HeadlessRisk::Normal);
            assert_eq!(entry.required_payload_keys, vec!["model".to_string()]);
            assert_eq!(
                entry.output_keys,
                vec![
                    "job_id".to_string(),
                    "status".to_string(),
                    "progress".to_string(),
                    "job".to_string(),
                ]
            );
        }

        serde_json::to_value(&manifest).expect("manifest should serialize");
    }
}
