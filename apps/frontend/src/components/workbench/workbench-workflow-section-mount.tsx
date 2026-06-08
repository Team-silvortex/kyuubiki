"use client";

import type {
  JobState,
  WorkflowCatalogEntry,
  WorkflowGraphDefinition,
  WorkflowOperatorDescriptor,
} from "@/lib/api";

import { WorkbenchWorkflowSidebar } from "@/components/workbench/workflow/workbench-workflow-sidebar";
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
  latestJob: JobState | null;
  latestWorkflowSummary: string | null;
  workflowRuns: WorkflowRunRecord[];
  refreshWorkflowCatalog: () => Promise<void>;
  setSelectedWorkflowId: (workflowId: string | null) => void;
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
  latestJob,
  latestWorkflowSummary,
  workflowRuns,
  refreshWorkflowCatalog,
  setSelectedWorkflowId,
  runWorkflowCatalogEntry,
  runWorkflowDraft,
  openHistoryJob,
}: WorkbenchWorkflowSectionMountProps) {
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
