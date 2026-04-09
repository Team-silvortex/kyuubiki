import { NextResponse } from "next/server";

import {
  resolveAxialBarJobInput,
  resolvePlaneTriangle2dJobInput,
  resolveTruss2dJobInput,
  resolveTruss3dJobInput,
  type AxialBarJobInput,
  type PlaneTriangle2dJobInput,
  type Truss2dJobInput,
  type Truss3dJobInput,
} from "@/lib/api";
import { solveViaDirectMesh } from "@/lib/direct-mesh/rpc";
import { putDirectMeshResult } from "@/lib/direct-mesh/results";
import { authorizeDirectMeshRequest } from "@/lib/direct-mesh/security";

export const runtime = "nodejs";

type DirectMeshSolveBody = {
  endpoints?: string[];
  selection_mode?: "first_reachable" | "healthiest";
  study_kind:
    | "axial_bar_1d"
    | "truss_2d"
    | "truss_3d"
    | "plane_triangle_2d";
  input: Record<string, unknown>;
};

function methodForStudyKind(kind: DirectMeshSolveBody["study_kind"]) {
  switch (kind) {
    case "axial_bar_1d":
      return "solve_bar_1d" as const;
    case "truss_2d":
      return "solve_truss_2d" as const;
    case "truss_3d":
      return "solve_truss_3d" as const;
    case "plane_triangle_2d":
      return "solve_plane_triangle_2d" as const;
  }
}

function normalizeInputForStudyKind(
  kind: DirectMeshSolveBody["study_kind"],
  input: DirectMeshSolveBody["input"],
) {
  switch (kind) {
    case "axial_bar_1d":
      return resolveAxialBarJobInput(input as AxialBarJobInput);
    case "truss_2d":
      return resolveTruss2dJobInput(input as Truss2dJobInput);
    case "truss_3d":
      return resolveTruss3dJobInput(input as Truss3dJobInput);
    case "plane_triangle_2d":
      return resolvePlaneTriangle2dJobInput(input as PlaneTriangle2dJobInput);
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
    const solved = await solveViaDirectMesh(
      method,
      normalizedInput,
      body.endpoints,
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
