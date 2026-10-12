import { captureWorkbenchApiAuthority } from "@/lib/api/auth-context";
import { createWorkbenchAdminRecordSources } from "./workbench-admin-record-source";

export const workbenchLibraryRecordSources = createWorkbenchAdminRecordSources(() =>
  captureWorkbenchApiAuthority("/api/v1/projects"));

export const workbenchLibraryVersionSources = createWorkbenchAdminRecordSources(() =>
  captureWorkbenchApiAuthority("/api/v1/models", "/api/v1/model-versions"));
