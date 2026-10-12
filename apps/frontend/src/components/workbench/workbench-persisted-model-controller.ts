"use client";

import type { Dispatch, SetStateAction } from "react";
import { applyImportedWorkbenchModel } from "@/components/workbench/workbench-model-load";
import type { WorkbenchAlertItem } from "@/components/workbench/workbench-alert-strip";
import { dismissWorkbenchAlert, upsertWorkbenchAlert } from "@/components/workbench/workbench-alert-state";
import {
  dismissWorkbenchNotice,
  showWorkbenchNotice,
  type WorkbenchNoticeItem,
  type WorkbenchNoticeStateSetter,
} from "@/components/workbench/workbench-notice-state";
import type { ModelRecord, ModelVersionRecord } from "@/lib/api/project-types";
import type { CheckpointVersionGuard } from "@/lib/workbench/checkpoint-recovery";
import { captureWorkbenchApiAuthority } from "@/lib/api/auth-context";
import { getWorkbenchLibraryMutations, runWorkbenchLibraryTransition, WorkbenchLibraryWritePendingError } from "./workbench-library-mutations";
import { getWorkbenchLibraryMutationPendingLabel } from "./workbench-library-mutation-copy";
import { parsePlaygroundModel } from "@/lib/models/model-import";
import { saveWorkbenchMacroPreset, saveWorkbenchSnippetPreset } from "@/lib/scripting/workbench-script-runtime";
import { isSensitivePresetSaveError } from "@/lib/scripting/workbench-script-preset-security";
import {
  workbenchProjectContextChangedError,
  type WorkbenchProjectContext,
  type WorkbenchProjectRefresh,
} from "@/lib/workbench/project-context";
import {
  persistWorkspaceStoreManifest,
  rewriteWorkspaceStoreManifestProject,
} from "@/lib/workbench/store-manifest";
import {
  runWorkbenchTransitionOperation,
  workbenchOperationFailure,
  type WorkbenchOperationResult,
} from "@/lib/workbench/operation-result";

type PersistedModelEffects = {
  setLoadedModelName: (value: string) => void;
  setSelectedModelId: (value: string | null) => void;
  setSelectedVersionId: (value: string | null) => void;
  setModelVersions: (value: any[]) => void;
  setStudyKind: (value: any) => void;
  setAxialForm: (value: any) => void;
  setHeatBarModel: (value: any) => void;
  setHeatPlaneModel: (value: any) => void;
  setThermalBarModel: (value: any) => void;
  setThermalBeamModel: (value: any) => void;
  setThermalFrameModel: (value: any) => void;
  setThermalTrussModel: (value: any) => void;
  setThermalTruss3dModel: (value: any) => void;
  setSpringModel: (value: any) => void;
  setSpring2dModel: (value: any) => void;
  setSpring3dModel: (value: any) => void;
  setTrussModel: (value: any) => void;
  setTruss3dModel: (value: any) => void;
  setPlaneModel: (value: any) => void;
  setFrameModel: (value: any) => void;
  setBeamModel: (value: any) => void;
  setTorsionModel: (value: any) => void;
  setPlaneResultField: (value: any) => void;
  setParametric: (updater: (current: any) => any) => void;
  setActiveMaterial: (value: string) => void;
};

type PersistedModelControllerDeps = PersistedModelEffects & {
  projectContext: WorkbenchProjectContext;
  language?: string;
  activeMaterial: string;
  startTransition?: (callback: () => void) => void;
  createProject: (input: { name: string; description: string }) => Promise<any>;
  createModel: (projectId: string, input: ModelMutationInput) => Promise<any>;
  createModelVersion: (modelId: string, input: ModelMutationInput) => Promise<any>;
  updateModelVersion: (versionId: string, input: { name: string }) => Promise<any>;
  fetchModel: (modelId: string) => Promise<any>;
  fetchModelVersion: (versionId: string) => Promise<any>;
  refreshProjects: WorkbenchProjectRefresh;
  refreshVersions: (modelId: string) => Promise<void>;
  recordHistory: (label: string) => void;
  resetActiveResult: () => void;
  importActionLabel: string;
  historyActionLabel: string;
  importedModelLabel: string;
  importedProjectLabel: string;
  importedVersionLabel: string;
  importFailedLabel: string;
  storeManifestPersistenceFailedLabel: string;
  formatImportNotice: (skippedSensitivePresetCount: number) => WorkbenchNoticeItem;
  setMessage: (value: string) => void;
  setSystemAlerts: Dispatch<SetStateAction<WorkbenchAlertItem[]>>;
  setImportNotice: WorkbenchNoticeStateSetter;
  setSelectedProjectId: (value: string | null) => void;
  setSidebarSection?: (value: any) => void;
};

type ModelMutationInput = {
  name: string;
  kind: string;
  material?: string;
  model_schema_version?: string;
  payload: Record<string, unknown>;
};

export function applyPersistedWorkbenchPayload(
  payload: Record<string, unknown>,
  fallbackName: string | undefined,
  effects: PersistedModelControllerDeps,
) {
  const imported = parsePlaygroundModel(JSON.stringify(payload));
  applyImportedWorkbenchModel(imported, effects);
  effects.setLoadedModelName(fallbackName ?? imported.name);
  effects.setActiveMaterial("material" in imported ? imported.material : effects.activeMaterial);
  effects.resetActiveResult();
}

export function importWorkbenchProjectBundle(file: File | undefined, effects: PersistedModelControllerDeps) {
  if (!file || !effects.projectContext.capture()()) return Promise.resolve();
  const ownsIntent = effects.projectContext.capture();
  const pendingLabel = getWorkbenchLibraryMutationPendingLabel(effects.language ?? "en");
  return getWorkbenchLibraryMutations(effects.projectContext).run({ kind: "importProject" },
    () => runWorkbenchLibraryTransition(effects.startTransition ?? ((callback) => callback()),
      () => importProjectBundleInScope(file, effects, pendingLabel)), pendingLabel).catch((error) => {
    if (ownsIntent() && (!(error instanceof WorkbenchLibraryWritePendingError) || error.isCurrent())) {
      effects.setMessage(error instanceof Error ? error.message : effects.importFailedLabel);
    }
  });
}

async function importProjectBundleInScope(file: File, effects: PersistedModelControllerDeps, pendingLabel: string) {
  let isCurrent = effects.projectContext.begin();
  let ownsAuthority = () => true;
  // Same-source navigation may leave an import running; backend changes may not.
  const ownsSource = () => {
    try { return effects.projectContext.capture()() && ownsAuthority(); } catch { return false; }
  };

  try {
    ownsAuthority = captureWorkbenchApiAuthority("/api/v1/projects", "/api/v1/models", "/api/v1/model-versions");
    dismissWorkbenchAlert(effects.setSystemAlerts, "project-import-error");
    dismissWorkbenchAlert(effects.setSystemAlerts, "project-import-store-manifest-warning");
    dismissWorkbenchNotice(effects.setImportNotice);
    const { parseProjectBundleFile } = await import("@/lib/projects/project-format");
    if (!isCurrent() || !ownsSource()) return;
    const bundle = await parseProjectBundleFile(file);
    if (!isCurrent() || !ownsSource()) return;
    const activeModel = bundle.models.find((model) => model.model_id === bundle.active_model_id) ?? bundle.models[0];
    const activeVersion = bundle.model_versions.find((version) =>
      version.version_id === bundle.active_version_id && version.model_id === activeModel?.model_id);
    const workspacePayload = bundle.workspace_snapshot ?? activeVersion?.payload ?? activeModel?.payload;
    // Validate the payload before creating persistent records or changing the active workspace.
    const importedWorkspace = workspacePayload ? parsePlaygroundModel(JSON.stringify(workspacePayload)) : null;
    if (!isCurrent() || !ownsSource()) return;
    const createdProject = await effects.createProject({
      name: bundle.project.name,
      description: bundle.project.description ?? "",
    });
    if (!ownsSource()) return;

    // The import slot excludes overlapping files; the new project excludes all of its child writes.
    await getWorkbenchLibraryMutations(effects.projectContext).run({ kind: "project", projectId: createdProject.project.project_id }, async () => {
      const modelIdMap = new Map<string, string>();
      const versionIdMap = new Map<string, string>();

      for (const bundledModel of bundle.models) {
        if (!ownsSource()) return;
        const bundledVersions = bundle.model_versions
          .filter((version) => version.model_id === bundledModel.model_id)
          .sort((left, right) => left.version_number - right.version_number);

        const baseVersion = bundledVersions[0];
        const createdModel = await effects.createModel(createdProject.project.project_id, {
          name: baseVersion?.name || bundledModel.name,
          kind: bundledModel.kind,
          material: bundledModel.material ?? undefined,
          model_schema_version: bundledModel.model_schema_version,
          payload: (baseVersion?.payload ?? bundledModel.payload) as Record<string, unknown>,
        });
        if (!ownsSource()) return;

        const newModelId = createdModel.model.model_id;
        modelIdMap.set(bundledModel.model_id, newModelId);

        const initialVersionId = createdModel.model.latest_version_id ?? createdModel.model.versions?.[0]?.version_id;
        if (initialVersionId && baseVersion) versionIdMap.set(baseVersion.version_id, initialVersionId);
        if (createdModel.model.versions?.[0]?.version_id && baseVersion?.name) {
          await effects.updateModelVersion(initialVersionId, { name: baseVersion.name });
          if (!ownsSource()) return;
        }

        for (const extraVersion of bundledVersions.slice(1)) {
          const createdVersion = await effects.createModelVersion(newModelId, {
            name: extraVersion.name,
            kind: extraVersion.kind,
            material: extraVersion.material ?? undefined,
            model_schema_version: extraVersion.model_schema_version,
            payload: extraVersion.payload,
          });
          if (!ownsSource()) return;
          versionIdMap.set(extraVersion.version_id, createdVersion.version.version_id);
        }
      }

      if (!ownsSource()) return;
      let skippedSensitivePresetCount = 0;
      for (const preset of bundle.automation_presets ?? []) {
        try {
          saveWorkbenchMacroPreset({
            projectId: createdProject.project.project_id,
            presetId: preset.presetId,
            name: preset.name,
            macro: preset.macro,
          });
        } catch (error) {
          if (isSensitivePresetSaveError(error)) {
            skippedSensitivePresetCount += 1;
          }
          // Ignore malformed preset payloads so model/project import stays usable.
        }
      }
      for (const preset of bundle.snippet_presets ?? []) {
        try {
          saveWorkbenchSnippetPreset({
            projectId: createdProject.project.project_id,
            presetId: preset.presetId,
            snippetId: preset.snippetId,
            name: preset.name,
            parameters: preset.parameters,
          });
        } catch (error) {
          if (isSensitivePresetSaveError(error)) {
            skippedSensitivePresetCount += 1;
          }
          // Ignore malformed snippet preset payloads so model/project import stays usable.
        }
      }

      let manifestPersisted = true;
      if (bundle.store_manifest) {
        manifestPersisted = persistWorkspaceStoreManifest(
          rewriteWorkspaceStoreManifestProject(bundle.store_manifest, createdProject.project.project_id),
        );
      }

      if (!ownsSource()) return;
      await effects.refreshProjects(false, undefined, { preserveSelection: true });
      if (!isCurrent() || !ownsSource()) return;
      if (!manifestPersisted) {
        upsertWorkbenchAlert(effects.setSystemAlerts, {
          id: "project-import-store-manifest-warning",
          message: effects.storeManifestPersistenceFailedLabel,
          tone: "warning",
        });
      }

      const importedActiveModelId =
        (activeModel && modelIdMap.get(activeModel.model_id)) || null;
      const sourceVersionId = activeVersion?.version_id ?? activeModel?.latest_version_id;
      const importedActiveVersionId = (sourceVersionId && versionIdMap.get(sourceVersionId)) || null;

      if (importedWorkspace) {
        effects.recordHistory(effects.importActionLabel);
        applyImportedWorkbenchModel(importedWorkspace, effects);
        if (!bundle.workspace_snapshot) effects.setLoadedModelName(activeVersion?.name ?? activeModel?.name ?? importedWorkspace.name);
        effects.resetActiveResult();
      }

      // Loading a payload clears saved-model associations; restore the remapped IDs afterwards.
      isCurrent = effects.projectContext.update({ projectId: createdProject.project.project_id,
        modelId: importedActiveModelId, versionId: importedActiveVersionId });
      effects.setSelectedProjectId(createdProject.project.project_id);
      effects.setSelectedModelId(importedActiveModelId);
      effects.setSelectedVersionId(importedActiveVersionId);
      effects.setModelVersions([]);

      if (importedActiveModelId && ownsSource()) await effects.refreshVersions(importedActiveModelId);
      if (!isCurrent() || !ownsSource()) return;
      effects.setMessage(effects.importedProjectLabel);
      if (skippedSensitivePresetCount > 0) {
        showWorkbenchNotice(effects.setImportNotice, effects.formatImportNotice(skippedSensitivePresetCount));
      } else {
        dismissWorkbenchNotice(effects.setImportNotice);
      }
    }, pendingLabel);
  } catch (error) {
    if (!isCurrent() || !ownsSource()) return;
    dismissWorkbenchNotice(effects.setImportNotice);
    const message = error instanceof Error ? error.message : effects.importFailedLabel;
    upsertWorkbenchAlert(effects.setSystemAlerts, {
      id: "project-import-error",
      message,
      tone: "error",
    });
    effects.setMessage(message);
  }
}

export function openPersistedWorkbenchModel(model: ModelRecord, effects: PersistedModelControllerDeps,
  scope?: WorkbenchVersionLoadScope) {
  let isCurrent = effects.projectContext.begin();
  let ownsAuthority = () => true;
  const safely = (check?: () => boolean) => { try { return check?.() ?? true; } catch { return false; } };
  const current = () => isCurrent() && safely(ownsAuthority) && safely(scope?.isCurrent);
  const run = async (): Promise<WorkbenchOperationResult> => {
    try {
      if (!current() || !safely(scope?.ownsFeedback)) return workbenchOperationFailure(workbenchProjectContextChangedError(), effects.importFailedLabel);
      ownsAuthority = captureWorkbenchApiAuthority(`/api/v1/models/${encodeURIComponent(model.model_id)}`);
      dismissWorkbenchAlert(effects.setSystemAlerts, "persisted-model-open-error");
      dismissWorkbenchNotice(effects.setImportNotice);
      const payload = await effects.fetchModel(model.model_id);
      if (!current() || !safely(scope?.ownsFeedback)) return workbenchOperationFailure(workbenchProjectContextChangedError(), effects.importFailedLabel);
      if (payload?.model?.model_id !== model.model_id || payload.model.project_id !== model.project_id
        || typeof model.project_id !== "string" || !model.project_id.trim()
        || (payload.model.latest_version_id != null && (typeof payload.model.latest_version_id !== "string"
          || !payload.model.latest_version_id.trim()))) {
        throw new Error("WORKBENCH_SAVED_MODEL_INVALID: the returned model does not match the selected saved record.");
      }
      effects.recordHistory(effects.historyActionLabel);
      applyPersistedWorkbenchPayload(payload.model.payload, payload.model.name, effects);
      isCurrent = effects.projectContext.update({ projectId: payload.model.project_id, modelId: payload.model.model_id,
        versionId: payload.model.latest_version_id ?? null });
      effects.setSelectedProjectId(payload.model.project_id);
      effects.setSelectedModelId(payload.model.model_id);
      effects.setSelectedVersionId(payload.model.latest_version_id ?? null);
      effects.setMessage(effects.importedModelLabel);
      scope?.onApplied();
      if (!current()) return workbenchOperationFailure(workbenchProjectContextChangedError(), effects.importFailedLabel);
      await effects.refreshVersions(payload.model.model_id);
      if (!current()) return workbenchOperationFailure(workbenchProjectContextChangedError(), effects.importFailedLabel);
      return { ok: true };
    } catch (error) {
      const failure = workbenchOperationFailure(error, effects.importFailedLabel);
      if (!current() || !safely(scope?.ownsFeedback)) return failure;
      upsertWorkbenchAlert(effects.setSystemAlerts, {
        id: "persisted-model-open-error",
        message: failure.error.message,
        tone: "error",
      });
      effects.setMessage(failure.error.message);
      return failure;
    }
  };

  return runWorkbenchTransitionOperation(effects.startTransition, run);
}

export function openPersistedWorkbenchVersion(version: ModelVersionRecord, effects: PersistedModelControllerDeps,
  scope?: WorkbenchVersionLoadScope) {
  return openPersistedWorkbenchVersionById(version.version_id, effects, (loaded) => {
    if (loaded?.version_id !== version.version_id || loaded.model_id !== version.model_id
      || loaded.project_id !== version.project_id) {
      throw new Error("WORKBENCH_SAVED_VERSION_INVALID: the returned version does not match the selected saved record.");
    }
  }, scope);
}

export type WorkbenchVersionLoadScope = {
  isCurrent: () => boolean;
  ownsFeedback: () => boolean;
  onApplied: () => void;
};

export function openPersistedWorkbenchVersionById(versionId: string, effects: PersistedModelControllerDeps,
  validate?: CheckpointVersionGuard, scope?: WorkbenchVersionLoadScope) {
  let isCurrent = effects.projectContext.begin();
  let ownsAuthority = () => true;
  const safely = (check?: () => boolean) => { try { return check?.() ?? true; } catch { return false; } };
  const current = () => isCurrent() && safely(ownsAuthority) && safely(scope?.isCurrent);
  const run = async (): Promise<WorkbenchOperationResult> => {
    try {
      if (!current() || !safely(scope?.ownsFeedback)) return workbenchOperationFailure(workbenchProjectContextChangedError(), effects.importFailedLabel);
      ownsAuthority = captureWorkbenchApiAuthority(`/api/v1/model-versions/${encodeURIComponent(versionId)}`);
      dismissWorkbenchAlert(effects.setSystemAlerts, "persisted-version-open-error");
      dismissWorkbenchNotice(effects.setImportNotice);
      const payload = await effects.fetchModelVersion(versionId);
      if (!current() || !safely(scope?.ownsFeedback)) return workbenchOperationFailure(workbenchProjectContextChangedError(), effects.importFailedLabel);
      validate?.(payload.version);
      if (payload?.version?.version_id !== versionId || typeof payload.version.model_id !== "string"
        || !payload.version.model_id.trim() || typeof payload.version.project_id !== "string"
        || !payload.version.project_id.trim()) {
        throw new Error("WORKBENCH_SAVED_VERSION_INVALID: expected the requested version with identified model and project.");
      }
      effects.recordHistory(effects.historyActionLabel);
      applyPersistedWorkbenchPayload(payload.version.payload, payload.version.name, effects);
      isCurrent = effects.projectContext.update({ projectId: payload.version.project_id, modelId: payload.version.model_id,
        versionId: payload.version.version_id });
      effects.setSelectedModelId(payload.version.model_id);
      effects.setSelectedProjectId(payload.version.project_id);
      effects.setSelectedVersionId(payload.version.version_id);
      effects.setMessage(effects.importedVersionLabel);
      effects.setSidebarSection?.("model");
      scope?.onApplied();
      if (!current()) return workbenchOperationFailure(workbenchProjectContextChangedError(), effects.importFailedLabel);
      await effects.refreshVersions(payload.version.model_id);
      if (!current()) return workbenchOperationFailure(workbenchProjectContextChangedError(), effects.importFailedLabel);
      return { ok: true };
    } catch (error) {
      const failure = workbenchOperationFailure(error, effects.importFailedLabel);
      if (!current() || !safely(scope?.ownsFeedback)) return failure;
      upsertWorkbenchAlert(effects.setSystemAlerts, {
        id: "persisted-version-open-error",
        message: failure.error.message,
        tone: "error",
      });
      effects.setMessage(failure.error.message);
      return failure;
    }
  };

  return runWorkbenchTransitionOperation(effects.startTransition, run);
}
