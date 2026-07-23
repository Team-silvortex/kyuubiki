use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    SolveAcousticBar1dResult, SolveAdvectionDiffusionBar1dResult, SolveBarResult,
    SolveBeam1dResult, SolveBucklingBeam1dResult, SolveBucklingFrame2dResult,
    SolveContactGap1dResult, SolveElectrostaticBar1dResult, SolveElectrostaticPlaneQuad2dResult,
    SolveElectrostaticPlaneTriangle2dResult, SolveFrame2dPDeltaResult, SolveFrame2dResult,
    SolveFrame3dResult, SolveHarmonicSpring1dResult, SolveHeatBar1dResult,
    SolveHeatPlaneQuad2dResult, SolveHeatPlaneTriangle2dResult, SolveMagnetostaticBar1dResult,
    SolveMagnetostaticPlaneQuad2dResult, SolveMagnetostaticPlaneTriangle2dResult,
    SolveModalFrame2dResult, SolveModalFrame3dResult, SolveNonlinearSpring1dResult,
    SolvePlaneQuad2dResult, SolvePlaneTriangle2dResult, SolveSolidTetra3dResult,
    SolveSpring1dResult, SolveSpring2dResult, SolveSpring3dResult,
    SolveStokesFlowPlaneQuad2dResult, SolveStokesFlowPlaneTriangle2dResult,
    SolveThermalBar1dResult, SolveThermalBeam1dResult, SolveThermalFrame2dResult,
    SolveThermalFrame3dResult, SolveThermalPlaneQuad2dResult, SolveThermalPlaneTriangle2dResult,
    SolveThermalTruss2dResult, SolveThermalTruss3dResult, SolveTorsion1dResult,
    SolveTransientHeatBar1dResult, SolveTransientSpring1dResult, SolveTruss2dResult,
    SolveTruss3dResult,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
// Keep public result construction stable; boxing only the largest variant would break callers.
#[allow(clippy::large_enum_variant)]
pub enum AnalysisResult {
    Bar1d(SolveBarResult),
    AcousticBar1d(SolveAcousticBar1dResult),
    ThermalBar1d(SolveThermalBar1dResult),
    HeatBar1d(SolveHeatBar1dResult),
    TransientHeatBar1d(SolveTransientHeatBar1dResult),
    ElectrostaticBar1d(SolveElectrostaticBar1dResult),
    MagnetostaticBar1d(SolveMagnetostaticBar1dResult),
    AdvectionDiffusionBar1d(SolveAdvectionDiffusionBar1dResult),
    ElectrostaticPlaneTriangle2d(SolveElectrostaticPlaneTriangle2dResult),
    ElectrostaticPlaneQuad2d(SolveElectrostaticPlaneQuad2dResult),
    MagnetostaticPlaneTriangle2d(SolveMagnetostaticPlaneTriangle2dResult),
    MagnetostaticPlaneQuad2d(SolveMagnetostaticPlaneQuad2dResult),
    HeatPlaneTriangle2d(SolveHeatPlaneTriangle2dResult),
    HeatPlaneQuad2d(SolveHeatPlaneQuad2dResult),
    StokesFlowPlaneTriangle2d(SolveStokesFlowPlaneTriangle2dResult),
    StokesFlowPlaneQuad2d(SolveStokesFlowPlaneQuad2dResult),
    ThermalTruss2d(SolveThermalTruss2dResult),
    ThermalTruss3d(SolveThermalTruss3dResult),
    Spring1d(SolveSpring1dResult),
    TransientSpring1d(SolveTransientSpring1dResult),
    HarmonicSpring1d(SolveHarmonicSpring1dResult),
    NonlinearSpring1d(SolveNonlinearSpring1dResult),
    ContactGap1d(SolveContactGap1dResult),
    Spring2d(SolveSpring2dResult),
    Spring3d(SolveSpring3dResult),
    Beam1d(SolveBeam1dResult),
    ThermalBeam1d(SolveThermalBeam1dResult),
    Torsion1d(SolveTorsion1dResult),
    Truss2d(SolveTruss2dResult),
    Truss3d(SolveTruss3dResult),
    Frame3d(SolveFrame3dResult),
    SolidTetra3d(SolveSolidTetra3dResult),
    PlaneTriangle2d(SolvePlaneTriangle2dResult),
    ThermalPlaneTriangle2d(SolveThermalPlaneTriangle2dResult),
    PlaneQuad2d(SolvePlaneQuad2dResult),
    ThermalPlaneQuad2d(SolveThermalPlaneQuad2dResult),
    Frame2d(SolveFrame2dResult),
    ModalFrame2d(SolveModalFrame2dResult),
    ModalFrame3d(SolveModalFrame3dResult),
    ThermalFrame2d(SolveThermalFrame2dResult),
    ThermalFrame3d(SolveThermalFrame3dResult),
    BucklingBeam1d(SolveBucklingBeam1dResult),
    BucklingFrame2d(SolveBucklingFrame2dResult),
    Frame2dPDelta(SolveFrame2dPDeltaResult),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResultChunkKind {
    Nodes,
    Elements,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResultChunkRequest {
    pub kind: ResultChunkKind,
    pub offset: usize,
    pub limit: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResultChunkResponse {
    pub kind: ResultChunkKind,
    pub offset: usize,
    pub limit: usize,
    pub returned: usize,
    pub total: usize,
    pub items: Vec<Value>,
}
