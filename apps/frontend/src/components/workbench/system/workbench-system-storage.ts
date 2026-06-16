"use client";

type StorageBucketDefinition = {
  id: string;
  label: string;
  keyPrefixes: string[];
  mode: "safe" | "careful";
  cleanupLabel: string;
  detail: string;
};

export type WorkbenchStorageBucket = {
  id: string;
  label: string;
  bytes: number;
  entries: number;
  mode: "safe" | "careful";
};

export type WorkbenchStorageSnapshot = {
  totalBytes: number;
  localStorageKeys: number;
  quotaBytes: number | null;
  usageBytes: number | null;
  buckets: WorkbenchStorageBucket[];
};

export type WorkbenchStorageRule = {
  id: string;
  label: string;
  keyPrefixes: string[];
  mode: "safe" | "careful";
  cleanupLabel: string;
  detail: string;
};

const STORAGE_BUCKETS: StorageBucketDefinition[] = [
  {
    id: "workflow_snapshots",
    label: "Workflow snapshots",
    keyPrefixes: [
      "kyuubiki.workbench.workflowSnapshots.index.v1",
      "kyuubiki.workbench.workflowSnapshots.payload.v1:",
    ],
    mode: "safe",
    cleanupLabel: "Safe to clear",
    detail: "Execution snapshots and deferred payload chunks used for workflow trace replay.",
  },
  {
    id: "workflow_drafts",
    label: "Workflow drafts",
    keyPrefixes: ["kyuubiki.workbench.workflowDrafts.v1"],
    mode: "safe",
    cleanupLabel: "Safe to clear",
    detail: "Temporary workflow draft saves created during composition before promotion.",
  },
  {
    id: "runtime_temp",
    label: "Runtime temp",
    keyPrefixes: [
      "kyuubiki.workbench.workflowPackageMaintenanceLog.v1",
      "kyuubiki-workbench-python-panel",
    ],
    mode: "safe",
    cleanupLabel: "Safe to clear",
    detail: "Short-lived runtime cache, package maintenance receipts, and script editor buffer state.",
  },
  {
    id: "local_workflows",
    label: "Local workflow library",
    keyPrefixes: ["kyuubiki.workbench.workflowLibrary.v1"],
    mode: "careful",
    cleanupLabel: "Manual review first",
    detail: "User-kept local workflow assets promoted from catalog or package imports.",
  },
  {
    id: "script_presets",
    label: "Script presets",
    keyPrefixes: [
      "kyuubiki-workbench-macro-presets",
      "kyuubiki-workbench-snippet-presets",
    ],
    mode: "careful",
    cleanupLabel: "Manual review first",
    detail: "Saved macro presets and snippet presets tied to project scripting workflows.",
  },
  {
    id: "workflow_favorites",
    label: "Template favorites",
    keyPrefixes: [
      "kyuubiki.workflow.favoriteTemplateChains",
      "kyuubiki.workflow.favoriteTemplateChainAliases",
    ],
    mode: "careful",
    cleanupLabel: "Manual review first",
    detail: "Favorite template chains and user aliases used by workflow search and quick insertion.",
  },
  {
    id: "settings",
    label: "Workbench settings",
    keyPrefixes: [
      "kyuubiki-workbench-settings",
      "kyuubiki-workbench-secrets",
      "kyuubiki-workbench-language-packs",
    ],
    mode: "careful",
    cleanupLabel: "Manual review first",
    detail: "Theme, runtime mode, language packs, and legacy workbench storage keys. Runtime secrets now stay in memory for the active session.",
  },
];

function encodeBytes(value: string) {
  return new TextEncoder().encode(value).length;
}

function listLocalStorageEntries() {
  if (typeof window === "undefined") return [] as Array<{ key: string; value: string }>;
  const entries: Array<{ key: string; value: string }> = [];
  for (let index = 0; index < window.localStorage.length; index += 1) {
    const key = window.localStorage.key(index);
    if (!key) continue;
    const value = window.localStorage.getItem(key);
    if (typeof value !== "string") continue;
    entries.push({ key, value });
  }
  return entries;
}

function bucketMatchesKey(bucket: StorageBucketDefinition, key: string) {
  return bucket.keyPrefixes.some((prefix) => key === prefix || key.startsWith(prefix));
}

async function readStorageEstimate() {
  if (typeof navigator === "undefined" || !navigator.storage?.estimate) {
    return { usageBytes: null, quotaBytes: null };
  }
  try {
    const estimate = await navigator.storage.estimate();
    return {
      usageBytes: typeof estimate.usage === "number" ? estimate.usage : null,
      quotaBytes: typeof estimate.quota === "number" ? estimate.quota : null,
    };
  } catch {
    return { usageBytes: null, quotaBytes: null };
  }
}

export async function inspectWorkbenchStorage(): Promise<WorkbenchStorageSnapshot> {
  const entries = listLocalStorageEntries();
  const buckets = STORAGE_BUCKETS.map((bucket) => {
    const matchingEntries = entries.filter((entry) => bucketMatchesKey(bucket, entry.key));
    const bytes = matchingEntries.reduce(
      (sum, entry) => sum + encodeBytes(entry.key) + encodeBytes(entry.value),
      0,
    );
    return {
      id: bucket.id,
      label: bucket.label,
      bytes,
      entries: matchingEntries.length,
      mode: bucket.mode,
    } satisfies WorkbenchStorageBucket;
  }).sort((left, right) => right.bytes - left.bytes);

  const totalBytes = entries.reduce(
    (sum, entry) => sum + encodeBytes(entry.key) + encodeBytes(entry.value),
    0,
  );
  const estimate = await readStorageEstimate();

  return {
    totalBytes,
    localStorageKeys: entries.length,
    quotaBytes: estimate.quotaBytes,
    usageBytes: estimate.usageBytes,
    buckets,
  };
}

export function listWorkbenchStorageRules(): WorkbenchStorageRule[] {
  return STORAGE_BUCKETS.map((bucket) => ({
    id: bucket.id,
    label: bucket.label,
    keyPrefixes: bucket.keyPrefixes,
    mode: bucket.mode,
    cleanupLabel: bucket.cleanupLabel,
    detail: bucket.detail,
  }));
}

export function clearWorkbenchStorageBucket(bucketId: string) {
  if (typeof window === "undefined") return;
  const bucket = STORAGE_BUCKETS.find((entry) => entry.id === bucketId);
  if (!bucket) return;
  const keysToRemove = listLocalStorageEntries()
    .map((entry) => entry.key)
    .filter((key) => bucketMatchesKey(bucket, key));
  for (const key of keysToRemove) {
    window.localStorage.removeItem(key);
  }
}

export function clearWorkbenchSafeStorage() {
  for (const bucket of STORAGE_BUCKETS) {
    if (bucket.mode !== "safe") continue;
    clearWorkbenchStorageBucket(bucket.id);
  }
}
