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
import type { JobResultRecord, JobState } from "@/lib/api/fem-shared";
import { exportProjectBundle } from "@/lib/models/modeler";
import { listWorkbenchMacroPresets, listWorkbenchSnippetPresets } from "@/lib/scripting/workbench-script-runtime";
import { readWorkspaceStoreManifest } from "@/lib/workbench/store-manifest";
import type { WorkbenchProjectContext, WorkbenchProjectRefresh } from "@/lib/workbench/project-context";
import type { RemoveConfirmedWorkbenchLibraryRecord } from "./workbench-library-record-recovery";
import type { ModelRecord, ModelVersionRecord, ProjectRecord } from "@/lib/api/project-types";
import { workbenchOperationFailure } from "@/lib/workbench/operation-result";
import type { WorkbenchFeedback } from "./workbench-feedback";
import { isWorkbenchAdminRecordSourceCurrent, type WorkbenchAdminRecordSource } from "./workbench-admin-record-source";
import { workbenchLibraryRecordSources, workbenchLibraryVersionSources } from "./workbench-library-record-source";
import { captureWorkbenchLibraryWriteSource, type WorkbenchLibraryWriteKind } from "./workbench-library-write-source";
import { getWorkbenchLibraryMutations, resolveWorkbenchLibraryMutationTarget, runWorkbenchLibraryTransition,
  WorkbenchLibraryWritePendingError } from "./workbench-library-mutations";
import { getWorkbenchLibraryMutationPendingLabel } from "./workbench-library-mutation-copy";
import type {
  WorkbenchProjectLibraryBackendService,
} from "@/lib/workbench/project-library-backend-service-core";
import type {
  WorkbenchAdminDataBackendService,
} from "@/lib/workbench/admin-data-backend-service-core";

type ProjectStorageControllerDeps = {
  language?: string;
  projectContext: WorkbenchProjectContext;
  feedback: WorkbenchFeedback;
  sourceChangedLabel: string;
  getProjects: () => ProjectRecord[];
  getModelVersions: () => ModelVersionRecord[];
  removeConfirmedLibraryRecord: RemoveConfirmedWorkbenchLibraryRecord;
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
  jobHistory: JobState[];
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
  adminDataBackendService: WorkbenchAdminDataBackendService;
  projectLibraryBackendService: WorkbenchProjectLibraryBackendService;
  refreshProjects: WorkbenchProjectRefresh;
  refreshVersions: (modelId: string) => Promise<void>;
  serializeCurrentModel: () => Record<string, unknown>;
  getPersistedModelEffects: () => any;
};

export function createWorkbenchProjectStorageController({
  language = "en",
  projectContext,
  feedback,
  sourceChangedLabel,
  getProjects,
  getModelVersions,
  removeConfirmedLibraryRecord,
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
  adminDataBackendService,
  projectLibraryBackendService,
  refreshProjects,
  refreshVersions,
  serializeCurrentModel,
  getPersistedModelEffects,
}: ProjectStorageControllerDeps) {
  const runMutation = (action: string, run: () => Promise<void> | void) => {
    const target = resolveWorkbenchLibraryMutationTarget(action, selectedProjectId, selectedModelId);
    if (!target) return Promise.resolve(run());
    const ownsIntent = projectContext.capture();
    return getWorkbenchLibraryMutations(projectContext).run(target, () => Promise.resolve(run()),
      getWorkbenchLibraryMutationPendingLabel(language)).catch((error) => {
      if (ownsIntent() && (!(error instanceof WorkbenchLibraryWritePendingError) || error.isCurrent())) {
        setMessage(error instanceof Error ? error.message : t.initialFailed);
      }
    });
  };
  const captureWriteSource = (kind: WorkbenchLibraryWriteKind) => {
    try {
      return captureWorkbenchLibraryWriteSource({ kind, projectId: selectedProjectId, modelId: selectedModelId,
        versionId: selectedVersionId, projects: getProjects(), versions: getModelVersions(), projectContext, sourceChangedLabel });
    } catch (error) {
      if (projectContext.capture()()) setMessage(error instanceof Error ? error.message : t.initialFailed);
      throw error;
    }
  };
  const buildProjectBundleJson = async () => {
    const selectedProject = getSelectedProject();
    const selectedProjectModels = getSelectedProjectModels();

    if (!selectedProject) {
      throw new Error(t.projectRequired);
    }

    const modelDetailsSettled = await Promise.allSettled(
      selectedProjectModels.map(async (model) => {
        const modelEnvelope = await projectLibraryBackendService.fetchModel(model.model_id);
        const versionsEnvelope = await projectLibraryBackendService.fetchModelVersions(model.model_id);
        return {
          model: modelEnvelope.model,
          versions: versionsEnvelope.versions,
        };
      }),
    );

    const modelDetails = modelDetailsSettled.flatMap((entry) => (entry.status === "fulfilled" ? [entry.value] : []));

    // Runtime history is global; a project bundle must never include another project's results.
    const projectJobs = jobHistory.filter((entry) => entry.project_id === selectedProject.project_id);
    const resultJobs = projectJobs.filter((entry) => entry.has_result);
    const resultCandidatesSettled = await Promise.allSettled(
      resultJobs.map(async (historyJob): Promise<JobResultRecord | null> => {
        const payload = await adminDataBackendService.fetchJob(historyJob.job_id);
        if (payload.result === undefined || payload.result === null ||
            payload.job.job_id !== historyJob.job_id ||
            payload.job.project_id !== selectedProject.project_id) return null;
        return {
          job_id: historyJob.job_id,
          status: payload.job.status,
          worker_id: payload.job.worker_id,
          result: payload.result,
        };
      }),
    );
    const results = resultCandidatesSettled.flatMap((entry) =>
      entry.status === "fulfilled" && entry.value !== null ? [entry.value] : [],
    );
    const partial =
      modelDetails.length !== selectedProjectModels.length ||
      results.length !== resultJobs.length;

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
        jobs: projectJobs,
        results,
      }),
      partial,
    };
  };

  const downloadProjectBundleJson = async () => {
    const selectedProject = getSelectedProject();
    return downloadWorkbenchProjectBundleJson({
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
    return downloadWorkbenchProjectBundleZip({
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
    return downloadWorkbenchDatabaseSnapshot({
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
    const isCurrent = projectContext.begin();
    let ownsSource = () => false;
    return runWorkbenchLibraryTransition(startTransition, async () => {
      try {
        ownsSource = captureWriteSource("createProject");
        const payload = await projectLibraryBackendService.createProject({
          name: projectNameDraft || t.defaultProject,
          description: projectDescriptionDraft,
        });
        if (!ownsSource()) return;
        await refreshProjects(false, undefined, { preserveSelection: true });
        if (!isCurrent() || !ownsSource()) return;
        setSelectedProjectId(payload.project.project_id);
        setSelectedModelId(null);
        setSelectedVersionId(null);
        setModelVersions([]);
        setMessage(t.projectCreated);
      } catch (error) {
        if (!isCurrent() || !ownsSource()) return;
        setMessage(error instanceof Error ? error.message : t.initialFailed);
      }
    });
  };

  const updateProjectRecord = () => {
    if (!selectedProjectId) return;
    const isCurrent = projectContext.begin();
    let ownsSource = () => false;

    return runWorkbenchLibraryTransition(startTransition, async () => {
      try {
        ownsSource = captureWriteSource("project");
        await projectLibraryBackendService.updateProject(selectedProjectId, {
          name: projectNameDraft || t.defaultProject,
          description: projectDescriptionDraft,
        });
        if (!ownsSource()) return;
        await refreshProjects(false, undefined, { preserveSelection: true });
        if (!isCurrent() || !ownsSource()) return;
        setMessage(t.projectUpdated);
      } catch (error) {
        if (!isCurrent() || !ownsSource()) return;
        setMessage(error instanceof Error ? error.message : t.initialFailed);
      }
    });
  };

  const deleteProjectRecord = () => {
    if (!selectedProjectId) return;
    if (typeof window !== "undefined" && !window.confirm(projectNameDraft)) return;
    const isCurrent = projectContext.begin();
    let ownsSource = () => false;

    return runWorkbenchLibraryTransition(startTransition, async () => {
      try {
        ownsSource = captureWriteSource("project");
        await projectLibraryBackendService.deleteProject(selectedProjectId);
        if (!ownsSource()) return;
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
        if (!contextChanged && ownsFeedback() && ownsSource()) setMessage(t.projectDeleted);
      } catch (error) {
        if (!isCurrent() || !ownsSource()) return;
        setMessage(error instanceof Error ? error.message : t.initialFailed);
      }
    });
  };

  const saveModelVersion = (saveAs: boolean) => {
    if (!selectedProjectId) {
      setMessage(t.projectRequired);
      return;
    }

    let isCurrent = projectContext.begin();
    let ownsSource = () => false;

    return runWorkbenchLibraryTransition(startTransition, async () => {
      try {
        ownsSource = captureWriteSource(!selectedModelId || saveAs ? "project" : "model");
        const payload = serializeCurrentModel() as Record<string, unknown> & { model_schema_version?: string };
        const modelPayload = {
          name: loadedModelName,
          kind: studyKind,
          material: activeMaterial,
          model_schema_version: String(payload.model_schema_version ?? "kyuubiki.model/v1"),
          payload,
        };

        if (!selectedModelId || saveAs) {
          const created = await projectLibraryBackendService.createModel(selectedProjectId, modelPayload);
          if (!ownsSource()) return;
          await refreshProjects(false, undefined, { preserveSelection: true });
          if (!isCurrent() || !ownsSource()) return;
          isCurrent = projectContext.update({ projectId: selectedProjectId, modelId: created.model.model_id,
            versionId: created.model.latest_version_id ?? null });
          setSelectedModelId(created.model.model_id);
          setSelectedVersionId(created.model.latest_version_id ?? null);
          setMessage(t.modelCreated);
          await refreshVersions(created.model.model_id);
          return;
        }

        // The backend checkpoint updates model metadata and geometry in the same transaction.
        const version = await projectLibraryBackendService.createModelVersion(selectedModelId, modelPayload);
        if (!ownsSource()) return;
        await refreshProjects(false, undefined, { preserveSelection: true });
        if (!isCurrent() || !ownsSource()) return;
        isCurrent = projectContext.update({ projectId: selectedProjectId, modelId: selectedModelId,
          versionId: version.version.version_id });
        setSelectedVersionId(version.version.version_id);
        setMessage(t.modelSaved);
        await refreshVersions(selectedModelId);
      } catch (error) {
        if (!isCurrent() || !ownsSource()) return;
        setMessage(error instanceof Error ? error.message : t.initialFailed);
      }
    });
  };

  const navigationScope = (source: WorkbenchAdminRecordSource | null) => {
    let ownsFeedback = feedback.begin();
    return { isCurrent: () => isWorkbenchAdminRecordSourceCurrent(source, source),
      ownsFeedback: () => ownsFeedback(), onApplied: () => { ownsFeedback = feedback.begin(); } };
  };
  const rejectNavigation = () => {
    const message = `WORKBENCH_RECORD_SOURCE_CHANGED: ${sourceChangedLabel}`;
    if (feedback.begin()() && projectContext.capture()()) setMessage(message);
    return Promise.resolve(workbenchOperationFailure(new Error(message), message));
  };
  const openSavedModel = (model: ModelRecord) => {
    const projects = getProjects(), selection = projectContext.current();
    const scope = navigationScope(workbenchLibraryRecordSources.sourceOf(projects));
    if (!projectContext.capture()() || !scope.isCurrent() || !scope.ownsFeedback()
      || model.project_id !== selection.projectId
      || !projects.some((project) => project.project_id === model.project_id && project.models?.includes(model))) return rejectNavigation();
    return openPersistedWorkbenchModel(model, getPersistedModelEffects(), scope);
  };

  const openSavedVersion = (version: ModelVersionRecord) => {
    const versions = getModelVersions(), selection = projectContext.current();
    const scope = navigationScope(workbenchLibraryVersionSources.sourceOf(versions));
    if (!projectContext.capture()() || !scope.isCurrent() || !scope.ownsFeedback() || !versions.includes(version)
      || version.model_id !== selection.modelId || version.project_id !== selection.projectId) return rejectNavigation();
    return openPersistedWorkbenchVersion(version, getPersistedModelEffects(), scope);
  };

  const openModelVersionById = (versionId: string, validate?: import("@/lib/workbench/checkpoint-recovery").CheckpointVersionGuard,
    scope?: import("./workbench-persisted-model-controller").WorkbenchVersionLoadScope) => {
    return openPersistedWorkbenchVersionById(versionId, getPersistedModelEffects(), validate, scope);
  };

  const renameSelectedVersion = () => {
    if (!selectedVersionId) return;
    const isCurrent = projectContext.begin();
    let ownsSource = () => false;

    return runWorkbenchLibraryTransition(startTransition, async () => {
      try {
        ownsSource = captureWriteSource("version");
        await projectLibraryBackendService.updateModelVersion(selectedVersionId, { name: loadedModelName });
        if (!ownsSource()) return;
        if (selectedModelId && projectContext.hasModel(selectedModelId)) await refreshVersions(selectedModelId);
        if (!isCurrent() || !ownsSource()) return;
        setMessage(t.versionRenamed);
      } catch (error) {
        if (!isCurrent() || !ownsSource()) return;
        setMessage(error instanceof Error ? error.message : t.initialFailed);
      }
    });
  };

  const deleteSelectedVersion = () => {
    if (!selectedVersionId) return;
    const isCurrent = projectContext.begin();
    let ownsSource = () => false;

    return runWorkbenchLibraryTransition(startTransition, async () => {
      try {
        ownsSource = captureWriteSource("version");
        const removed = await projectLibraryBackendService.deleteModelVersion(selectedVersionId);
        if (!ownsSource()) return;
        const contextChanged = !isCurrent();
        removeConfirmedLibraryRecord("versionId", selectedVersionId, removed.version.model_id);
        if (projectContext.detachDeleted("versionId", selectedVersionId)) setSelectedVersionId(null);
        const ownsFeedback = projectContext.capture();
        await Promise.all([refreshProjects(false, undefined, { preserveSelection: true }),
          selectedModelId && projectContext.hasModel(selectedModelId) ? refreshVersions(selectedModelId) : Promise.resolve()]);
        if (!contextChanged && ownsFeedback() && ownsSource()) setMessage(t.versionDeleted);
      } catch (error) {
        if (!isCurrent() || !ownsSource()) return;
        setMessage(error instanceof Error ? error.message : t.initialFailed);
      }
    });
  };

  const deleteSavedModelRecord = () => {
    if (!selectedModelId) return;
    const isCurrent = projectContext.begin();
    let ownsSource = () => false;

    return runWorkbenchLibraryTransition(startTransition, async () => {
      try {
        ownsSource = captureWriteSource("model");
        await projectLibraryBackendService.deleteModel(selectedModelId);
        if (!ownsSource()) return;
        const contextChanged = !isCurrent();
        removeConfirmedLibraryRecord("modelId", selectedModelId);
        if (projectContext.detachDeleted("modelId", selectedModelId)) {
          setSelectedModelId(null);
          setSelectedVersionId(null);
          setModelVersions([]);
        }
        const ownsFeedback = projectContext.capture();
        await refreshProjects(false, undefined, { preserveSelection: true });
        if (!contextChanged && ownsFeedback() && ownsSource()) setMessage(t.modelDeletedStored);
      } catch (error) {
        if (!isCurrent() || !ownsSource()) return;
        setMessage(error instanceof Error ? error.message : t.initialFailed);
      }
    });
  };

  return {
    buildProjectBundleJson,
    createProjectRecord: () => runMutation("project/create", createProjectRecord),
    deleteProjectRecord: () => runMutation("project/deleteSelected", deleteProjectRecord),
    deleteSavedModelRecord: () => runMutation("model/deleteSelected", deleteSavedModelRecord),
    deleteSelectedVersion: () => runMutation("model/deleteSelectedVersion", deleteSelectedVersion),
    downloadDatabaseSnapshot,
    downloadProjectBundleJson,
    downloadProjectBundleZip,
    importProjectBundle,
    openModelVersionById,
    openSavedModel,
    openSavedVersion,
    renameSelectedVersion: () => runMutation("model/renameSelectedVersion", renameSelectedVersion),
    saveModelVersion: (saveAs: boolean) => runMutation(saveAs ? "model/saveAs" : "model/save", () => saveModelVersion(saveAs)),
    updateProjectRecord: () => runMutation("project/updateSelected", updateProjectRecord),
  };
}
