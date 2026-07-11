"use client";

import type { Dispatch, SetStateAction } from "react";
import type { WorkbenchAlertItem } from "@/components/workbench/workbench-alert-strip";
import type {
  ProtocolAgentDescriptor,
  WorkflowCatalogEntry,
  WorkflowGraphDefinition,
  WorkflowOperatorDescriptor,
} from "@/lib/api";
import type {
  HeatPlaneStudyJobInput,
  PlaneStudyJobInput,
  StudyKind,
} from "@/components/workbench/workbench-types";
import type {
  WorkflowRunRecord,
  WorkflowSidebarLabels,
} from "@/components/workbench/workflow/workbench-workflow-types";
import type { WorkflowAuditNavigationTarget } from "@/components/workbench/workflow/workbench-workflow-audit-targets";

export const EMPTY_PROTOCOL_AGENTS: ProtocolAgentDescriptor[] = [];

export type WorkbenchWorkflowBuilderCardProps = {
  labels: WorkflowSidebarLabels;
  selectedWorkflow: WorkflowCatalogEntry | null;
  operatorDescriptors?: WorkflowOperatorDescriptor[];
  currentStudyKind: StudyKind;
  currentHeatPlaneModel: HeatPlaneStudyJobInput;
  currentPlaneModel: PlaneStudyJobInput;
  recentRunStatus?: string | null;
  latestRun?: WorkflowRunRecord | null;
  protocolAgents?: ProtocolAgentDescriptor[];
  frontendRuntimeMode: "orchestrated_gui" | "direct_mesh_gui";
  onRefreshWorkflowCatalog: () => void;
  onRunWorkflowCatalog: (workflowId: string) => void;
  onRunWorkflowDraft: (
    workflowId: string,
    graph: WorkflowGraphDefinition,
    inputArtifacts: Record<string, unknown>,
  ) => void;
  setSystemAlerts: Dispatch<SetStateAction<WorkbenchAlertItem[]>>;
  traceFocusNodeId?: string | null;
  traceFocusToken?: number;
  traceFocusBranchNodeId?: string | null;
  traceFocusBranchOutputId?: string | null;
  traceFocusBranchToken?: number;
  traceFocusDatasetNodeId?: string | null;
  traceFocusDatasetPortId?: string | null;
  traceFocusDatasetToken?: number;
  onLocateAuditTarget?: (target: WorkflowAuditNavigationTarget) => void;
};
