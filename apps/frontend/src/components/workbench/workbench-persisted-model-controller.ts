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
import { parsePlaygroundModel } from "@/lib/models/model-import";
import { parseProjectBundleFile } from "@/lib/projects/project-format";
import { saveWorkbenchMacroPreset, saveWorkbenchSnippetPreset } from "@/lib/scripting/workbench-script-runtime";
import { isSensitivePresetSaveError } from "@/lib/scripting/workbench-script-preset-security";
import {
  persistWorkspaceStoreManifest,
  rewriteWorkspaceStoreManifestProject,
} from "@/lib/workbench/store-manifest";

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
  activeMaterial: string;
  startTransition?: (callback: () => void) => void;
  createProject: (input: { name: string; description: string }) => Promise<any>;
  createModel: (projectId: string, input: ModelMutationInput) => Promise<any>;
  createModelVersion: (modelId: string, input: ModelMutationInput) => Promise<any>;
  updateModelVersion: (versionId: string, input: { name: string }) => Promise<any>;
  fetchModel: (modelId: string) => Promise<any>;
  fetchModelVersion: (versionId: string) => Promise<any>;
  refreshProjects: () => Promise<void>;
  refreshVersions: (modelId: string) => Promise<void>;
  recordHistory: (label: string) => void;
  resetActiveResult: () => void;
  importActionLabel: string;
  historyActionLabel: string;
  importedModelLabel: string;
  importedProjectLabel: string;
  importedVersionLabel: string;
  importFailedLabel: string;
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

export async function importWorkbenchProjectBundle(file: File | undefined, effects: PersistedModelControllerDeps) {
  if (!file) return;

  try {
    dismissWorkbenchAlert(effects.setSystemAlerts, "project-import-error");
    dismissWorkbenchNotice(effects.setImportNotice);
    const bundle = await parseProjectBundleFile(file);
    const createdProject = await effects.createProject({
      name: bundle.project.name,
      description: bundle.project.description ?? "",
    });

    const modelIdMap = new Map<string, string>();

    for (const bundledModel of bundle.models) {
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

      const newModelId = createdModel.model.model_id;
      modelIdMap.set(bundledModel.model_id, newModelId);

      const initialVersionId = createdModel.model.versions?.[0]?.version_id;
      if (initialVersionId && baseVersion?.name) {
        await effects.updateModelVersion(initialVersionId, { name: baseVersion.name });
      }

      for (const extraVersion of bundledVersions.slice(1)) {
        await effects.createModelVersion(newModelId, {
          name: extraVersion.name,
          kind: extraVersion.kind,
          material: extraVersion.material ?? undefined,
          model_schema_version: extraVersion.model_schema_version,
          payload: extraVersion.payload,
        });
      }
    }

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

    if (bundle.store_manifest) {
      persistWorkspaceStoreManifest(
        rewriteWorkspaceStoreManifestProject(bundle.store_manifest, createdProject.project.project_id),
      );
    }

    await effects.refreshProjects();

    const importedActiveModelId =
      (bundle.active_model_id && modelIdMap.get(bundle.active_model_id)) ||
      [...modelIdMap.values()][0] ||
      null;

    effects.setSelectedProjectId(createdProject.project.project_id);
    effects.setSelectedModelId(importedActiveModelId);

    if (bundle.workspace_snapshot) {
      effects.recordHistory(effects.importActionLabel);
      applyPersistedWorkbenchPayload(bundle.workspace_snapshot, bundle.project.name, effects);
    }

    if (importedActiveModelId) {
      await effects.refreshVersions(importedActiveModelId);
    } else {
      effects.setModelVersions([]);
    }

    effects.setMessage(effects.importedProjectLabel);
    if (skippedSensitivePresetCount > 0) {
      showWorkbenchNotice(effects.setImportNotice, effects.formatImportNotice(skippedSensitivePresetCount));
    } else {
      dismissWorkbenchNotice(effects.setImportNotice);
    }
  } catch (error) {
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

export function openPersistedWorkbenchModel(model: ModelRecord, effects: PersistedModelControllerDeps) {
  const run = async () => {
    try {
      dismissWorkbenchAlert(effects.setSystemAlerts, "persisted-model-open-error");
      dismissWorkbenchNotice(effects.setImportNotice);
      const payload = await effects.fetchModel(model.model_id);
      effects.recordHistory(effects.historyActionLabel);
      applyPersistedWorkbenchPayload(payload.model.payload, payload.model.name, effects);
      effects.setSelectedProjectId(payload.model.project_id);
      effects.setSelectedModelId(payload.model.model_id);
      effects.setSelectedVersionId(payload.model.latest_version_id ?? null);
      await effects.refreshVersions(payload.model.model_id);
      effects.setMessage(effects.importedModelLabel);
    } catch (error) {
      const message = error instanceof Error ? error.message : effects.importFailedLabel;
      upsertWorkbenchAlert(effects.setSystemAlerts, {
        id: "persisted-model-open-error",
        message,
        tone: "error",
      });
      effects.setMessage(message);
    }
  };

  if (effects.startTransition) {
    effects.startTransition(() => {
      void run();
    });
    return;
  }

  void run();
}

export function openPersistedWorkbenchVersion(version: ModelVersionRecord, effects: PersistedModelControllerDeps) {
  openPersistedWorkbenchVersionById(version.version_id, effects);
}

export function openPersistedWorkbenchVersionById(versionId: string, effects: PersistedModelControllerDeps) {
  const run = async () => {
    try {
      dismissWorkbenchAlert(effects.setSystemAlerts, "persisted-version-open-error");
      dismissWorkbenchNotice(effects.setImportNotice);
      const payload = await effects.fetchModelVersion(versionId);
      effects.recordHistory(effects.historyActionLabel);
      applyPersistedWorkbenchPayload(payload.version.payload, payload.version.name, effects);
      effects.setSelectedModelId(payload.version.model_id);
      effects.setSelectedProjectId(payload.version.project_id);
      effects.setSelectedVersionId(payload.version.version_id);
      effects.setMessage(effects.importedVersionLabel);
      effects.setSidebarSection?.("model");
    } catch (error) {
      const message = error instanceof Error ? error.message : effects.importFailedLabel;
      upsertWorkbenchAlert(effects.setSystemAlerts, {
        id: "persisted-version-open-error",
        message,
        tone: "error",
      });
      effects.setMessage(message);
    }
  };

  if (effects.startTransition) {
    effects.startTransition(() => {
      void run();
    });
    return;
  }

  void run();
}
