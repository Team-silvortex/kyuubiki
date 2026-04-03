export type SampleEntry = {
  id: string;
  name: string;
  kind: "axial_bar_1d" | "truss_2d" | "truss_3d" | "plane_triangle_2d";
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
];
