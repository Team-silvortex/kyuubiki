"use client";

import {
  fetchDirectMeshAgents,
  fetchHealth,
  fetchProtocolAgents,
  fetchRegisteredAgents,
} from "@/lib/api/runtime-client";
import {
  createRuntimeStatusBackendService,
  type WorkbenchRuntimeStatusBackendService,
  type WorkbenchRuntimeStatusRequest,
  type WorkbenchRuntimeStatusSnapshot,
  type WorkbenchRuntimeStatusTransport,
} from "@/lib/workbench/runtime-status-backend-service-core";

export {
  createRuntimeStatusBackendService,
  type WorkbenchRuntimeStatusBackendService,
  type WorkbenchRuntimeStatusRequest,
  type WorkbenchRuntimeStatusSnapshot,
  type WorkbenchRuntimeStatusTransport,
};

export const workbenchRuntimeStatusBackendService = createRuntimeStatusBackendService({
  fetchDirectMeshAgents,
  fetchHealth,
  fetchProtocolAgents,
  fetchRegisteredAgents,
});
