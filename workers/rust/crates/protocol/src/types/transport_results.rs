use crate::SolveAdvectionDiffusionBar1dRequest;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AdvectionDiffusionBar1dNodeResult {
    pub index: usize,
    pub id: String,
    pub x: f64,
    pub concentration: f64,
    pub source: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AdvectionDiffusionBar1dElementResult {
    pub index: usize,
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub length: f64,
    pub average_concentration: f64,
    pub concentration_gradient: f64,
    pub diffusive_flux: f64,
    pub advective_flux: f64,
    pub total_flux: f64,
    pub peclet_number: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveAdvectionDiffusionBar1dResult {
    pub input: SolveAdvectionDiffusionBar1dRequest,
    pub nodes: Vec<AdvectionDiffusionBar1dNodeResult>,
    pub elements: Vec<AdvectionDiffusionBar1dElementResult>,
    pub max_concentration: f64,
    pub max_total_flux: f64,
    pub max_peclet_number: f64,
}
