import {
  validateDurableDirectHandle,
  DurableDirectSubmissionRejected,
  type DurableDirectApiClient,
  type DurableDirectEnvelope,
  type DurableDirectRecoveryHandle,
} from "../api/durable-direct-client.ts";
import { directMeshMethodForStudyKind, type DirectMeshStudyKind } from "../runtime-gateway/direct-mesh-study-contract.ts";

export type { DurableDirectRecoveryHandle } from "../api/durable-direct-client.ts";

export class DurableDirectSubmissionUncertain extends Error {
  readonly handle: DurableDirectRecoveryHandle;
  constructor(handle: DurableDirectRecoveryHandle, cause: unknown) {
    super(`Submission unconfirmed: ${handle.execution_id} at ${handle.endpoint}. Inspect this execution; do not resubmit.`, { cause });
    this.name = "DurableDirectSubmissionUncertain";
    this.handle = handle;
  }
}

type Options = {
  client: DurableDirectApiClient;
  gatewayId: () => string;
  // The owner supplies its durable storage boundary. No implicit localStorage or RAM fallback.
  saveHandle: (handle: DurableDirectRecoveryHandle) => Promise<void>;
  releaseUnsubmittedHandle?: (handle: DurableDirectRecoveryHandle) => Promise<void>;
  newId?: () => string;
};

export function createDurableDirectRecovery({ client, gatewayId, saveHandle, releaseUnsubmittedHandle, newId = () => crypto.randomUUID().replaceAll("-", "") }: Options) {
  async function release(handle: DurableDirectRecoveryHandle) {
    try { await releaseUnsubmittedHandle?.(handle); }
    catch (cause) {
      throw new Error(`direct:journal_cleanup_pending: no execution started for ${handle.execution_id} at ${handle.endpoint}, but its local reservation could not be removed. No automatic retry.`, { cause });
    }
  }
  function validate(handle: DurableDirectRecoveryHandle) {
    validateDurableDirectHandle(handle);
    if (handle.schema_version !== "kyuubiki.direct-execution-handle/v1"
      || !handle.gateway_id || handle.gateway_id !== gatewayId()) {
      throw new Error("Restore the original gateway before inspecting this execution.");
    }
  }

  async function inspect<TResult>(handle: DurableDirectRecoveryHandle): Promise<DurableDirectEnvelope<TResult>> {
    const pinned = Object.freeze({ ...handle });
    validate(pinned);
    const response = await client.inspectDurableDirectExecution<TResult>(pinned);
    validate(pinned); // A connection change while awaiting must not publish into the new workspace.
    if (response.execution.status !== "not_found" && response.execution.method !== directMeshMethodForStudyKind(pinned.study_kind)) {
      throw new Error("Recovered execution belongs to a different study method.");
    }
    return response;
  }

  return {
    async submit<TResult>(studyKind: DirectMeshStudyKind, input: Record<string, unknown>, endpoint: string, storeId?: string) {
      const handle: DurableDirectRecoveryHandle = Object.freeze({
        schema_version: "kyuubiki.direct-execution-handle/v1",
        execution_id: newId(), endpoint, gateway_id: gatewayId(), study_kind: studyKind,
        ...(storeId === undefined ? {} : { store_id: storeId }),
      });
      validate(handle);
      // Snapshot before awaiting storage; edits to the model must not alter a pending submission.
      const snapshot = structuredClone(input);
      await saveHandle(handle);
      try { validate(handle); }
      catch (cause) { await release(handle); throw cause; }
      let envelope: DurableDirectEnvelope<TResult>;
      try {
        envelope = await client.submitDurableDirectStudy<TResult>(handle, studyKind, snapshot);
      } catch (cause) {
        if (cause instanceof DurableDirectSubmissionRejected && cause.handle.execution_id === handle.execution_id
          && cause.handle.endpoint === handle.endpoint && cause.handle.store_id === handle.store_id
          && cause.method === directMeshMethodForStudyKind(studyKind)) {
          // Only a validated live submit decision may release a reservation, never not_found.
          try { validate(handle); } catch { throw new DurableDirectSubmissionUncertain(handle, cause); }
          await release(handle);
          throw cause;
        }
        throw new DurableDirectSubmissionUncertain(handle, cause);
      }
      try { validate(handle); } catch (cause) { throw new DurableDirectSubmissionUncertain(handle, cause); }
      return { handle, envelope };
    },
    inspect,
  };
}
