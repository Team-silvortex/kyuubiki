import { NextResponse } from "next/server";

import { authorizeDirectMeshRequest } from "@/lib/direct-mesh/security";
import { inspectDirectMeshRuntime } from "@/lib/runtime-gateway/direct-mesh-gateway";

export const runtime = "nodejs";

export async function POST(request: Request) {
  const unauthorized = authorizeDirectMeshRequest(request);
  if (unauthorized) return unauthorized;

  try {
    const body = (await request.json().catch(() => ({}))) as { endpoints?: string[] };
    return NextResponse.json(await inspectDirectMeshRuntime(body));
  } catch (error) {
    return NextResponse.json(
      { error: error instanceof Error ? error.message : "failed to inspect direct mesh agents" },
      { status: 500 },
    );
  }
}
