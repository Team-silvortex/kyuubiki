import { createMaterialDefinition } from "@/lib/materials";
import type {
  DirectMeshSelectionMode,
  ElectrostaticPlaneQuad2dJobInput,
  ElectrostaticPlaneTriangle2dJobInput,
  Frame2dJobInput,
  HeatBar1dJobInput,
  HeatPlaneQuad2dJobInput,
  HeatPlaneTriangle2dJobInput,
  Spring1dJobInput,
  Spring2dJobInput,
  Spring3dJobInput,
  ThermalBar1dJobInput,
  ThermalBeam1dJobInput,
  ThermalFrame2dJobInput,
  ThermalPlaneQuad2dJobInput,
  ThermalPlaneTriangle2dJobInput,
  ThermalTruss2dJobInput,
  ThermalTruss3dJobInput,
  Truss2dJobInput,
  Truss3dJobInput,
  PlaneQuad2dJobInput,
  PlaneTriangle2dJobInput,
  Beam1dJobInput,
  Torsion1dJobInput,
} from "@/lib/api";
import type { ParametricPanelConfig, ParametricTrussConfig } from "@/lib/models";

export type AxialFormState = {
  length: number;
  area: number;
  elements: number;
  tipForce: number;
  material: string;
  youngsModulusGpa: number;
};

export type DisplayTrussNode = {
  index: number;
  id: string;
  x: number;
  y: number;
  ux: number;
  uy: number;
  fix_x: boolean;
  fix_y: boolean;
  load_x: number;
  load_y: number;
};

export type DisplayTrussElement = {
  index: number;
  id: string;
  node_i: number;
  node_j: number;
  length: number;
  strain: number;
  stress: number;
  axial_force: number;
  axial_stress?: number;
  max_bending_stress?: number;
  max_combined_stress?: number;
  axial_force_i?: number;
  shear_force_i?: number;
  moment_i?: number;
  axial_force_j?: number;
  shear_force_j?: number;
  moment_j?: number;
  average_temperature_delta?: number;
  temperature_gradient_y?: number;
  thermal_curvature?: number;
  material_id?: string;
};

export type DisplayTruss3dNode = {
  index: number;
  id: string;
  x: number;
  y: number;
  z: number;
  ux: number;
  uy: number;
  uz: number;
  temperature_delta?: number;
};

export type DisplayTruss3dElement = {
  index: number;
  id: string;
  node_i: number;
  node_j: number;
  length: number;
  strain: number;
  stress: number;
  axial_force: number;
  average_temperature_delta?: number;
  thermal_strain?: number;
  mechanical_strain?: number;
  total_strain?: number;
  material_id?: string;
};

export type SelectionKind = "node" | "element";

export type TrussSuggestion =
  | { id: string; kind: "fix_support"; axis: "x" | "y"; nodeIndex: number; label: string }
  | { id: string; kind: "connect_nearest"; nodeIndex: number; label: string };

export type TrussDiagnostics = {
  blockingMessages: string[];
  nodeIssues: Record<number, string[]>;
  suggestions: TrussSuggestion[];
};

export type StabilitySummary = {
  score: number;
  tone: "good" | "watch" | "risk";
  hotspotNodes: number[];
};

export type PlaneResultField =
  | "von_mises"
  | "principal_stress_1"
  | "max_in_plane_shear"
  | "average_potential"
  | "potential_gradient_x"
  | "potential_gradient_y"
  | "electric_field_x"
  | "electric_field_y"
  | "electric_field_magnitude"
  | "electric_flux_density_x"
  | "electric_flux_density_y"
  | "electric_flux_density_magnitude"
  | "average_temperature"
  | "average_temperature_delta"
  | "temperature_gradient_x"
  | "temperature_gradient_y"
  | "heat_flux_x"
  | "heat_flux_y"
  | "heat_flux_magnitude"
  | "thermal_strain"
  | "mechanical_strain";

export type PlaneNodeDisplay = {
  index: number;
  id: string;
  x: number;
  y: number;
  ux: number;
  uy: number;
  potential?: number;
  charge_density?: number;
  fix_potential?: boolean;
  fix_x: boolean;
  fix_y: boolean;
  load_x: number;
  load_y: number;
};

export type PlaneElementDisplay = {
  index: number;
  id: string;
  node_i: number;
  node_j: number;
  node_k: number;
  node_l?: number;
  von_mises?: number;
  principal_stress_1?: number;
  max_in_plane_shear?: number;
  average_temperature?: number;
  average_temperature_delta?: number;
  temperature_gradient_x?: number;
  temperature_gradient_y?: number;
  heat_flux_x?: number;
  heat_flux_y?: number;
  heat_flux_magnitude?: number;
  average_potential?: number;
  potential_gradient_x?: number;
  potential_gradient_y?: number;
  electric_field_x?: number;
  electric_field_y?: number;
  electric_field_magnitude?: number;
  electric_flux_density_x?: number;
  electric_flux_density_y?: number;
  electric_flux_density_magnitude?: number;
  thermal_strain?: number;
  mechanical_strain_x?: number;
  mechanical_strain_y?: number;
  material_id?: string;
};

export type DirectMeshExecutionState = {
  endpoint: string;
  strategy: DirectMeshSelectionMode;
  at: string;
};

export const defaultAxial: AxialFormState = {
  length: 1.2,
  area: 0.01,
  elements: 6,
  tipForce: 1800,
  material: "210",
  youngsModulusGpa: 210,
};

export const defaultTruss: Truss2dJobInput = {
  materials: [createMaterialDefinition("70", 1, { id: "mat-1" })],
  nodes: [
    { id: "n0", x: 0, y: 0, fix_x: true, fix_y: true, load_x: 0, load_y: 0 },
    { id: "n1", x: 1, y: 0, fix_x: false, fix_y: true, load_x: 0, load_y: 0 },
    { id: "n2", x: 0.5, y: 0.8, fix_x: false, fix_y: false, load_x: 0, load_y: -1000 },
  ],
  elements: [
    { id: "e0", node_i: 0, node_j: 2, area: 0.01, youngs_modulus: 70e9, material_id: "mat-1" },
    { id: "e1", node_i: 1, node_j: 2, area: 0.01, youngs_modulus: 70e9, material_id: "mat-1" },
    { id: "e2", node_i: 0, node_j: 1, area: 0.01, youngs_modulus: 70e9, material_id: "mat-1" },
  ],
};

export const defaultParametric: ParametricTrussConfig = {
  bays: 4,
  span: 12,
  height: 3,
  area: 0.01,
  youngsModulusGpa: 70,
  loadY: -1200,
};

export const defaultPanelParametric: ParametricPanelConfig = {
  width: 3.2,
  height: 1.8,
  divisionsX: 4,
  divisionsY: 3,
  thickness: 0.02,
  youngsModulusGpa: 70,
  poissonRatio: 0.33,
  loadY: -1200,
};

export const defaultPlaneTriangle: PlaneTriangle2dJobInput = {
  materials: [createMaterialDefinition("70", 1, { id: "mat-1", poisson_ratio: 0.33 })],
  nodes: [
    { id: "n0", x: 0, y: 0, fix_x: true, fix_y: true, load_x: 0, load_y: 0 },
    { id: "n1", x: 1, y: 0, fix_x: false, fix_y: true, load_x: 0, load_y: 0 },
    { id: "n2", x: 1, y: 1, fix_x: false, fix_y: false, load_x: 0, load_y: -800 },
    { id: "n3", x: 0, y: 1, fix_x: true, fix_y: false, load_x: 0, load_y: -800 },
  ],
  elements: [
    { id: "p0", node_i: 0, node_j: 1, node_k: 2, thickness: 0.02, youngs_modulus: 70e9, poisson_ratio: 0.33, material_id: "mat-1" },
    { id: "p1", node_i: 0, node_j: 2, node_k: 3, thickness: 0.02, youngs_modulus: 70e9, poisson_ratio: 0.33, material_id: "mat-1" },
  ],
};

export const defaultPlaneQuad: PlaneQuad2dJobInput = {
  materials: [createMaterialDefinition("70", 1, { id: "mat-1", poisson_ratio: 0.33 })],
  nodes: [
    { id: "n0", x: 0, y: 0, fix_x: true, fix_y: true, load_x: 0, load_y: 0 },
    { id: "n1", x: 1, y: 0, fix_x: false, fix_y: true, load_x: 0, load_y: 0 },
    { id: "n2", x: 1, y: 1, fix_x: false, fix_y: false, load_x: 0, load_y: -800 },
    { id: "n3", x: 0, y: 1, fix_x: true, fix_y: false, load_x: 0, load_y: -800 },
  ],
  elements: [
    {
      id: "q0",
      node_i: 0,
      node_j: 1,
      node_k: 2,
      node_l: 3,
      thickness: 0.02,
      youngs_modulus: 70e9,
      poisson_ratio: 0.33,
      material_id: "mat-1",
    },
  ],
};

export const defaultThermalPlaneTriangle: ThermalPlaneTriangle2dJobInput = {
  materials: [createMaterialDefinition("70", 1, { id: "mat-1", poisson_ratio: 0.33 })],
  nodes: [
    { id: "n0", x: 0, y: 0, fix_x: true, fix_y: true, load_x: 0, load_y: 0, temperature_delta: 40 },
    { id: "n1", x: 1, y: 0, fix_x: true, fix_y: true, load_x: 0, load_y: 0, temperature_delta: 40 },
    { id: "n2", x: 1, y: 1, fix_x: true, fix_y: true, load_x: 0, load_y: 0, temperature_delta: 40 },
    { id: "n3", x: 0, y: 1, fix_x: true, fix_y: true, load_x: 0, load_y: 0, temperature_delta: 40 },
  ],
  elements: [
    { id: "tp0", node_i: 0, node_j: 1, node_k: 2, thickness: 0.02, youngs_modulus: 70e9, poisson_ratio: 0.33, thermal_expansion: 12e-6, material_id: "mat-1" },
    { id: "tp1", node_i: 0, node_j: 2, node_k: 3, thickness: 0.02, youngs_modulus: 70e9, poisson_ratio: 0.33, thermal_expansion: 12e-6, material_id: "mat-1" },
  ],
};

export const defaultThermalPlaneQuad: ThermalPlaneQuad2dJobInput = {
  materials: [createMaterialDefinition("70", 1, { id: "mat-1", poisson_ratio: 0.33 })],
  nodes: [
    { id: "n0", x: 0, y: 0, fix_x: true, fix_y: true, load_x: 0, load_y: 0, temperature_delta: 30 },
    { id: "n1", x: 1, y: 0, fix_x: true, fix_y: true, load_x: 0, load_y: 0, temperature_delta: 30 },
    { id: "n2", x: 1, y: 1, fix_x: true, fix_y: true, load_x: 0, load_y: 0, temperature_delta: 30 },
    { id: "n3", x: 0, y: 1, fix_x: true, fix_y: true, load_x: 0, load_y: 0, temperature_delta: 30 },
  ],
  elements: [
    { id: "tq0", node_i: 0, node_j: 1, node_k: 2, node_l: 3, thickness: 0.02, youngs_modulus: 70e9, poisson_ratio: 0.33, thermal_expansion: 11e-6, material_id: "mat-1" },
  ],
};

export const defaultHeatPlaneTriangle: HeatPlaneTriangle2dJobInput = {
  nodes: [
    { id: "h0", x: 0, y: 0, fix_temperature: true, temperature: 100, heat_load: 0 },
    { id: "h1", x: 1, y: 0, fix_temperature: false, temperature: 0, heat_load: 0 },
    { id: "h2", x: 1, y: 1, fix_temperature: true, temperature: 20, heat_load: 0 },
    { id: "h3", x: 0, y: 1, fix_temperature: true, temperature: 20, heat_load: 0 },
  ],
  elements: [
    { id: "hp0", node_i: 0, node_j: 1, node_k: 2, thickness: 0.02, conductivity: 45 },
    { id: "hp1", node_i: 0, node_j: 2, node_k: 3, thickness: 0.02, conductivity: 45 },
  ],
};

export const defaultHeatPlaneQuad: HeatPlaneQuad2dJobInput = {
  nodes: [
    { id: "h0", x: 0, y: 0, fix_temperature: true, temperature: 100, heat_load: 0 },
    { id: "h1", x: 1, y: 0, fix_temperature: false, temperature: 0, heat_load: 0 },
    { id: "h2", x: 1, y: 1, fix_temperature: true, temperature: 20, heat_load: 0 },
    { id: "h3", x: 0, y: 1, fix_temperature: true, temperature: 20, heat_load: 0 },
  ],
  elements: [
    { id: "hq0", node_i: 0, node_j: 1, node_k: 2, node_l: 3, thickness: 0.02, conductivity: 45 },
  ],
};

export const defaultElectrostaticPlaneTriangle: ElectrostaticPlaneTriangle2dJobInput = {
  nodes: [
    { id: "et0", x: 0, y: 0, fix_potential: true, potential: 10, charge_density: 0 },
    { id: "et1", x: 1, y: 0, fix_potential: true, potential: 0, charge_density: 0 },
    { id: "et2", x: 0, y: 1, fix_potential: false, potential: 0, charge_density: 0 },
  ],
  elements: [{ id: "ept0", node_i: 0, node_j: 1, node_k: 2, thickness: 0.05, permittivity: 2 }],
};

export const defaultElectrostaticPlaneQuad: ElectrostaticPlaneQuad2dJobInput = {
  nodes: [
    { id: "ep0", x: 0, y: 0, fix_potential: true, potential: 10, charge_density: 0 },
    { id: "ep1", x: 1, y: 0, fix_potential: true, potential: 0, charge_density: 0 },
    { id: "ep2", x: 1, y: 1, fix_potential: true, potential: 0, charge_density: 0 },
    { id: "ep3", x: 0, y: 1, fix_potential: true, potential: 10, charge_density: 0 },
  ],
  elements: [{ id: "epq0", node_i: 0, node_j: 1, node_k: 2, node_l: 3, thickness: 0.05, permittivity: 2 }],
};

export const defaultTruss3d: Truss3dJobInput = {
  materials: [createMaterialDefinition("70", 1, { id: "mat-1" })],
  nodes: [
    { id: "b0", x: 0, y: 0, z: 0, fix_x: true, fix_y: true, fix_z: true, load_x: 0, load_y: 0, load_z: 0 },
    { id: "b1", x: 1.2, y: 0, z: 0, fix_x: true, fix_y: true, fix_z: true, load_x: 0, load_y: 0, load_z: 0 },
    { id: "b2", x: 0.1, y: 1.0, z: 0, fix_x: true, fix_y: true, fix_z: true, load_x: 0, load_y: 0, load_z: 0 },
    { id: "top", x: 0.35, y: 0.3, z: 1.0, fix_x: false, fix_y: false, fix_z: false, load_x: 0, load_y: 0, load_z: -1500 },
  ],
  elements: [
    { id: "e0", node_i: 0, node_j: 1, area: 0.01, youngs_modulus: 70e9, material_id: "mat-1" },
    { id: "e1", node_i: 1, node_j: 2, area: 0.01, youngs_modulus: 70e9, material_id: "mat-1" },
    { id: "e2", node_i: 2, node_j: 0, area: 0.01, youngs_modulus: 70e9, material_id: "mat-1" },
    { id: "e3", node_i: 0, node_j: 3, area: 0.01, youngs_modulus: 70e9, material_id: "mat-1" },
    { id: "e4", node_i: 1, node_j: 3, area: 0.01, youngs_modulus: 70e9, material_id: "mat-1" },
    { id: "e5", node_i: 2, node_j: 3, area: 0.01, youngs_modulus: 70e9, material_id: "mat-1" },
  ],
};

export const defaultBeam1d: Beam1dJobInput = {
  materials: [createMaterialDefinition("210", 1, { id: "mat-1" })],
  nodes: [
    { id: "b0", x: 0, fix_y: true, fix_rz: true, load_y: 0, moment_z: 0 },
    { id: "b1", x: 2.4, fix_y: false, fix_rz: false, load_y: -12000, moment_z: 0 },
  ],
  elements: [
    {
      id: "m0",
      node_i: 0,
      node_j: 1,
      youngs_modulus: 210e9,
      moment_of_inertia: 1.2e-4,
      section_modulus: 1.1e-3,
      distributed_load_y: 0,
      material_id: "mat-1",
    },
  ],
};

export const defaultThermalBeam1d: ThermalBeam1dJobInput = {
  materials: [createMaterialDefinition("210", 1, { id: "mat-1" })],
  nodes: [
    { id: "tb0", x: 0, fix_y: true, fix_rz: true, load_y: 0, moment_z: 0 },
    { id: "tb1", x: 2.4, fix_y: false, fix_rz: false, load_y: 0, moment_z: 0 },
  ],
  elements: [
    {
      id: "tm0",
      node_i: 0,
      node_j: 1,
      youngs_modulus: 210e9,
      moment_of_inertia: 1.2e-4,
      section_modulus: 1.1e-3,
      thermal_expansion: 1.2e-5,
      section_depth: 0.3,
      distributed_load_y: 0,
      temperature_gradient_y: 45,
      material_id: "mat-1",
    },
  ],
};

export const defaultTorsion1d: Torsion1dJobInput = {
  nodes: [
    { id: "t0", x: 0, fix_rz: true, torque_z: 0 },
    { id: "t1", x: 1.5, fix_rz: false, torque_z: 2500 },
  ],
  elements: [
    {
      id: "s0",
      node_i: 0,
      node_j: 1,
      shear_modulus: 79e9,
      polar_moment: 1.8e-6,
      section_modulus: 1.2e-4,
    },
  ],
};

export const defaultSpring1d: Spring1dJobInput = {
  nodes: [
    { id: "s0", x: 0, fix_x: true, load_x: 0 },
    { id: "s1", x: 1.2, fix_x: false, load_x: 0 },
    { id: "s2", x: 2.4, fix_x: false, load_x: 1200 },
  ],
  elements: [
    { id: "k0", node_i: 0, node_j: 1, stiffness: 35000 },
    { id: "k1", node_i: 1, node_j: 2, stiffness: 20000 },
  ],
};

export const defaultThermalBar1d: ThermalBar1dJobInput = {
  nodes: [
    { id: "t0", x: 0, fix_x: true, load_x: 0, temperature_delta: 40 },
    { id: "t1", x: 1.5, fix_x: true, load_x: 0, temperature_delta: 40 },
  ],
  elements: [
    { id: "tb0", node_i: 0, node_j: 1, area: 0.01, youngs_modulus: 210e9, thermal_expansion: 1.2e-5 },
  ],
};

export const defaultHeatBar1d: HeatBar1dJobInput = {
  nodes: [
    { id: "h0", x: 0, fix_temperature: true, temperature: 100, heat_load: 0 },
    { id: "h1", x: 1, fix_temperature: false, temperature: 0, heat_load: 0 },
    { id: "h2", x: 2, fix_temperature: true, temperature: 20, heat_load: 0 },
  ],
  elements: [
    { id: "he0", node_i: 0, node_j: 1, area: 0.01, conductivity: 45 },
    { id: "he1", node_i: 1, node_j: 2, area: 0.01, conductivity: 45 },
  ],
};

export const defaultThermalTruss2d: ThermalTruss2dJobInput = {
  materials: [createMaterialDefinition("70", 1, { id: "mat-1" })],
  nodes: [
    { id: "tt0", x: 0, y: 0, fix_x: true, fix_y: true, load_x: 0, load_y: 0, temperature_delta: 40 },
    { id: "tt1", x: 1, y: 0, fix_x: false, fix_y: true, load_x: 0, load_y: 0, temperature_delta: 40 },
    { id: "tt2", x: 0.5, y: 0.8, fix_x: false, fix_y: false, load_x: 0, load_y: -400, temperature_delta: 25 },
  ],
  elements: [
    { id: "tte0", node_i: 0, node_j: 2, area: 0.01, youngs_modulus: 70e9, thermal_expansion: 1.2e-5, material_id: "mat-1" },
    { id: "tte1", node_i: 1, node_j: 2, area: 0.01, youngs_modulus: 70e9, thermal_expansion: 1.2e-5, material_id: "mat-1" },
    { id: "tte2", node_i: 0, node_j: 1, area: 0.01, youngs_modulus: 70e9, thermal_expansion: 1.2e-5, material_id: "mat-1" },
  ],
};

export const defaultThermalTruss3d: ThermalTruss3dJobInput = {
  materials: [createMaterialDefinition("70", 1, { id: "mat-1" })],
  nodes: [
    { id: "tb0", x: 0, y: 0, z: 0, fix_x: true, fix_y: true, fix_z: true, load_x: 0, load_y: 0, load_z: 0, temperature_delta: 35 },
    { id: "tb1", x: 1.2, y: 0, z: 0, fix_x: true, fix_y: true, fix_z: true, load_x: 0, load_y: 0, load_z: 0, temperature_delta: 35 },
    { id: "tb2", x: 0.1, y: 1.0, z: 0, fix_x: true, fix_y: true, fix_z: true, load_x: 0, load_y: 0, load_z: 0, temperature_delta: 35 },
    { id: "tb3", x: 0.35, y: 0.3, z: 1.0, fix_x: false, fix_y: false, fix_z: false, load_x: 0, load_y: 0, load_z: -900, temperature_delta: 15 },
  ],
  elements: [
    { id: "tte0", node_i: 0, node_j: 1, area: 0.01, youngs_modulus: 70e9, thermal_expansion: 1.2e-5, material_id: "mat-1" },
    { id: "tte1", node_i: 1, node_j: 2, area: 0.01, youngs_modulus: 70e9, thermal_expansion: 1.2e-5, material_id: "mat-1" },
    { id: "tte2", node_i: 2, node_j: 0, area: 0.01, youngs_modulus: 70e9, thermal_expansion: 1.2e-5, material_id: "mat-1" },
    { id: "tte3", node_i: 0, node_j: 3, area: 0.01, youngs_modulus: 70e9, thermal_expansion: 1.2e-5, material_id: "mat-1" },
    { id: "tte4", node_i: 1, node_j: 3, area: 0.01, youngs_modulus: 70e9, thermal_expansion: 1.2e-5, material_id: "mat-1" },
    { id: "tte5", node_i: 2, node_j: 3, area: 0.01, youngs_modulus: 70e9, thermal_expansion: 1.2e-5, material_id: "mat-1" },
  ],
};

export const defaultSpring2d: Spring2dJobInput = {
  nodes: [
    { id: "s0", x: 0, y: 0, fix_x: true, fix_y: true, load_x: 0, load_y: 0 },
    { id: "s1", x: 1.2, y: 0, fix_x: false, fix_y: true, load_x: 0, load_y: 0 },
    { id: "s2", x: 1.2, y: 1.2, fix_x: false, fix_y: false, load_x: 1200, load_y: -600 },
    { id: "s3", x: 0, y: 1.2, fix_x: true, fix_y: false, load_x: 0, load_y: 0 },
  ],
  elements: [
    { id: "k0", node_i: 0, node_j: 1, stiffness: 28000 },
    { id: "k1", node_i: 1, node_j: 2, stiffness: 18000 },
    { id: "k2", node_i: 2, node_j: 3, stiffness: 22000 },
    { id: "k3", node_i: 3, node_j: 0, stiffness: 18000 },
    { id: "k4", node_i: 0, node_j: 2, stiffness: 12000 },
  ],
};

export const defaultSpring3d: Spring3dJobInput = {
  nodes: [
    { id: "s0", x: 0, y: 0, z: 0, fix_x: true, fix_y: true, fix_z: true, load_x: 0, load_y: 0, load_z: 0 },
    { id: "s1", x: 1.2, y: 0, z: 0, fix_x: true, fix_y: true, fix_z: true, load_x: 0, load_y: 0, load_z: 0 },
    { id: "s2", x: 0, y: 1.0, z: 0, fix_x: true, fix_y: true, fix_z: true, load_x: 0, load_y: 0, load_z: 0 },
    { id: "top", x: 0.45, y: 0.35, z: 1.1, fix_x: false, fix_y: false, fix_z: false, load_x: 250, load_y: 0, load_z: -1100 },
  ],
  elements: [
    { id: "k0", node_i: 0, node_j: 3, stiffness: 18000 },
    { id: "k1", node_i: 1, node_j: 3, stiffness: 22000 },
    { id: "k2", node_i: 2, node_j: 3, stiffness: 16000 },
    { id: "k3", node_i: 0, node_j: 1, stiffness: 9000 },
    { id: "k4", node_i: 1, node_j: 2, stiffness: 9000 },
    { id: "k5", node_i: 2, node_j: 0, stiffness: 9000 },
  ],
};

export const defaultFrame2d: Frame2dJobInput = {
  materials: [createMaterialDefinition("210", 1, { id: "mat-1" })],
  nodes: [
    { id: "f0", x: 0, y: 0, fix_x: true, fix_y: true, fix_rz: true, load_x: 0, load_y: 0, moment_z: 0 },
    { id: "f1", x: 0, y: 2.4, fix_x: false, fix_y: false, fix_rz: false, load_x: 0, load_y: 0, moment_z: 0 },
    { id: "f2", x: 3.2, y: 2.4, fix_x: false, fix_y: false, fix_rz: false, load_x: 0, load_y: -12000, moment_z: 0 },
    { id: "f3", x: 3.2, y: 0, fix_x: false, fix_y: true, fix_rz: false, load_x: 0, load_y: 0, moment_z: 0 },
  ],
  elements: [
    { id: "c0", node_i: 0, node_j: 1, area: 0.018, youngs_modulus: 210e9, moment_of_inertia: 1.8e-4, section_modulus: 1.6e-3, material_id: "mat-1" },
    { id: "b0", node_i: 1, node_j: 2, area: 0.018, youngs_modulus: 210e9, moment_of_inertia: 1.2e-4, section_modulus: 1.1e-3, material_id: "mat-1" },
    { id: "c1", node_i: 2, node_j: 3, area: 0.018, youngs_modulus: 210e9, moment_of_inertia: 1.8e-4, section_modulus: 1.6e-3, material_id: "mat-1" },
  ],
};

export const defaultThermalFrame2d: ThermalFrame2dJobInput = {
  materials: [createMaterialDefinition("210", 1, { id: "mat-1" })],
  nodes: [
    { id: "tf0", x: 0, y: 0, fix_x: true, fix_y: true, fix_rz: true, load_x: 0, load_y: 0, moment_z: 0, temperature_delta: 20 },
    { id: "tf1", x: 0, y: 2.4, fix_x: false, fix_y: false, fix_rz: false, load_x: 0, load_y: 0, moment_z: 0, temperature_delta: 40 },
    { id: "tf2", x: 3.2, y: 2.4, fix_x: false, fix_y: false, fix_rz: false, load_x: 0, load_y: 0, moment_z: 0, temperature_delta: 40 },
    { id: "tf3", x: 3.2, y: 0, fix_x: true, fix_y: true, fix_rz: true, load_x: 0, load_y: 0, moment_z: 0, temperature_delta: 20 },
  ],
  elements: [
    { id: "tc0", node_i: 0, node_j: 1, area: 0.018, youngs_modulus: 210e9, moment_of_inertia: 1.8e-4, section_modulus: 1.6e-3, thermal_expansion: 1.2e-5, section_depth: 0.32, temperature_gradient_y: 0, material_id: "mat-1" },
    { id: "tb0", node_i: 1, node_j: 2, area: 0.018, youngs_modulus: 210e9, moment_of_inertia: 1.2e-4, section_modulus: 1.1e-3, thermal_expansion: 1.2e-5, section_depth: 0.28, temperature_gradient_y: 30, material_id: "mat-1" },
    { id: "tc1", node_i: 2, node_j: 3, area: 0.018, youngs_modulus: 210e9, moment_of_inertia: 1.8e-4, section_modulus: 1.6e-3, thermal_expansion: 1.2e-5, section_depth: 0.32, temperature_gradient_y: 0, material_id: "mat-1" },
  ],
};
