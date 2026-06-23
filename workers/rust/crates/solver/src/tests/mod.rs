
use super::{
    MockSolver, solve_bar_1d, solve_beam_1d, solve_electrostatic_bar_1d,
    solve_electrostatic_plane_quad_2d, solve_electrostatic_plane_triangle_2d, solve_frame_2d,
    solve_frame_3d, solve_heat_bar_1d, solve_heat_plane_quad_2d, solve_heat_plane_triangle_2d,
    solve_plane_quad_2d, solve_plane_triangle_2d, solve_spring_1d, solve_spring_2d,
    solve_spring_3d, solve_thermal_bar_1d, solve_thermal_beam_1d, solve_thermal_frame_2d,
    solve_thermal_frame_3d, solve_thermal_plane_quad_2d, solve_thermal_plane_triangle_2d,
    solve_thermal_truss_2d, solve_thermal_truss_3d, solve_torsion_1d, solve_truss_2d,
    solve_truss_3d,
};
use kyuubiki_protocol::{
    Beam1dElementInput, Beam1dNodeInput, ElectrostaticBar1dElementInput,
    ElectrostaticBar1dNodeInput, ElectrostaticPlaneNodeInput, ElectrostaticPlaneQuadElementInput,
    ElectrostaticPlaneTriangleElementInput, Frame2dElementInput, Frame2dNodeInput,
    Frame3dElementInput, Frame3dNodeInput, HeatBar1dElementInput, HeatBar1dNodeInput,
    HeatPlaneNodeInput, HeatPlaneQuadElementInput, HeatPlaneTriangleElementInput, Job, JobStatus,
    PlaneNodeInput, PlaneQuadElementInput, PlaneTriangleElementInput, SolveBarRequest,
    SolveBeam1dRequest, SolveElectrostaticBar1dRequest, SolveElectrostaticPlaneQuad2dRequest,
    SolveElectrostaticPlaneTriangle2dRequest, SolveFrame2dRequest, SolveFrame3dRequest,
    SolveHeatBar1dRequest, SolveHeatPlaneQuad2dRequest, SolveHeatPlaneTriangle2dRequest,
    SolvePlaneQuad2dRequest, SolvePlaneTriangle2dRequest, SolveSpring1dRequest,
    SolveSpring2dRequest, SolveSpring3dRequest, SolveThermalBar1dRequest,
    SolveThermalBeam1dRequest, SolveThermalFrame2dRequest, SolveThermalFrame3dRequest,
    SolveThermalPlaneQuad2dRequest, SolveThermalPlaneTriangle2dRequest, SolveThermalTruss2dRequest,
    SolveThermalTruss3dRequest, SolveTorsion1dRequest, SolveTruss2dRequest, SolveTruss3dRequest,
    Spring1dElementInput, Spring1dNodeInput, Spring2dElementInput, Spring2dNodeInput,
    Spring3dElementInput, Spring3dNodeInput, ThermalBar1dElementInput, ThermalBar1dNodeInput,
    ThermalBeam1dElementInput, ThermalBeam1dNodeInput, ThermalFrame3dElementInput,
    ThermalFrame3dNodeInput, ThermalPlaneNodeInput, ThermalPlaneQuadElementInput,
    ThermalPlaneTriangleElementInput, ThermalTruss2dElementInput, ThermalTruss2dNodeInput,
    ThermalTruss3dElementInput, ThermalTruss3dNodeInput, Torsion1dElementInput, Torsion1dNodeInput,
    Truss3dElementInput, Truss3dNodeInput, TrussElementInput, TrussNodeInput,
};

mod core_fields;
mod mechanics;
mod plane_structural;
mod thermal_structural;
