"use client";

import { useDeferredValue } from "react";
import {
  buildWorkbenchAdminSelections,
} from "@/components/workbench/workbench-admin-context";
import { useWorkbenchAdminSecurityState } from "@/components/workbench/workbench-admin-security-state";
import { useWorkbenchDataRefreshController } from "@/components/workbench/workbench-data-refresh-controller";
import { useWorkbenchJobHistoryController } from "@/components/workbench/workbench-job-history-controller";
import { buildWorkbenchProjectFlows } from "@/components/workbench/workbench-project-flows";
import {
  useWorkbenchWorkflowController,
} from "@/components/workbench/workflow/workbench-workflow-controller";

export function useWorkbenchSessionComposition(props: Record<string, any>) {
  const jobHistoryController = useWorkbenchJobHistoryController({
    labels: {
      jobCancelled: props.t.jobCancelled,
      initialFailed: props.t.initialFailed,
      requestTimedOut: props.t.requestTimedOut,
    },
    job: props.job,
    jobIsActive: props.jobIsActive,
    jobPollTokenRef: props.jobPollTokenRef,
    setJob: props.setJob,
    setMessage: props.setMessage,
    startTransition: props.startTransition,
  });
  const {
    jobHistory,
    selectedAdminJobId,
    refreshJobHistory,
  } = jobHistoryController;

  const adminSecurityState = useWorkbenchAdminSecurityState({
    jobHistory,
    resultRecords: props.resultRecords,
    selectedAdminJobId,
  });
  const {
    selectedAdminResultJobId,
    adminFilterProjectId,
    adminFilterModelVersionId,
    securityEventRecords,
    securityEventWindowFilter,
    securityEventSourceFilter,
    securityEventRiskFilter,
    securityEventStatusFilter,
    securityEventActionFilter,
    adminJobMessage,
    adminJobProjectId,
    adminJobModelVersionId,
    adminJobCaseId,
    adminResultDraft,
  } = adminSecurityState;

  const workflowController = useWorkbenchWorkflowController({
    labels: {
      workflowCatalogLoaded: props.t.workflowCatalogLoaded,
      workflowCatalogUnsupported: props.t.workflowCatalogUnsupported,
      workflowCatalogQueued: props.t.workflowCatalogQueued,
      workflowCatalogCompleted: props.t.workflowCatalogCompleted,
      workflowCatalogFailed: props.t.workflowCatalogFailed,
      initialFailed: props.t.initialFailed,
      pollingDetached: props.t.pollingDetached,
    },
    jobPollTokenRef: props.jobPollTokenRef,
    refreshJobHistory,
    setRuntimeRecovery: props.setRuntimeRecovery,
    setJob: props.setJob,
    setMessage: props.setMessage,
    setSystemAlerts: props.setSystemAlerts,
    openWorkflowRunsSurface: () => {
      props.setSidebarSection("workflow");
    },
  });
  const sessionWorkflowController = {
    ...workflowController,
    runWorkflowCatalogEntry: async (workflowId: string) => {
      workflowController.setSelectedWorkflowId(workflowId);
      workflowController.setWorkflowPanelTab("runs");
      await workflowController.runWorkflowCatalogEntry(workflowId);
    },
  };

  const { selectedProject, selectedProjectModels, selectedAdminJob, selectedAdminResult } = buildWorkbenchAdminSelections({
    jobHistory,
    resultRecords: props.resultRecords,
    projects: props.projects,
    selectedProjectId: props.selectedProjectId,
    selectedAdminJobId,
    selectedAdminResultJobId,
  });
  const deferredProjectModels = useDeferredValue(selectedProjectModels);
  const deferredModelVersions = useDeferredValue(props.modelVersions);
  const deferredJobHistory = useDeferredValue(jobHistory);
  const deferredResultRecords = useDeferredValue(props.resultRecords);

  const {
    refreshHealth,
    refreshProjects,
    refreshSecurityEvents,
    refreshVersions,
  } = useWorkbenchDataRefreshController({
    directMeshEndpointsText: props.directMeshEndpointsText,
    directMeshSelectionMode: props.directMeshSelectionMode,
    frontendRuntimeMode: props.frontendRuntimeMode,
    securityEventActionFilter,
    securityEventRiskFilter,
    securityEventSourceFilter,
    securityEventStatusFilter,
    securityEventWindowFilter,
    selectedModelId: props.selectedModelId,
    selectedProjectId: props.selectedProjectId,
    setHealth: props.setHealth,
    setModelVersions: props.setModelVersions,
    setProjects: props.setProjects,
    setProtocolAgents: props.setProtocolAgents,
    setRuntimeRecovery: props.setRuntimeRecovery,
    setSecurityEventRecords: adminSecurityState.setSecurityEventRecords,
    setSelectedModelId: props.setSelectedModelId,
    setSelectedProjectId: props.setSelectedProjectId,
    setSelectedVersionId: props.setSelectedVersionId,
    refreshJobHistory,
    refreshResults: props.refreshResults,
    securityEventWindowMs: props.securityEventWindowMs,
  });

  const projectFlows = buildWorkbenchProjectFlows({
    selectedAdminJob,
    selectedAdminJobId,
    selectedAdminResultJobId,
    jobHistory,
    projects: props.projects,
    refreshVersions,
    setAdminFilterProjectId: adminSecurityState.setAdminFilterProjectId,
    setAdminFilterModelVersionId: adminSecurityState.setAdminFilterModelVersionId,
    setAdminJobCaseId: adminSecurityState.setAdminJobCaseId,
    setLibraryTab: props.setLibraryTab,
    setSelectedProjectId: props.setSelectedProjectId,
    setSelectedModelId: props.setSelectedModelId,
    setSelectedVersionId: props.setSelectedVersionId,
    setModelVersions: props.setModelVersions,
    setSidebarSection: props.setSidebarSection,
    setMessage: props.setMessage,
    setSystemAlerts: props.setSystemAlerts,
    language: props.language,
    t: props.t,
    startTransition: props.startTransition,
    activeMaterial: props.activeMaterial,
    createProject: props.createProject,
    createModel: props.createModel,
    createModelVersion: props.createModelVersion,
    updateModelVersion: props.updateModelVersion,
    fetchModel: props.fetchModel,
    fetchModelVersion: props.fetchModelVersion,
    refreshProjects,
    recordHistory: props.recordHistory,
    resetActiveResult: props.resetActiveResult,
    setLoadedModelName: props.setLoadedModelName,
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
    selectedProject,
    selectedProjectId: props.selectedProjectId,
    selectedProjectModels,
    selectedModelId: props.selectedModelId,
    selectedVersionId: props.selectedVersionId,
    projectNameDraft: props.projectNameDraft,
    projectDescriptionDraft: props.projectDescriptionDraft,
    loadedModelName: props.loadedModelName,
    studyKind: props.studyKind,
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
    updateProject: props.updateProject,
    deleteProject: props.deleteProject,
    updateModel: props.updateModel,
    deleteModel: props.deleteModel,
    deleteModelVersion: props.deleteModelVersion,
    fetchModelVersions: props.fetchModelVersions,
    fetchJobStatus: props.fetchJobStatus,
    serializeCurrentModel: props.serializeCurrentModel,
  });

  return {
    jobHistoryController,
    adminSecurityState,
    workflowController: sessionWorkflowController,
    selectedProject,
    selectedProjectModels,
    selectedAdminJob,
    selectedAdminResult,
    deferredProjectModels,
    deferredModelVersions,
    deferredJobHistory,
    deferredResultRecords,
    refreshHealth,
    refreshProjects,
    refreshSecurityEvents,
    refreshVersions,
    retryRuntimeRecovery: async () => {
      await Promise.allSettled([
        refreshHealth(),
        refreshProjects(),
        refreshSecurityEvents(),
        sessionWorkflowController.refreshWorkflowCatalog(),
      ]);
    },
    projectFlows,
    adminJobMessage,
    adminJobProjectId,
    adminJobModelVersionId,
    adminJobCaseId,
    adminResultDraft,
    adminFilterProjectId,
    adminFilterModelVersionId,
    securityEventRecords,
  };
}
