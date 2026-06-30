import { NextResponse } from "next/server";

import { authorizeDirectMeshRequest } from "@/lib/direct-mesh/security";
import { runDirectMeshStudy } from "@/lib/runtime-gateway/direct-mesh-gateway";
import type { DirectMeshSolveBody } from "@/lib/runtime-gateway/direct-mesh-study-contract";

export const runtime = "nodejs";

export async function POST(request: Request) {
  const unauthorized = authorizeDirectMeshRequest(request);
  if (unauthorized) return unauthorized;

  try {
    const body = (await request.json()) as DirectMeshSolveBody;
    return NextResponse.json(await runDirectMeshStudy(body));
  } catch (error) {
    return NextResponse.json(
      { error: error instanceof Error ? error.message : "direct mesh solve failed" },
      { status: 502 },
    );
  }
}
