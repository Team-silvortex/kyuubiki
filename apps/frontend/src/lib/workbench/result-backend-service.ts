"use client";

import { defaultSecurityResultsApiClient, type FrontendRuntimeMode } from "@/lib/api";
import {
  createResultBackendService,
  type WorkbenchResultBackendService,
  type WorkbenchResultBackendTransport,
  type WorkbenchResultChunkInput,
} from "@/lib/workbench/result-backend-service-core";

export {
  createResultBackendService,
  type WorkbenchResultBackendService,
  type WorkbenchResultBackendTransport,
  type WorkbenchResultChunkInput,
};

export const orchestratedResultBackendService = createResultBackendService({
  backendId: "orchestrated_gui",
  fetchChunk: defaultSecurityResultsApiClient.fetchResultChunk,
});

export const directMeshResultBackendService = createResultBackendService({
  backendId: "direct_mesh_gui",
  fetchChunk: defaultSecurityResultsApiClient.fetchDirectMeshResultChunk,
});

export function resolveResultBackendService(
  frontendRuntimeMode: FrontendRuntimeMode,
): WorkbenchResultBackendService {
  return frontendRuntimeMode === "direct_mesh_gui"
    ? directMeshResultBackendService
    : orchestratedResultBackendService;
}
