"use client";

import type { JobState, ProjectRecord } from "@/lib/api";
import {
  workbenchOperationFailure,
  type WorkbenchOperationResult,
} from "@/lib/workbench/operation-result";

type AdminDataControllerDeps = {
  selectedAdminJob: JobState | null;
  selectedAdminJobId: string | null;
  selectedAdminResultJobId: string | null;
  jobHistory: JobState[];
  projects: ProjectRecord[];
  refreshVersions: (modelId: string) => Promise<void>;
  openModelVersionById: (versionId: string) => Promise<WorkbenchOperationResult>;
  setAdminFilterProjectId: (value: string) => void;
  setAdminFilterModelVersionId: (value: string) => void;
  setAdminJobCaseId: (value: string) => void;
  setLibraryTab: (value: any) => void;
  setSelectedProjectId: (value: string | null) => void;
  setSelectedModelId: (value: string | null) => void;
  setSelectedVersionId: (value: string | null) => void;
  setModelVersions: (value: any[]) => void;
  setSidebarSection: (value: any) => void;
  setMessage: (value: string) => void;
  labels: {
    noJobVersion: string;
    noResultVersion: string;
    noRecordContext: string;
    linkedProjectMissing: string;
    linkedProjectOpened: string;
    noJobProject: string;
    noResultProject: string;
    selectJobFirst: string;
    missingResultJob: string;
    recordContextApplied: string;
  };
};

export async function openProjectContextById(
  projectId: string,
  deps: AdminDataControllerDeps,
): Promise<WorkbenchOperationResult> {
  const project = deps.projects.find((entry) => entry.project_id === projectId);

  if (!project) {
    deps.setMessage(deps.labels.linkedProjectMissing);
    return workbenchOperationFailure(
      new Error(deps.labels.linkedProjectMissing),
      deps.labels.linkedProjectMissing,
    );
  }

  const firstModelId = project.models?.[0]?.model_id ?? null;
  const firstVersionId = project.models?.[0]?.latest_version_id ?? null;

  deps.setSelectedProjectId(project.project_id);
  deps.setSelectedModelId(firstModelId);
  deps.setSelectedVersionId(firstVersionId);
  deps.setSidebarSection("library");

  try {
    if (firstModelId) {
      await deps.refreshVersions(firstModelId);
    } else {
      deps.setModelVersions([]);
    }
  } catch (error) {
    const failure = workbenchOperationFailure(error, deps.labels.linkedProjectMissing);
    deps.setMessage(failure.error.message);
    return failure;
  }

  deps.setMessage(deps.labels.linkedProjectOpened);
  return { ok: true };
}

export async function applyJobContextToWorkbench(
  entry: JobState,
  deps: AdminDataControllerDeps,
): Promise<WorkbenchOperationResult> {
  deps.setAdminFilterProjectId(entry.project_id ?? "");
  deps.setAdminFilterModelVersionId(entry.model_version_id ?? "");
  deps.setAdminJobCaseId(entry.simulation_case_id ?? "");
  deps.setLibraryTab("projects");

  if (entry.model_version_id) {
    return deps.openModelVersionById(entry.model_version_id);
  }

  if (entry.project_id) {
    return openProjectContextById(entry.project_id, deps);
  }

  deps.setMessage(deps.labels.noRecordContext);
  return workbenchOperationFailure(
    new Error(deps.labels.noRecordContext),
    deps.labels.noRecordContext,
  );
}

export function openSelectedAdminJobVersion(deps: AdminDataControllerDeps) {
  if (!deps.selectedAdminJob?.model_version_id) {
    deps.setMessage(deps.labels.noJobVersion);
    return;
  }

  void deps.openModelVersionById(deps.selectedAdminJob.model_version_id);
}

export function openSelectedAdminResultVersion(deps: AdminDataControllerDeps) {
  const linkedJob = deps.jobHistory.find((entry) => entry.job_id === deps.selectedAdminResultJobId);

  if (!linkedJob?.model_version_id) {
    deps.setMessage(deps.labels.noResultVersion);
    return;
  }

  void deps.openModelVersionById(linkedJob.model_version_id);
}

export function openSelectedAdminJobProject(deps: AdminDataControllerDeps) {
  if (!deps.selectedAdminJob?.project_id) {
    deps.setMessage(deps.labels.noJobProject);
    return;
  }

  void openProjectContextById(deps.selectedAdminJob.project_id, deps);
}

export function openSelectedAdminResultProject(deps: AdminDataControllerDeps) {
  const linkedJob = deps.jobHistory.find((entry) => entry.job_id === deps.selectedAdminResultJobId);

  if (!linkedJob?.project_id) {
    deps.setMessage(deps.labels.noResultProject);
    return;
  }

  void openProjectContextById(linkedJob.project_id, deps);
}

export async function applySelectedAdminJobContext(deps: AdminDataControllerDeps) {
  if (!deps.selectedAdminJob) {
    deps.setMessage(deps.labels.selectJobFirst);
    return;
  }

  const result = await applyJobContextToWorkbench(deps.selectedAdminJob, deps);
  if (!result.ok) return;
  deps.setMessage(deps.labels.recordContextApplied);
}

export async function applySelectedAdminResultContext(deps: AdminDataControllerDeps) {
  const linkedJob = deps.jobHistory.find((entry) => entry.job_id === deps.selectedAdminResultJobId);

  if (!linkedJob) {
    deps.setMessage(deps.labels.missingResultJob);
    return;
  }

  const result = await applyJobContextToWorkbench(linkedJob, deps);
  if (!result.ok) return;
  deps.setMessage(deps.labels.recordContextApplied);
}

export function resolveScriptLinkedJob(payload: Record<string, unknown>, deps: AdminDataControllerDeps) {
  const target = payload.target === "job" || payload.target === "result" ? payload.target : "job";
  const explicitJobId = typeof payload.jobId === "string" ? payload.jobId : null;
  const explicitResultJobId = typeof payload.resultJobId === "string" ? payload.resultJobId : null;

  if (target === "job") {
    const jobId = explicitJobId ?? deps.selectedAdminJobId;
    return jobId ? deps.jobHistory.find((entry) => entry.job_id === jobId) ?? null : null;
  }

  const resultJobId = explicitResultJobId ?? explicitJobId ?? deps.selectedAdminResultJobId;
  return resultJobId ? deps.jobHistory.find((entry) => entry.job_id === resultJobId) ?? null : null;
}
