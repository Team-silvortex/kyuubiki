export type SampleEntry = {
  id: string;
  name: string;
  kind:
    | "axial_bar_1d"
    | "heat_bar_1d"
    | "electrostatic_plane_triangle_2d"
    | "electrostatic_plane_quad_2d"
    | "heat_plane_triangle_2d"
    | "heat_plane_quad_2d"
    | "thermal_bar_1d"
    | "thermal_beam_1d"
    | "thermal_frame_2d"
    | "thermal_truss_2d"
    | "thermal_truss_3d"
    | "thermal_plane_triangle_2d"
    | "thermal_plane_quad_2d"
    | "spring_1d"
    | "spring_2d"
    | "spring_3d"
    | "beam_1d"
    | "torsion_1d"
    | "truss_2d"
    | "truss_3d"
    | "plane_triangle_2d"
    | "plane_quad_2d"
    | "frame_2d"
    | "buckling_beam_1d"
    | "buckling_frame_2d";
  href: string;
  summary: string;
};

export const SAMPLE_LIBRARY: SampleEntry[] = [
  {
    id: "axial-steel-bar",
    name: "Axial Steel Bar",
    kind: "axial_bar_1d",
    href: "/models/axial-steel-bar.json",
    summary: "Baseline 1D tensile bar for quick stiffness checks.",
  },
  {
    id: "heat-bar-1d",
    name: "Heat Bar 1D",
    kind: "heat_bar_1d",
    href: "/models/heat-bar-1d.json",
    summary: "Steady 1D conduction bar with prescribed end temperatures. Thermal intent: conduction field, with a clean bridge into Thermal Bar 1D.",
  },
  {
    id: "heat-bar-source-1d",
    name: "Heat Bar Source 1D",
    kind: "heat_bar_1d",
    href: "/models/heat-bar-source-1d.json",
    summary: "1D conduction bar with prescribed end temperatures and nodal heat sources. Thermal intent: conduction plus heat-source field.",
  },
  {
    id: "electrostatic-plane-triangle-2d",
    name: "Electrostatic Plane Triangle 2D",
    kind: "electrostatic_plane_triangle_2d",
    href: "/models/electrostatic-plane-triangle-2d.json",
    summary: "Triangular electrostatic patch with fixed-potential boundary nodes and a charged free node for field-gradient checks.",
  },
  {
    id: "electrostatic-plane-quad-2d",
    name: "Electrostatic Plane Quad 2D",
    kind: "electrostatic_plane_quad_2d",
    href: "/models/electrostatic-plane-quad-2d.json",
    summary: "Single quad electrostatic plate with driven potential difference and dielectric thickness for electric-field and flux-density checks.",
  },
  {
    id: "heat-plane-triangle-2d",
    name: "Heat Plane Triangle 2D",
    kind: "heat_plane_triangle_2d",
    href: "/models/heat-plane-triangle-2d.json",
    summary: "Steady CST conduction patch with prescribed temperatures. Thermal intent: planar conduction field, with a bridge into Thermal Plane Triangle 2D.",
  },
  {
    id: "heat-strip-triangle-2d",
    name: "Heat Strip Triangle 2D",
    kind: "heat_plane_triangle_2d",
    href: "/models/heat-strip-triangle-2d.json",
    summary: "Triangulated heat strip with left-right prescribed temperatures. Thermal intent: directional conduction benchmark.",
  },
  {
    id: "heat-plane-quad-2d",
    name: "Heat Plane Quad 2D",
    kind: "heat_plane_quad_2d",
    href: "/models/heat-plane-quad-2d.json",
    summary: "Single quad conduction patch with prescribed temperatures. Thermal intent: planar conduction field, with a bridge into Thermal Plane Quad 2D.",
  },
  {
    id: "heat-strip-quad-2d",
    name: "Heat Strip Quad 2D",
    kind: "heat_plane_quad_2d",
    href: "/models/heat-strip-quad-2d.json",
    summary: "Two-quad conduction strip with prescribed end temperatures. Thermal intent: directional heat-flow benchmark.",
  },
  {
    id: "thermal-bar-1d",
    name: "Thermal Bar 1D",
    kind: "thermal_bar_1d",
    href: "/models/thermal-bar-1d.json",
    summary: "Restrained thermal-expansion bar. Thermal intent: nodal temperature rise driving axial stress and force response.",
  },
  {
    id: "thermal-beam-1d",
    name: "Thermal Beam 1D",
    kind: "thermal_beam_1d",
    href: "/models/thermal-beam-1d.json",
    summary: "Cantilever beam with through-depth temperature gradient. Thermal intent: member gradient driving thermal curvature and bending response.",
  },
  {
    id: "thermal-frame-2d",
    name: "Thermal Frame 2D",
    kind: "thermal_frame_2d",
    href: "/models/thermal-frame-2d.json",
    summary: "Portal frame with heated members. Thermal intent: nodal rise plus member gradients driving restrained frame response.",
  },
  {
    id: "thermal-truss-2d",
    name: "Thermal Truss 2D",
    kind: "thermal_truss_2d",
    href: "/models/thermal-truss-2d.json",
    summary: "Restrained 2D truss with temperature rise. Thermal intent: heated nodes driving thermal force paths.",
  },
  {
    id: "thermal-truss-3d",
    name: "Thermal Truss 3D",
    kind: "thermal_truss_3d",
    href: "/models/thermal-truss-3d.json",
    summary: "Spatial truss sample with heated nodes. Thermal intent: 3D thermal force-path checks under restrained supports.",
  },
  {
    id: "thermal-plane-triangle-2d",
    name: "Thermal Plane Triangle 2D",
    kind: "thermal_plane_triangle_2d",
    href: "/models/thermal-plane-triangle-2d.json",
    summary: "Restrained CST patch with nodal temperature rise. Thermal intent: thermoelastic plane stress response.",
  },
  {
    id: "thermal-plane-quad-2d",
    name: "Thermal Plane Quad 2D",
    kind: "thermal_plane_quad_2d",
    href: "/models/thermal-plane-quad-2d.json",
    summary: "Restrained quad patch with nodal temperature rise. Thermal intent: thermoelastic stress-field checks.",
  },
  {
    id: "cantilever-beam-1d",
    name: "Cantilever Beam 1D",
    kind: "beam_1d",
    href: "/models/cantilever-beam-1d.json",
    summary: "Euler-Bernoulli cantilever beam with tip load and bending response.",
  },
  {
    id: "spring-chain-1d",
    name: "Spring Chain 1D",
    kind: "spring_1d",
    href: "/models/spring-chain-1d.json",
    summary: "Two inline axial springs with a fixed anchor and tip load for quick extension checks.",
  },
  {
    id: "spring-grid-2d",
    name: "Spring Grid 2D",
    kind: "spring_2d",
    href: "/models/spring-grid-2d.json",
    summary: "Planar spring lattice with anchored supports and lateral load for quick force-distribution checks.",
  },
  {
    id: "spring-cage-3d",
    name: "Spring Cage 3D",
    kind: "spring_3d",
    href: "/models/spring-cage-3d.json",
    summary: "Compact 3D spring cage with anchored base nodes and a loaded top node for spatial extension checks.",
  },
  {
    id: "uniform-load-beam-1d",
    name: "Uniform Load Beam 1D",
    kind: "beam_1d",
    href: "/models/uniform-load-beam-1d.json",
    summary: "Cantilever beam driven by element-level distributed load for equivalent nodal load checks.",
  },
  {
    id: "torsion-shaft-1d",
    name: "Torsion Shaft 1D",
    kind: "torsion_1d",
    href: "/models/torsion-shaft-1d.json",
    summary: "Single-span shaft with end torque for quick torsional twist and stress checks.",
  },
  {
    id: "braced-truss-2d",
    name: "Braced Truss 2D",
    kind: "truss_2d",
    href: "/models/braced-truss-2d.json",
    summary: "Compact 2D truss with pinned supports and roof load.",
  },
  {
    id: "cantilever-plate-2d",
    name: "Cantilever Plate 2D",
    kind: "plane_triangle_2d",
    href: "/models/cantilever-plate-2d.json",
    summary: "Two CST triangles representing a small cantilever plate patch.",
  },
  {
    id: "space-frame-pyramid-3d",
    name: "Space Frame Pyramid",
    kind: "truss_3d",
    href: "/models/space-frame-pyramid-3d.json",
    summary: "Compact 3D truss pyramid for first-pass spatial stiffness checks.",
  },
  {
    id: "window-frame-truss-2d",
    name: "Window Frame Truss",
    kind: "truss_2d",
    href: "/models/window-frame-truss-2d.json",
    summary: "Rectangular braced panel for displacement pattern comparisons.",
  },
  {
    id: "aluminum-panel-2d",
    name: "Aluminum Panel",
    kind: "plane_triangle_2d",
    href: "/models/aluminum-panel-2d.json",
    summary: "Plane stress panel with symmetric bottom supports and edge load.",
  },
  {
    id: "quad-plate-patch-2d",
    name: "Quad Plate Patch 2D",
    kind: "plane_quad_2d",
    href: "/models/quad-plate-patch-2d.json",
    summary: "Single bilinear quad patch for quick plane-stress solver checks.",
  },
  {
    id: "portal-frame-2d",
    name: "Portal Frame 2D",
    kind: "frame_2d",
    href: "/models/portal-frame-2d.json",
    summary: "Compact 2D frame with bending and rotation response for first-pass beam-column checks.",
  },
  {
    id: "buckling-column-1d",
    name: "Pinned Buckling Column 1D",
    kind: "buckling_beam_1d",
    href: "/models/buckling-beam-1d.json",
    summary: "Pinned Euler column for linear eigenvalue buckling convergence and critical-load screening.",
  },
  {
    id: "buckling-frame-column-2d",
    name: "Preloaded Buckling Frame 2D",
    kind: "buckling_frame_2d",
    href: "/models/buckling-frame-2d.json",
    summary: "Pinned 2D frame column with static preload extraction and linear eigenvalue buckling modes.",
  },
];
