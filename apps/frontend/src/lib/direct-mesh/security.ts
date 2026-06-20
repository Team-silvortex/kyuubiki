import { NextResponse } from "next/server";
import { parseDirectMeshEndpoint } from "@/lib/direct-mesh/rpc";

const DIRECT_MESH_MAX_REQUEST_ENDPOINTS = 32;

function extractToken(request: Request) {
  const authorization = request.headers.get("authorization");
  if (authorization?.startsWith("Bearer ")) {
    return authorization.slice("Bearer ".length);
  }

  return request.headers.get("x-kyuubiki-token");
}

function normalizeEndpoints(input: string[]) {
  return [...new Set(input.map((value) => value.trim()).filter(Boolean))];
}

function validateAuthorizedEndpoints(input: string[]) {
  if (input.length > DIRECT_MESH_MAX_REQUEST_ENDPOINTS) {
    throw new Error(`too many direct mesh endpoints; max ${DIRECT_MESH_MAX_REQUEST_ENDPOINTS}`);
  }
  return input.map((endpoint) => parseDirectMeshEndpoint(endpoint).endpoint);
}

function configuredDirectMeshEndpoints() {
  return normalizeEndpoints(
    (process.env.KYUUBIKI_DIRECT_MESH_ENDPOINTS ?? process.env.KYUUBIKI_AGENT_ENDPOINTS ?? "").split(","),
  );
}

function deploymentMode() {
  return process.env.KYUUBIKI_DEPLOYMENT_MODE ?? "local";
}

function directMeshEnabledForDeployment() {
  const configured = process.env.KYUUBIKI_DIRECT_MESH_ENABLED;
  if (configured === "true") return true;
  if (configured === "false") return false;
  return deploymentMode() === "local";
}

function allowRequestDefinedEndpoints() {
  const configured = process.env.KYUUBIKI_DIRECT_MESH_ALLOW_REQUEST_ENDPOINTS;
  if (configured === "true") return true;
  if (configured === "false") return false;
  return false;
}

export function authorizeDirectMeshRequest(request: Request) {
  const enabled = directMeshEnabledForDeployment();
  if (!enabled) {
    return NextResponse.json(
      {
        error: "direct_mesh_disabled",
        message: "direct mesh runtime is disabled for this deployment; set KYUUBIKI_DIRECT_MESH_ENABLED=true to opt in",
      },
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

export function resolveAuthorizedDirectMeshEndpoints(input?: string[]) {
  const requested = validateAuthorizedEndpoints(normalizeEndpoints(input ?? []));
  const configured = validateAuthorizedEndpoints(configuredDirectMeshEndpoints());

  if (requested.length === 0) {
    if (configured.length > 0) return configured;
    throw new Error("no configured direct mesh endpoints");
  }

  if (allowRequestDefinedEndpoints()) {
    return requested;
  }

  if (configured.length === 0) {
    throw new Error("request-defined direct mesh endpoints are disabled for this deployment");
  }

  const configuredSet = new Set(configured);
  const unauthorized = requested.filter((endpoint) => !configuredSet.has(endpoint));

  if (unauthorized.length > 0) {
    throw new Error(`direct mesh endpoint is not allowlisted: ${unauthorized[0]}`);
  }

  return requested;
}
