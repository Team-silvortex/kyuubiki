"use client";

import { buildWorkbenchAdminDataEffects } from "@/components/workbench/workbench-admin-context";
import { createWorkbenchProjectStorageController } from "@/components/workbench/workbench-project-storage-controller";
import type { WorkbenchNoticeItem } from "@/components/workbench/workbench-notice-state";

export function buildWorkbenchProjectFlows(props: Record<string, any>) {
  let projectStorageControllerRef: { openModelVersionById?: (versionId: string) => void } | null = null;

  const adminDataEffects = buildWorkbenchAdminDataEffects({
    selectedAdminJob: props.selectedAdminJob,
    selectedAdminJobId: props.selectedAdminJobId,
    selectedAdminResultJobId: props.selectedAdminResultJobId,
    jobHistory: props.jobHistory,
    projects: props.projects,
    refreshVersions: props.refreshVersions,
    openModelVersionById: (versionId: string) => projectStorageControllerRef?.openModelVersionById?.(versionId),
    setAdminFilterProjectId: props.setAdminFilterProjectId,
    setAdminFilterModelVersionId: props.setAdminFilterModelVersionId,
    setAdminJobCaseId: props.setAdminJobCaseId,
    setLibraryTab: props.setLibraryTab,
    setSelectedProjectId: props.setSelectedProjectId,
    setSelectedModelId: props.setSelectedModelId,
    setSelectedVersionId: props.setSelectedVersionId,
    setModelVersions: props.setModelVersions,
    setSidebarSection: props.setSidebarSection,
    setMessage: props.setMessage,
    labels: {
      noJobVersion:
        props.language === "zh"
          ? "这个任务还没有关联模型版本。"
          : props.language === "ja"
            ? "このジョブには関連するモデルバージョンがまだありません。"
            : "This job does not have a linked model version.",
      noResultVersion:
        props.language === "zh"
          ? "这个结果还没有关联模型版本。"
          : props.language === "ja"
            ? "この結果には関連するモデルバージョンがまだありません。"
            : "This result does not have a linked model version.",
      noRecordContext:
        props.language === "zh"
          ? "这条记录还没有可应用的项目或版本上下文。"
          : props.language === "ja"
            ? "このレコードには適用できる project / version の文脈がまだありません。"
            : "This record does not have a linked project or version context yet.",
      linkedProjectMissing:
        props.language === "zh"
          ? "找不到关联项目。"
          : props.language === "ja"
            ? "関連プロジェクトが見つかりませんでした。"
            : "Could not find the linked project.",
      linkedProjectOpened: props.t.linkedProjectOpened,
      noJobProject:
        props.language === "zh"
          ? "这个任务还没有关联项目。"
          : props.language === "ja"
            ? "このジョブには関連プロジェクトがまだありません。"
            : "This job does not have a linked project.",
      noResultProject:
        props.language === "zh"
          ? "这个结果还没有关联项目。"
          : props.language === "ja"
            ? "この結果には関連プロジェクトがまだありません。"
            : "This result does not have a linked project.",
      selectJobFirst:
        props.language === "zh"
          ? "请先选择一条任务记录。"
          : props.language === "ja"
            ? "先にジョブレコードを選択してください。"
            : "Select a job record first.",
      missingResultJob:
        props.language === "zh"
          ? "找不到这条结果对应的任务记录。"
          : props.language === "ja"
            ? "この結果に対応するジョブレコードが見つかりませんでした。"
            : "Could not find the job record linked to this result.",
      recordContextApplied: props.t.recordContextApplied,
    },
  });

  const persistedModelEffects = {
    startTransition: props.startTransition,
    activeMaterial: props.activeMaterial,
    createProject: props.createProject,
    createModel: props.createModel,
    createModelVersion: props.createModelVersion,
    updateModelVersion: props.updateModelVersion,
    fetchModel: props.fetchModel,
    fetchModelVersion: props.fetchModelVersion,
    refreshProjects: props.refreshProjects,
    refreshVersions: props.refreshVersions,
    recordHistory: props.recordHistory,
    resetActiveResult: props.resetActiveResult,
    importActionLabel: props.t.importAction,
    historyActionLabel: props.t.historyAction,
    importedModelLabel: props.t.persistedModelLoaded,
    importedProjectLabel: props.t.projectImported,
    importedVersionLabel: props.t.versionLoaded,
    importFailedLabel: props.t.importFailed,
    formatImportNotice: (skippedSensitivePresetCount: number): WorkbenchNoticeItem => ({
      id: "project-import-notice",
      tone: "warning",
      message:
        props.language === "zh"
          ? `项目导入时跳过了 ${skippedSensitivePresetCount} 个敏感自动化预设。`
          : props.language === "ja"
            ? `プロジェクトの取り込み時に機微な automation preset を ${skippedSensitivePresetCount} 件スキップしました。`
            : `Skipped ${skippedSensitivePresetCount} sensitive automation preset(s) during project import.`,
    }),
    setMessage: props.setMessage,
    setSystemAlerts: props.setSystemAlerts,
    setImportNotice: props.setImportNotice,
    setSelectedProjectId: props.setSelectedProjectId,
    setSidebarSection: props.setSidebarSection,
    setLoadedModelName: props.setLoadedModelName,
    setSelectedModelId: props.setSelectedModelId,
    setSelectedVersionId: props.setSelectedVersionId,
    setModelVersions: props.setModelVersions,
    setStudyKind: props.setStudyKind,
    setAxialForm: props.setAxialForm,
    setHeatBarModel: props.setHeatBarModel,
    setHeatPlaneModel: props.setHeatPlaneModel,
    setThermalBarModel: props.setThermalBarModel,
    setThermalBeamModel: props.setThermalBeamModel,
    setThermalFrameModel: props.setThermalFrameModel,
    setThermalTrussModel: props.setThermalTrussModel,
    setThermalTruss3dModel: props.setThermalTruss3dModel,
    setSpringModel: props.setSpringModel,
    setSpring2dModel: props.setSpring2dModel,
    setSpring3dModel: props.setSpring3dModel,
    setTrussModel: props.setTrussModel,
    setTruss3dModel: props.setTruss3dModel,
    setPlaneModel: props.setPlaneModel,
    setFrameModel: props.setFrameModel,
    setBeamModel: props.setBeamModel,
    setTorsionModel: props.setTorsionModel,
    setPlaneResultField: props.setPlaneResultField,
    setParametric: props.setParametric,
    setActiveMaterial: props.setActiveMaterial,
  };

  const projectStorageController = createWorkbenchProjectStorageController({
    t: props.t,
    getSelectedProject: () => props.selectedProject,
    selectedProjectId: props.selectedProjectId,
    getSelectedProjectModels: () => props.selectedProjectModels,
    selectedModelId: props.selectedModelId,
    selectedVersionId: props.selectedVersionId,
    projectNameDraft: props.projectNameDraft,
    projectDescriptionDraft: props.projectDescriptionDraft,
    loadedModelName: props.loadedModelName,
    activeMaterial: props.activeMaterial,
    studyKind: props.studyKind,
    jobHistory: props.jobHistory,
    axialForm: props.axialForm,
    heatBarModel: props.heatBarModel,
    heatPlaneModel: props.heatPlaneModel,
    thermalBarModel: props.thermalBarModel,
    thermalBeamModel: props.thermalBeamModel,
    thermalFrameModel: props.thermalFrameModel,
    thermalTrussModel: props.thermalTrussModel,
    trussModel: props.trussModel,
    thermalTruss3dModel: props.thermalTruss3dModel,
    truss3dModel: props.truss3dModel,
    planeModel: props.planeModel,
    frameModel: props.frameModel,
    beamModel: props.beamModel,
    torsionModel: props.torsionModel,
    springModel: props.springModel,
    spring2dModel: props.spring2dModel,
    spring3dModel: props.spring3dModel,
    parametric: props.parametric,
    round: props.round,
    startTransition: props.startTransition,
    setMessage: props.setMessage,
    setSelectedProjectId: props.setSelectedProjectId,
    setSelectedModelId: props.setSelectedModelId,
    setSelectedVersionId: props.setSelectedVersionId,
    setModelVersions: props.setModelVersions,
    createProject: props.createProject,
    updateProject: props.updateProject,
    deleteProject: props.deleteProject,
    createModel: props.createModel,
    updateModel: props.updateModel,
    deleteModel: props.deleteModel,
    createModelVersion: props.createModelVersion,
    updateModelVersion: props.updateModelVersion,
    deleteModelVersion: props.deleteModelVersion,
    fetchModel: props.fetchModel,
    fetchModelVersion: props.fetchModelVersion,
    fetchModelVersions: props.fetchModelVersions,
    fetchJobStatus: props.fetchJobStatus,
    refreshProjects: props.refreshProjects,
    refreshVersions: props.refreshVersions,
    serializeCurrentModel: props.serializeCurrentModel,
    getPersistedModelEffects: () => persistedModelEffects,
  });
  projectStorageControllerRef = projectStorageController;

  return {
    adminDataEffects,
    projectStorageController,
  };
}
