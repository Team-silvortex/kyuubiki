"use client";

import {
  deleteJobRecord,
  fetchJobStatus,
  updateJobRecord,
} from "@/lib/api/runtime-client";
import {
  deleteResultRecord,
  fetchResults,
  updateResultRecord,
} from "@/lib/api/security-results-client";
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

export const workbenchAdminDataBackendService = createAdminDataBackendService({
  deleteJob: deleteJobRecord,
  deleteResult: deleteResultRecord,
  fetchJob: fetchJobStatus,
  fetchResults,
  updateJob: updateJobRecord,
  updateResult: updateResultRecord,
});
