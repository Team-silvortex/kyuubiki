"use client";

import type {
  WorkbenchModelCreateInput,
  WorkbenchProjectLibraryBackendService,
} from "@/lib/workbench/project-library-backend-service-core";
import type { WorkbenchDownloadResult } from "@/components/workbench/workbench-export-controller";
import type { ModelVersionRecord, ProjectRecord } from "@/lib/api/project-types";
import type { WorkbenchProjectContext, WorkbenchProjectRefresh } from "@/lib/workbench/project-context";
import type { CheckpointRecovery, CheckpointVersionOpener } from "@/lib/workbench/checkpoint-recovery";
import type { RemoveConfirmedWorkbenchLibraryRecord } from "./workbench-library-record-recovery";
import { workbenchLibraryRecordSources } from "./workbench-library-record-source";
import { isWorkbenchAdminRecordSourceCurrent } from "./workbench-admin-record-source";
import { getWorkbenchAdminRecordSourceChangedLabel } from "./workbench-admin-record-source-copy";
import { workbenchProjectContextChangedError } from "@/lib/workbench/project-context";
import { captureWorkbenchLibraryWriteSource, type WorkbenchLibraryWriteKind } from "./workbench-library-write-source";

type ScriptProjectModelControllerDeps = {
  language?: string;
  checkpointRecovery?: CheckpointRecovery;
  openModelVersionById?: CheckpointVersionOpener;
  projectContext: WorkbenchProjectContext;
  removeConfirmedLibraryRecord: RemoveConfirmedWorkbenchLibraryRecord;
  action: string;
  payload: Record<string, unknown>;
  projects: ProjectRecord[];
  modelVersions: ModelVersionRecord[];
  selectedProjectId: string | null;
  selectedModelId: string | null;
  selectedVersionId: string | null;
  projectNameDraft: string;
  projectDescriptionDraft: string;
  loadedModelName: string;
  activeMaterial: string;
  studyKind: string;
  setSelectedProjectId: (value: string | null) => void;
  setProjectNameDraft: (value: string) => void;
  setProjectDescriptionDraft: (value: string) => void;
  setSelectedModelId: (value: string | null) => void;
  setSelectedVersionId: (value: string | null) => void;
  setModelVersions: (value: any[]) => void;
  setLoadedModelName: (value: string) => void;
  setActiveMaterial: (value: string) => void;
  refreshProjects: WorkbenchProjectRefresh;
  refreshVersions: (modelId: string) => Promise<void>;
  downloadProjectBundleJson: () => Promise<WorkbenchDownloadResult>;
  downloadProjectBundleZip: () => Promise<WorkbenchDownloadResult>;
  generateModel: () => void;
  generatePanelModel: () => void;
  serializeCurrentModel: () => Record<string, unknown>;
  projectLibraryBackendService: WorkbenchProjectLibraryBackendService;
  projectRequiredLabel: string;
  defaultProjectLabel: string;
  projectCreatedLabel: string;
  projectUpdatedLabel: string;
  projectDeletedLabel: string;
  noSavedModelsLabel: string;
  noVersionsLabel: string;
  modelCreatedLabel: string;
  modelSavedLabel: string;
  modelDeletedLabel: string;
  versionRenamedLabel: string;
  versionDeletedLabel: string;
  setMessage: (value: string) => void;
};

export async function handleWorkbenchScriptProjectModelAction({
  language = "en",
  checkpointRecovery,
  openModelVersionById,
  projectContext,
  removeConfirmedLibraryRecord,
  action,
  payload,
  projects,
  modelVersions,
  selectedProjectId,
  selectedModelId,
  selectedVersionId,
  projectNameDraft,
  projectDescriptionDraft,
  loadedModelName,
  activeMaterial,
  studyKind,
  setSelectedProjectId,
  setProjectNameDraft,
  setProjectDescriptionDraft,
  setSelectedModelId,
  setSelectedVersionId,
  setModelVersions,
  setLoadedModelName,
  setActiveMaterial,
  refreshProjects,
  refreshVersions,
  downloadProjectBundleJson,
  downloadProjectBundleZip,
  generateModel,
  generatePanelModel,
  serializeCurrentModel,
  projectLibraryBackendService,
  projectRequiredLabel,
  defaultProjectLabel,
  projectCreatedLabel,
  projectUpdatedLabel,
  projectDeletedLabel,
  noSavedModelsLabel,
  noVersionsLabel,
  modelCreatedLabel,
  modelSavedLabel,
  modelDeletedLabel,
  versionRenamedLabel,
  versionDeletedLabel,
  setMessage,
}: ScriptProjectModelControllerDeps): Promise<Record<string, unknown> | null> {
  const captureWriteSource = (kind: WorkbenchLibraryWriteKind) => {
    try {
      return captureWorkbenchLibraryWriteSource({ kind, projectId: selectedProjectId, modelId: selectedModelId,
        versionId: selectedVersionId, projects, versions: modelVersions, projectContext,
        sourceChangedLabel: getWorkbenchAdminRecordSourceChangedLabel(language) });
    } catch (error) {
      if (projectContext.capture()()) setMessage(error instanceof Error ? error.message : String(error));
      throw error;
    }
  };
  switch (action) {
    case "model/listPendingSaves":
    case "model/checkPendingSave":
    case "model/openRecoveredSave":
    case "model/acknowledgeSave": {
      if (!checkpointRecovery) throw new Error("checkpoint:recovery_unavailable");
      if (action === "model/listPendingSaves") return { ok: true, action, pending: await checkpointRecovery.list() };
      const key = payload.key as string;
      if (action === "model/checkPendingSave") return { ok: true, action, checkpoint: await checkpointRecovery.check(key) };
      if (action === "model/acknowledgeSave") return { ok: true, action, checkpoint: await checkpointRecovery.acknowledge(key) };
      if (!openModelVersionById) throw new Error("checkpoint:recovery_unavailable");
      const isCurrent = projectContext.begin();
      return { ok: true, action, checkpoint: await checkpointRecovery.open(key, (id, guard) => {
        if (!isCurrent()) throw new Error("checkpoint:context_changed");
        return openModelVersionById(id, guard);
      }) };
    }
    case "project/create": {
      const ownsSource = captureWriteSource("createProject");
      const isCurrent = projectContext.begin();
      const name = typeof payload.name === "string" && payload.name.trim() ? payload.name.trim() : defaultProjectLabel;
      const description = typeof payload.description === "string" ? payload.description : "";
      const created = await projectLibraryBackendService.createProject({ name, description });
      if (!ownsSource()) return { ok: true, action, projectId: created.project.project_id, contextChanged: true };
      await refreshProjects(false, undefined, { preserveSelection: true });
      if (!isCurrent() || !ownsSource()) return { ok: true, action, projectId: created.project.project_id, contextChanged: true };
      setSelectedProjectId(created.project.project_id);
      setSelectedModelId(null);
      setSelectedVersionId(null);
      setModelVersions([]);
      setProjectNameDraft(created.project.name);
      setProjectDescriptionDraft(created.project.description ?? "");
      setMessage(projectCreatedLabel);
      return { ok: true, action, projectId: created.project.project_id };
    }
    case "project/select": {
      const projectId = typeof payload.projectId === "string" ? payload.projectId.trim() : null;
      const project = projects?.find((entry) => entry.project_id === projectId);
      if (!project && payload.projectId !== null) throw new Error(projectRequiredLabel);
      if (!projectContext.capture()()) throw workbenchProjectContextChangedError();
      const source = workbenchLibraryRecordSources.sourceOf(projects);
      if (project && !isWorkbenchAdminRecordSourceCurrent(source, source)) {
        const message = `WORKBENCH_RECORD_SOURCE_CHANGED: ${getWorkbenchAdminRecordSourceChangedLabel(language)}`;
        setMessage(message);
        throw new Error(message);
      }
      // Choosing a project does not load geometry, even when another backend reused its ID.
      projectContext.begin();
      projectContext.update({ projectId: project?.project_id ?? null, modelId: null, versionId: null });
      setSelectedModelId(null);
      setSelectedVersionId(null);
      setModelVersions([]);
      setSelectedProjectId(project?.project_id ?? null);
      setProjectNameDraft(project?.name ?? "");
      setProjectDescriptionDraft(project?.description ?? "");
      return { ok: true, action, projectId: project?.project_id ?? null };
    }
    case "project/updateSelected": {
      if (!selectedProjectId) {
        throw new Error(projectRequiredLabel);
      }
      const ownsSource = captureWriteSource("project");
      const isCurrent = projectContext.begin();
      const name = typeof payload.name === "string" && payload.name.trim() ? payload.name.trim() : projectNameDraft || defaultProjectLabel;
      const description = typeof payload.description === "string" ? payload.description : projectDescriptionDraft;
      await projectLibraryBackendService.updateProject(selectedProjectId, { name, description });
      if (!ownsSource()) return { ok: true, action, projectId: selectedProjectId, contextChanged: true };
      await refreshProjects(false, undefined, { preserveSelection: true });
      if (!isCurrent() || !ownsSource()) return { ok: true, action, projectId: selectedProjectId, contextChanged: true };
      setProjectNameDraft(name);
      setProjectDescriptionDraft(description);
      setMessage(projectUpdatedLabel);
      return { ok: true, action, projectId: selectedProjectId };
    }
    case "project/deleteSelected": {
      if (!selectedProjectId) {
        throw new Error(projectRequiredLabel);
      }
      const ownsSource = captureWriteSource("project");
      const isCurrent = projectContext.begin();
      await projectLibraryBackendService.deleteProject(selectedProjectId);
      if (!ownsSource()) return { ok: true, action, projectId: selectedProjectId, contextChanged: true };
      const contextChanged = !isCurrent();
      removeConfirmedLibraryRecord("projectId", selectedProjectId);
      if (projectContext.detachDeleted("projectId", selectedProjectId)) {
        setSelectedProjectId(null);
        setSelectedModelId(null);
        setSelectedVersionId(null);
        setModelVersions([]);
      }
      const ownsFeedback = projectContext.capture();
      await refreshProjects(false, undefined, { preserveSelection: true });
      if (contextChanged || !ownsFeedback() || !ownsSource()) return { ok: true, action, projectId: selectedProjectId, contextChanged: true };
      setMessage(projectDeletedLabel);
      return { ok: true, action };
    }
    case "project/exportJson": {
      const download = await downloadProjectBundleJson();
      if (!download.ok) throw download.error;
      return { ok: true, action, partial: download.partial ?? false };
    }
    case "project/exportZip": {
      const download = await downloadProjectBundleZip();
      if (!download.ok) throw download.error;
      return { ok: true, action, partial: download.partial ?? false };
    }
    case "model/generateTruss": {
      generateModel();
      return { ok: true, action };
    }
    case "model/generatePanel": {
      generatePanelModel();
      return { ok: true, action };
    }
    case "model/save":
    case "model/saveAs": {
      if (!selectedProjectId) {
        throw new Error(projectRequiredLabel);
      }
      if (payload.name !== undefined && (typeof payload.name !== "string" || !payload.name.trim() || payload.name.length > 256)) {
        throw new Error("model:invalid_name");
      }
      if (payload.material !== undefined && (typeof payload.material !== "string" || payload.material.length > 256)) {
        throw new Error("model:invalid_material");
      }
      if (payload.request_id !== undefined && (typeof payload.request_id !== "string" || !/^[A-Za-z0-9_-]{16,128}$/.test(payload.request_id))) {
        throw new Error("model:invalid_checkpoint_request_id");
      }
      const name = typeof payload.name === "string" ? payload.name.trim() : loadedModelName;
      const material = typeof payload.material === "string" ? payload.material.trim() : activeMaterial;
      const ownsSource = captureWriteSource(!selectedModelId || action === "model/saveAs" ? "project" : "model");
      let isCurrent = projectContext.begin();
      const payloadModel: Record<string, unknown> = { ...serializeCurrentModel(), name, material };
      const modelPayload: WorkbenchModelCreateInput = {
        ...(typeof payload.request_id === "string" ? { request_id: payload.request_id } : {}),
        name,
        kind: studyKind,
        material,
        model_schema_version: String(payloadModel.model_schema_version ?? "kyuubiki.model/v1"),
        payload: payloadModel,
      };

      if (!selectedModelId || action === "model/saveAs") {
        const created = await projectLibraryBackendService.createModel(selectedProjectId, modelPayload);
        if (!ownsSource()) return { ok: true, action, modelId: created.model.model_id, contextChanged: true };
        await refreshProjects(false, undefined, { preserveSelection: true });
        if (!isCurrent() || !ownsSource()) return { ok: true, action, modelId: created.model.model_id, contextChanged: true };
        isCurrent = projectContext.update({ projectId: selectedProjectId, modelId: created.model.model_id,
          versionId: created.model.latest_version_id ?? null });
        setSelectedModelId(created.model.model_id);
        setSelectedVersionId(created.model.latest_version_id ?? null);
        if (payload.name !== undefined) setLoadedModelName(name);
        if (payload.material !== undefined) setActiveMaterial(material);
        setMessage(modelCreatedLabel);
        await refreshVersions(created.model.model_id);
        if (!isCurrent() || !ownsSource()) return { ok: true, action, modelId: created.model.model_id, contextChanged: true };
        return { ok: true, action, modelId: created.model.model_id };
      }

      // Checkpoint creation already commits the model metadata, payload and version atomically.
      const version = await projectLibraryBackendService.createModelVersion(selectedModelId, modelPayload);
      if (!ownsSource()) return { ok: true, action, versionId: version.version.version_id, contextChanged: true };
      await refreshProjects(false, undefined, { preserveSelection: true });
      if (!isCurrent() || !ownsSource()) return { ok: true, action, versionId: version.version.version_id, contextChanged: true };
      isCurrent = projectContext.update({ projectId: selectedProjectId, modelId: selectedModelId,
        versionId: version.version.version_id });
      setSelectedVersionId(version.version.version_id);
      if (payload.name !== undefined) setLoadedModelName(name);
      if (payload.material !== undefined) setActiveMaterial(material);
      setMessage(modelSavedLabel);
      await refreshVersions(selectedModelId);
      if (!isCurrent() || !ownsSource()) return { ok: true, action, versionId: version.version.version_id, contextChanged: true };
      return { ok: true, action, versionId: version.version.version_id };
    }
    case "model/deleteSelected": {
      if (!selectedModelId) {
        throw new Error(noSavedModelsLabel);
      }
      const ownsSource = captureWriteSource("model");
      const isCurrent = projectContext.begin();
      await projectLibraryBackendService.deleteModel(selectedModelId);
      if (!ownsSource()) return { ok: true, action, modelId: selectedModelId, contextChanged: true };
      const contextChanged = !isCurrent();
      removeConfirmedLibraryRecord("modelId", selectedModelId);
      if (projectContext.detachDeleted("modelId", selectedModelId)) {
        setSelectedModelId(null);
        setSelectedVersionId(null);
        setModelVersions([]);
      }
      const ownsFeedback = projectContext.capture();
      await refreshProjects(false, undefined, { preserveSelection: true });
      if (contextChanged || !ownsFeedback() || !ownsSource()) return { ok: true, action, modelId: selectedModelId, contextChanged: true };
      setMessage(modelDeletedLabel);
      return { ok: true, action };
    }
    case "model/renameSelectedVersion": {
      if (!selectedVersionId) {
        throw new Error(noVersionsLabel);
      }
      const ownsSource = captureWriteSource("version");
      const isCurrent = projectContext.begin();
      await projectLibraryBackendService.updateModelVersion(selectedVersionId, { name: loadedModelName });
      if (!ownsSource()) return { ok: true, action, versionId: selectedVersionId, contextChanged: true };
      if (selectedModelId && projectContext.hasModel(selectedModelId)) await refreshVersions(selectedModelId);
      if (!isCurrent() || !ownsSource()) return { ok: true, action, versionId: selectedVersionId, contextChanged: true };
      setMessage(versionRenamedLabel);
      return { ok: true, action, versionId: selectedVersionId };
    }
    case "model/deleteSelectedVersion": {
      if (!selectedVersionId) {
        throw new Error(noVersionsLabel);
      }
      const ownsSource = captureWriteSource("version");
      const isCurrent = projectContext.begin();
      const removed = await projectLibraryBackendService.deleteModelVersion(selectedVersionId);
      if (!ownsSource()) return { ok: true, action, versionId: selectedVersionId, contextChanged: true };
      const contextChanged = !isCurrent();
      removeConfirmedLibraryRecord("versionId", selectedVersionId, removed.version.model_id);
      if (projectContext.detachDeleted("versionId", selectedVersionId)) setSelectedVersionId(null);
      const ownsFeedback = projectContext.capture();
      await Promise.all([refreshProjects(false, undefined, { preserveSelection: true }),
        selectedModelId && projectContext.hasModel(selectedModelId) ? refreshVersions(selectedModelId) : Promise.resolve()]);
      if (contextChanged || !ownsFeedback() || !ownsSource()) return { ok: true, action, versionId: selectedVersionId, contextChanged: true };
      setMessage(versionDeletedLabel);
      return { ok: true, action };
    }
    case "model/setWorkspaceMeta": {
      if (typeof payload.loadedModelName === "string" || typeof payload.activeMaterial === "string") {
        if (!projectContext.capture()()) throw workbenchProjectContextChangedError();
        projectContext.begin();
      }
      if (typeof payload.loadedModelName === "string") {
        setLoadedModelName(payload.loadedModelName);
      }
      if (typeof payload.activeMaterial === "string") {
        setActiveMaterial(payload.activeMaterial);
      }
      return { ok: true, action };
    }
    default:
      return null;
  }
}
