import { NextResponse } from "next/server";

import { chunkDirectMeshResult } from "@/lib/direct-mesh/results";
import { authorizeDirectMeshRequest } from "@/lib/direct-mesh/security";

export const runtime = "nodejs";

type RouteContext = {
  params: Promise<{
    jobId: string;
    kind: string;
  }>;
};

function isChunkKind(kind: string): kind is "nodes" | "elements" {
  return kind === "nodes" || kind === "elements";
}

export async function GET(request: Request, context: RouteContext) {
  const unauthorized = authorizeDirectMeshRequest(request);
  if (unauthorized) return unauthorized;

  try {
    const params = await context.params;
    if (!isChunkKind(params.kind)) {
      return NextResponse.json({ error: "unsupported direct mesh chunk kind" }, { status: 400 });
    }
    const { searchParams } = new URL(request.url);
    const offset = Number(searchParams.get("offset") ?? "0");
    const limit = Number(searchParams.get("limit") ?? "200");
    const payload = chunkDirectMeshResult(params.jobId, params.kind, { offset, limit });
    return NextResponse.json(payload);
  } catch (error) {
    return NextResponse.json(
      { error: error instanceof Error ? error.message : "failed to fetch direct mesh chunk" },
      { status: 404 },
    );
  }
}
