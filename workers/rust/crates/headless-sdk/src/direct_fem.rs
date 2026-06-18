pub fn direct_fem_submit_route(action: &str) -> Option<&'static str> {
    match action {
        "solve_bar_1d" => Some("/api/v1/fem/axial-bar/jobs"),
        "solve_thermal_bar_1d" => Some("/api/v1/fem/thermal-bar-1d/jobs"),
        "solve_heat_bar_1d" => Some("/api/v1/fem/heat-bar-1d/jobs"),
        "solve_electrostatic_bar_1d" => Some("/api/v1/fem/electrostatic-bar-1d/jobs"),
        "solve_electrostatic_plane_triangle_2d" => {
            Some("/api/v1/fem/electrostatic-plane-triangle-2d/jobs")
        }
        "solve_electrostatic_plane_quad_2d" => {
            Some("/api/v1/fem/electrostatic-plane-quad-2d/jobs")
        }
        "solve_heat_plane_triangle_2d" => Some("/api/v1/fem/heat-plane-triangle-2d/jobs"),
        "solve_heat_plane_quad_2d" => Some("/api/v1/fem/heat-plane-quad-2d/jobs"),
        "solve_thermal_truss_2d" => Some("/api/v1/fem/thermal-truss-2d/jobs"),
        "solve_thermal_truss_3d" => Some("/api/v1/fem/thermal-truss-3d/jobs"),
        "solve_beam_1d" => Some("/api/v1/fem/beam-1d/jobs"),
        "solve_thermal_plane_triangle_2d" => Some("/api/v1/fem/thermal-plane-triangle-2d/jobs"),
        "solve_thermal_plane_quad_2d" => Some("/api/v1/fem/thermal-plane-quad-2d/jobs"),
        "solve_thermal_beam_1d" => Some("/api/v1/fem/thermal-beam-1d/jobs"),
        "solve_thermal_frame_2d" => Some("/api/v1/fem/thermal-frame-2d/jobs"),
        "solve_thermal_frame_3d" => Some("/api/v1/fem/thermal-frame-3d/jobs"),
        "solve_torsion_1d" => Some("/api/v1/fem/torsion-1d/jobs"),
        "solve_spring_1d" => Some("/api/v1/fem/spring-1d/jobs"),
        "solve_spring_2d" => Some("/api/v1/fem/spring-2d/jobs"),
        "solve_spring_3d" => Some("/api/v1/fem/spring-3d/jobs"),
        "solve_truss_2d" => Some("/api/v1/fem/truss-2d/jobs"),
        "solve_truss_3d" => Some("/api/v1/fem/truss-3d/jobs"),
        "solve_plane_triangle_2d" => Some("/api/v1/fem/plane-triangle-2d/jobs"),
        "solve_plane_quad_2d" => Some("/api/v1/fem/plane-quad-2d/jobs"),
        "solve_frame_2d" => Some("/api/v1/fem/frame-2d/jobs"),
        "solve_frame_3d" => Some("/api/v1/fem/frame-3d/jobs"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::direct_fem_submit_route;

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
}
