import { NextResponse } from "next/server";

import {
  resolveAxialBarJobInput,
  resolveBeam1dJobInput,
  resolveHeatBar1dJobInput,
  resolveHeatPlaneQuad2dJobInput,
  resolveHeatPlaneTriangle2dJobInput,
  resolveFrame2dJobInput,
  resolvePlaneQuad2dJobInput,
  resolvePlaneTriangle2dJobInput,
  resolveSpring1dJobInput,
  resolveSpring2dJobInput,
  resolveSpring3dJobInput,
  resolveThermalBar1dJobInput,
  resolveThermalBeam1dJobInput,
  resolveThermalFrame2dJobInput,
  resolveThermalPlaneQuad2dJobInput,
  resolveThermalPlaneTriangle2dJobInput,
  resolveThermalTruss2dJobInput,
  resolveThermalTruss3dJobInput,
  resolveTorsion1dJobInput,
  resolveTruss2dJobInput,
  resolveTruss3dJobInput,
  type AxialBarJobInput,
  type Beam1dJobInput,
  type HeatBar1dJobInput,
  type HeatPlaneQuad2dJobInput,
  type HeatPlaneTriangle2dJobInput,
  type Frame2dJobInput,
  type PlaneQuad2dJobInput,
  type PlaneTriangle2dJobInput,
  type Spring1dJobInput,
  type Spring2dJobInput,
  type Spring3dJobInput,
  type ThermalBar1dJobInput,
  type ThermalBeam1dJobInput,
  type ThermalFrame2dJobInput,
  type ThermalPlaneQuad2dJobInput,
  type ThermalPlaneTriangle2dJobInput,
  type ThermalTruss2dJobInput,
  type ThermalTruss3dJobInput,
  type Torsion1dJobInput,
  type Truss2dJobInput,
  type Truss3dJobInput,
} from "@/lib/api";
import { solveViaDirectMesh } from "@/lib/direct-mesh/rpc";
import { putDirectMeshResult } from "@/lib/direct-mesh/results";
import { authorizeDirectMeshRequest, resolveAuthorizedDirectMeshEndpoints } from "@/lib/direct-mesh/security";

export const runtime = "nodejs";

type DirectMeshSolveBody = {
  endpoints?: string[];
  selection_mode?: "first_reachable" | "healthiest";
  study_kind:
    | "axial_bar_1d"
    | "thermal_bar_1d"
    | "heat_bar_1d"
    | "heat_plane_triangle_2d"
    | "heat_plane_quad_2d"
    | "thermal_truss_2d"
    | "thermal_truss_3d"
    | "spring_1d"
    | "spring_2d"
    | "spring_3d"
    | "beam_1d"
    | "thermal_beam_1d"
    | "thermal_frame_2d"
    | "torsion_1d"
    | "truss_2d"
    | "truss_3d"
    | "plane_triangle_2d"
    | "thermal_plane_triangle_2d"
    | "plane_quad_2d"
    | "thermal_plane_quad_2d"
    | "frame_2d";
  input: Record<string, unknown>;
};

function methodForStudyKind(kind: DirectMeshSolveBody["study_kind"]) {
  switch (kind) {
    case "axial_bar_1d":
      return "solve_bar_1d" as const;
    case "thermal_bar_1d":
      return "solve_thermal_bar_1d" as const;
    case "heat_bar_1d":
      return "solve_heat_bar_1d" as const;
    case "heat_plane_triangle_2d":
      return "solve_heat_plane_triangle_2d" as const;
    case "heat_plane_quad_2d":
      return "solve_heat_plane_quad_2d" as const;
    case "thermal_truss_2d":
      return "solve_thermal_truss_2d" as const;
    case "thermal_truss_3d":
      return "solve_thermal_truss_3d" as const;
    case "spring_1d":
      return "solve_spring_1d" as const;
    case "spring_2d":
      return "solve_spring_2d" as const;
    case "spring_3d":
      return "solve_spring_3d" as const;
    case "beam_1d":
      return "solve_beam_1d" as const;
    case "thermal_beam_1d":
      return "solve_thermal_beam_1d" as const;
    case "thermal_frame_2d":
      return "solve_thermal_frame_2d" as const;
    case "torsion_1d":
      return "solve_torsion_1d" as const;
    case "truss_2d":
      return "solve_truss_2d" as const;
    case "truss_3d":
      return "solve_truss_3d" as const;
    case "plane_triangle_2d":
      return "solve_plane_triangle_2d" as const;
    case "thermal_plane_triangle_2d":
      return "solve_thermal_plane_triangle_2d" as const;
    case "plane_quad_2d":
      return "solve_plane_quad_2d" as const;
    case "thermal_plane_quad_2d":
      return "solve_thermal_plane_quad_2d" as const;
    case "frame_2d":
      return "solve_frame_2d" as const;
  }
}

function normalizeInputForStudyKind(
  kind: DirectMeshSolveBody["study_kind"],
  input: DirectMeshSolveBody["input"],
) {
  switch (kind) {
    case "axial_bar_1d":
      return resolveAxialBarJobInput(input as AxialBarJobInput);
    case "thermal_bar_1d":
      return resolveThermalBar1dJobInput(input as ThermalBar1dJobInput);
    case "heat_bar_1d":
      return resolveHeatBar1dJobInput(input as HeatBar1dJobInput);
    case "heat_plane_triangle_2d":
      return resolveHeatPlaneTriangle2dJobInput(input as HeatPlaneTriangle2dJobInput);
    case "heat_plane_quad_2d":
      return resolveHeatPlaneQuad2dJobInput(input as HeatPlaneQuad2dJobInput);
    case "thermal_truss_2d":
      return resolveThermalTruss2dJobInput(input as ThermalTruss2dJobInput);
    case "thermal_truss_3d":
      return resolveThermalTruss3dJobInput(input as ThermalTruss3dJobInput);
    case "spring_1d":
      return resolveSpring1dJobInput(input as Spring1dJobInput);
    case "spring_2d":
      return resolveSpring2dJobInput(input as Spring2dJobInput);
    case "spring_3d":
      return resolveSpring3dJobInput(input as Spring3dJobInput);
    case "beam_1d":
      return resolveBeam1dJobInput(input as Beam1dJobInput);
    case "thermal_beam_1d":
      return resolveThermalBeam1dJobInput(input as ThermalBeam1dJobInput);
    case "thermal_frame_2d":
      return resolveThermalFrame2dJobInput(input as ThermalFrame2dJobInput);
    case "torsion_1d":
      return resolveTorsion1dJobInput(input as Torsion1dJobInput);
    case "truss_2d":
      return resolveTruss2dJobInput(input as Truss2dJobInput);
    case "truss_3d":
      return resolveTruss3dJobInput(input as Truss3dJobInput);
    case "plane_triangle_2d":
      return resolvePlaneTriangle2dJobInput(input as PlaneTriangle2dJobInput);
    case "thermal_plane_triangle_2d":
      return resolveThermalPlaneTriangle2dJobInput(input as ThermalPlaneTriangle2dJobInput);
    case "plane_quad_2d":
      return resolvePlaneQuad2dJobInput(input as PlaneQuad2dJobInput);
    case "thermal_plane_quad_2d":
      return resolveThermalPlaneQuad2dJobInput(input as ThermalPlaneQuad2dJobInput);
    case "frame_2d":
      return resolveFrame2dJobInput(input as Frame2dJobInput);
  }
}

export async function POST(request: Request) {
  const unauthorized = authorizeDirectMeshRequest(request);
  if (unauthorized) return unauthorized;

  try {
    const body = (await request.json()) as DirectMeshSolveBody;
    const startedAt = new Date().toISOString();
    const method = methodForStudyKind(body.study_kind);
    const normalizedInput = normalizeInputForStudyKind(body.study_kind, body.input);
    const endpoints = resolveAuthorizedDirectMeshEndpoints(body.endpoints);
    const solved = await solveViaDirectMesh(
      method,
      normalizedInput,
      endpoints,
      body.selection_mode ?? "healthiest",
    );
    const jobId = `direct-${Date.now().toString(36)}`;

    putDirectMeshResult(jobId, {
      studyKind: body.study_kind,
      result: (solved.result ?? {}) as Record<string, unknown>,
      endpoint: solved.endpoint,
      storedAt: new Date().toISOString(),
    });

    return NextResponse.json({
      job: {
        job_id: jobId,
        status: "completed",
        worker_id: `direct-mesh@${solved.endpoint}`,
        progress: 1,
        message: "completed through direct mesh gui",
        created_at: startedAt,
        updated_at: new Date().toISOString(),
        has_result: true,
      },
      result: solved.result,
      direct_mesh: {
        endpoint: solved.endpoint,
        strategy: solved.strategy,
        progress_frames: solved.progress_frames,
      },
    });
  } catch (error) {
    return NextResponse.json(
      { error: error instanceof Error ? error.message : "direct mesh solve failed" },
      { status: 502 },
    );
  }
}
