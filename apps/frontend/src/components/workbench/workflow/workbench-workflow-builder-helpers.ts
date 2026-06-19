"use client";

import type { Dispatch, RefObject, SetStateAction } from "react";
import type { WorkbenchAlertItem } from "@/components/workbench/workbench-alert-strip";
import { dismissWorkbenchAlert, upsertWorkbenchAlert } from "@/components/workbench/workbench-alert-state";
import {
  dismissWorkbenchNotice,
  showWorkbenchNotice,
  type WorkbenchNoticeItem,
} from "@/components/workbench/workbench-notice-state";
import type { WorkflowCatalogEntryArtifact, WorkflowDatasetValueInfo } from "@/lib/api";
import type { WorkflowValidationFixSummaryEntry } from "@/components/workbench/workflow/workbench-workflow-validation-summary";
import { flashWorkflowFixReceiptHighlights } from "@/components/workbench/workflow/workbench-workflow-fix-receipt-flash";
import type { WorkflowBridgeRuntimeValidationIssue } from "@/components/workbench/workflow/workbench-workflow-bridge-runtime-validation";
import type { WorkbenchAuditTimelineEntry } from "@/lib/workbench/workbench-audit-timeline";
import { buildWorkflowAuditReplayPlan } from "@/components/workbench/workflow/workbench-workflow-audit-replay";

export type WorkflowBuilderLocateTarget =
  | { kind: "node"; nodeId: string }
  | { kind: "edge"; edgeId: string }
  | { kind: "dataset"; datasetValueId?: string | null }
  | { kind: "snapshot" }
  | { kind: "local" }
  | {
      kind: "artifact";
      mode: "entry" | "output";
      nodeId: string;
      artifactType: string;
    };

type BuilderNoticeSetter = Dispatch<SetStateAction<WorkbenchNoticeItem | null>>;
type BuilderAlertSetter = Dispatch<SetStateAction<WorkbenchAlertItem[]>>;

type WorkflowBuilderFocusControllers = {
  setFocusedNodeId: Dispatch<SetStateAction<string | null>>;
  setFocusedEdgeId: Dispatch<SetStateAction<string | null>>;
  setFocusedArtifactKey: Dispatch<SetStateAction<string | null>>;
  setFocusedDatasetValueId: Dispatch<SetStateAction<string | null>>;
  setHighlightedNodeIds: Dispatch<SetStateAction<string[]>>;
  setHighlightedEdgeIds: Dispatch<SetStateAction<string[]>>;
  setHighlightedArtifactKeys: Dispatch<SetStateAction<string[]>>;
  setHighlightedPortKeys: Dispatch<SetStateAction<string[]>>;
  setHighlightDatasetEditor: Dispatch<SetStateAction<boolean>>;
};

export function clearWorkflowBuilderAlert(
  setSystemAlerts: BuilderAlertSetter,
  alertId: string,
) {
  dismissWorkbenchAlert(setSystemAlerts, alertId);
}

export function pushWorkflowBuilderAlert(
  setSystemAlerts: BuilderAlertSetter,
  alertId: string,
  message: string,
  tone: WorkbenchAlertItem["tone"] = "error",
) {
  upsertWorkbenchAlert(setSystemAlerts, { id: alertId, message, tone });
}

export function clearWorkflowBuilderNotice(setImportNotice: BuilderNoticeSetter) {
  dismissWorkbenchNotice(setImportNotice);
}

export function showWorkflowBuilderNotice(
  setImportNotice: BuilderNoticeSetter,
  id: string,
  message: string,
  tone: WorkbenchNoticeItem["tone"] = "info",
) {
  showWorkbenchNotice(setImportNotice, { id, message, tone });
}

export function resetWorkflowBuilderFocus(
  controllers: WorkflowBuilderFocusControllers,
) {
  controllers.setFocusedNodeId(null);
  controllers.setFocusedEdgeId(null);
  controllers.setFocusedArtifactKey(null);
  controllers.setFocusedDatasetValueId(null);
  controllers.setHighlightedNodeIds([]);
  controllers.setHighlightedEdgeIds([]);
  controllers.setHighlightedArtifactKeys([]);
  controllers.setHighlightedPortKeys([]);
  controllers.setHighlightDatasetEditor(false);
}

export function flashWorkflowBuilderHighlightedEdges(
  edgeIds: string[],
  setHighlightedEdgeIds: Dispatch<SetStateAction<string[]>>,
) {
  if (edgeIds.length === 0) return;
  setHighlightedEdgeIds(edgeIds);
  window.setTimeout(
    () => setHighlightedEdgeIds((current) => (current === edgeIds ? [] : current)),
    2200,
  );
}

export function flashWorkflowBuilderFixReceiptHighlights(params: {
  builderRootRef: RefObject<HTMLElement | null>;
  summary: WorkflowValidationFixSummaryEntry[];
  setFocusedNodeId: Dispatch<SetStateAction<string | null>>;
  setFocusedEdgeId: Dispatch<SetStateAction<string | null>>;
  setFocusedArtifactKey: Dispatch<SetStateAction<string | null>>;
  setHighlightedNodeIds: Dispatch<SetStateAction<string[]>>;
  setHighlightedEdgeIds: Dispatch<SetStateAction<string[]>>;
  setHighlightedPortKeys: Dispatch<SetStateAction<string[]>>;
  setHighlightedArtifactKeys: Dispatch<SetStateAction<string[]>>;
}) {
  flashWorkflowFixReceiptHighlights(params);
}

export function locateWorkflowBuilderIssue(params: {
  locate: WorkflowBuilderLocateTarget;
  builderRootRef: RefObject<HTMLElement | null>;
  selectedDatasetValues: WorkflowDatasetValueInfo[];
  selectedEntryInputs: WorkflowCatalogEntryArtifact[];
  selectedOutputArtifacts: WorkflowCatalogEntryArtifact[];
  resetBuilderFocus: () => void;
  setFocusedNodeId: Dispatch<SetStateAction<string | null>>;
  setFocusedEdgeId: Dispatch<SetStateAction<string | null>>;
  setSelectedDatasetValueId: Dispatch<SetStateAction<string | null>>;
  setFocusedDatasetValueId: Dispatch<SetStateAction<string | null>>;
  setHighlightDatasetEditor: Dispatch<SetStateAction<boolean>>;
  setFocusedArtifactKey: Dispatch<SetStateAction<string | null>>;
}) {
  const {
    locate,
    builderRootRef,
    selectedDatasetValues,
    selectedEntryInputs,
    selectedOutputArtifacts,
    resetBuilderFocus,
    setFocusedNodeId,
    setFocusedEdgeId,
    setSelectedDatasetValueId,
    setFocusedDatasetValueId,
    setHighlightDatasetEditor,
    setFocusedArtifactKey,
  } = params;
  resetBuilderFocus();
  if (locate.kind === "node") {
    setFocusedNodeId(locate.nodeId);
    queueMicrotask(() => {
      builderRootRef.current
        ?.querySelector<HTMLElement>(`[data-workflow-node-id="${locate.nodeId}"]`)
        ?.scrollIntoView({ block: "nearest", behavior: "smooth" });
    });
    return;
  }
  if (locate.kind === "edge") {
    setFocusedEdgeId(locate.edgeId);
    queueMicrotask(() => {
      builderRootRef.current
        ?.querySelector<HTMLElement>(`[data-workflow-edge-id="${locate.edgeId}"]`)
        ?.scrollIntoView({ block: "nearest", behavior: "smooth" });
    });
    return;
  }
  if (locate.kind === "dataset") {
    const existingValue = locate.datasetValueId
      ? selectedDatasetValues.find((value) => value.id === locate.datasetValueId)
      : null;
    if (existingValue) {
      setSelectedDatasetValueId(existingValue.id);
      setFocusedDatasetValueId(existingValue.id);
    }
    setHighlightDatasetEditor(true);
    queueMicrotask(() => {
      builderRootRef.current
        ?.querySelector<HTMLElement>('[data-workflow-dataset-editor="editor"]')
        ?.scrollIntoView({ block: "nearest", behavior: "smooth" });
    });
    return;
  }
  if (locate.kind === "snapshot" || locate.kind === "local") {
    queueMicrotask(() => {
      builderRootRef.current
        ?.querySelector<HTMLElement>(
          locate.kind === "snapshot"
            ? '[data-workflow-snapshot-card="card"]'
            : '[data-workflow-local-card="card"]',
        )
        ?.scrollIntoView({ block: "nearest", behavior: "smooth" });
    });
    return;
  }
  const artifacts =
    locate.mode === "entry" ? selectedEntryInputs : selectedOutputArtifacts;
  const artifactIndex = artifacts.findIndex(
    (artifact) =>
      artifact.node_id === locate.nodeId &&
      artifact.artifact_type === locate.artifactType,
  );
  if (artifactIndex < 0) return;
  const artifactKey = `${locate.mode}:${locate.nodeId}:${locate.artifactType}:${artifactIndex}`;
  setFocusedArtifactKey(artifactKey);
  queueMicrotask(() => {
    builderRootRef.current
      ?.querySelector<HTMLElement>(`[data-workflow-artifact-key="${artifactKey}"]`)
      ?.scrollIntoView({ block: "nearest", behavior: "smooth" });
  });
}

export function locateWorkflowBridgeRuntimeIssue(params: {
  issue: WorkflowBridgeRuntimeValidationIssue;
  builderRootRef: RefObject<HTMLElement | null>;
  resetBuilderFocus: () => void;
  setFocusedNodeId: Dispatch<SetStateAction<string | null>>;
  setFocusedEdgeId: Dispatch<SetStateAction<string | null>>;
  setHighlightedNodeIds: Dispatch<SetStateAction<string[]>>;
  flashHighlightedEdges: (edgeIds: string[]) => void;
}) {
  const {
    issue,
    builderRootRef,
    resetBuilderFocus,
    setFocusedNodeId,
    setFocusedEdgeId,
    setHighlightedNodeIds,
    flashHighlightedEdges,
  } = params;
  resetBuilderFocus();
  setFocusedNodeId(issue.nodeId);
  const nodeIds = [
    issue.upstreamNodeId,
    issue.nodeId,
    ...(issue.downstreamNodeIds ?? []),
  ].filter(Boolean) as string[];
  setHighlightedNodeIds(nodeIds);
  const edgeIds = [
    issue.inputEdgeId,
    ...(issue.outputEdgeIds ?? []),
    issue.outputEdgeId,
  ].filter(
    (value, index, list): value is string =>
      Boolean(value) && list.indexOf(value) === index,
  );
  if (edgeIds[0]) setFocusedEdgeId(edgeIds[0]);
  if (edgeIds.length > 0) flashHighlightedEdges(edgeIds);
  queueMicrotask(() => {
    builderRootRef.current
      ?.querySelector<HTMLElement>(`[data-workflow-node-id="${issue.nodeId}"]`)
      ?.scrollIntoView({ block: "nearest", behavior: "smooth" });
  });
  window.setTimeout(
    () =>
      setHighlightedNodeIds((current) =>
        current.includes(issue.nodeId) ? [] : current,
      ),
    2200,
  );
}

export function replayWorkflowBuilderAuditEntry(params: {
  entry: WorkbenchAuditTimelineEntry;
  setFocusedNodeId: Dispatch<SetStateAction<string | null>>;
  setFocusedEdgeId: Dispatch<SetStateAction<string | null>>;
  setHighlightedNodeIds: Dispatch<SetStateAction<string[]>>;
  flashHighlightedEdges: (edgeIds: string[]) => void;
}) {
  const {
    entry,
    setFocusedNodeId,
    setFocusedEdgeId,
    setHighlightedNodeIds,
    flashHighlightedEdges,
  } = params;
  const plan = buildWorkflowAuditReplayPlan(entry);
  if (plan.nodeId) setFocusedNodeId(plan.nodeId);
  if (plan.edgeIds[0]) setFocusedEdgeId(plan.edgeIds[0]);
  if (plan.nodeIds.length > 0) setHighlightedNodeIds(plan.nodeIds);
  if (plan.edgeIds.length > 0) flashHighlightedEdges(plan.edgeIds);
  window.setTimeout(
    () =>
      setHighlightedNodeIds((current) =>
        current.join(",") === plan.nodeIds.join(",") ? [] : current,
      ),
    2200,
  );
}
