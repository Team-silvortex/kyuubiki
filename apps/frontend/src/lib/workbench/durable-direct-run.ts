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
    endpoint: intent.endpoint, gateway_id: intent.scope, study_kind: intent.study_kind as DirectMeshStudyKind };
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
        schema_version?: string; enabled?: boolean; admission_blocked_after_write_failure?: boolean;
      } } }>;
      const agent = agents?.length === 1 ? agents[0] : undefined, policy = agent?.descriptor?.durable_execution;
      if (!agent || `${agent.host}:${agent.port}` !== endpoint || policy?.schema_version !== "kyuubiki.agent-durable-execution/v1"
        || policy.enabled !== true || policy.admission_blocked_after_write_failure === true) {
        throw new Error("direct:durable_agent_required: enable the Agent execution store before running. No task was submitted.");
      }
      const recovery = createDurableDirectRecovery({ client, gatewayId: () => { ctx.check(); return ctx.scope; }, newId,
        saveHandle: async handle => {
          const reserved = await journal.reserve({ scope: ctx.scope, endpoint: handle.endpoint, study_kind: handle.study_kind,
            execution_id: handle.execution_id, created_at: Date.now() });
          ctx.check();
          if (!reserved.is_new) throw new Error(`direct:recovery_required: ${reserved.intent.execution_id} at ${endpoint}. Inspect the original execution before starting another run.`);
        } });
      return recovery.submit<TResult>(studyKind, snapshot, endpoint);
    },
    inspect,
    async pending() {
      const ctx = await context();
      const entries = await journal.list(); ctx.check();
      return entries.filter(e => e.scope === ctx.scope).map(handleFor);
    },
    async acknowledge(handle: DurableDirectRecoveryHandle, receipt: DurableDirectEnvelope) {
      const pinned = { ...handle }, ctx = await context();
      if (pinned.gateway_id !== ctx.scope) throw new Error("direct:authority_changed");
      const { execution } = validateDurableDirectEnvelope(receipt, pinned);
      if (!["completed", "failed"].includes(execution.status) || execution.status === "not_found"
        || execution.method !== directMeshMethodForStudyKind(pinned.study_kind)) throw new Error("direct:terminal_receipt_required");
      const entries = await journal.list(); ctx.check();
      const intent = entries.find(e => e.scope === ctx.scope && e.execution_id === pinned.execution_id && e.endpoint === pinned.endpoint && e.study_kind === pinned.study_kind);
      if (intent) await journal.remove(intent);
      ctx.check();
    },
    async cancel(handle: DurableDirectRecoveryHandle) {
      const pinned = { ...handle }, ctx = await context();
      const { execution } = await inspect(pinned); ctx.check();
      if (execution.status !== "running") throw new Error(`direct:not_running: ${execution.status}. Inspect the original execution.`);
      const value = await client.cancelDurableDirectExecution({ endpoint: pinned.endpoint, execution_id: pinned.execution_id,
        request_digest: execution.request_digest, process_instance_id: execution.process_instance_id });
      ctx.check();
      return value.recovery;
    },
  };
}
export type DurableDirectRunService = ReturnType<typeof createDurableDirectRunService>;
export const workbenchDurableDirectRunService = createDurableDirectRunService();
