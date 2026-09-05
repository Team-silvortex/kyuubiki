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
};

type WindowWithIdleCallback = Window & {
  requestIdleCallback?: (callback: IdleRequestCallback, options?: IdleRequestOptions) => number;
  cancelIdleCallback?: (handle: number) => void;
};

const pendingSnapshotPayloads = new Map<string, PendingSnapshotPayload>();
const pendingSnapshotWrites = new Map<string, { kind: "idle" | "timeout"; handle: number }>();
let snapshotIndexCache: StoredWorkflowSnapshotSummary[] | null = null;
const latestSnapshotFingerprintCache = new Map<string, { snapshotId: string; fingerprint: string }>();
let snapshotIdSequence = 0;

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function asStringArray(value: unknown): string[] {
  return Array.isArray(value) ? value.filter((entry): entry is string => typeof entry === "string") : [];
}

function buildSnapshotPayload(payload: PendingSnapshotPayload) {
  return JSON.stringify(payload);
}

function cloneWorkflowGraph(graph: WorkflowGraphDefinition): WorkflowGraphDefinition {
  return JSON.parse(JSON.stringify(graph)) as WorkflowGraphDefinition;
}

function cloneSnapshotSummary(summary: StoredWorkflowSnapshotSummary): StoredWorkflowSnapshotSummary {
  return { ...summary, summary: [...summary.summary] };
}

function buildSnapshotId() {
  snapshotIdSequence = (snapshotIdSequence + 1) % Number.MAX_SAFE_INTEGER;
  return `snapshot_${Date.now()}_${snapshotIdSequence.toString(36)}`;
}

function utf8ByteLength(value: string) {
  return new TextEncoder().encode(value).byteLength;
}

function asStoredWorkflowSnapshotSummary(
  value: unknown,
): StoredWorkflowSnapshotSummary | null {
  if (
    !isRecord(value) ||
    typeof value.id !== "string" ||
    value.id.trim().length === 0 ||
    typeof value.workflowId !== "string" ||
    value.workflowId.trim().length === 0 ||
    typeof value.workflowName !== "string" ||
    typeof value.createdAt !== "string" ||
    !Number.isFinite(Date.parse(value.createdAt)) ||
    typeof value.reason !== "string"
  ) {
    return null;
  }
  if (
    value.payloadState !== undefined &&
    value.payloadState !== "full" &&
    value.payloadState !== "summary_only"
  ) {
    return null;
  }
  return {
    id: value.id,
    workflowId: value.workflowId,
    workflowName: value.workflowName,
    createdAt: value.createdAt,
    reason: value.reason,
    summary: asStringArray(value.summary),
    payloadState: value.payloadState === "summary_only" ? "summary_only" : "full",
  };
}

function reconcileStoredSnapshotPayloads(
  index: StoredWorkflowSnapshotSummary[],
  indexWasSanitized: boolean,
) {
  if (typeof window === "undefined") return;
  let indexChanged = indexWasSanitized;
  index.forEach((entry) => {
    if (entry.payloadState !== "full") return;
    const pendingPayload = pendingSnapshotPayloads.get(entry.id);
    if (pendingPayload) {
      pendingSnapshotPayloads.set(entry.id, { graph: pendingPayload.graph });
      return;
    }
    let raw: string | null;
    try {
      raw = window.localStorage.getItem(snapshotPayloadKey(entry.id));
    } catch {
      return;
    }
    if (!raw) {
      entry.payloadState = "summary_only";
      indexChanged = true;
      return;
    }
    let parsed: unknown;
    try {
      parsed = JSON.parse(raw) as unknown;
    } catch {
      entry.payloadState = "summary_only";
      indexChanged = true;
      return;
    }
    const graph = isRecord(parsed) ? asWorkflowGraphDefinition(parsed.graph) : null;
    if (!graph) {
      entry.payloadState = "summary_only";
      indexChanged = true;
      try {
        window.localStorage.removeItem(snapshotPayloadKey(entry.id));
      } catch {
        // Keep the invalid payload isolated behind the summary-only index state.
      }
      return;
    }
    if (isRecord(parsed) && "inputArtifactTexts" in parsed) {
      try {
        window.localStorage.setItem(snapshotPayloadKey(entry.id), buildSnapshotPayload({ graph }));
      } catch {
        pendingSnapshotPayloads.set(entry.id, { graph });
      }
    }
  });
  if (!indexChanged) return;
  try {
    window.localStorage.setItem(
      WORKBENCH_WORKFLOW_SNAPSHOT_INDEX_KEY,
      JSON.stringify(index.map(cloneSnapshotSummary)),
    );
  } catch {
    // The reconciled in-memory index remains authoritative for this session.
  }
}

type SnapshotIndexReadResult = {
  entries: StoredWorkflowSnapshotSummary[];
  readable: boolean;
};

function readSnapshotIndexState(): SnapshotIndexReadResult {
  if (typeof window === "undefined") return { entries: [], readable: false };
  if (snapshotIndexCache) return { entries: snapshotIndexCache, readable: true };
  try {
    const raw = window.localStorage.getItem(WORKBENCH_WORKFLOW_SNAPSHOT_INDEX_KEY);
    if (raw === null) return { entries: [], readable: true };
    const parsed = JSON.parse(raw) as unknown;
    if (!Array.isArray(parsed)) return { entries: [], readable: false };
    const seenIds = new Set<string>();
    snapshotIndexCache = parsed.flatMap((entry) => {
      const summary = asStoredWorkflowSnapshotSummary(entry);
      if (!summary || seenIds.has(summary.id)) return [];
      seenIds.add(summary.id);
      return [summary];
    }).slice(0, WORKBENCH_WORKFLOW_SNAPSHOT_LIMIT);
    reconcileStoredSnapshotPayloads(
      snapshotIndexCache,
      snapshotIndexCache.length !== parsed.length,
    );
    return { entries: snapshotIndexCache, readable: true };
  } catch {
    return { entries: [], readable: false };
  }
}

function readSnapshotIndex(): StoredWorkflowSnapshotSummary[] {
  return readSnapshotIndexState().entries;
}

function writeSnapshotIndex(records: StoredWorkflowSnapshotSummary[]) {
  if (typeof window === "undefined") return;
  const nextRecords = records.map(cloneSnapshotSummary);
  window.localStorage.setItem(WORKBENCH_WORKFLOW_SNAPSHOT_INDEX_KEY, JSON.stringify(nextRecords));
  snapshotIndexCache = nextRecords;
  latestSnapshotFingerprintCache.clear();
}

function snapshotPayloadKey(snapshotId: string) {
  return `${WORKBENCH_WORKFLOW_SNAPSHOT_PAYLOAD_PREFIX}${snapshotId}`;
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

function downgradeSnapshotPayload(
  snapshotId: string,
  payload: PendingSnapshotPayload,
) {
  const index = readSnapshotIndex();
  const snapshot = index.find((entry) => entry.id === snapshotId);
  if (!snapshot) return;
  const nextIndex = index.map((entry) =>
    entry.id === snapshotId ? { ...entry, payloadState: "summary_only" as const } : entry,
  );
  try {
    writeSnapshotIndex(nextIndex);
  } catch {
    snapshotIndexCache = nextIndex.map(cloneSnapshotSummary);
    latestSnapshotFingerprintCache.clear();
  }
  latestSnapshotFingerprintCache.set(snapshot.workflowId, {
    snapshotId,
    fingerprint: buildSnapshotFingerprint(payload.graph),
  });
}

function flushSnapshotPayload(snapshotId: string) {
  const payload = pendingSnapshotPayloads.get(snapshotId);
  pendingSnapshotWrites.delete(snapshotId);
  if (!payload) return;
  try {
    window.localStorage.setItem(snapshotPayloadKey(snapshotId), buildSnapshotPayload(payload));
  } catch {
    downgradeSnapshotPayload(snapshotId, payload);
  }
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

function canonicalizeSnapshotValue(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(canonicalizeSnapshotValue);
  if (!isRecord(value)) return value;
  return Object.fromEntries(
    Object.keys(value)
      .sort((left, right) => left.localeCompare(right))
      .map((key) => [key, canonicalizeSnapshotValue(value[key])]),
  );
}

function buildSnapshotFingerprint(graph: WorkflowGraphDefinition) {
  return JSON.stringify(canonicalizeSnapshotValue(graph));
}

function pruneSnapshots(index: StoredWorkflowSnapshotSummary[]) {
  return index.slice(0, WORKBENCH_WORKFLOW_SNAPSHOT_LIMIT);
}

function discardPrunedSnapshotPayloads(entries: StoredWorkflowSnapshotSummary[]) {
  for (const entry of entries) {
    cancelPendingSnapshotWrite(entry.id);
    pendingSnapshotPayloads.delete(entry.id);
    try {
      window.localStorage.removeItem(snapshotPayloadKey(entry.id));
    } catch {
      // Index retention already succeeded; stale payload cleanup is best effort.
    }
  }
}

function readLatestSnapshotFingerprint(
  latestEntry: StoredWorkflowSnapshotSummary | undefined,
): string | null {
  if (!latestEntry) return null;
  const cached = latestSnapshotFingerprintCache.get(latestEntry.workflowId);
  if (cached?.snapshotId === latestEntry.id) return cached.fingerprint;
  if (latestEntry.payloadState !== "full") return null;
  const latestSnapshot = loadStoredWorkflowSnapshot(latestEntry.id);
  if (!latestSnapshot) return null;
  const fingerprint = buildSnapshotFingerprint(latestSnapshot.graph);
  latestSnapshotFingerprintCache.set(latestEntry.workflowId, {
    snapshotId: latestEntry.id,
    fingerprint,
  });
  return fingerprint;
}

export function listStoredWorkflowSnapshots(workflowId: string): StoredWorkflowSnapshotSummary[] {
  return readSnapshotIndex()
    .filter((entry) => entry.workflowId === workflowId)
    .sort((left, right) => right.createdAt.localeCompare(left.createdAt))
    .map(cloneSnapshotSummary);
}

export function loadStoredWorkflowSnapshot(snapshotId: string): StoredWorkflowSnapshot | null {
  if (typeof window === "undefined") return null;
  const indexEntry = readSnapshotIndex().find((entry) => entry.id === snapshotId);
  if (!indexEntry) return null;
  if (indexEntry.payloadState === "summary_only") return null;
  const pendingPayload = pendingSnapshotPayloads.get(snapshotId);
  if (pendingPayload) return { ...cloneSnapshotSummary(indexEntry), graph: cloneWorkflowGraph(pendingPayload.graph) };
  try {
    const raw = window.localStorage.getItem(snapshotPayloadKey(snapshotId));
    if (!raw) return null;
    const parsed = JSON.parse(raw) as unknown;
    if (!isRecord(parsed)) return null;
    const graph = asWorkflowGraphDefinition(parsed.graph);
    if (!graph) return null;
    return { ...cloneSnapshotSummary(indexEntry), graph: cloneWorkflowGraph(graph) };
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
  const storedIndex = readSnapshotIndexState();
  if (!storedIndex.readable) return null;
  const index = storedIndex.entries;
  const capturedGraph = cloneWorkflowGraph(params.graph);
  const nextFingerprint = buildSnapshotFingerprint(capturedGraph);
  const payload = { graph: capturedGraph };
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
  const id = buildSnapshotId();
  const indexEntry: StoredWorkflowSnapshotSummary = {
    id,
    workflowId: params.workflowId,
    workflowName: params.workflowName,
    createdAt: new Date().toISOString(),
    reason: params.reason,
    summary: params.summary,
    payloadState: utf8ByteLength(payloadText) > WORKBENCH_WORKFLOW_SNAPSHOT_PAYLOAD_MAX_BYTES ? "summary_only" : "full",
  };
  const candidateIndex = [indexEntry, ...index];
  const nextIndex = pruneSnapshots(candidateIndex);
  try {
    writeSnapshotIndex(nextIndex);
  } catch {
    return null;
  }
  discardPrunedSnapshotPayloads(
    candidateIndex.slice(WORKBENCH_WORKFLOW_SNAPSHOT_LIMIT),
  );
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
  return cloneSnapshotSummary(indexEntry);
}

function discardRemovedSnapshotPayloads(snapshots: StoredWorkflowSnapshotSummary[]) {
  for (const snapshot of snapshots) {
    cancelPendingSnapshotWrite(snapshot.id);
    pendingSnapshotPayloads.delete(snapshot.id);
    try {
      window.localStorage.removeItem(snapshotPayloadKey(snapshot.id));
    } catch {
      // The index is authoritative; safe storage cleanup can remove an orphaned payload later.
    }
  }
}

function removeStoredWorkflowSnapshots(
  predicate: (entry: StoredWorkflowSnapshotSummary) => boolean,
): boolean {
  if (typeof window === "undefined") return false;
  const storedIndex = readSnapshotIndexState();
  if (!storedIndex.readable) return false;
  const removed = storedIndex.entries.filter(predicate);
  if (removed.length === 0) return true;
  try {
    writeSnapshotIndex(storedIndex.entries.filter((entry) => !predicate(entry)));
  } catch {
    return false;
  }
  discardRemovedSnapshotPayloads(removed);
  return true;
}

export function removeStoredWorkflowSnapshot(snapshotId: string): boolean {
  return removeStoredWorkflowSnapshots((entry) => entry.id === snapshotId);
}

export function removeStoredWorkflowSnapshotsByWorkflowId(workflowId: string): boolean {
  return removeStoredWorkflowSnapshots((entry) => entry.workflowId === workflowId);
}

export function removeStoredWorkflowSummaryOnlySnapshots(workflowId: string): boolean {
  return removeStoredWorkflowSnapshots(
    (entry) => entry.workflowId === workflowId && entry.payloadState === "summary_only",
  );
}
