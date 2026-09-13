import { checkpointDigest, validateCheckpointIntent, type CheckpointIntent, type CheckpointJournal } from "./checkpoint-journal.ts";
import type { CheckpointReceipt, ModelVersionRecord } from "../api/project-types.ts";
import type { WorkbenchOperationResult } from "./operation-result.ts";

export type CheckpointVersionGuard = (version: ModelVersionRecord) => void;
export type CheckpointVersionOpener = (id: string, validate: CheckpointVersionGuard) => Promise<WorkbenchOperationResult>;

export function createCheckpointRecovery(options: {
  journal: CheckpointJournal;
  scope: () => string;
  lookup: (operation: "model" | "version", parent: string, requestId: string) => Promise<{ checkpoint: CheckpointReceipt }>;
}) {
  const current = (scope: string) => {
    if (options.scope() !== scope) throw new Error("checkpoint:context_changed");
  };
  async function listInScope(scope: string) {
    const hash = await checkpointDigest(scope);
    const entries = (await options.journal.list()).map(validateCheckpointIntent);
    current(scope);
    return entries.filter((entry) => entry.scope === hash).sort((a, b) => a.createdAt - b.createdAt || a.key.localeCompare(b.key));
  }
  async function lookup(key: string, scope: string) {
    if (typeof key !== "string" || !/^[a-f0-9]{64}$/.test(key)) throw new Error("checkpoint:invalid_recovery_key");
    const intent = (await listInScope(scope)).find((entry) => entry.key === key);
    if (!intent) throw new Error("checkpoint:pending_not_found");
    current(scope);
    const response = await options.lookup(intent.operation, intent.parentId, intent.requestId);
    current(scope);
    return { intent, receipt: validateReceipt(response?.checkpoint, intent) };
  }
  return {
    list: () => listInScope(options.scope()),
    check: async (key: string) => (await lookup(key, options.scope())).receipt,
    async acknowledge(key: string) {
      const scope = options.scope();
      const { intent, receipt } = await lookup(key, scope);
      if (receipt.status === "unknown") throw new Error("checkpoint:commit_unconfirmed");
      current(scope);
      await options.journal.remove(key, intent.requestId);
      return receipt;
    },
    async open(key: string, openVersion: CheckpointVersionOpener) {
      const scope = options.scope();
      const { receipt } = await lookup(key, scope);
      if (receipt.status !== "committed") throw new Error(`checkpoint:${receipt.status === "deleted" ? "result_deleted" : "commit_unconfirmed"}`);
      current(scope);
      // Validate again after the version fetch, before the workspace is changed.
      const result = await openVersion(receipt.version_id, (version) => {
        current(scope);
        if (version?.version_id !== receipt.version_id || version.model_id !== receipt.model_id || version.project_id !== receipt.project_id) {
          throw new Error("checkpoint:invalid_response");
        }
      });
      if (!result.ok) throw result.error;
      current(scope);
      return receipt;
    },
  };
}

function validateReceipt(value: unknown, intent: CheckpointIntent): CheckpointReceipt {
  if (value && typeof value === "object") {
    const receipt = value as Record<string, unknown>;
    if (receipt.status === "unknown") return { status: "unknown" };
    const validId = (id: unknown): id is string => typeof id === "string" && id.length > 0 && id.length <= 128;
    if ((receipt.status === "committed" || receipt.status === "deleted")
        && validId(receipt.project_id) && validId(receipt.model_id) && validId(receipt.version_id)
        && (intent.operation === "model" ? receipt.project_id : receipt.model_id) === intent.parentId) {
      return { status: receipt.status, project_id: receipt.project_id, model_id: receipt.model_id, version_id: receipt.version_id };
    }
  }
  throw new Error("checkpoint:invalid_response");
}

export type CheckpointRecovery = ReturnType<typeof createCheckpointRecovery>;
