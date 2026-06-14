import type { HeadlessOrchestraHandoffEnvelope } from "@/lib/scripting/workbench-headless-orchestra-handoff";

export type HeadlessHandoffStage =
  | "received"
  | "queued"
  | "dispatch_planned"
  | "ready_for_orchestra";

export type HeadlessHandoffStatusRecord = {
  accepted: boolean;
  handoff_id: string;
  workflow_id: string;
  received_at: string;
  authority_mode: string;
  step_count: number;
  chosen_agent_count: number;
  warning_count: number;
  target_clusters: string[];
  has_dispatch_override: boolean;
  dispatch_override_count: number;
  override_acknowledged: boolean;
  override_note: string | null;
  override_step_keys: string[];
  override_summary: string | null;
  stage: HeadlessHandoffStage;
  status_message: string;
};

export type HeadlessHandoffSnapshotRecord = HeadlessHandoffStatusRecord & {
  envelope: HeadlessOrchestraHandoffEnvelope;
};

type StoredHeadlessHandoff = {
  handoff: HeadlessOrchestraHandoffEnvelope;
  receipt: Omit<HeadlessHandoffStatusRecord, "stage" | "status_message">;
};

const registry = new Map<string, StoredHeadlessHandoff>();

function buildReceipt(
  handoffId: string,
  payload: HeadlessOrchestraHandoffEnvelope,
): Omit<HeadlessHandoffStatusRecord, "stage" | "status_message"> {
  const overrideCount =
    payload.dispatch_overrides?.length ??
    payload.dispatch_override_ref?.override_count ??
    0;
  const overrideStepKeys = (payload.dispatch_overrides ?? []).map((entry) => entry.step_key);
  return {
    accepted: true,
    handoff_id: handoffId,
    workflow_id: payload.workflow_id,
    received_at: new Date().toISOString(),
    authority_mode: payload.runtime_manifest.authority_mode,
    step_count: payload.execution_batch.steps.length,
    chosen_agent_count: payload.dispatch_plan.steps.filter(
      (step) => typeof step.chosen_agent_id === "string" && step.chosen_agent_id.trim().length > 0,
    ).length,
    warning_count: payload.dispatch_plan.warnings.length,
    target_clusters: payload.runtime_manifest.target_clusters,
    has_dispatch_override: overrideCount > 0,
    dispatch_override_count: overrideCount,
    override_acknowledged: overrideCount > 0,
    override_note:
      overrideCount > 0
        ? "Dispatch override was acknowledged for audit and UI visibility, but execution still follows the embedded dispatch plan in the current version."
        : null,
    override_step_keys: overrideStepKeys,
    override_summary:
      overrideCount > 0
        ? `${overrideCount} override step(s): ${overrideStepKeys.join(", ")}`
        : null,
  };
}

function resolveStage(receivedAt: string): Pick<HeadlessHandoffStatusRecord, "stage" | "status_message"> {
  const elapsedMs = Date.now() - new Date(receivedAt).getTime();
  if (elapsedMs < 2_000) {
    return { stage: "received", status_message: "handoff accepted and waiting for queue admission" };
  }
  if (elapsedMs < 5_000) {
    return { stage: "queued", status_message: "handoff is queued for orchestrator intake" };
  }
  if (elapsedMs < 8_000) {
    return { stage: "dispatch_planned", status_message: "dispatch plan has been materialized and is awaiting orchestrator pickup" };
  }
  return { stage: "ready_for_orchestra", status_message: "handoff envelope is ready for orchestrator pickup" };
}

export function registerHeadlessHandoff(payload: HeadlessOrchestraHandoffEnvelope) {
  const handoffId = `handoff_${Date.now().toString(36)}`;
  const receipt = buildReceipt(handoffId, payload);
  registry.set(handoffId, { handoff: payload, receipt });
  return { ...receipt, ...resolveStage(receipt.received_at) } satisfies HeadlessHandoffStatusRecord;
}

export function getHeadlessHandoffStatus(handoffId: string) {
  const record = registry.get(handoffId);
  if (!record) return null;
  return {
    ...record.receipt,
    ...resolveStage(record.receipt.received_at),
  } satisfies HeadlessHandoffStatusRecord;
}

export function listHeadlessHandoffs() {
  return Array.from(registry.values())
    .map((record) => ({
      ...record.receipt,
      ...resolveStage(record.receipt.received_at),
    }))
    .sort((left, right) => right.received_at.localeCompare(left.received_at));
}

export function getHeadlessHandoffSnapshot(handoffId: string) {
  const record = registry.get(handoffId);
  if (!record) return null;
  return {
    ...record.receipt,
    ...resolveStage(record.receipt.received_at),
    envelope: record.handoff,
  } satisfies HeadlessHandoffSnapshotRecord;
}
