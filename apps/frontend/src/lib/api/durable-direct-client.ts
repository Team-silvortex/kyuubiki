import { directMeshMethodForStudyKind, normalizeDirectMeshStudyInput, type DirectMeshStudyKind } from "../runtime-gateway/direct-mesh-study-contract.ts";

export const DURABLE_DIRECT_SCHEMA = "kyuubiki.agent-durable-execution/v1";

export type DurableDirectHandle = { execution_id: string; endpoint: string };
export type DurableDirectRecoveryHandle = Readonly<DurableDirectHandle & {
  schema_version: "kyuubiki.direct-execution-handle/v1";
  gateway_id: string;
  study_kind: DirectMeshStudyKind;
}>;
export type DurableDirectExecution<TResult = unknown> = {
  schema_version: typeof DURABLE_DIRECT_SCHEMA;
  execution_id: string;
  automatic_replay_authorized: false;
} & (
  | { status: "not_found"; response?: null }
  | ({ request_id: string; request_digest: string; process_instance_id: string; method: string; resumable: false } & (
    | { status: "running" | "outcome_unknown"; response: null }
    | { status: "completed"; response: { rpc_version: 1; id: string; ok: true; result: TResult; error?: null } }
    | { status: "failed"; response: { rpc_version: 1; id: string; ok: false; result?: null; error: { code: string; message: string; details?: unknown } } }
  ))
);
export type DurableDirectEnvelope<TResult = unknown> = {
  execution: DurableDirectExecution<TResult>;
  endpoint: string;
  gateway_runtime: "rust-native";
};
type Request = <T>(url: string, init?: RequestInit, timeoutMs?: number) => Promise<T>;

export function validateDurableDirectHandle(handle: DurableDirectHandle) {
  if (!/^[0-9a-f]{32}$/u.test(handle.execution_id)) throw new Error("Invalid durable execution ID.");
  const match = /^([A-Za-z0-9][A-Za-z0-9.-]*):([0-9]+)$/u.exec(handle.endpoint);
  if (!match || handle.endpoint.length > 255 || match[1] === "0.0.0.0"
    || Number(match[2]) < 1 || Number(match[2]) > 65535 || String(Number(match[2])) !== match[2]) {
    throw new Error("Durable execution requires one canonical host:port endpoint.");
  }
}

function record(value: unknown): Record<string, unknown> {
  return value !== null && typeof value === "object" && !Array.isArray(value) ? value as Record<string, unknown> : {};
}

export function validateDurableDirectEnvelope<TResult>(value: unknown, handle: DurableDirectHandle): DurableDirectEnvelope<TResult> {
  const envelope = record(value);
  const report = record(envelope.execution);
  const response = record(report.response);
  const error = record(response.error);
  const text = (v: unknown) => typeof v === "string" && v.length > 0;
  const identity = text(report.request_id) && text(report.process_instance_id) && text(report.method)
    && typeof report.request_digest === "string" && /^[0-9a-f]{64}$/u.test(report.request_digest)
    && report.resumable === false;
  const terminalIdentity = response.rpc_version === 1 && response.id === report.request_id;
  let validState = false;
  switch (report.status) {
    case "not_found": validState = report.response == null; break;
    case "running":
    case "outcome_unknown": validState = identity && report.response === null; break;
    case "completed":
      validState = identity && terminalIdentity && response.ok === true && response.error == null && response.result != null;
      break;
    case "failed":
      validState = identity && terminalIdentity && response.ok === false && response.result == null && text(error.code) && text(error.message);
      break;
  }
  if (envelope.gateway_runtime !== "rust-native" || envelope.endpoint !== handle.endpoint
    || report.schema_version !== DURABLE_DIRECT_SCHEMA || report.execution_id !== handle.execution_id
    || report.automatic_replay_authorized !== false || !validState) {
    throw new Error("Invalid or mismatched durable execution receipt; inspect the original Agent, do not resubmit.");
  }
  return value as DurableDirectEnvelope<TResult>;
}

export function createDurableDirectApiClient(request: Request) {
  async function exchange<TResult>(handle: DurableDirectHandle, path: string, body: Record<string, unknown>) {
    const pinned = { ...handle };
    validateDurableDirectHandle(pinned);
    const value = await request<unknown>(path, {
      method: "POST", headers: { "Content-Type": "application/json" }, cache: "no-store",
      body: JSON.stringify({ ...body, execution_id: pinned.execution_id, endpoints: [pinned.endpoint] }),
    });
    return validateDurableDirectEnvelope<TResult>(value, pinned);
  }
  return {
    async submitDurableDirectStudy<TResult>(handle: DurableDirectHandle, studyKind: DirectMeshStudyKind, input: Record<string, unknown>) {
      const envelope = await exchange<TResult>(handle, "/api/direct-mesh/tasks", {
        study_kind: studyKind, input: normalizeDirectMeshStudyInput(studyKind, input),
      });
      if (envelope.execution.status === "not_found" || envelope.execution.method !== directMeshMethodForStudyKind(studyKind)) {
        throw new Error("Submission did not return the expected admitted execution.");
      }
      return envelope;
    },
    inspectDurableDirectExecution<TResult>(handle: DurableDirectHandle) {
      return exchange<TResult>(handle, "/api/direct-mesh/tasks/inspect", {});
    },
  };
}

export type DurableDirectApiClient = ReturnType<typeof createDurableDirectApiClient>;
