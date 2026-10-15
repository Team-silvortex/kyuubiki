import { defaultRuntimeApiClient, type RuntimeApiClient } from "../api/runtime-client.ts";
import { captureWorkbenchApiAuthority } from "../api/auth-context.ts";
import { workbenchDurableDirectRunService, type DurableDirectRunService } from "./durable-direct-run.ts";

type Client = Pick<RuntimeApiClient, "listDurableDirectExecutions" | "inspectDurableDirectExecution" | "cancelDurableDirectExecution">;
export type DurableDirectInspection = {
  store_id?: string;
  execution_id: string;
  status: "running" | "completed" | "failed" | "outcome_unknown" | "not_found";
  automatic_replay_authorized: false;
  request_id?: string;
  process_instance_id?: string;
  request_digest?: string;
  method?: string;
  has_result: boolean;
  error_code?: string;
  error_message?: string;
};

export function createDurableDirectActions(client: Client, captureAuthority = captureWorkbenchApiAuthority, runService: DurableDirectRunService = workbenchDurableDirectRunService) {
  return async (action: string, payload: Record<string, unknown>): Promise<Record<string, unknown> | null> => {
    if (!["directExecution/pending", "directExecution/list", "directExecution/inspect", "directExecution/cancel"].includes(action)) return null;
    const allowed = action.endsWith("/list") ? ["endpoint", "after", "storeId"]
      : action.endsWith("/cancel") ? ["endpoint", "executionId", "processInstanceId", "requestDigest", "storeId"] : action.endsWith("/pending") ? ["endpoint"] : ["endpoint", "executionId", "storeId"];
    if (Object.keys(payload).some(key => !allowed.includes(key))) throw new Error("Unexpected direct execution recovery field.");
    const text = (key: string) => {
      const value = payload[key];
      if (typeof value !== "string" || !value) throw new Error(`Missing direct execution ${key}.`);
      return value;
    };
    const endpoint = text("endpoint");
    let storeId = payload.storeId === undefined ? undefined : text("storeId");
    if (storeId !== undefined && !/^[a-f0-9]{32}$/u.test(storeId)) throw new Error("Invalid Agent store identity.");
    const current = captureAuthority("/api/direct-mesh/tasks/list", "/api/direct-mesh/tasks/inspect", "/api/direct-mesh/tasks/cancel");
    if (action.endsWith("/inspect") || action.endsWith("/cancel")) {
      const pending = (await runService.pending()).find(handle => handle.endpoint === endpoint && handle.execution_id === text("executionId"));
      if (pending) {
        if (!pending.store_id || (storeId !== undefined && storeId !== pending.store_id)) {
          throw new Error("direct:owner_fence_required: supplied owner does not match the saved execution; no automatic adoption.");
        }
        storeId = pending.store_id;
      }
      if (!storeId) throw new Error("direct:owner_fence_required: use the saved handle or the original index storeId; no automatic adoption.");
      if (!current()) throw new Error("Direct execution authority changed.");
    }
    let result: Record<string, unknown>;
    if (action === "directExecution/pending") {
      result = { handles: (await runService.pending()).filter(handle => handle.endpoint === endpoint) };
    } else if (action === "directExecution/list") {
      if (payload.after !== undefined && payload.after !== null && typeof payload.after !== "string") throw new Error("Invalid direct execution cursor.");
      if (payload.after != null && !storeId) throw new Error("direct:owner_fence_required: pagination requires the original index storeId.");
      const value = await client.listDurableDirectExecutions(endpoint, 24, payload.after as string | null ?? null, storeId);
      result = { index: value.recovery };
    } else if (action === "directExecution/cancel") {
      const value = await client.cancelDurableDirectExecution({ endpoint, execution_id: text("executionId"),
        store_id: storeId,
        process_instance_id: text("processInstanceId"), request_digest: text("requestDigest") });
      result = { cancellation: value.recovery };
    } else {
      const envelope = await client.inspectDurableDirectExecution({ endpoint, execution_id: text("executionId"), store_id: storeId });
      const { execution } = envelope;
      // PWDT action logs and GUI recovery do not retain large solver results or error details.
      const inspection: DurableDirectInspection = {
        execution_id: execution.execution_id, status: execution.status,
        store_id: execution.store_id,
        automatic_replay_authorized: false, has_result: execution.status === "completed",
        ...(execution.status === "not_found" ? {} : {
          request_id: execution.request_id, process_instance_id: execution.process_instance_id,
          request_digest: execution.request_digest, method: execution.method,
        }),
        ...(execution.status === "failed" ? {
          error_code: execution.response.error.code.slice(0, 128), error_message: execution.response.error.message.slice(0, 512),
        } : {}),
      };
      result = { execution: inspection };
      if (execution.status === "completed" || execution.status === "failed") {
        if (!current()) throw new Error("Direct execution authority changed.");
        try {
          const handle = (await runService.pending()).find(item => item.endpoint === endpoint && item.execution_id === execution.execution_id);
          if (!current()) throw new Error("Direct execution authority changed.");
          if (handle) await runService.acknowledge(handle, envelope);
        } catch { result.journal_cleanup_pending = true; }
      }
    }
    if (!current()) throw new Error("Direct execution authority changed; restore the original gateway before inspecting again.");
    return { ok: true, action, endpoint, ...result };
  };
}

export const workbenchDurableDirectActions = createDurableDirectActions(defaultRuntimeApiClient);
