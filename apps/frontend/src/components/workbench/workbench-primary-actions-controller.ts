"use client";

import {
  applySelectedAdminJobContext as applySelectedAdminJobContextWithDeps,
  applySelectedAdminResultContext as applySelectedAdminResultContextWithDeps,
  openSelectedAdminJobProject as openSelectedAdminJobProjectWithDeps,
  openSelectedAdminJobVersion as openSelectedAdminJobVersionWithDeps,
  openSelectedAdminResultProject as openSelectedAdminResultProjectWithDeps,
  openSelectedAdminResultVersion as openSelectedAdminResultVersionWithDeps,
} from "@/components/workbench/workbench-admin-data-controller";
import {
  deleteWorkbenchAdminResultRecord,
  exportWorkbenchAdminResultRecord,
  refreshWorkbenchResults,
  saveWorkbenchAdminResultRecord,
} from "@/components/workbench/workbench-admin-result-controller";
import { resetActiveResult } from "@/components/workbench/workbench-file-helpers";
import { applyHistoryJobPayload } from "@/components/workbench/workbench-history-result";
import {
  importWorkbenchModelFile,
  openWorkbenchSample,
} from "@/components/workbench/workbench-sample-import-controller";
import { runWorkbenchAnalysis } from "@/components/workbench/workbench-run-controller";
import {
  ensurePlaneModelMaterials,
  ensureTrussModelMaterials,
} from "@/lib/workbench/material-commands";
import { scientific, serializeCurrentModel } from "@/lib/workbench/helpers";
import {
  generatePrattTruss,
  generateRectangularPanelMesh,
  generateRectangularQuadPanelMesh,
} from "@/lib/models";

type PrimaryActionsControllerDeps = {
  t: any;
  setMessage: (value: string) => void;
  recordHistory: (label: string) => void;
  setAxialForm: (value: any) => void;
  setParametric: (value: any) => void;
  setPanelParametric: (value: any) => void;
  startTransition: (callback: () => void) => void;
  resultRefreshSeqRef: { current: number };
  fetchResults: () => Promise<{ results: any[] }>;
  setResultRecords: (value: any[] | ((current: any[]) => any[])) => void;
  setSelectedAdminResultJobId: (value: any) => void;
  directMeshEndpointsText: string;
  directMeshSelectionMode: any;
  frontendRuntimeMode: any;
  jobPollTokenRef: { current: number };
  selectedProjectId: string | null;
  selectedVersionId: string | null;
  setDirectMeshExecution: (value: any) => void;
  setJob: (value: any) => void;
  setResult: (value: any) => void;
  studyKind: any;
  axialForm: any;
  beamModel: any;
  frameModel: any;
  heatBarModel: any;
  heatPlaneModel: any;
  planeModel: any;
  springModel: any;
  spring2dModel: any;
  spring3dModel: any;
  thermalBarModel: any;
  thermalBeamModel: any;
  thermalFrameModel: any;
  thermalTrussModel: any;
  thermalTruss3dModel: any;
  torsionModel: any;
  trussModel: any;
  truss3dModel: any;
  trussDiagnostics: any;
  refreshJobHistory: () => Promise<void>;
  fetchJobStatus: (jobId: string) => Promise<any>;
  setSidebarSection: (value: any) => void;
  setWorkflowPanelTab: (value: any) => void;
  setSelectedWorkflowId: (value: any) => void;
  setWorkflowRuns: (value: any) => void;
  openWorkspaceStudy: (tab?: any) => void;
  setStudyKind: (value: any) => void;
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
  setLoadedModelName: (value: string) => void;
  setSelectedModelId: (value: string | null) => void;
  setSelectedVersionId: (value: string | null) => void;
  setModelVersions: (value: any[]) => void;
  setActiveMaterial: (value: string) => void;
  setSelectedNode: (value: number | null) => void;
  setSelectedElement: (value: number | null) => void;
  setMemberDraftNodes: (value: number[]) => void;
  activeMaterial: string;
  round: (value: number, digits?: number) => number;
  parametric: any;
  panelParametric: any;
  loadedModelName: string;
  updateJobRecord: (jobId: string, payload: Record<string, unknown>) => Promise<unknown>;
  deleteJobRecord: (jobId: string) => Promise<unknown>;
  selectedAdminJobId: string | null;
  adminJobMessage: string;
  adminJobProjectId: string;
  adminJobModelVersionId: string;
  adminJobCaseId: string;
  updateResultRecord: (jobId: string, payload: Record<string, unknown>) => Promise<unknown>;
  deleteResultRecord: (jobId: string) => Promise<unknown>;
  adminResultDraft: string;
  selectedAdminResultJobId: string | null;
  downloadTextFile: (name: string, contents: string) => void;
  adminDataEffects: any;
};

export function createWorkbenchPrimaryActionsController(deps: PrimaryActionsControllerDeps) {
  async function refreshResults() {
    await refreshWorkbenchResults({
      resultRefreshSeqRef: deps.resultRefreshSeqRef,
      fetchResults: deps.fetchResults,
      setResultRecords: deps.setResultRecords,
      setSelectedAdminResultJobId: deps.setSelectedAdminResultJobId,
    });
  }

  const runAnalysis = () => {
    deps.startTransition(() => {
      void (async () => {
        try {
          await runWorkbenchAnalysis({
            axialForm: deps.axialForm,
            beamModel: deps.beamModel,
            copy: deps.t,
            directMeshEndpointsText: deps.directMeshEndpointsText,
            directMeshSelectionMode: deps.directMeshSelectionMode,
            frontendRuntimeMode: deps.frontendRuntimeMode,
            frameModel: deps.frameModel,
            heatBarModel: deps.heatBarModel,
            heatPlaneModel: deps.heatPlaneModel,
            jobPollTokenRef: deps.jobPollTokenRef,
            labels: {
              precheckPrefix: deps.t.precheckPrefix,
              dispatching: deps.t.dispatching,
              directMeshEndpointsHelp: deps.t.directMeshEndpointsHelp,
              directMeshCompleted: deps.t.directMeshCompleted,
              requestTimedOut: deps.t.requestTimedOut,
              initialFailed: deps.t.initialFailed,
              pollingDetached: deps.t.pollingDetached,
            },
            planeModel: deps.planeModel,
            refreshJobHistory: deps.refreshJobHistory,
            selectedProjectId: deps.selectedProjectId,
            selectedVersionId: deps.selectedVersionId,
            setDirectMeshExecution: deps.setDirectMeshExecution,
            setJob: deps.setJob,
            setMessage: deps.setMessage,
            setResult: deps.setResult,
            spring2dModel: deps.spring2dModel,
            spring3dModel: deps.spring3dModel,
            springModel: deps.springModel,
            studyKind: deps.studyKind,
            thermalBarModel: deps.thermalBarModel,
            thermalBeamModel: deps.thermalBeamModel,
            thermalFrameModel: deps.thermalFrameModel,
            thermalTruss3dModel: deps.thermalTruss3dModel,
            thermalTrussModel: deps.thermalTrussModel,
            torsionModel: deps.torsionModel,
            truss3dModel: deps.truss3dModel,
            trussDiagnostics: deps.trussDiagnostics,
            trussModel: deps.trussModel,
          });
        } catch (error) {
          deps.setMessage(
            error instanceof Error
              ? error.message.startsWith("request timed out:")
                ? deps.t.requestTimedOut
                : error.message
              : deps.t.initialFailed,
          );
        }
      })();
    });
  };

  const openHistoryJob = (jobId: string) => {
    deps.jobPollTokenRef.current += 1;

    deps.startTransition(() => {
      void (async () => {
        try {
          const payload = await deps.fetchJobStatus(jobId);
          applyHistoryJobPayload(payload, {
            activeMaterial: deps.activeMaterial,
            copy: {
              historyAction: deps.t.historyAction,
              historyLoaded: deps.t.historyLoaded,
              workflowCatalogCompleted: deps.t.workflowCatalogCompleted,
            },
            setJob: deps.setJob,
            setResult: deps.setResult,
            setSidebarSection: deps.setSidebarSection,
            setWorkflowPanelTab: deps.setWorkflowPanelTab,
            setSelectedWorkflowId: deps.setSelectedWorkflowId,
            setWorkflowRuns: deps.setWorkflowRuns,
            setMessage: deps.setMessage,
            recordHistory: deps.recordHistory,
            openWorkspaceStudy: deps.openWorkspaceStudy,
            setStudyKind: deps.setStudyKind,
            setAxialForm: deps.setAxialForm,
            setThermalBarModel: deps.setThermalBarModel,
            setHeatBarModel: deps.setHeatBarModel,
            setHeatPlaneModel: deps.setHeatPlaneModel,
            setPlaneResultField: deps.setPlaneResultField,
            setThermalBeamModel: deps.setThermalBeamModel,
            setThermalTrussModel: deps.setThermalTrussModel,
            setThermalTruss3dModel: deps.setThermalTruss3dModel,
            setSpringModel: deps.setSpringModel,
            setSpring2dModel: deps.setSpring2dModel,
            setSpring3dModel: deps.setSpring3dModel,
            setBeamModel: deps.setBeamModel,
            setTorsionModel: deps.setTorsionModel,
            setTrussModel: deps.setTrussModel,
            setTruss3dModel: deps.setTruss3dModel,
            setFrameModel: deps.setFrameModel,
            setThermalFrameModel: deps.setThermalFrameModel,
            setPlaneModel: deps.setPlaneModel,
          });
        } catch (error) {
          deps.setMessage(error instanceof Error ? error.message : deps.t.initialFailed);
        }
      })();
    });
  };

  const importModel = async (file: File | undefined) => {
    await importWorkbenchModelFile({
      file,
      labels: {
        importAction: deps.t.importAction,
        importedModel: deps.t.importedModel,
        importFailed: deps.t.importFailed,
      },
      recordHistory: deps.recordHistory,
      applyImportedModel: {
        setLoadedModelName: deps.setLoadedModelName,
        setSelectedModelId: deps.setSelectedModelId,
        setSelectedVersionId: deps.setSelectedVersionId,
        setModelVersions: deps.setModelVersions,
        setStudyKind: deps.setStudyKind,
        setAxialForm: deps.setAxialForm,
        setHeatBarModel: deps.setHeatBarModel,
        setHeatPlaneModel: deps.setHeatPlaneModel,
        setThermalBarModel: deps.setThermalBarModel,
        setThermalBeamModel: deps.setThermalBeamModel,
        setThermalFrameModel: deps.setThermalFrameModel,
        setThermalTrussModel: deps.setThermalTrussModel,
        setThermalTruss3dModel: deps.setThermalTruss3dModel,
        setSpringModel: deps.setSpringModel,
        setSpring2dModel: deps.setSpring2dModel,
        setSpring3dModel: deps.setSpring3dModel,
        setTrussModel: deps.setTrussModel,
        setTruss3dModel: deps.setTruss3dModel,
        setPlaneModel: deps.setPlaneModel,
        setFrameModel: deps.setFrameModel,
        setBeamModel: deps.setBeamModel,
        setTorsionModel: deps.setTorsionModel,
        setPlaneResultField: deps.setPlaneResultField,
        setParametric: deps.setParametric,
        setActiveMaterial: deps.setActiveMaterial,
      },
      setMessage: deps.setMessage,
    });
  };

  const openSample = (href: string) => {
    deps.startTransition(() => {
      void openWorkbenchSample({
        href,
        labels: {
          sampleAction: deps.t.sampleAction,
          importedModel: deps.t.importedModel,
          importFailed: deps.t.importFailed,
          requestTimedOut: deps.t.requestTimedOut,
        },
        recordHistory: deps.recordHistory,
        applyImportedModel: {
          setLoadedModelName: deps.setLoadedModelName,
          setSelectedModelId: deps.setSelectedModelId,
          setSelectedVersionId: deps.setSelectedVersionId,
          setModelVersions: deps.setModelVersions,
          setStudyKind: deps.setStudyKind,
          setAxialForm: deps.setAxialForm,
          setHeatBarModel: deps.setHeatBarModel,
          setHeatPlaneModel: deps.setHeatPlaneModel,
          setThermalBarModel: deps.setThermalBarModel,
          setThermalBeamModel: deps.setThermalBeamModel,
          setThermalFrameModel: deps.setThermalFrameModel,
          setThermalTrussModel: deps.setThermalTrussModel,
          setThermalTruss3dModel: deps.setThermalTruss3dModel,
          setSpringModel: deps.setSpringModel,
          setSpring2dModel: deps.setSpring2dModel,
          setSpring3dModel: deps.setSpring3dModel,
          setTrussModel: deps.setTrussModel,
          setTruss3dModel: deps.setTruss3dModel,
          setPlaneModel: deps.setPlaneModel,
          setFrameModel: deps.setFrameModel,
          setBeamModel: deps.setBeamModel,
          setTorsionModel: deps.setTorsionModel,
          setPlaneResultField: deps.setPlaneResultField,
          setParametric: deps.setParametric,
          setActiveMaterial: deps.setActiveMaterial,
        },
        setMessage: deps.setMessage,
      });
    });
  };

  const handleAxialFieldChange = (key: string, value: number | string) => {
    deps.recordHistory(deps.t.editAxialField);
    deps.setAxialForm((current: any) => ({ ...current, [key]: value }));
  };

  const handleParametricChange = (key: string, value: number) => {
    deps.recordHistory(deps.t.editParametric);
    deps.setParametric((current: any) => ({ ...current, [key]: value }));
  };

  const handlePanelParametricChange = (key: string, value: number) => {
    deps.recordHistory(deps.t.editParametric);
    deps.setPanelParametric((current: any) => ({ ...current, [key]: value }));
  };

  const generateModel = () => {
    deps.recordHistory(deps.t.generateAction);
    const nextModel = ensureTrussModelMaterials(generatePrattTruss(deps.parametric), deps.activeMaterial);
    deps.setStudyKind("truss_2d");
    deps.setTrussModel(nextModel);
    deps.setSelectedNode(null);
    deps.setSelectedElement(null);
    deps.setSelectedModelId(null);
    deps.setSelectedVersionId(null);
    deps.setModelVersions([]);
    deps.setMemberDraftNodes([]);
    deps.setLoadedModelName("parametric-pratt-truss");
    deps.setMessage(deps.t.generatedModel);
    deps.setSidebarSection("model");
  };

  const generatePanelModel = () => {
    deps.recordHistory(deps.t.generateAction);
    const nextModel = ensurePlaneModelMaterials(
      deps.studyKind === "plane_quad_2d"
        ? generateRectangularQuadPanelMesh(deps.panelParametric)
        : generateRectangularPanelMesh(deps.panelParametric),
      deps.activeMaterial,
    );
    deps.setStudyKind(deps.studyKind === "plane_quad_2d" ? "plane_quad_2d" : "plane_triangle_2d");
    deps.setPlaneModel(nextModel);
    deps.setSelectedNode(null);
    deps.setSelectedElement(null);
    deps.setSelectedModelId(null);
    deps.setSelectedVersionId(null);
    deps.setModelVersions([]);
    deps.setMemberDraftNodes([]);
    deps.setLoadedModelName("parametric-panel-mesh");
    deps.setMessage(deps.t.panelGenerated);
    deps.setSidebarSection("model");
    resetActiveResult(deps.setResult, deps.setJob);
  };

  const downloadModel = () => {
    const contents = JSON.stringify(
      serializeCurrentModel(
        deps.studyKind,
        deps.loadedModelName,
        deps.activeMaterial,
        deps.axialForm,
        deps.heatBarModel,
        deps.heatPlaneModel,
        deps.thermalBarModel,
        deps.thermalBeamModel,
        deps.thermalFrameModel,
        deps.thermalTrussModel,
        deps.trussModel,
        deps.thermalTruss3dModel,
        deps.truss3dModel,
        deps.planeModel,
        deps.frameModel,
        deps.beamModel,
        deps.torsionModel,
        deps.springModel,
        deps.spring2dModel,
        deps.spring3dModel,
        deps.parametric,
        deps.round,
      ),
      null,
      2,
    );

    deps.downloadTextFile(`${deps.loadedModelName || "kyuubiki-model"}.json`, contents);
    deps.setMessage(deps.t.modelDownloaded);
  };

  const saveAdminJobRecord = () => {
    if (!deps.selectedAdminJobId) return;

    deps.startTransition(() => {
      void (async () => {
        try {
          await deps.updateJobRecord(deps.selectedAdminJobId!, {
            message: deps.adminJobMessage,
            project_id: deps.adminJobProjectId || undefined,
            model_version_id: deps.adminJobModelVersionId || undefined,
            simulation_case_id: deps.adminJobCaseId || undefined,
          });
          await deps.refreshJobHistory();
          deps.setMessage(deps.t.jobSaved);
        } catch (error) {
          deps.setMessage(error instanceof Error ? error.message : deps.t.initialFailed);
        }
      })();
    });
  };

  const deleteAdminJobRecord = () => {
    if (!deps.selectedAdminJobId) return;

    deps.startTransition(() => {
      void (async () => {
        try {
          await deps.deleteJobRecord(deps.selectedAdminJobId!);
          await deps.refreshJobHistory();
          await refreshResults();
          deps.setMessage(deps.t.jobDeleted);
        } catch (error) {
          deps.setMessage(error instanceof Error ? error.message : deps.t.initialFailed);
        }
      })();
    });
  };

  const adminResultLabels = {
    resultSaved: deps.t.resultSaved,
    resultDeleted: deps.t.resultDeleted,
    resultJsonDownloaded: deps.t.resultJsonDownloaded,
    invalidJson: deps.t.invalidJson,
    initialFailed: deps.t.initialFailed,
  };

  const saveAdminResultRecord = () => {
    if (!deps.selectedAdminResultJobId) return;
    deps.startTransition(() => {
      void saveWorkbenchAdminResultRecord({
        resultRefreshSeqRef: deps.resultRefreshSeqRef,
        fetchResults: deps.fetchResults,
        setResultRecords: deps.setResultRecords as any,
        setSelectedAdminResultJobId: deps.setSelectedAdminResultJobId,
        selectedAdminResultJobId: deps.selectedAdminResultJobId,
        adminResultDraft: deps.adminResultDraft,
        updateResultRecord: deps.updateResultRecord,
        deleteResultRecord: deps.deleteResultRecord,
        downloadTextFile: deps.downloadTextFile,
        setMessage: deps.setMessage,
        labels: adminResultLabels,
      });
    });
  };

  const deleteAdminResultRecord = () => {
    if (!deps.selectedAdminResultJobId) return;
    deps.startTransition(() => {
      void deleteWorkbenchAdminResultRecord({
        resultRefreshSeqRef: deps.resultRefreshSeqRef,
        fetchResults: deps.fetchResults,
        setResultRecords: deps.setResultRecords as any,
        setSelectedAdminResultJobId: deps.setSelectedAdminResultJobId,
        selectedAdminResultJobId: deps.selectedAdminResultJobId,
        adminResultDraft: deps.adminResultDraft,
        updateResultRecord: deps.updateResultRecord,
        deleteResultRecord: deps.deleteResultRecord,
        downloadTextFile: deps.downloadTextFile,
        setMessage: deps.setMessage,
        labels: adminResultLabels,
      });
    });
  };

  const exportAdminResultRecord = () => {
    exportWorkbenchAdminResultRecord({
      resultRefreshSeqRef: deps.resultRefreshSeqRef,
      fetchResults: deps.fetchResults,
      setResultRecords: deps.setResultRecords as any,
      setSelectedAdminResultJobId: deps.setSelectedAdminResultJobId,
      selectedAdminResultJobId: deps.selectedAdminResultJobId,
      adminResultDraft: deps.adminResultDraft,
      updateResultRecord: deps.updateResultRecord,
      deleteResultRecord: deps.deleteResultRecord,
      downloadTextFile: deps.downloadTextFile,
      setMessage: deps.setMessage,
      labels: adminResultLabels,
    });
  };

  const openSelectedAdminJobVersion = () => {
    openSelectedAdminJobVersionWithDeps(deps.adminDataEffects);
  };

  const openSelectedAdminResultVersion = () => {
    openSelectedAdminResultVersionWithDeps(deps.adminDataEffects);
  };

  const openSelectedAdminJobProject = () => {
    openSelectedAdminJobProjectWithDeps(deps.adminDataEffects);
  };

  const openSelectedAdminResultProject = () => {
    openSelectedAdminResultProjectWithDeps(deps.adminDataEffects);
  };

  const applySelectedAdminJobContext = () => {
    applySelectedAdminJobContextWithDeps(deps.adminDataEffects);
  };

  const applySelectedAdminResultContext = () => {
    applySelectedAdminResultContextWithDeps(deps.adminDataEffects);
  };

  return {
    refreshResults,
    runAnalysis,
    openHistoryJob,
    importModel,
    openSample,
    handleAxialFieldChange,
    handleParametricChange,
    handlePanelParametricChange,
    generateModel,
    generatePanelModel,
    downloadModel,
    saveAdminJobRecord,
    deleteAdminJobRecord,
    saveAdminResultRecord,
    deleteAdminResultRecord,
    exportAdminResultRecord,
    openSelectedAdminJobVersion,
    openSelectedAdminResultVersion,
    openSelectedAdminJobProject,
    openSelectedAdminResultProject,
    applySelectedAdminJobContext,
    applySelectedAdminResultContext,
  };
}
