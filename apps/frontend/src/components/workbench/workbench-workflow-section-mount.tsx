"use client";

import { useEffect } from "react";
import type { Dispatch, SetStateAction } from "react";
import type {
  JobState,
  WorkflowCatalogEntry,
  WorkflowGraphDefinition,
  WorkflowOperatorDescriptor,
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
  workflowCatalogBusy: boolean;
  selectedWorkflowId: string | null;
  selectedWorkflow: WorkflowCatalogEntry | null;
  currentStudyKind: StudyKind;
  currentHeatPlaneModel: HeatPlaneStudyJobInput;
  currentPlaneModel: PlaneStudyJobInput;
  latestJob: JobState | null;
  latestWorkflowSummary: string | null;
  workflowRuns: WorkflowRunRecord[];
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
};

export function WorkbenchWorkflowSectionMount({
  surfaceTab,
  onSurfaceTabChange,
  labels,
  workflowCatalogEntries,
  workflowOperatorDescriptors,
  workflowCatalogBusy,
  selectedWorkflowId,
  selectedWorkflow,
  currentStudyKind,
  currentHeatPlaneModel,
  currentPlaneModel,
  latestJob,
  latestWorkflowSummary,
  workflowRuns,
  refreshWorkflowCatalog,
  setSelectedWorkflowId,
  setWorkflowRuns,
  runWorkflowCatalogEntry,
  runWorkflowDraft,
  openHistoryJob,
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
      workflowCatalogBusy={workflowCatalogBusy}
      selectedWorkflowId={selectedWorkflowId}
      selectedWorkflow={selectedWorkflow}
      currentStudyKind={currentStudyKind}
      currentHeatPlaneModel={currentHeatPlaneModel}
      currentPlaneModel={currentPlaneModel}
      latestJob={latestJob}
      latestWorkflowSummary={latestWorkflowSummary}
      workflowRuns={workflowRuns}
      onRefreshWorkflowCatalog={() => void refreshWorkflowCatalog()}
      onSelectWorkflow={(workflowId) => setSelectedWorkflowId(workflowId)}
      onRunWorkflowCatalog={runWorkflowCatalogEntry}
      onRunWorkflowDraft={runWorkflowDraft}
      onOpenWorkflowRun={openHistoryJob}
    />
  );
}
