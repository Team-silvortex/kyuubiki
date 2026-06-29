"use client";

import { cancelJob, fetchJobHistory } from "@/lib/api/runtime-client";
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

export const workbenchJobHistoryBackendService = createJobHistoryBackendService({
  cancelJob,
  fetchJobHistory,
});
