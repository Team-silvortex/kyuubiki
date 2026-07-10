"use client";

import { defaultWorkbenchRuntimeBackedBackendServices } from "@/lib/workbench/backend-service-composer";
import {
  createAdminDataBackendService,
  type WorkbenchAdminDataBackendService,
  type WorkbenchAdminDataBackendTransport,
  type WorkbenchDeletedJobEnvelope,
  type WorkbenchDeletedResultEnvelope,
  type WorkbenchJobRecordPatch,
  type WorkbenchResultRecordEnvelope,
} from "@/lib/workbench/admin-data-backend-service-core";

export {
  createAdminDataBackendService,
  type WorkbenchAdminDataBackendService,
  type WorkbenchAdminDataBackendTransport,
  type WorkbenchDeletedJobEnvelope,
  type WorkbenchDeletedResultEnvelope,
  type WorkbenchJobRecordPatch,
  type WorkbenchResultRecordEnvelope,
};

export const workbenchAdminDataBackendService =
  defaultWorkbenchRuntimeBackedBackendServices.adminData;
