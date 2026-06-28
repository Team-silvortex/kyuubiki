mod core;
mod field_rpc;
mod frame_beam_rpc;
mod spring_control_rpc;
mod thermal_plane_rpc;
mod workflows;

mod prelude {
    pub(super) use crate::{
        AcousticBar1dElementInput, AcousticBar1dNodeInput, AgentDescriptor, Beam1dElementInput,
        Beam1dNodeInput, ElectrostaticBar1dElementInput, ElectrostaticBar1dNodeInput,
        ElectrostaticPlaneNodeInput, ElectrostaticPlaneQuadElementInput,
        ElectrostaticPlaneTriangleElementInput, Frame2dElementInput, Frame2dNodeInput,
        Frame3dElementInput, Frame3dNodeInput, HeatBar1dElementInput, HeatBar1dNodeInput,
        HeatPlaneNodeInput, HeatPlaneNodeResult, HeatPlaneQuadElementInput,
        HeatPlaneQuadElementResult, HeatPlaneTriangleElementInput, HeatPlaneTriangleElementResult,
        HeatToThermoPlaneQuad2dWorkflowRequest, HeatToThermoPlaneQuad2dWorkflowResult,
        HeatToThermoPlaneTriangle2dWorkflowRequest, HeatToThermoPlaneTriangle2dWorkflowResult, Job,
        JobStatus, MagnetostaticBar1dElementInput, MagnetostaticBar1dNodeInput,
        OperatorArtifactRef, OperatorDescriptor, OperatorKind, OperatorOrigin,
        OperatorPortDescriptor, OperatorRunContext, OperatorRunRequest, OperatorRunResult,
        OperatorSchemaRef, OperatorValidationProfile, OperatorValidationStatus,
        PlaneQuadElementInput, ProgressEvent, RPC_VERSION, RpcMethod, RpcProgress, RpcRequest,
        RpcResponse, SolveAcousticBar1dRequest, SolveBarRequest, SolveBeam1dRequest,
        SolveElectrostaticBar1dRequest, SolveElectrostaticPlaneQuad2dRequest,
        SolveElectrostaticPlaneTriangle2dRequest, SolveFrame2dRequest, SolveFrame3dRequest,
        SolveHeatBar1dRequest, SolveHeatPlaneQuad2dRequest, SolveHeatPlaneQuad2dResult,
        SolveHeatPlaneTriangle2dRequest, SolveHeatPlaneTriangle2dResult,
        SolveMagnetostaticBar1dRequest, SolvePlaneQuad2dRequest, SolvePlaneTriangle2dRequest,
        SolveSpring1dRequest, SolveSpring2dRequest, SolveSpring3dRequest, SolveThermalBar1dRequest,
        SolveThermalBeam1dRequest, SolveThermalFrame2dRequest, SolveThermalFrame3dRequest,
        SolveThermalPlaneQuad2dRequest, SolveThermalPlaneQuad2dResult,
        SolveThermalPlaneTriangle2dRequest, SolveThermalPlaneTriangle2dResult,
        SolveThermalTruss2dRequest, SolveTorsion1dRequest, SolveTruss3dRequest,
        Spring1dElementInput, Spring1dNodeInput, Spring2dElementInput, Spring2dNodeInput,
        Spring3dElementInput, Spring3dNodeInput, ThermalBar1dElementInput, ThermalBar1dNodeInput,
        ThermalBeam1dElementInput, ThermalBeam1dNodeInput, ThermalFrame2dElementInput,
        ThermalFrame2dNodeInput, ThermalFrame3dElementInput, ThermalFrame3dNodeInput,
        ThermalPlaneNodeInput, ThermalPlaneNodeResult, ThermalPlaneQuadElementInput,
        ThermalPlaneQuadElementResult, ThermalPlaneTriangleElementInput,
        ThermalPlaneTriangleElementResult, ThermalTruss2dElementInput, ThermalTruss2dNodeInput,
        Torsion1dElementInput, Torsion1dNodeInput, WorkflowCachePolicy, WorkflowDatasetAxis,
        WorkflowDatasetContract, WorkflowDatasetEncoding, WorkflowDatasetShape,
        WorkflowDatasetValueInfo, WorkflowDefaults, WorkflowEdge, WorkflowGraph,
        WorkflowGraphRunRequest, WorkflowGraphRunResult, WorkflowNode, WorkflowNodeKind,
        WorkflowNodePortRef, WorkflowPort,
    };
}
