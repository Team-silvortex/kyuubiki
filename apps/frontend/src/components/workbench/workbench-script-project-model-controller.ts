"use client";

import type {
  WorkbenchModelCreateInput,
  WorkbenchProjectLibraryBackendService,
} from "@/lib/workbench/project-library-backend-service-core";
import type { WorkbenchDownloadResult } from "@/components/workbench/workbench-export-controller";
import type { ProjectRecord } from "@/lib/api/project-types";
import type { WorkbenchProjectContext, WorkbenchProjectRefresh } from "@/lib/workbench/project-context";

type ScriptProjectModelControllerDeps = {
  projectContext: WorkbenchProjectContext;
  action: string;
  payload: Record<string, unknown>;
  projects: ProjectRecord[];
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
  projectContext,
  action,
  payload,
  projects,
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
  switch (action) {
    case "project/create": {
      const isCurrent = projectContext.begin();
      const name = typeof payload.name === "string" && payload.name.trim() ? payload.name.trim() : defaultProjectLabel;
      const description = typeof payload.description === "string" ? payload.description : "";
      const created = await projectLibraryBackendService.createProject({ name, description });
      await refreshProjects(false, undefined, { preserveSelection: true });
      if (!isCurrent()) return { ok: true, action, projectId: created.project.project_id, contextChanged: true };
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
      if (!project) throw new Error(projectRequiredLabel);
      if (projectId !== selectedProjectId) {
        setSelectedModelId(null);
        setSelectedVersionId(null);
        setModelVersions([]);
      }
      setSelectedProjectId(project.project_id);
      setProjectNameDraft(project.name);
      setProjectDescriptionDraft(project.description ?? "");
      return { ok: true, action, projectId };
    }
    case "project/updateSelected": {
      if (!selectedProjectId) {
        throw new Error(projectRequiredLabel);
      }
      const isCurrent = projectContext.begin();
      const name = typeof payload.name === "string" && payload.name.trim() ? payload.name.trim() : projectNameDraft || defaultProjectLabel;
      const description = typeof payload.description === "string" ? payload.description : projectDescriptionDraft;
      await projectLibraryBackendService.updateProject(selectedProjectId, { name, description });
      await refreshProjects(false, undefined, { preserveSelection: true });
      if (!isCurrent()) return { ok: true, action, projectId: selectedProjectId, contextChanged: true };
      setProjectNameDraft(name);
      setProjectDescriptionDraft(description);
      setMessage(projectUpdatedLabel);
      return { ok: true, action, projectId: selectedProjectId };
    }
    case "project/deleteSelected": {
      if (!selectedProjectId) {
        throw new Error(projectRequiredLabel);
      }
      const isCurrent = projectContext.begin();
      await projectLibraryBackendService.deleteProject(selectedProjectId);
      await refreshProjects(false, undefined, { preserveSelection: true });
      const contextChanged = !isCurrent();
      if (projectContext.detachDeleted("projectId", selectedProjectId)) {
        setSelectedProjectId(null);
        setSelectedModelId(null);
        setSelectedVersionId(null);
        setModelVersions([]);
      }
      if (contextChanged) return { ok: true, action, projectId: selectedProjectId, contextChanged: true };
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
      const isCurrent = projectContext.begin();
      const payloadModel = serializeCurrentModel();
      const modelPayload: WorkbenchModelCreateInput = {
        name: loadedModelName,
        kind: studyKind,
        material: activeMaterial,
        model_schema_version: String(payloadModel.model_schema_version ?? "kyuubiki.model/v1"),
        payload: payloadModel,
      };

      if (!selectedModelId || action === "model/saveAs") {
        const created = await projectLibraryBackendService.createModel(selectedProjectId, modelPayload);
        await refreshProjects(false, undefined, { preserveSelection: true });
        if (!isCurrent()) return { ok: true, action, modelId: created.model.model_id, contextChanged: true };
        setSelectedModelId(created.model.model_id);
        setSelectedVersionId(created.model.latest_version_id ?? null);
        setMessage(modelCreatedLabel);
        await refreshVersions(created.model.model_id);
        return { ok: true, action, modelId: created.model.model_id };
      }

      await projectLibraryBackendService.updateModel(selectedModelId, modelPayload);
      const version = await projectLibraryBackendService.createModelVersion(selectedModelId, modelPayload);
      await refreshProjects(false, undefined, { preserveSelection: true });
      if (!isCurrent()) return { ok: true, action, versionId: version.version.version_id, contextChanged: true };
      setSelectedVersionId(version.version.version_id);
      setMessage(modelSavedLabel);
      await refreshVersions(selectedModelId);
      return { ok: true, action, versionId: version.version.version_id };
    }
    case "model/deleteSelected": {
      if (!selectedModelId) {
        throw new Error(noSavedModelsLabel);
      }
      const isCurrent = projectContext.begin();
      await projectLibraryBackendService.deleteModel(selectedModelId);
      await refreshProjects(false, undefined, { preserveSelection: true });
      const contextChanged = !isCurrent();
      if (projectContext.detachDeleted("modelId", selectedModelId)) {
        setSelectedModelId(null);
        setSelectedVersionId(null);
        setModelVersions([]);
      }
      if (contextChanged) return { ok: true, action, modelId: selectedModelId, contextChanged: true };
      setMessage(modelDeletedLabel);
      return { ok: true, action };
    }
    case "model/renameSelectedVersion": {
      if (!selectedVersionId) {
        throw new Error(noVersionsLabel);
      }
      const isCurrent = projectContext.begin();
      await projectLibraryBackendService.updateModelVersion(selectedVersionId, { name: loadedModelName });
      if (selectedModelId && projectContext.hasModel(selectedModelId)) await refreshVersions(selectedModelId);
      if (!isCurrent()) return { ok: true, action, versionId: selectedVersionId, contextChanged: true };
      setMessage(versionRenamedLabel);
      return { ok: true, action, versionId: selectedVersionId };
    }
    case "model/deleteSelectedVersion": {
      if (!selectedVersionId) {
        throw new Error(noVersionsLabel);
      }
      const isCurrent = projectContext.begin();
      await projectLibraryBackendService.deleteModelVersion(selectedVersionId);
      await refreshProjects(false, undefined, { preserveSelection: true });
      if (selectedModelId && projectContext.hasModel(selectedModelId)) await refreshVersions(selectedModelId);
      const contextChanged = !isCurrent();
      if (projectContext.detachDeleted("versionId", selectedVersionId)) setSelectedVersionId(null);
      if (contextChanged) return { ok: true, action, versionId: selectedVersionId, contextChanged: true };
      setMessage(versionDeletedLabel);
      return { ok: true, action };
    }
    case "model/setWorkspaceMeta": {
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
