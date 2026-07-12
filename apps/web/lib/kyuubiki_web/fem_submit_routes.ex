defmodule KyuubikiWeb.FemSubmitRoutes do
  @moduledoc false

  @routes [
    {"/api/v1/fem/axial-bar/jobs", :submit_axial_bar},
    {"/api/v1/fem/acoustic-bar-1d/jobs", :submit_acoustic_bar_1d},
    {"/api/v1/fem/thermal-bar-1d/jobs", :submit_thermal_bar_1d},
    {"/api/v1/fem/heat-bar-1d/jobs", :submit_heat_bar_1d},
    {"/api/v1/fem/transient-heat-bar-1d/jobs", :submit_transient_heat_bar_1d},
    {"/api/v1/fem/electrostatic-bar-1d/jobs", :submit_electrostatic_bar_1d},
    {"/api/v1/fem/magnetostatic-bar-1d/jobs", :submit_magnetostatic_bar_1d},
    {"/api/v1/fem/electrostatic-plane-triangle-2d/jobs", :submit_electrostatic_plane_triangle_2d},
    {"/api/v1/fem/electrostatic-plane-quad-2d/jobs", :submit_electrostatic_plane_quad_2d},
    {"/api/v1/fem/magnetostatic-plane-triangle-2d/jobs", :submit_magnetostatic_plane_triangle_2d},
    {"/api/v1/fem/magnetostatic-plane-quad-2d/jobs", :submit_magnetostatic_plane_quad_2d},
    {"/api/v1/fem/heat-plane-triangle-2d/jobs", :submit_heat_plane_triangle_2d},
    {"/api/v1/fem/heat-plane-quad-2d/jobs", :submit_heat_plane_quad_2d},
    {"/api/v1/fem/stokes-flow-plane-quad-2d/jobs", :submit_stokes_flow_plane_quad_2d},
    {"/api/v1/fem/stokes-flow-plane-triangle-2d/jobs",
     :submit_stokes_flow_plane_triangle_2d},
    {"/api/v1/fem/thermal-truss-2d/jobs", :submit_thermal_truss_2d},
    {"/api/v1/fem/thermal-truss-3d/jobs", :submit_thermal_truss_3d},
    {"/api/v1/fem/beam-1d/jobs", :submit_beam_1d},
    {"/api/v1/fem/thermal-plane-triangle-2d/jobs", :submit_thermal_plane_triangle_2d},
    {"/api/v1/fem/thermal-plane-quad-2d/jobs", :submit_thermal_plane_quad_2d},
    {"/api/v1/fem/thermal-beam-1d/jobs", :submit_thermal_beam_1d},
    {"/api/v1/fem/thermal-frame-2d/jobs", :submit_thermal_frame_2d},
    {"/api/v1/fem/thermal-frame-3d/jobs", :submit_thermal_frame_3d},
    {"/api/v1/fem/torsion-1d/jobs", :submit_torsion_1d},
    {"/api/v1/fem/spring-1d/jobs", :submit_spring_1d},
    {"/api/v1/fem/transient-spring-1d/jobs", :submit_transient_spring_1d},
    {"/api/v1/fem/harmonic-spring-1d/jobs", :submit_harmonic_spring_1d},
    {"/api/v1/fem/nonlinear-spring-1d/jobs", :submit_nonlinear_spring_1d},
    {"/api/v1/fem/contact-gap-1d/jobs", :submit_contact_gap_1d},
    {"/api/v1/fem/spring-2d/jobs", :submit_spring_2d},
    {"/api/v1/fem/spring-3d/jobs", :submit_spring_3d},
    {"/api/v1/fem/truss-2d/jobs", :submit_truss_2d},
    {"/api/v1/fem/truss-3d/jobs", :submit_truss_3d},
    {"/api/v1/fem/plane-triangle-2d/jobs", :submit_plane_triangle_2d},
    {"/api/v1/fem/plane-quad-2d/jobs", :submit_plane_quad_2d},
    {"/api/v1/fem/frame-2d/jobs", :submit_frame_2d},
    {"/api/v1/fem/modal-frame-2d/jobs", :submit_modal_frame_2d},
    {"/api/v1/fem/frame-3d/jobs", :submit_frame_3d},
    {"/api/v1/fem/solid-tetra-3d/jobs", :submit_solid_tetra_3d},
    {"/api/v1/fem/modal-frame-3d/jobs", :submit_modal_frame_3d}
  ]

  def routes, do: @routes
end
