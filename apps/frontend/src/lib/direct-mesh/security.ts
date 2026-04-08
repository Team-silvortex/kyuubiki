import { NextResponse } from "next/server";

function extractToken(request: Request) {
  const authorization = request.headers.get("authorization");
  if (authorization?.startsWith("Bearer ")) {
    return authorization.slice("Bearer ".length);
  }

  return request.headers.get("x-kyuubiki-token");
}

export function authorizeDirectMeshRequest(request: Request) {
  const enabled = process.env.KYUUBIKI_DIRECT_MESH_ENABLED !== "false";
  if (!enabled) {
    return NextResponse.json(
      { error: "direct_mesh_disabled", message: "direct mesh runtime is disabled for this deployment" },
      { status: 403 },
    );
  }

  const token = process.env.KYUUBIKI_DIRECT_MESH_TOKEN;
  if (!token) return null;

  if (extractToken(request) === token) return null;

  return NextResponse.json(
    { error: "unauthorized", message: "missing or invalid direct mesh token" },
    { status: 401 },
  );
}
