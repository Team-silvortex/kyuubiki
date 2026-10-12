import type { ModelVersionRecord, ProjectRecord } from "@/lib/api/project-types";
import { captureWorkbenchApiAuthority } from "@/lib/api/auth-context";
import { workbenchProjectContextChangedError, type WorkbenchProjectContext } from "@/lib/workbench/project-context";
import { isWorkbenchAdminRecordSourceCurrent } from "./workbench-admin-record-source";
import { workbenchLibraryRecordSources, workbenchLibraryVersionSources } from "./workbench-library-record-source";

export type WorkbenchLibraryWriteKind = "createProject" | "project" | "model" | "version";

export function captureWorkbenchLibraryWriteSource({ kind, projectId, modelId, versionId, projects, versions,
  projectContext, sourceChangedLabel }: {
  kind: WorkbenchLibraryWriteKind;
  projectId: string | null;
  modelId: string | null;
  versionId: string | null;
  projects: ProjectRecord[];
  versions: ModelVersionRecord[];
  projectContext: WorkbenchProjectContext;
  sourceChangedLabel: string;
}) {
  if (!projectContext.capture()()) throw workbenchProjectContextChangedError();
  const selection = projectContext.current();
  if (kind !== "createProject" && (selection.projectId !== projectId
    || (kind !== "project" && selection.modelId !== modelId)
    || (kind === "version" && selection.versionId !== versionId))) throw workbenchProjectContextChangedError();
  const source = kind === "version" ? workbenchLibraryVersionSources.sourceOf(versions)
    : workbenchLibraryRecordSources.sourceOf(projects);
  const ownsSource = () => kind === "createProject" || isWorkbenchAdminRecordSourceCurrent(source, source);
  const project = projects?.find((entry) => entry.project_id === projectId);
  const identified = (value: unknown) => typeof value === "string" && Boolean(value.trim());
  const member = kind === "createProject" || (identified(projectId) && (kind === "version"
    ? identified(modelId) && identified(versionId) && Array.isArray(versions) && versions.some((version) =>
      version.version_id === versionId && version.model_id === modelId && version.project_id === projectId)
    : Boolean(project) && (kind === "project" || (identified(modelId) && Array.isArray(project?.models)
      && project.models.some((model) => model.model_id === modelId && model.project_id === projectId)))));
  if (!member || !ownsSource()) throw new Error(`WORKBENCH_RECORD_SOURCE_CHANGED: ${sourceChangedLabel}`);
  const url = kind === "createProject" ? "/api/v1/projects"
    : kind === "project" ? `/api/v1/projects/${encodeURIComponent(projectId!)}`
    : kind === "model" ? `/api/v1/models/${encodeURIComponent(modelId!)}`
    : `/api/v1/model-versions/${encodeURIComponent(versionId!)}`;
  const ownsAuthority = captureWorkbenchApiAuthority(url);
  // Keep receipt cleanup separate from workspace ownership: same-source deletion still reconciles its record.
  return () => { try { return projectContext.capture()() && ownsSource() && ownsAuthority(); } catch { return false; } };
}
