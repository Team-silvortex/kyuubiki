"use client";

import {
  downloadWorkbenchDatabaseSnapshot,
  downloadWorkbenchProjectBundleJson,
  downloadWorkbenchProjectBundleZip,
} from "@/components/workbench/workbench-export-controller";
import {
  importWorkbenchProjectBundle,
  openPersistedWorkbenchModel,
  openPersistedWorkbenchVersion,
  openPersistedWorkbenchVersionById,
} from "@/components/workbench/workbench-persisted-model-controller";
import type { JobResultRecord } from "@/lib/api";
import { exportProjectBundle } from "@/lib/models/modeler";
import { listWorkbenchMacroPresets, listWorkbenchSnippetPresets } from "@/lib/scripting/workbench-script-runtime";
import { readWorkspaceStoreManifest } from "@/lib/workbench/store-manifest";

type ProjectStorageControllerDeps = {
  t: any;
  getSelectedProject: () => any;
  selectedProjectId: string | null;
  getSelectedProjectModels: () => any[];
  selectedModelId: string | null;
  selectedVersionId: string | null;
  projectNameDraft: string;
  projectDescriptionDraft: string;
  loadedModelName: string;
  activeMaterial: string;
  studyKind: string;
  jobHistory: any[];
  axialForm: any;
  heatBarModel: any;
  heatPlaneModel: any;
  thermalBarModel: any;
  thermalBeamModel: any;
  thermalFrameModel: any;
  thermalTrussModel: any;
  trussModel: any;
  thermalTruss3dModel: any;
  truss3dModel: any;
  planeModel: any;
  frameModel: any;
  beamModel: any;
  torsionModel: any;
  springModel: any;
  spring2dModel: any;
  spring3dModel: any;
  parametric: any;
  round: (value: number, digits?: number) => number;
  startTransition: (callback: () => void) => void;
  setMessage: (value: string) => void;
  setSelectedProjectId: (value: string | null) => void;
  setSelectedModelId: (value: string | null) => void;
  setSelectedVersionId: (value: string | null) => void;
  setModelVersions: (value: any[]) => void;
  createProject: (input: { name: string; description: string }) => Promise<any>;
  updateProject: (projectId: string, input: { name: string; description: string }) => Promise<any>;
  deleteProject: (projectId: string) => Promise<any>;
  createModel: (projectId: string, input: any) => Promise<any>;
  updateModel: (modelId: string, input: any) => Promise<any>;
  deleteModel: (modelId: string) => Promise<any>;
  createModelVersion: (modelId: string, input: any) => Promise<any>;
  updateModelVersion: (versionId: string, input: { name: string }) => Promise<any>;
  deleteModelVersion: (versionId: string) => Promise<any>;
  fetchModel: (modelId: string) => Promise<any>;
  fetchModelVersion: (versionId: string) => Promise<any>;
  fetchModelVersions: (modelId: string) => Promise<any>;
  fetchJobStatus: (jobId: string) => Promise<any>;
  refreshProjects: () => Promise<void>;
  refreshVersions: (modelId: string) => Promise<void>;
  serializeCurrentModel: () => Record<string, unknown>;
  getPersistedModelEffects: () => any;
};

export function createWorkbenchProjectStorageController({
  t,
  getSelectedProject,
  selectedProjectId,
  getSelectedProjectModels,
  selectedModelId,
  selectedVersionId,
  projectNameDraft,
  projectDescriptionDraft,
  loadedModelName,
  activeMaterial,
  studyKind,
  jobHistory,
  axialForm,
  heatBarModel,
  heatPlaneModel,
  thermalBarModel,
  thermalBeamModel,
  thermalFrameModel,
  thermalTrussModel,
  trussModel,
  thermalTruss3dModel,
  truss3dModel,
  planeModel,
  frameModel,
  beamModel,
  torsionModel,
  springModel,
  spring2dModel,
  spring3dModel,
  parametric,
  round,
  startTransition,
  setMessage,
  setSelectedProjectId,
  setSelectedModelId,
  setSelectedVersionId,
  setModelVersions,
  createProject,
  updateProject,
  deleteProject,
  createModel,
  updateModel,
  deleteModel,
  createModelVersion,
  updateModelVersion,
  deleteModelVersion,
  fetchModel,
  fetchModelVersion,
  fetchModelVersions,
  fetchJobStatus,
  refreshProjects,
  refreshVersions,
  serializeCurrentModel,
  getPersistedModelEffects,
}: ProjectStorageControllerDeps) {
  const buildProjectBundleJson = async () => {
    const selectedProject = getSelectedProject();
    const selectedProjectModels = getSelectedProjectModels();

    if (!selectedProject) {
      throw new Error(t.projectRequired);
    }

    const modelDetailsSettled = await Promise.allSettled(
      selectedProjectModels.map(async (model) => {
        const modelEnvelope = await fetchModel(model.model_id);
        const versionsEnvelope = await fetchModelVersions(model.model_id);
        return {
          model: modelEnvelope.model,
          versions: versionsEnvelope.versions,
        };
      }),
    );

    const modelDetails = modelDetailsSettled.flatMap((entry) => (entry.status === "fulfilled" ? [entry.value] : []));

    const resultCandidatesSettled = await Promise.allSettled(
      jobHistory
        .filter((historyJob) => historyJob.has_result)
        .map(async (historyJob) => {
          try {
            const payload = await fetchJobStatus(historyJob.job_id);

            if (!payload.result) {
              return null;
            }

            return {
              job_id: historyJob.job_id,
              status: payload.job.status,
              worker_id: payload.job.worker_id,
              result: payload.result,
            };
          } catch {
            return null;
          }
        }),
    );

    const resultCandidates = resultCandidatesSettled.flatMap((entry): Array<JobResultRecord | null> =>
      entry.status === "fulfilled" ? [entry.value as JobResultRecord | null] : [],
    );
    const results = resultCandidates.filter((entry): entry is JobResultRecord => entry !== null);
    const partial =
      modelDetails.length !== selectedProjectModels.length ||
      resultCandidatesSettled.some((entry) => entry.status === "rejected");

    return {
      bundle: exportProjectBundle({
        project: selectedProject,
        models: modelDetails.map((entry) => entry.model),
        modelVersions: modelDetails.flatMap((entry) => entry.versions),
        activeModelId: selectedModelId,
        activeVersionId: selectedVersionId,
        workspaceSnapshot: serializeCurrentModel(),
        automationPresets: listWorkbenchMacroPresets(selectedProject.project_id),
        snippetPresets: listWorkbenchSnippetPresets(selectedProject.project_id),
        storeManifest: readWorkspaceStoreManifest(selectedProject.project_id),
        jobs: jobHistory,
        results,
      }),
      partial,
    };
  };

  const downloadProjectBundleJson = async () => {
    const selectedProject = getSelectedProject();
    await downloadWorkbenchProjectBundleJson({
      selectedProject,
      buildBundle: buildProjectBundleJson,
      setMessage,
      labels: {
        projectExported: t.projectExported,
        projectExportedPartial: t.projectExportedPartial,
        initialFailed: t.initialFailed,
      },
    });
  };

  const downloadProjectBundleZip = async () => {
    const selectedProject = getSelectedProject();
    await downloadWorkbenchProjectBundleZip({
      selectedProject,
      buildBundle: buildProjectBundleJson,
      setMessage,
      labels: {
        projectExported: t.projectExported,
        projectExportedPartial: t.projectExportedPartial,
        initialFailed: t.initialFailed,
      },
    });
  };

  const downloadDatabaseSnapshot = async () => {
    await downloadWorkbenchDatabaseSnapshot({
      setMessage,
      labels: {
        databaseExported: t.databaseExported,
        initialFailed: t.initialFailed,
      },
    });
  };

  const importProjectBundle = async (file: File | undefined) => {
    await importWorkbenchProjectBundle(file, getPersistedModelEffects());
  };

  const createProjectRecord = () => {
    startTransition(async () => {
      try {
        const payload = await createProject({
          name: projectNameDraft || t.defaultProject,
          description: projectDescriptionDraft,
        });
        setSelectedProjectId(payload.project.project_id);
        await refreshProjects();
        setMessage(t.projectCreated);
      } catch (error) {
        setMessage(error instanceof Error ? error.message : t.initialFailed);
      }
    });
  };

  const updateProjectRecord = () => {
    if (!selectedProjectId) return;

    startTransition(async () => {
      try {
        await updateProject(selectedProjectId, {
          name: projectNameDraft || t.defaultProject,
          description: projectDescriptionDraft,
        });
        await refreshProjects();
        setMessage(t.projectUpdated);
      } catch (error) {
        setMessage(error instanceof Error ? error.message : t.initialFailed);
      }
    });
  };

  const deleteProjectRecord = () => {
    if (!selectedProjectId) return;
    if (typeof window !== "undefined" && !window.confirm(projectNameDraft)) return;

    startTransition(async () => {
      try {
        await deleteProject(selectedProjectId);
        setSelectedModelId(null);
        setSelectedVersionId(null);
        await refreshProjects();
        setMessage(t.projectDeleted);
      } catch (error) {
        setMessage(error instanceof Error ? error.message : t.initialFailed);
      }
    });
  };

  const saveModelVersion = (saveAs: boolean) => {
    if (!selectedProjectId) {
      setMessage(t.projectRequired);
      return;
    }

    const payload = serializeCurrentModel() as Record<string, unknown> & { model_schema_version?: string };

    startTransition(async () => {
      try {
        const modelPayload = {
          name: loadedModelName,
          kind: studyKind,
          material: activeMaterial,
          model_schema_version: String(payload.model_schema_version ?? "kyuubiki.model/v1"),
          payload,
        };

        if (!selectedModelId || saveAs) {
          const created = await createModel(selectedProjectId, modelPayload);
          setSelectedModelId(created.model.model_id);
          setSelectedVersionId(created.model.latest_version_id ?? null);
          await refreshProjects();
          await refreshVersions(created.model.model_id);
          setMessage(t.modelCreated);
          return;
        }

        await updateModel(selectedModelId, modelPayload);
        const version = await createModelVersion(selectedModelId, modelPayload);

        setSelectedVersionId(version.version.version_id);
        await refreshProjects();
        await refreshVersions(selectedModelId);
        setMessage(t.modelSaved);
      } catch (error) {
        setMessage(error instanceof Error ? error.message : t.initialFailed);
      }
    });
  };

  const openSavedModel = (model: any) => {
    openPersistedWorkbenchModel(model, getPersistedModelEffects());
  };

  const openSavedVersion = (version: any) => {
    openPersistedWorkbenchVersion(version, getPersistedModelEffects());
  };

  const openModelVersionById = (versionId: string) => {
    openPersistedWorkbenchVersionById(versionId, getPersistedModelEffects());
  };

  const renameSelectedVersion = () => {
    if (!selectedVersionId) return;

    startTransition(async () => {
      try {
        await updateModelVersion(selectedVersionId, { name: loadedModelName });
        await refreshVersions(selectedModelId ?? "");
        setMessage(t.versionRenamed);
      } catch (error) {
        setMessage(error instanceof Error ? error.message : t.initialFailed);
      }
    });
  };

  const deleteSelectedVersion = () => {
    if (!selectedVersionId) return;

    startTransition(async () => {
      try {
        await deleteModelVersion(selectedVersionId);
        setSelectedVersionId(null);
        if (selectedModelId) {
          await refreshVersions(selectedModelId);
        }
        await refreshProjects();
        setMessage(t.versionDeleted);
      } catch (error) {
        setMessage(error instanceof Error ? error.message : t.initialFailed);
      }
    });
  };

  const deleteSavedModelRecord = () => {
    if (!selectedModelId) return;

    startTransition(async () => {
      try {
        await deleteModel(selectedModelId);
        setSelectedModelId(null);
        setSelectedVersionId(null);
        setModelVersions([]);
        await refreshProjects();
        setMessage(t.modelDeletedStored);
      } catch (error) {
        setMessage(error instanceof Error ? error.message : t.initialFailed);
      }
    });
  };

  return {
    buildProjectBundleJson,
    createProjectRecord,
    deleteProjectRecord,
    deleteSavedModelRecord,
    deleteSelectedVersion,
    downloadDatabaseSnapshot,
    downloadProjectBundleJson,
    downloadProjectBundleZip,
    importProjectBundle,
    openModelVersionById,
    openSavedModel,
    openSavedVersion,
    renameSelectedVersion,
    saveModelVersion,
    updateProjectRecord,
  };
}
