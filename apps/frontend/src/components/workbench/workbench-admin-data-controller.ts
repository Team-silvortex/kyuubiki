"use client";

import type { JobState, ProjectRecord, ResultRecord } from "@/lib/api";
import { workbenchProjectContextChangedError, type WorkbenchProjectContext } from "@/lib/workbench/project-context";
import { workbenchOperationFailure, type WorkbenchOperationResult } from "@/lib/workbench/operation-result";
import type { CheckpointVersionGuard } from "@/lib/workbench/checkpoint-recovery";
import type { WorkbenchVersionLoadScope } from "./workbench-persisted-model-controller";
import type { WorkbenchFeedback } from "./workbench-feedback";
import { isWorkbenchAdminRecordSourceCurrent, workbenchAdminRecordSources } from "./workbench-admin-record-source";
import { workbenchLibraryRecordSources } from "./workbench-library-record-source";

export type WorkbenchLinkedContext = {
  jobId: string; projectId: string | null; modelVersionId: string | null;
};
export type AdminDataControllerDeps = {
  selectedAdminJob: JobState | null;
  selectedAdminJobId: string | null;
  selectedAdminResultJobId: string | null;
  jobHistory: JobState[];
  resultRecords: ResultRecord[];
  projects: ProjectRecord[];
  projectContext: WorkbenchProjectContext;
  feedback: WorkbenchFeedback;
  refreshVersions: (modelId: string) => Promise<void>;
  openModelVersionById: (versionId: string, validate?: CheckpointVersionGuard, scope?: WorkbenchVersionLoadScope) => Promise<WorkbenchOperationResult>;
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
    noJobVersion: string; noResultVersion: string; noRecordContext: string;
    linkedProjectMissing: string; linkedProjectOpened: string;
    noJobProject: string; noResultProject: string; selectJobFirst: string;
    missingResultJob: string; recordContextApplied: string; sourceChanged: string;
  };
};

export function resolveScriptLinkedJob(payload: Record<string, unknown>, deps: AdminDataControllerDeps) {
  const isResult = payload.target === "result";
  const explicitJobId = typeof payload.jobId === "string" ? payload.jobId : null;
  const id = isResult
    ? (typeof payload.resultJobId === "string" ? payload.resultJobId : explicitJobId ?? deps.selectedAdminResultJobId)
    : explicitJobId ?? deps.selectedAdminJobId;
  if (!id || (isResult && !deps.resultRecords.some((record) => record.job_id === id))) return null;
  return deps.jobHistory.find((record) => record.job_id === id) ?? null;
}

export async function openWorkbenchLinkedContext(payload: Record<string, unknown>, deps: AdminDataControllerDeps): Promise<WorkbenchOperationResult<WorkbenchLinkedContext>> {
  const isResult = payload.target === "result";
  const mode = payload.mode === "version" || payload.mode === "project" ? payload.mode : "apply";
  const job = resolveScriptLinkedJob(payload, deps);
  let ownsFeedback = deps.feedback.begin();
  if (!ownsFeedback()) return workbenchOperationFailure(workbenchProjectContextChangedError(), deps.labels.noRecordContext);
  const fail = (message: string) => {
    if (ownsFeedback()) deps.setMessage(message);
    return workbenchOperationFailure(new Error(message), message);
  };
  if (!job) return fail(isResult ? deps.labels.missingResultJob : deps.labels.selectJobFirst);
  const source = workbenchAdminRecordSources.sourceOf(deps.jobHistory);
  const resultSource = workbenchAdminRecordSources.sourceOf(deps.resultRecords);
  const ownsSource = () => isWorkbenchAdminRecordSourceCurrent(source, source)
    && (!isResult || isWorkbenchAdminRecordSourceCurrent(source, resultSource));
  const sourceChanged = `WORKBENCH_RECORD_SOURCE_CHANGED: ${deps.labels.sourceChanged}`;
  if (!ownsSource()) return fail(sourceChanged);
  const versionId = mode === "project" ? null : job.model_version_id;
  if (mode === "version" && !versionId) return fail(isResult ? deps.labels.noResultVersion : deps.labels.noJobVersion);
  if (!versionId && !job.project_id) return fail(mode === "project"
    ? isResult ? deps.labels.noResultProject : deps.labels.noJobProject : deps.labels.noRecordContext);
  const applyFilters = () => {
    deps.setAdminFilterProjectId(job.project_id ?? "");
    deps.setAdminFilterModelVersionId(job.model_version_id ?? "");
    deps.setAdminJobCaseId(job.simulation_case_id ?? "");
    deps.setLibraryTab("projects");
  };

  if (versionId) {
    let result: WorkbenchOperationResult;
    try {
      result = await deps.openModelVersionById(versionId, (version) => {
        if (version?.version_id !== versionId || typeof version.model_id !== "string" || !version.model_id.trim()
          || typeof version.project_id !== "string" || !version.project_id.trim()
          || (job.project_id && version.project_id !== job.project_id)) {
          throw new Error("WORKBENCH_LINKED_VERSION_INVALID: the returned version does not match the linked record.");
        }
      }, {
        isCurrent: ownsSource,
        ownsFeedback: () => ownsFeedback(),
        onApplied: () => {
          // Commit context with the validated payload, not with a later optional success message.
          if (mode === "apply") applyFilters();
          ownsFeedback = deps.feedback.begin();
        },
      });
    } catch (error) {
      const failure = workbenchOperationFailure(error, deps.labels.noJobVersion);
      if (ownsSource() && ownsFeedback()) deps.setMessage(failure.error.message);
      return failure;
    }
    if (!result.ok) return result;
    if (!ownsSource()) return workbenchOperationFailure(new Error(sourceChanged), sourceChanged);
  } else {
    const project = deps.projects.find((entry) => entry.project_id === job.project_id);
    const projectSource = workbenchLibraryRecordSources.sourceOf(deps.projects);
    if (!isWorkbenchAdminRecordSourceCurrent(projectSource, projectSource)) return fail(sourceChanged);
    if (!project) return fail(deps.labels.linkedProjectMissing);
    if (!deps.projectContext.begin()()) return workbenchOperationFailure(workbenchProjectContextChangedError(), deps.labels.noRecordContext);
    deps.projectContext.update({ projectId: project.project_id, modelId: null, versionId: null });
    deps.setSelectedProjectId(project.project_id);
    // Project navigation does not load geometry, so it cannot attach a saved model/version.
    deps.setSelectedModelId(null);
    deps.setSelectedVersionId(null);
    deps.setModelVersions([]);
    deps.setLibraryTab("projects");
    deps.setSidebarSection("library");
    if (mode === "apply") applyFilters();
    ownsFeedback = deps.feedback.begin();
  }
  if (mode === "apply" && ownsFeedback()) {
    deps.setMessage(deps.labels.recordContextApplied);
  } else if (!versionId && ownsFeedback()) deps.setMessage(deps.labels.linkedProjectOpened);
  return { ok: true, jobId: job.job_id, projectId: job.project_id ?? null, modelVersionId: job.model_version_id ?? null };
}

export const openSelectedAdminJobVersion = (deps: AdminDataControllerDeps) => openWorkbenchLinkedContext({ mode: "version" }, deps);
export const openSelectedAdminResultVersion = (deps: AdminDataControllerDeps) => openWorkbenchLinkedContext({ target: "result", mode: "version" }, deps);
export const openSelectedAdminJobProject = (deps: AdminDataControllerDeps) => openWorkbenchLinkedContext({ mode: "project" }, deps);
export const openSelectedAdminResultProject = (deps: AdminDataControllerDeps) => openWorkbenchLinkedContext({ target: "result", mode: "project" }, deps);
export const applySelectedAdminJobContext = (deps: AdminDataControllerDeps) => openWorkbenchLinkedContext({}, deps);
export const applySelectedAdminResultContext = (deps: AdminDataControllerDeps) => openWorkbenchLinkedContext({ target: "result" }, deps);
