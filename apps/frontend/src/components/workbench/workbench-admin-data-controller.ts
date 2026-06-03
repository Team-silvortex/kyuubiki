"use client";

import type { JobState, ProjectRecord } from "@/lib/api";

type AdminDataControllerDeps = {
  selectedAdminJob: JobState | null;
  selectedAdminJobId: string | null;
  selectedAdminResultJobId: string | null;
  jobHistory: JobState[];
  projects: ProjectRecord[];
  refreshVersions: (modelId: string) => Promise<void>;
  openModelVersionById: (versionId: string) => void;
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

export function openProjectContextById(projectId: string, deps: AdminDataControllerDeps) {
  const project = deps.projects.find((entry) => entry.project_id === projectId);

  if (!project) {
    deps.setMessage(deps.labels.linkedProjectMissing);
    return;
  }

  const firstModelId = project.models?.[0]?.model_id ?? null;
  const firstVersionId = project.models?.[0]?.latest_version_id ?? null;

  deps.setSelectedProjectId(project.project_id);
  deps.setSelectedModelId(firstModelId);
  deps.setSelectedVersionId(firstVersionId);
  deps.setSidebarSection("library");

  if (firstModelId) {
    void deps.refreshVersions(firstModelId);
  } else {
    deps.setModelVersions([]);
  }

  deps.setMessage(deps.labels.linkedProjectOpened);
}

export function applyJobContextToWorkbench(entry: JobState, deps: AdminDataControllerDeps) {
  deps.setAdminFilterProjectId(entry.project_id ?? "");
  deps.setAdminFilterModelVersionId(entry.model_version_id ?? "");
  deps.setAdminJobCaseId(entry.simulation_case_id ?? "");
  deps.setLibraryTab("projects");

  if (entry.model_version_id) {
    deps.openModelVersionById(entry.model_version_id);
    return;
  }

  if (entry.project_id) {
    openProjectContextById(entry.project_id, deps);
    return;
  }

  deps.setMessage(deps.labels.noRecordContext);
}

export function openSelectedAdminJobVersion(deps: AdminDataControllerDeps) {
  if (!deps.selectedAdminJob?.model_version_id) {
    deps.setMessage(deps.labels.noJobVersion);
    return;
  }

  deps.openModelVersionById(deps.selectedAdminJob.model_version_id);
}

export function openSelectedAdminResultVersion(deps: AdminDataControllerDeps) {
  const linkedJob = deps.jobHistory.find((entry) => entry.job_id === deps.selectedAdminResultJobId);

  if (!linkedJob?.model_version_id) {
    deps.setMessage(deps.labels.noResultVersion);
    return;
  }

  deps.openModelVersionById(linkedJob.model_version_id);
}

export function openSelectedAdminJobProject(deps: AdminDataControllerDeps) {
  if (!deps.selectedAdminJob?.project_id) {
    deps.setMessage(deps.labels.noJobProject);
    return;
  }

  openProjectContextById(deps.selectedAdminJob.project_id, deps);
}

export function openSelectedAdminResultProject(deps: AdminDataControllerDeps) {
  const linkedJob = deps.jobHistory.find((entry) => entry.job_id === deps.selectedAdminResultJobId);

  if (!linkedJob?.project_id) {
    deps.setMessage(deps.labels.noResultProject);
    return;
  }

  openProjectContextById(linkedJob.project_id, deps);
}

export function applySelectedAdminJobContext(deps: AdminDataControllerDeps) {
  if (!deps.selectedAdminJob) {
    deps.setMessage(deps.labels.selectJobFirst);
    return;
  }

  applyJobContextToWorkbench(deps.selectedAdminJob, deps);
  if (!deps.selectedAdminJob.model_version_id && !deps.selectedAdminJob.project_id) {
    return;
  }
  deps.setMessage(deps.labels.recordContextApplied);
}

export function applySelectedAdminResultContext(deps: AdminDataControllerDeps) {
  const linkedJob = deps.jobHistory.find((entry) => entry.job_id === deps.selectedAdminResultJobId);

  if (!linkedJob) {
    deps.setMessage(deps.labels.missingResultJob);
    return;
  }

  applyJobContextToWorkbench(linkedJob, deps);
  if (!linkedJob.model_version_id && !linkedJob.project_id) {
    return;
  }
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
