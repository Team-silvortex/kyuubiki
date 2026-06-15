"use client";

import type { WorkflowGraphDefinition } from "@/lib/api";
import { appendWorkflowActivityLogEntry } from "@/lib/workbench/workflow-activity-log";
import { asWorkflowGraphDefinition } from "@/components/workbench/workflow/workbench-workflow-builder-import";

export const WORKBENCH_WORKFLOW_SNAPSHOT_INDEX_KEY = "kyuubiki.workbench.workflowSnapshots.index.v1";
export const WORKBENCH_WORKFLOW_SNAPSHOT_PAYLOAD_PREFIX = "kyuubiki.workbench.workflowSnapshots.payload.v1:";
export const WORKBENCH_WORKFLOW_SNAPSHOT_LIMIT = 20;
const WORKBENCH_WORKFLOW_SNAPSHOT_COOLDOWN_MS = 4000;
const WORKBENCH_WORKFLOW_SNAPSHOT_FALLBACK_DELAY_MS = 120;
const WORKBENCH_WORKFLOW_SNAPSHOT_PAYLOAD_MAX_BYTES = 180000;

export type StoredWorkflowSnapshotSummary = {
  id: string;
  workflowId: string;
  workflowName: string;
  createdAt: string;
  reason: string;
  summary: string[];
  payloadState: "full" | "summary_only";
};

export type StoredWorkflowSnapshot = StoredWorkflowSnapshotSummary & {
  graph: WorkflowGraphDefinition;
  inputArtifactTexts?: Record<string, string>;
};

type PendingSnapshotPayload = {
  graph: WorkflowGraphDefinition;
  inputArtifactTexts?: Record<string, string>;
};

type WindowWithIdleCallback = Window & {
  requestIdleCallback?: (callback: IdleRequestCallback, options?: IdleRequestOptions) => number;
  cancelIdleCallback?: (handle: number) => void;
};

const pendingSnapshotPayloads = new Map<string, PendingSnapshotPayload>();
const pendingSnapshotWrites = new Map<string, { kind: "idle" | "timeout"; handle: number }>();
let snapshotIndexCache: StoredWorkflowSnapshotSummary[] | null = null;
const latestSnapshotFingerprintCache = new Map<string, { snapshotId: string; fingerprint: string }>();

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function asStringRecord(value: unknown): Record<string, string> | undefined {
  if (!isRecord(value)) return undefined;
  return Object.fromEntries(Object.entries(value).filter(([, entryValue]) => typeof entryValue === "string")) as Record<string, string>;
}

function asStringArray(value: unknown): string[] {
  return Array.isArray(value) ? value.filter((entry): entry is string => typeof entry === "string") : [];
}

function buildSnapshotPayload(payload: PendingSnapshotPayload) {
  return JSON.stringify(payload);
}

function readSnapshotIndex(): StoredWorkflowSnapshotSummary[] {
  if (typeof window === "undefined") return [];
  if (snapshotIndexCache) return snapshotIndexCache;
  try {
    const raw = window.localStorage.getItem(WORKBENCH_WORKFLOW_SNAPSHOT_INDEX_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw) as unknown;
    if (!Array.isArray(parsed)) return [];
    snapshotIndexCache = parsed.flatMap((entry) => {
      if (!isRecord(entry) || typeof entry.id !== "string" || typeof entry.workflowId !== "string" || typeof entry.workflowName !== "string" || typeof entry.createdAt !== "string" || typeof entry.reason !== "string") return [];
      return [{ id: entry.id, workflowId: entry.workflowId, workflowName: entry.workflowName, createdAt: entry.createdAt, reason: entry.reason, summary: asStringArray(entry.summary), payloadState: entry.payloadState === "summary_only" ? "summary_only" : "full" }];
    });
    return snapshotIndexCache;
  } catch {
    return [];
  }
}

function writeSnapshotIndex(records: StoredWorkflowSnapshotSummary[]) {
  if (typeof window === "undefined") return;
  snapshotIndexCache = records;
  latestSnapshotFingerprintCache.clear();
  window.localStorage.setItem(WORKBENCH_WORKFLOW_SNAPSHOT_INDEX_KEY, JSON.stringify(records));
}

function snapshotPayloadKey(snapshotId: string) {
  return `${WORKBENCH_WORKFLOW_SNAPSHOT_PAYLOAD_PREFIX}${snapshotId}`;
}

function stringifySortedRecord(record?: Record<string, string> | Record<string, unknown> | null) {
  if (!record) return "";
  return JSON.stringify(
    Object.fromEntries(
      Object.entries(record)
        .sort(([left], [right]) => left.localeCompare(right))
        .map(([key, value]) => [key, value ?? null]),
    ),
  );
}

function getWindowWithIdleCallback() {
  return window as WindowWithIdleCallback;
}

function cancelPendingSnapshotWrite(snapshotId: string) {
  const pendingWrite = pendingSnapshotWrites.get(snapshotId);
  if (!pendingWrite) return;
  const idleWindow = getWindowWithIdleCallback();
  if (pendingWrite.kind === "idle" && idleWindow.cancelIdleCallback) idleWindow.cancelIdleCallback(pendingWrite.handle);
  else window.clearTimeout(pendingWrite.handle);
  pendingSnapshotWrites.delete(snapshotId);
}

function flushSnapshotPayload(snapshotId: string) {
  const payload = pendingSnapshotPayloads.get(snapshotId);
  pendingSnapshotWrites.delete(snapshotId);
  if (!payload) return;
  window.localStorage.setItem(snapshotPayloadKey(snapshotId), buildSnapshotPayload(payload));
  pendingSnapshotPayloads.delete(snapshotId);
}

function scheduleSnapshotPayloadWrite(snapshotId: string, payload: PendingSnapshotPayload) {
  pendingSnapshotPayloads.set(snapshotId, payload);
  cancelPendingSnapshotWrite(snapshotId);
  const idleWindow = getWindowWithIdleCallback();
  if (idleWindow.requestIdleCallback) {
    const handle = idleWindow.requestIdleCallback(() => flushSnapshotPayload(snapshotId), { timeout: WORKBENCH_WORKFLOW_SNAPSHOT_FALLBACK_DELAY_MS });
    pendingSnapshotWrites.set(snapshotId, { kind: "idle", handle });
    return;
  }
  const handle = window.setTimeout(() => flushSnapshotPayload(snapshotId), WORKBENCH_WORKFLOW_SNAPSHOT_FALLBACK_DELAY_MS);
  pendingSnapshotWrites.set(snapshotId, { kind: "timeout", handle });
}

function buildSnapshotFingerprint(graph: WorkflowGraphDefinition, inputArtifactTexts?: Record<string, string>) {
  return JSON.stringify({
    schema: graph.schema_version,
    id: graph.id,
    version: graph.version ?? "",
    name: graph.name ?? "",
    entryNodes: [...(graph.entry_nodes ?? [])].sort(),
    outputNodes: [...(graph.output_nodes ?? [])].sort(),
    defaults: stringifySortedRecord(graph.defaults),
    entryInputs: [...(graph.entry_inputs ?? [])]
      .map((artifact) => `${artifact.node_id}:${artifact.artifact_type}:${artifact.description}`)
      .sort(),
    outputArtifacts: [...(graph.output_artifacts ?? [])]
      .map((artifact) => `${artifact.node_id}:${artifact.artifact_type}:${artifact.description}`)
      .sort(),
    nodes: graph.nodes
      .map((node) => ({
        id: node.id,
        kind: node.kind,
        operatorId: node.operator_id ?? "",
        config: stringifySortedRecord(node.config),
        inputs: [...(node.inputs ?? [])].map((port) => `${port.id}:${port.artifact_type}:${port.dataset_value ?? ""}:${port.description ?? ""}`).sort(),
        outputs: [...(node.outputs ?? [])].map((port) => `${port.id}:${port.artifact_type}:${port.dataset_value ?? ""}:${port.description ?? ""}`).sort(),
      }))
      .sort((left, right) => left.id.localeCompare(right.id)),
    edges: [...(graph.edges ?? [])]
      .map((edge) => `${edge.id}:${edge.from.node}.${edge.from.port}:${edge.to.node}.${edge.to.port}:${edge.artifact_type}:${edge.dataset_value ?? ""}`)
      .sort(),
    dataset: graph.dataset_contract
      ? {
          schema: graph.dataset_contract.schema_version,
          id: graph.dataset_contract.id,
          version: graph.dataset_contract.version,
          name: graph.dataset_contract.name ?? "",
          description: graph.dataset_contract.description ?? "",
          metadata: stringifySortedRecord(graph.dataset_contract.metadata),
          values: graph.dataset_contract.values
            .map((value) => ({
              id: value.id,
              class: value.data_class,
              element: value.element_type,
              semantic: value.semantic_type ?? "",
              unit: value.unit ?? "",
              encoding: value.encoding ?? "",
              schemaRef: value.schema_ref ? `${value.schema_ref.schema}@${value.schema_ref.version}` : "",
              axes: [...(value.shape.axes ?? [])].map((axis) => `${axis.id}:${axis.label ?? ""}:${axis.size ?? ""}:${axis.semantic ?? ""}`).sort(),
            }))
            .sort((left, right) => left.id.localeCompare(right.id)),
        }
      : null,
    inputArtifactTexts: stringifySortedRecord(inputArtifactTexts),
  });
}

function pruneSnapshots(index: StoredWorkflowSnapshotSummary[]) {
  if (typeof window === "undefined") return index;
  const next = index.slice(0, WORKBENCH_WORKFLOW_SNAPSHOT_LIMIT);
  for (const entry of index.slice(WORKBENCH_WORKFLOW_SNAPSHOT_LIMIT)) {
    cancelPendingSnapshotWrite(entry.id);
    pendingSnapshotPayloads.delete(entry.id);
    window.localStorage.removeItem(snapshotPayloadKey(entry.id));
  }
  return next;
}

function readLatestSnapshotFingerprint(
  latestEntry: StoredWorkflowSnapshotSummary | undefined,
): string | null {
  if (!latestEntry || latestEntry.payloadState !== "full") return null;
  const cached = latestSnapshotFingerprintCache.get(latestEntry.workflowId);
  if (cached?.snapshotId === latestEntry.id) return cached.fingerprint;
  const latestSnapshot = loadStoredWorkflowSnapshot(latestEntry.id);
  if (!latestSnapshot) return null;
  const fingerprint = buildSnapshotFingerprint(latestSnapshot.graph, latestSnapshot.inputArtifactTexts);
  latestSnapshotFingerprintCache.set(latestEntry.workflowId, {
    snapshotId: latestEntry.id,
    fingerprint,
  });
  return fingerprint;
}

export function listStoredWorkflowSnapshots(workflowId: string): StoredWorkflowSnapshotSummary[] {
  return readSnapshotIndex().filter((entry) => entry.workflowId === workflowId).sort((left, right) => right.createdAt.localeCompare(left.createdAt));
}

export function loadStoredWorkflowSnapshot(snapshotId: string): StoredWorkflowSnapshot | null {
  if (typeof window === "undefined") return null;
  const indexEntry = readSnapshotIndex().find((entry) => entry.id === snapshotId);
  if (!indexEntry) return null;
  if (indexEntry.payloadState === "summary_only") return null;
  const pendingPayload = pendingSnapshotPayloads.get(snapshotId);
  if (pendingPayload) return { ...indexEntry, ...pendingPayload };
  try {
    const raw = window.localStorage.getItem(snapshotPayloadKey(snapshotId));
    if (!raw) return null;
    const parsed = JSON.parse(raw) as unknown;
    if (!isRecord(parsed)) return null;
    const graph = asWorkflowGraphDefinition(parsed.graph);
    if (!graph) return null;
    return { ...indexEntry, graph, inputArtifactTexts: asStringRecord(parsed.inputArtifactTexts) };
  } catch {
    return null;
  }
}

export function saveStoredWorkflowSnapshot(params: {
  workflowId: string;
  workflowName: string;
  reason: string;
  graph: WorkflowGraphDefinition;
  inputArtifactTexts?: Record<string, string>;
  summary: string[];
}) {
  if (typeof window === "undefined") return null;
  const index = readSnapshotIndex();
  const nextFingerprint = buildSnapshotFingerprint(params.graph, params.inputArtifactTexts);
  const payload = { graph: params.graph, inputArtifactTexts: params.inputArtifactTexts };
  const payloadText = buildSnapshotPayload(payload);
  const latestEntry = index.find((entry) => entry.workflowId === params.workflowId);
  if (latestEntry) {
    const latestFingerprint = readLatestSnapshotFingerprint(latestEntry);
    const latestCreatedAt = Date.parse(latestEntry.createdAt);
    if (
      latestFingerprint === nextFingerprint &&
      Number.isFinite(latestCreatedAt) &&
      Date.now() - latestCreatedAt < WORKBENCH_WORKFLOW_SNAPSHOT_COOLDOWN_MS
    ) {
      return latestEntry;
    }
  }
  const id = `snapshot_${Date.now()}`;
  const indexEntry: StoredWorkflowSnapshotSummary = {
    id,
    workflowId: params.workflowId,
    workflowName: params.workflowName,
    createdAt: new Date().toISOString(),
    reason: params.reason,
    summary: params.summary,
    payloadState: payloadText.length > WORKBENCH_WORKFLOW_SNAPSHOT_PAYLOAD_MAX_BYTES ? "summary_only" : "full",
  };
  writeSnapshotIndex(pruneSnapshots([indexEntry, ...index]));
  latestSnapshotFingerprintCache.set(params.workflowId, {
    snapshotId: id,
    fingerprint: nextFingerprint,
  });
  appendWorkflowActivityLogEntry({
    workflowId: params.workflowId,
    kind: "snapshot_saved",
    message: "Saved workflow snapshot.",
    detail: params.reason,
    count: params.summary.length,
  });
  if (indexEntry.payloadState === "full") scheduleSnapshotPayloadWrite(id, payload);
  return indexEntry;
}

export function removeStoredWorkflowSnapshot(snapshotId: string) {
  if (typeof window === "undefined") return;
  cancelPendingSnapshotWrite(snapshotId);
  pendingSnapshotPayloads.delete(snapshotId);
  window.localStorage.removeItem(snapshotPayloadKey(snapshotId));
  writeSnapshotIndex(readSnapshotIndex().filter((entry) => entry.id !== snapshotId));
}

export function removeStoredWorkflowSnapshotsByWorkflowId(workflowId: string) {
  if (typeof window === "undefined") return;
  const snapshots = readSnapshotIndex().filter((entry) => entry.workflowId === workflowId);
  for (const snapshot of snapshots) {
    cancelPendingSnapshotWrite(snapshot.id);
    pendingSnapshotPayloads.delete(snapshot.id);
    window.localStorage.removeItem(snapshotPayloadKey(snapshot.id));
  }
  writeSnapshotIndex(readSnapshotIndex().filter((entry) => entry.workflowId !== workflowId));
}

export function removeStoredWorkflowSummaryOnlySnapshots(workflowId: string) {
  if (typeof window === "undefined") return;
  const snapshots = readSnapshotIndex().filter((entry) => entry.workflowId === workflowId && entry.payloadState === "summary_only");
  for (const snapshot of snapshots) {
    cancelPendingSnapshotWrite(snapshot.id);
    pendingSnapshotPayloads.delete(snapshot.id);
    window.localStorage.removeItem(snapshotPayloadKey(snapshot.id));
  }
  writeSnapshotIndex(readSnapshotIndex().filter((entry) => !(entry.workflowId === workflowId && entry.payloadState === "summary_only")));
}
