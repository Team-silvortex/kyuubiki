"use client";

import { defaultWorkbenchRuntimeBackedBackendServices } from "@/lib/workbench/backend-service-composer";
import {
  createRuntimeStatusBackendService,
  type WorkbenchRuntimeStatusBackendService,
  type WorkbenchRuntimeStatusRequest,
  type WorkbenchRuntimeStatusSnapshot,
  type WorkbenchRuntimeStatusTransport,
} from "@/lib/runtime-gateway/runtime-status-gateway";

export {
  createRuntimeStatusBackendService,
  type WorkbenchRuntimeStatusBackendService,
  type WorkbenchRuntimeStatusRequest,
  type WorkbenchRuntimeStatusSnapshot,
  type WorkbenchRuntimeStatusTransport,
};

export const workbenchRuntimeStatusBackendService =
  defaultWorkbenchRuntimeBackedBackendServices.runtimeStatus;
