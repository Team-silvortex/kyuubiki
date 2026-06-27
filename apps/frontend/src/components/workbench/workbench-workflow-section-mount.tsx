"use client";

import { useEffect } from "react";
import type { Dispatch, SetStateAction } from "react";
import type { WorkbenchAlertItem } from "@/components/workbench/workbench-alert-strip";
import type {
  JobState,
  ProtocolAgentDescriptor,
  WorkflowCatalogEntry,
  WorkflowGraphDefinition,
  WorkflowOperatorDescriptor,
  WorkflowOperatorModuleSummary,
} from "@/lib/api";
import type { HeatPlaneStudyJobInput, PlaneStudyJobInput, StudyKind } from "@/components/workbench/workbench-types";

import { WorkbenchWorkflowSidebar } from "@/components/workbench/workflow/workbench-workflow-sidebar";
import {
  installWorkflowDebugBridge,
  markWorkflowSurfaceIntent,
} from "@/components/workbench/workflow/workbench-workflow-perf";
import type {
  WorkflowRunRecord,
  WorkflowSidebarLabels,
  WorkflowSurfaceTab,
} from "@/components/workbench/workflow/workbench-workflow-types";

type WorkbenchWorkflowSectionMountProps = {
  surfaceTab: WorkflowSurfaceTab;
  onSurfaceTabChange: (tab: WorkflowSurfaceTab) => void;
  labels: WorkflowSidebarLabels;
  workflowCatalogEntries: WorkflowCatalogEntry[];
  workflowOperatorDescriptors?: WorkflowOperatorDescriptor[];
  workflowOperatorModules?: WorkflowOperatorModuleSummary[];
  workflowCatalogBusy: boolean;
  selectedWorkflowId: string | null;
  selectedWorkflow: WorkflowCatalogEntry | null;
  currentStudyKind: StudyKind;
  currentHeatPlaneModel: HeatPlaneStudyJobInput;
  currentPlaneModel: PlaneStudyJobInput;
  latestJob: JobState | null;
  latestWorkflowSummary: string | null;
  workflowRuns: WorkflowRunRecord[];
  protocolAgents: ProtocolAgentDescriptor[];
  frontendRuntimeMode: "orchestrated_gui" | "direct_mesh_gui";
  refreshWorkflowCatalog: () => Promise<void>;
  setSelectedWorkflowId: (workflowId: string | null) => void;
  setWorkflowRuns: Dispatch<SetStateAction<WorkflowRunRecord[]>>;
  runWorkflowCatalogEntry: (workflowId: string) => void;
  runWorkflowDraft: (
    workflowId: string,
    graph: WorkflowGraphDefinition,
    inputArtifacts: Record<string, unknown>,
  ) => void;
  openHistoryJob: (jobId: string) => void;
  setSystemAlerts: Dispatch<SetStateAction<WorkbenchAlertItem[]>>;
};

export function WorkbenchWorkflowSectionMount({
  surfaceTab,
  onSurfaceTabChange,
  labels,
  workflowCatalogEntries,
  workflowOperatorDescriptors,
  workflowOperatorModules,
  workflowCatalogBusy,
  selectedWorkflowId,
  selectedWorkflow,
  currentStudyKind,
  currentHeatPlaneModel,
  currentPlaneModel,
  latestJob,
  latestWorkflowSummary,
  workflowRuns,
  protocolAgents,
  frontendRuntimeMode,
  refreshWorkflowCatalog,
  setSelectedWorkflowId,
  setWorkflowRuns,
  runWorkflowCatalogEntry,
  runWorkflowDraft,
  openHistoryJob,
  setSystemAlerts,
}: WorkbenchWorkflowSectionMountProps) {
  useEffect(() => {
    installWorkflowDebugBridge({
      getState: () => ({
        selectedWorkflowId,
        catalogWorkflowIds: workflowCatalogEntries.map((entry) => entry.id),
        workflowRunCount: workflowRuns.length,
      }),
      setSurfaceTab: (tab) => {
        markWorkflowSurfaceIntent(tab);
        onSurfaceTabChange(tab);
      },
      setSelectedWorkflowId,
      replaceRuns: (runs) => setWorkflowRuns(runs),
    });
    return () => installWorkflowDebugBridge(null);
  }, [onSurfaceTabChange, selectedWorkflowId, setSelectedWorkflowId, setWorkflowRuns, workflowCatalogEntries, workflowRuns.length]);

  return (
    <WorkbenchWorkflowSidebar
      surfaceTab={surfaceTab}
      onSurfaceTabChange={onSurfaceTabChange}
      labels={labels}
      workflowCatalogEntries={workflowCatalogEntries}
      workflowOperatorDescriptors={workflowOperatorDescriptors}
      workflowOperatorModules={workflowOperatorModules}
      workflowCatalogBusy={workflowCatalogBusy}
      selectedWorkflowId={selectedWorkflowId}
      selectedWorkflow={selectedWorkflow}
      currentStudyKind={currentStudyKind}
      currentHeatPlaneModel={currentHeatPlaneModel}
      currentPlaneModel={currentPlaneModel}
      latestJob={latestJob}
      latestWorkflowSummary={latestWorkflowSummary}
      workflowRuns={workflowRuns}
      protocolAgents={protocolAgents}
      frontendRuntimeMode={frontendRuntimeMode}
      onRefreshWorkflowCatalog={() => void refreshWorkflowCatalog()}
      onSelectWorkflow={(workflowId) => setSelectedWorkflowId(workflowId)}
      onRunWorkflowCatalog={runWorkflowCatalogEntry}
      onRunWorkflowDraft={runWorkflowDraft}
      onOpenWorkflowRun={openHistoryJob}
      setSystemAlerts={setSystemAlerts}
    />
  );
}
