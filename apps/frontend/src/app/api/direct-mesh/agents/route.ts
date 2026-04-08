import { NextResponse } from "next/server";

import { describeDirectMeshAgents } from "@/lib/direct-mesh/rpc";
import { authorizeDirectMeshRequest } from "@/lib/direct-mesh/security";

export const runtime = "nodejs";

export async function POST(request: Request) {
  const unauthorized = authorizeDirectMeshRequest(request);
  if (unauthorized) return unauthorized;

  try {
    const body = (await request.json().catch(() => ({}))) as { endpoints?: string[] };
    const { endpoints, agents } = await describeDirectMeshAgents(body.endpoints);

    return NextResponse.json({
      mode: "direct_mesh_gui",
      discovery: "manual",
      endpoint_count: endpoints.length,
      agents,
    });
  } catch (error) {
    return NextResponse.json(
      { error: error instanceof Error ? error.message : "failed to inspect direct mesh agents" },
      { status: 500 },
    );
  }
}
