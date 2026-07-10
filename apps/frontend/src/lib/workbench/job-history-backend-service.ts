"use client";

import { defaultWorkbenchRuntimeBackedBackendServices } from "@/lib/workbench/backend-service-composer";
import {
  createJobHistoryBackendService,
  type WorkbenchJobHistoryBackendService,
  type WorkbenchJobHistoryBackendTransport,
} from "@/lib/workbench/job-history-backend-service-core";

export {
  createJobHistoryBackendService,
  type WorkbenchJobHistoryBackendService,
  type WorkbenchJobHistoryBackendTransport,
};

export const workbenchJobHistoryBackendService =
  defaultWorkbenchRuntimeBackedBackendServices.jobHistory;
