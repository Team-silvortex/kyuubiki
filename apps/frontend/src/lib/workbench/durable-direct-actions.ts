import { defaultRuntimeApiClient, type RuntimeApiClient } from "../api/runtime-client.ts";
import { captureWorkbenchApiAuthority } from "../api/auth-context.ts";
import { workbenchDurableDirectRunService, type DurableDirectRunService } from "./durable-direct-run.ts";

type Client = Pick<RuntimeApiClient, "listDurableDirectExecutions" | "inspectDurableDirectExecution" | "cancelDurableDirectExecution">;
export type DurableDirectInspection = {
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
    const allowed = action.endsWith("/list") ? ["endpoint", "after"]
      : action.endsWith("/cancel") ? ["endpoint", "executionId", "processInstanceId", "requestDigest"] : action.endsWith("/pending") ? ["endpoint"] : ["endpoint", "executionId"];
    if (Object.keys(payload).some(key => !allowed.includes(key))) throw new Error("Unexpected direct execution recovery field.");
    const text = (key: string) => {
      const value = payload[key];
      if (typeof value !== "string" || !value) throw new Error(`Missing direct execution ${key}.`);
      return value;
    };
    const endpoint = text("endpoint");
    const current = captureAuthority("/api/direct-mesh/tasks/list", "/api/direct-mesh/tasks/inspect", "/api/direct-mesh/tasks/cancel");
    let result: Record<string, unknown>;
    if (action === "directExecution/pending") {
      result = { handles: (await runService.pending()).filter(handle => handle.endpoint === endpoint) };
    } else if (action === "directExecution/list") {
      if (payload.after !== undefined && payload.after !== null && typeof payload.after !== "string") throw new Error("Invalid direct execution cursor.");
      const value = await client.listDurableDirectExecutions(endpoint, 24, payload.after as string | null ?? null);
      result = { index: value.recovery };
    } else if (action === "directExecution/cancel") {
      const value = await client.cancelDurableDirectExecution({ endpoint, execution_id: text("executionId"),
        process_instance_id: text("processInstanceId"), request_digest: text("requestDigest") });
      result = { cancellation: value.recovery };
    } else {
      const envelope = await client.inspectDurableDirectExecution({ endpoint, execution_id: text("executionId") });
      const { execution } = envelope;
      // PWDT action logs and GUI recovery do not retain large solver results or error details.
      const inspection: DurableDirectInspection = {
        execution_id: execution.execution_id, status: execution.status,
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
