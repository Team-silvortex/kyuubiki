import type { Dispatch, SetStateAction } from "react";
import type { ModelRecord, ModelVersionRecord, ProjectRecord } from "@/lib/api/project-types";
import type { WorkbenchProjectContext, WorkbenchProjectSelection } from "@/lib/workbench/project-context";
import { workbenchLibraryRecordSources, workbenchLibraryVersionSources } from "./workbench-library-record-source";

export type WorkbenchLibraryRemovalKind = keyof WorkbenchProjectSelection;
export type RemoveConfirmedWorkbenchLibraryRecord = (kind: WorkbenchLibraryRemovalKind, id: string, modelId?: string) => void;

function withoutVersion(model: ModelRecord, id: string): ModelRecord {
  const latestRemoved = model.latest_version_id === id;
  if (!latestRemoved && !model.versions?.some((version) => version.version_id === id)) return model;
  return {
    ...model,
    // The receipt does not identify the replacement latest version; do not invent one from a partial list.
    ...(latestRemoved ? { latest_version_id: null, latest_version_number: null } : {}),
    ...(model.versions ? { versions: model.versions.filter((version) => version.version_id !== id) } : {}),
  };
}

export function removeConfirmedWorkbenchLibraryRecord({
  kind, id, modelId, projectContext, projectRefreshSeqRef, versionsRefreshSeqRef, versionListModelIdRef,
  setProjects, setModelVersions,
}: {
  kind: WorkbenchLibraryRemovalKind;
  id: string;
  modelId?: string;
  projectContext: WorkbenchProjectContext;
  projectRefreshSeqRef: { current: number };
  versionsRefreshSeqRef: { current: number };
  versionListModelIdRef: { current: string | null };
  setProjects: Dispatch<SetStateAction<ProjectRecord[]>>;
  setModelVersions: Dispatch<SetStateAction<ModelVersionRecord[]>>;
}) {
  if (!projectContext.capture()()) return;
  projectRefreshSeqRef.current += 1;
  const versionScope = versionListModelIdRef.current;
  if ((kind === "projectId" && projectContext.current().projectId === id) ||
      (kind === "modelId" && versionScope === id) || (kind === "versionId" && versionScope === modelId)) {
    versionsRefreshSeqRef.current += 1;
  }
  setProjects((projects) => kind === "projectId" ? workbenchLibraryRecordSources.filter(projects, (project) => project.project_id !== id)
    : workbenchLibraryRecordSources.map(projects, (project) => {
      if (!project.models) return project;
      const models = kind === "modelId" ? project.models.filter((model) => model.model_id !== id)
        : project.models.map((model) => withoutVersion(model, id));
      return models.length === project.models.length && models.every((model, index) => model === project.models![index])
        ? project : { ...project, models };
    }));
  setModelVersions((versions) => workbenchLibraryVersionSources.filter(versions, (version) => kind === "projectId" ? version.project_id !== id
    : kind === "modelId" ? version.model_id !== id : version.version_id !== id));
}
