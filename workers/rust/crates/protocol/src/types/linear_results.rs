use crate::{
    SolveBarRequest, SolveBeam1dRequest, SolveElectrostaticBar1dRequest,
    SolveElectrostaticPlaneQuad2dRequest, SolveElectrostaticPlaneTriangle2dRequest,
    SolveHeatBar1dRequest, SolveHeatPlaneQuad2dRequest, SolveHeatPlaneTriangle2dRequest,
    SolveSpring1dRequest, SolveSpring2dRequest, SolveSpring3dRequest, SolveThermalBar1dRequest,
    SolveThermalBeam1dRequest, SolveThermalTruss2dRequest, SolveThermalTruss3dRequest,
    SolveTorsion1dRequest,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NodeResult {
    pub index: usize,
    pub x: f64,
    pub displacement: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ElementResult {
    pub index: usize,
    pub x1: f64,
    pub x2: f64,
    pub strain: f64,
    pub stress: f64,
    pub axial_force: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveBarResult {
    pub input: SolveBarRequest,
    pub nodes: Vec<NodeResult>,
    pub elements: Vec<ElementResult>,
    pub tip_displacement: f64,
    pub reaction_force: f64,
    pub max_displacement: f64,
    pub max_stress: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThermalBar1dNodeResult {
    pub index: usize,
    pub id: String,
    pub x: f64,
    pub ux: f64,
    pub temperature_delta: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThermalBar1dElementResult {
    pub index: usize,
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub length: f64,
    pub average_temperature_delta: f64,
    pub thermal_strain: f64,
    pub mechanical_strain: f64,
    pub total_strain: f64,
    pub stress: f64,
    pub axial_force: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveThermalBar1dResult {
    pub input: SolveThermalBar1dRequest,
    pub nodes: Vec<ThermalBar1dNodeResult>,
    pub elements: Vec<ThermalBar1dElementResult>,
    pub max_displacement: f64,
    pub max_stress: f64,
    pub max_axial_force: f64,
    pub max_temperature_delta: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeatBar1dNodeResult {
    pub index: usize,
    pub id: String,
    pub x: f64,
    pub temperature: f64,
    pub heat_load: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeatBar1dElementResult {
    pub index: usize,
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub length: f64,
    pub average_temperature: f64,
    pub temperature_gradient: f64,
    pub heat_flux: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveHeatBar1dResult {
    pub input: SolveHeatBar1dRequest,
    pub nodes: Vec<HeatBar1dNodeResult>,
    pub elements: Vec<HeatBar1dElementResult>,
    pub max_temperature: f64,
    pub max_heat_flux: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ElectrostaticBar1dNodeResult {
    pub index: usize,
    pub id: String,
    pub x: f64,
    pub potential: f64,
    pub charge_density: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ElectrostaticBar1dElementResult {
    pub index: usize,
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub length: f64,
    pub average_potential: f64,
    pub potential_gradient: f64,
    pub electric_field: f64,
    pub electric_flux_density: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveElectrostaticBar1dResult {
    pub input: SolveElectrostaticBar1dRequest,
    pub nodes: Vec<ElectrostaticBar1dNodeResult>,
    pub elements: Vec<ElectrostaticBar1dElementResult>,
    pub max_potential: f64,
    pub max_electric_field: f64,
    pub max_flux_density: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeatPlaneNodeResult {
    pub index: usize,
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub temperature: f64,
    pub heat_load: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeatPlaneTriangleElementResult {
    pub index: usize,
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub node_k: usize,
    pub area: f64,
    pub average_temperature: f64,
    pub temperature_gradient_x: f64,
    pub temperature_gradient_y: f64,
    pub heat_flux_x: f64,
    pub heat_flux_y: f64,
    pub heat_flux_magnitude: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveHeatPlaneTriangle2dResult {
    pub input: SolveHeatPlaneTriangle2dRequest,
    pub nodes: Vec<HeatPlaneNodeResult>,
    pub elements: Vec<HeatPlaneTriangleElementResult>,
    pub max_temperature: f64,
    pub max_heat_flux: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ElectrostaticPlaneNodeResult {
    pub index: usize,
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub potential: f64,
    pub charge_density: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ElectrostaticPlaneTriangleElementResult {
    pub index: usize,
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub node_k: usize,
    pub area: f64,
    pub average_potential: f64,
    pub potential_gradient_x: f64,
    pub potential_gradient_y: f64,
    pub electric_field_x: f64,
    pub electric_field_y: f64,
    pub electric_field_magnitude: f64,
    pub electric_flux_density_x: f64,
    pub electric_flux_density_y: f64,
    pub electric_flux_density_magnitude: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveElectrostaticPlaneTriangle2dResult {
    pub input: SolveElectrostaticPlaneTriangle2dRequest,
    pub nodes: Vec<ElectrostaticPlaneNodeResult>,
    pub elements: Vec<ElectrostaticPlaneTriangleElementResult>,
    pub max_potential: f64,
    pub max_electric_field: f64,
    pub max_flux_density: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ElectrostaticPlaneQuadElementResult {
    pub index: usize,
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub node_k: usize,
    pub node_l: usize,
    pub area: f64,
    pub average_potential: f64,
    pub potential_gradient_x: f64,
    pub potential_gradient_y: f64,
    pub electric_field_x: f64,
    pub electric_field_y: f64,
    pub electric_field_magnitude: f64,
    pub electric_flux_density_x: f64,
    pub electric_flux_density_y: f64,
    pub electric_flux_density_magnitude: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveElectrostaticPlaneQuad2dResult {
    pub input: SolveElectrostaticPlaneQuad2dRequest,
    pub nodes: Vec<ElectrostaticPlaneNodeResult>,
    pub elements: Vec<ElectrostaticPlaneQuadElementResult>,
    pub max_potential: f64,
    pub max_electric_field: f64,
    pub max_flux_density: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeatPlaneQuadElementResult {
    pub index: usize,
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub node_k: usize,
    pub node_l: usize,
    pub area: f64,
    pub average_temperature: f64,
    pub temperature_gradient_x: f64,
    pub temperature_gradient_y: f64,
    pub heat_flux_x: f64,
    pub heat_flux_y: f64,
    pub heat_flux_magnitude: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveHeatPlaneQuad2dResult {
    pub input: SolveHeatPlaneQuad2dRequest,
    pub nodes: Vec<HeatPlaneNodeResult>,
    pub elements: Vec<HeatPlaneQuadElementResult>,
    pub max_temperature: f64,
    pub max_heat_flux: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThermalTruss2dNodeResult {
    pub index: usize,
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub ux: f64,
    pub uy: f64,
    pub temperature_delta: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThermalTruss2dElementResult {
    pub index: usize,
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub length: f64,
    pub average_temperature_delta: f64,
    pub thermal_strain: f64,
    pub mechanical_strain: f64,
    pub total_strain: f64,
    pub stress: f64,
    pub axial_force: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveThermalTruss2dResult {
    pub input: SolveThermalTruss2dRequest,
    pub nodes: Vec<ThermalTruss2dNodeResult>,
    pub elements: Vec<ThermalTruss2dElementResult>,
    pub max_displacement: f64,
    pub max_stress: f64,
    pub max_axial_force: f64,
    pub max_temperature_delta: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThermalTruss3dNodeResult {
    pub index: usize,
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub ux: f64,
    pub uy: f64,
    pub uz: f64,
    pub temperature_delta: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThermalTruss3dElementResult {
    pub index: usize,
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub length: f64,
    pub average_temperature_delta: f64,
    pub thermal_strain: f64,
    pub mechanical_strain: f64,
    pub total_strain: f64,
    pub stress: f64,
    pub axial_force: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveThermalTruss3dResult {
    pub input: SolveThermalTruss3dRequest,
    pub nodes: Vec<ThermalTruss3dNodeResult>,
    pub elements: Vec<ThermalTruss3dElementResult>,
    pub max_displacement: f64,
    pub max_stress: f64,
    pub max_axial_force: f64,
    pub max_temperature_delta: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Spring1dNodeResult {
    pub index: usize,
    pub id: String,
    pub x: f64,
    pub ux: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Spring1dElementResult {
    pub index: usize,
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub length: f64,
    pub extension: f64,
    pub force: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveSpring1dResult {
    pub input: SolveSpring1dRequest,
    pub nodes: Vec<Spring1dNodeResult>,
    pub elements: Vec<Spring1dElementResult>,
    pub max_displacement: f64,
    pub max_force: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Spring2dNodeResult {
    pub index: usize,
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub ux: f64,
    pub uy: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Spring2dElementResult {
    pub index: usize,
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub length: f64,
    pub extension: f64,
    pub force: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveSpring2dResult {
    pub input: SolveSpring2dRequest,
    pub nodes: Vec<Spring2dNodeResult>,
    pub elements: Vec<Spring2dElementResult>,
    pub max_displacement: f64,
    pub max_force: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Spring3dNodeResult {
    pub index: usize,
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub ux: f64,
    pub uy: f64,
    pub uz: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Spring3dElementResult {
    pub index: usize,
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub length: f64,
    pub extension: f64,
    pub force: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveSpring3dResult {
    pub input: SolveSpring3dRequest,
    pub nodes: Vec<Spring3dNodeResult>,
    pub elements: Vec<Spring3dElementResult>,
    pub max_displacement: f64,
    pub max_force: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Beam1dNodeResult {
    pub index: usize,
    pub id: String,
    pub x: f64,
    pub uy: f64,
    pub rz: f64,
    pub displacement_magnitude: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Beam1dElementResult {
    pub index: usize,
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub length: f64,
    pub shear_force_i: f64,
    pub moment_i: f64,
    pub shear_force_j: f64,
    pub moment_j: f64,
    pub max_bending_stress: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveBeam1dResult {
    pub input: SolveBeam1dRequest,
    pub nodes: Vec<Beam1dNodeResult>,
    pub elements: Vec<Beam1dElementResult>,
    pub max_displacement: f64,
    pub max_rotation: f64,
    pub max_moment: f64,
    pub max_stress: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThermalBeam1dNodeResult {
    pub index: usize,
    pub id: String,
    pub x: f64,
    pub uy: f64,
    pub rz: f64,
    pub displacement_magnitude: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThermalBeam1dElementResult {
    pub index: usize,
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub length: f64,
    pub temperature_gradient_y: f64,
    pub thermal_curvature: f64,
    pub shear_force_i: f64,
    pub moment_i: f64,
    pub shear_force_j: f64,
    pub moment_j: f64,
    pub max_bending_stress: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveThermalBeam1dResult {
    pub input: SolveThermalBeam1dRequest,
    pub nodes: Vec<ThermalBeam1dNodeResult>,
    pub elements: Vec<ThermalBeam1dElementResult>,
    pub max_displacement: f64,
    pub max_rotation: f64,
    pub max_moment: f64,
    pub max_stress: f64,
    pub max_temperature_gradient: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Torsion1dNodeResult {
    pub index: usize,
    pub id: String,
    pub x: f64,
    pub rz: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Torsion1dElementResult {
    pub index: usize,
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub length: f64,
    pub twist: f64,
    pub torque: f64,
    pub shear_stress: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveTorsion1dResult {
    pub input: SolveTorsion1dRequest,
    pub nodes: Vec<Torsion1dNodeResult>,
    pub elements: Vec<Torsion1dElementResult>,
    pub max_rotation: f64,
    pub max_torque: f64,
    pub max_stress: f64,
}
