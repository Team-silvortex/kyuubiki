"use client";

type ScriptProjectModelControllerDeps = {
  action: string;
  payload: Record<string, unknown>;
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
  refreshProjects: () => Promise<void>;
  refreshVersions: (modelId: string) => Promise<void>;
  downloadProjectBundleJson: () => Promise<void>;
  downloadProjectBundleZip: () => Promise<void>;
  generateModel: () => void;
  generatePanelModel: () => void;
  serializeCurrentModel: () => Record<string, unknown>;
  createProject: (input: { name: string; description: string }) => Promise<any>;
  updateProject: (projectId: string, input: { name: string; description: string }) => Promise<any>;
  deleteProject: (projectId: string) => Promise<any>;
  createModel: (projectId: string, input: ModelMutationInput) => Promise<any>;
  updateModel: (modelId: string, input: ModelMutationInput) => Promise<any>;
  deleteModel: (modelId: string) => Promise<any>;
  createModelVersion: (modelId: string, input: ModelMutationInput) => Promise<any>;
  updateModelVersion: (versionId: string, input: { name: string }) => Promise<any>;
  deleteModelVersion: (versionId: string) => Promise<any>;
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

type ModelMutationInput = {
  name: string;
  kind: string;
  material?: string;
  model_schema_version?: string;
  payload: Record<string, unknown>;
};

export async function handleWorkbenchScriptProjectModelAction({
  action,
  payload,
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
  createProject,
  updateProject,
  deleteProject,
  createModel,
  updateModel,
  deleteModel,
  createModelVersion,
  updateModelVersion,
  deleteModelVersion,
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
      const name = typeof payload.name === "string" && payload.name.trim() ? payload.name.trim() : defaultProjectLabel;
      const description = typeof payload.description === "string" ? payload.description : "";
      const created = await createProject({ name, description });
      setSelectedProjectId(created.project.project_id);
      setProjectNameDraft(created.project.name);
      setProjectDescriptionDraft(created.project.description ?? "");
      await refreshProjects();
      setMessage(projectCreatedLabel);
      return { ok: true, action, projectId: created.project.project_id };
    }
    case "project/select": {
      const projectId = typeof payload.projectId === "string" ? payload.projectId : null;
      if (projectId) {
        setSelectedProjectId(projectId);
      }
      return { ok: true, action, projectId };
    }
    case "project/updateSelected": {
      if (!selectedProjectId) {
        throw new Error(projectRequiredLabel);
      }
      const name = typeof payload.name === "string" && payload.name.trim() ? payload.name.trim() : projectNameDraft || defaultProjectLabel;
      const description = typeof payload.description === "string" ? payload.description : projectDescriptionDraft;
      await updateProject(selectedProjectId, { name, description });
      setProjectNameDraft(name);
      setProjectDescriptionDraft(description);
      await refreshProjects();
      setMessage(projectUpdatedLabel);
      return { ok: true, action, projectId: selectedProjectId };
    }
    case "project/deleteSelected": {
      if (!selectedProjectId) {
        throw new Error(projectRequiredLabel);
      }
      await deleteProject(selectedProjectId);
      setSelectedProjectId(null);
      setSelectedModelId(null);
      setSelectedVersionId(null);
      await refreshProjects();
      setMessage(projectDeletedLabel);
      return { ok: true, action };
    }
    case "project/exportJson": {
      await downloadProjectBundleJson();
      return { ok: true, action };
    }
    case "project/exportZip": {
      await downloadProjectBundleZip();
      return { ok: true, action };
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
      const payloadModel = serializeCurrentModel();
      const modelPayload = {
        name: loadedModelName,
        kind: studyKind,
        material: activeMaterial,
        model_schema_version: String(payloadModel.model_schema_version ?? "kyuubiki.model/v1"),
        payload: payloadModel,
      };

      if (!selectedModelId || action === "model/saveAs") {
        const created = await createModel(selectedProjectId, modelPayload);
        setSelectedModelId(created.model.model_id);
        setSelectedVersionId(created.model.latest_version_id ?? null);
        await refreshProjects();
        await refreshVersions(created.model.model_id);
        setMessage(modelCreatedLabel);
        return { ok: true, action, modelId: created.model.model_id };
      }

      await updateModel(selectedModelId, modelPayload);
      const version = await createModelVersion(selectedModelId, modelPayload);
      setSelectedVersionId(version.version.version_id);
      await refreshProjects();
      await refreshVersions(selectedModelId);
      setMessage(modelSavedLabel);
      return { ok: true, action, versionId: version.version.version_id };
    }
    case "model/deleteSelected": {
      if (!selectedModelId) {
        throw new Error(noSavedModelsLabel);
      }
      await deleteModel(selectedModelId);
      setSelectedModelId(null);
      setSelectedVersionId(null);
      setModelVersions([]);
      await refreshProjects();
      setMessage(modelDeletedLabel);
      return { ok: true, action };
    }
    case "model/renameSelectedVersion": {
      if (!selectedVersionId) {
        throw new Error(noVersionsLabel);
      }
      await updateModelVersion(selectedVersionId, { name: loadedModelName });
      await refreshVersions(selectedModelId ?? "");
      setMessage(versionRenamedLabel);
      return { ok: true, action, versionId: selectedVersionId };
    }
    case "model/deleteSelectedVersion": {
      if (!selectedVersionId) {
        throw new Error(noVersionsLabel);
      }
      await deleteModelVersion(selectedVersionId);
      setSelectedVersionId(null);
      if (selectedModelId) {
        await refreshVersions(selectedModelId);
      }
      await refreshProjects();
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
