export type SampleEntry = {
  id: string;
  name: string;
  kind: "axial_bar_1d" | "spring_1d" | "beam_1d" | "truss_2d" | "truss_3d" | "plane_triangle_2d" | "plane_quad_2d" | "frame_2d";
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
    id: "uniform-load-beam-1d",
    name: "Uniform Load Beam 1D",
    kind: "beam_1d",
    href: "/models/uniform-load-beam-1d.json",
    summary: "Cantilever beam driven by element-level distributed load for equivalent nodal load checks.",
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
];
