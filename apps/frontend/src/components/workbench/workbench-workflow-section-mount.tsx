"use client";

import type { JobState, WorkflowCatalogEntry } from "@/lib/api";

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
  workflowCatalogBusy: boolean;
  selectedWorkflowId: string | null;
  selectedWorkflow: WorkflowCatalogEntry | null;
  latestJob: JobState | null;
  latestWorkflowSummary: string | null;
  workflowRuns: WorkflowRunRecord[];
  refreshWorkflowCatalog: () => Promise<void>;
  setSelectedWorkflowId: (workflowId: string | null) => void;
  runWorkflowCatalogEntry: (workflowId: string) => void;
  openHistoryJob: (jobId: string) => void;
};

export function WorkbenchWorkflowSectionMount({
  surfaceTab,
  onSurfaceTabChange,
  labels,
  workflowCatalogEntries,
  workflowCatalogBusy,
  selectedWorkflowId,
  selectedWorkflow,
  latestJob,
  latestWorkflowSummary,
  workflowRuns,
  refreshWorkflowCatalog,
  setSelectedWorkflowId,
  runWorkflowCatalogEntry,
  openHistoryJob,
}: WorkbenchWorkflowSectionMountProps) {
  return (
    <WorkbenchWorkflowSidebar
      surfaceTab={surfaceTab}
      onSurfaceTabChange={onSurfaceTabChange}
      labels={labels}
      workflowCatalogEntries={workflowCatalogEntries}
      workflowCatalogBusy={workflowCatalogBusy}
      selectedWorkflowId={selectedWorkflowId}
      selectedWorkflow={selectedWorkflow}
      latestJob={latestJob}
      latestWorkflowSummary={latestWorkflowSummary}
      workflowRuns={workflowRuns}
      onRefreshWorkflowCatalog={() => void refreshWorkflowCatalog()}
      onSelectWorkflow={(workflowId) => setSelectedWorkflowId(workflowId)}
      onRunWorkflowCatalog={runWorkflowCatalogEntry}
      onOpenWorkflowRun={openHistoryJob}
    />
  );
}
