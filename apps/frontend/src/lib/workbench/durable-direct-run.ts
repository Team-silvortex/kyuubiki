import { captureWorkbenchApiAuthority } from "../api/auth-context.ts";
import { resolveWorkbenchApiUrl } from "../api/backend-target.ts";
import { defaultRuntimeApiClient, type RuntimeApiClient } from "../api/runtime-client.ts";
import { validateDurableDirectEnvelope, validateDurableDirectHandle, type DurableDirectEnvelope } from "../api/durable-direct-client.ts";
import { directMeshMethodForStudyKind, normalizeDirectMeshStudyInput, type DirectMeshStudyKind } from "../runtime-gateway/direct-mesh-study-contract.ts";
import { createDurableDirectRecovery, type DurableDirectRecoveryHandle } from "./durable-direct-recovery.ts";
import { workbenchDirectExecutionJournal, type DirectExecutionJournal, type DirectExecutionIntent } from "./direct-execution-journal.ts";
import { checkpointDigest } from "./checkpoint-journal.ts";

type Client = Pick<RuntimeApiClient, "fetchDirectMeshAgents" | "submitDurableDirectStudy" | "inspectDurableDirectExecution" | "cancelDurableDirectExecution">;
export function directExecutionGatewayIdentity() {
  if (typeof window === "undefined") throw new Error("direct:gateway_unavailable");
  const url = new URL(resolveWorkbenchApiUrl("/api/direct-mesh/tasks"), window.location.origin);
  if (url.username || url.password || url.search || url.hash) throw new Error("direct:unsafe_gateway_identity");
  return url.href;
}
function handleFor(intent: DirectExecutionIntent): DurableDirectRecoveryHandle {
  return { schema_version: "kyuubiki.direct-execution-handle/v1", execution_id: intent.execution_id,
    endpoint: intent.endpoint, gateway_id: intent.scope, study_kind: intent.study_kind as DirectMeshStudyKind,
    ...(intent.store_id === undefined ? {} : { store_id: intent.store_id }) };
}
export function createDurableDirectRunService({ client = defaultRuntimeApiClient, journal = workbenchDirectExecutionJournal,
  gatewayIdentity = directExecutionGatewayIdentity, captureAuthority = captureWorkbenchApiAuthority,
  newId = () => crypto.randomUUID().replaceAll("-", "") }: {
    client?: Client; journal?: DirectExecutionJournal; gatewayIdentity?: () => string;
    captureAuthority?: typeof captureWorkbenchApiAuthority; newId?: () => string;
  } = {}) {
  async function context() {
    const identity = gatewayIdentity(), authority = captureAuthority("/api/direct-mesh/tasks", "/api/direct-mesh/tasks/inspect", "/api/direct-mesh/tasks/cancel");
    const check = () => { if (identity !== gatewayIdentity() || !authority()) throw new Error("direct:authority_changed"); };
    const scope = await checkpointDigest(identity);
    check();
    return { scope, check };
  }
  async function inspect<TResult>(handle: DurableDirectRecoveryHandle): Promise<DurableDirectEnvelope<TResult>> {
    const pinned = { ...handle }, ctx = await context();
    if (!pinned.store_id) throw new Error("direct:owner_fence_required: legacy handle has no pinned Agent store; keep the reservation for explicit recovery.");
    if (pinned.gateway_id !== ctx.scope) throw new Error("Restore the original gateway before inspecting this execution.");
    const recovery = createDurableDirectRecovery({ client, gatewayId: () => ctx.scope, saveHandle: async () => { throw new Error("Read only"); } });
    const envelope = await recovery.inspect<TResult>(pinned);
    ctx.check();
    return envelope;
  }
  return {
    async submit<TResult>(studyKind: DirectMeshStudyKind, input: Record<string, unknown>, endpoint: string) {
      validateDurableDirectHandle({ endpoint, execution_id: "0".repeat(32) });
      // Reject malformed local input before reserving, and never snapshot after an await.
      const snapshot = structuredClone(normalizeDirectMeshStudyInput(studyKind, input)) as Record<string, unknown>;
      const ctx = await context();
      const discovery = await client.fetchDirectMeshAgents([endpoint]);
      ctx.check();
      const agents = discovery.agents as unknown as Array<{ host?: string; port?: number; descriptor?: { durable_execution?: {
        schema_version?: string; enabled?: boolean; available?: boolean; unavailable_reason?: string;
        admission_blocked_after_write_failure?: boolean; store_id?: string;
      } } }>;
      const agent = agents?.length === 1 ? agents[0] : undefined, policy = agent?.descriptor?.durable_execution;
      if (!agent || `${agent.host}:${agent.port}` !== endpoint || policy?.schema_version !== "kyuubiki.agent-durable-execution/v1"
        || policy.enabled !== true) {
        throw new Error("direct:durable_agent_required: enable the Agent execution store before running. No task was submitted.");
      }
      if (policy.available !== undefined && policy.available !== true) {
        if (policy.available === false && policy.unavailable_reason === "store_busy") {
          throw new Error("direct:store_busy: the Agent store is busy. Keep existing recovery handles and inspect the original Agent after it clears. No task was submitted.");
        }
        throw new Error("direct:store_unavailable: the Agent store is unavailable. Restore the original store before running; keep existing recovery handles. No task was submitted.");
      }
      if (policy.admission_blocked_after_write_failure === true) {
        throw new Error("direct:store_write_failed: a durable commit failed. Inspect the original execution and restore storage before running. No task was submitted.");
      }
      if (typeof policy.store_id !== "string" || !/^[0-9a-f]{32}$/u.test(policy.store_id)) {
        throw new Error("direct:owner_fence_required: update the Agent to expose a persistent store identity. No task was submitted.");
      }
      let ownedIntent: DirectExecutionIntent | undefined;
      const recovery = createDurableDirectRecovery({ client, gatewayId: () => { ctx.check(); return ctx.scope; }, newId,
        saveHandle: async handle => {
          const reserved = await journal.reserve({ scope: ctx.scope, endpoint: handle.endpoint, study_kind: handle.study_kind,
            execution_id: handle.execution_id, created_at: Date.now(), store_id: handle.store_id });
          if (!reserved.is_new) throw new Error(`direct:recovery_required: ${reserved.intent.execution_id} at ${endpoint}. Inspect the original execution before starting another run.`);
          ownedIntent = reserved.intent;
        }, releaseUnsubmittedHandle: async handle => {
          if (!ownedIntent || ownedIntent.execution_id !== handle.execution_id) throw new Error("direct:journal_identity_mismatch");
          await journal.remove(ownedIntent);
        } });
      return recovery.submit<TResult>(studyKind, snapshot, endpoint, policy.store_id);
    },
    inspect,
    async pending() {
      const ctx = await context();
      const entries = await journal.list(); ctx.check();
      return entries.filter(e => e.scope === ctx.scope).map(handleFor);
    },
    async acknowledge(handle: DurableDirectRecoveryHandle, receipt: DurableDirectEnvelope, isCurrent = () => true) {
      const checkCaller = () => { if (!isCurrent()) throw new Error("direct:authority_changed"); };
      checkCaller();
      const pinned = { ...handle }, ctx = await context();
      const check = () => { checkCaller(); ctx.check(); };
      check();
      if (!pinned.store_id) throw new Error("direct:owner_fence_required: cannot acknowledge an unfenced legacy execution.");
      if (pinned.gateway_id !== ctx.scope) throw new Error("direct:authority_changed");
      const { execution } = validateDurableDirectEnvelope(receipt, pinned);
      if (!["completed", "failed"].includes(execution.status) || execution.status === "not_found"
        || execution.method !== directMeshMethodForStudyKind(pinned.study_kind)) throw new Error("direct:terminal_receipt_required");
      const entries = await journal.list(); check();
      const intent = entries.find(e => e.scope === ctx.scope && e.execution_id === pinned.execution_id && e.endpoint === pinned.endpoint && e.study_kind === pinned.study_kind && e.store_id === pinned.store_id);
      if (intent) await journal.remove(intent, check);
      check();
    },
    async cancel(handle: DurableDirectRecoveryHandle) {
      const pinned = { ...handle }, ctx = await context();
      const { execution } = await inspect(pinned); ctx.check();
      if (execution.status !== "running") throw new Error(`direct:not_running: ${execution.status}. Inspect the original execution.`);
      const value = await client.cancelDurableDirectExecution({ endpoint: pinned.endpoint, execution_id: pinned.execution_id,
        store_id: pinned.store_id,
        request_digest: execution.request_digest, process_instance_id: execution.process_instance_id });
      ctx.check();
      return value.recovery;
    },
  };
}
export type DurableDirectRunService = ReturnType<typeof createDurableDirectRunService>;
export const workbenchDurableDirectRunService = createDurableDirectRunService();
