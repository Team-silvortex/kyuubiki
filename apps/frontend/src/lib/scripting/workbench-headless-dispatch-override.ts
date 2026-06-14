import type { HeadlessHandoffSnapshot } from "@/lib/api";

export type HeadlessDispatchOverrideDocument = {
  schema_version: "kyuubiki.headless-dispatch-override/v1";
  generated_at: string;
  handoff_id: string;
  workflow_id: string;
  overrides: Array<{
    step_key: string;
    index: number;
    action: string;
    original_agent_id: string | null;
    selected_agent_id: string;
    reason: "local_preferred_candidate";
  }>;
};

export type HeadlessDispatchOverrideReference = {
  schema_version: "kyuubiki.headless-dispatch-override-ref/v1";
  handoff_id: string;
  workflow_id: string;
  override_count: number;
};

export function buildHeadlessDispatchOverrideDocument(input: {
  localOverrides: Record<string, string>;
  snapshot: HeadlessHandoffSnapshot;
}) {
  const overrides = input.snapshot.envelope.dispatch_plan.steps.flatMap((step) => {
    const stepKey = `${step.index}:${step.action}`;
    const selectedAgentId = input.localOverrides[stepKey];
    if (!selectedAgentId || selectedAgentId === step.chosen_agent_id) return [];
    return [
      {
        step_key: stepKey,
        index: step.index,
        action: step.action,
        original_agent_id: step.chosen_agent_id,
        selected_agent_id: selectedAgentId,
        reason: "local_preferred_candidate" as const,
      },
    ];
  });

  return {
    schema_version: "kyuubiki.headless-dispatch-override/v1",
    generated_at: new Date().toISOString(),
    handoff_id: input.snapshot.handoff_id,
    workflow_id: input.snapshot.workflow_id,
    overrides,
  } satisfies HeadlessDispatchOverrideDocument;
}

export function asHeadlessDispatchOverrideDocument(value: unknown): HeadlessDispatchOverrideDocument | null {
  if (!value || typeof value !== "object") return null;
  const candidate = value as Record<string, unknown>;
  if (candidate.schema_version !== "kyuubiki.headless-dispatch-override/v1") return null;
  if (typeof candidate.handoff_id !== "string" || typeof candidate.workflow_id !== "string") return null;
  if (!Array.isArray(candidate.overrides)) return null;
  return candidate as HeadlessDispatchOverrideDocument;
}

export function toLocalDispatchOverrides(document: HeadlessDispatchOverrideDocument) {
  return Object.fromEntries(document.overrides.map((entry) => [entry.step_key, entry.selected_agent_id]));
}

export function buildHeadlessDispatchOverrideReference(document: HeadlessDispatchOverrideDocument) {
  return {
    schema_version: "kyuubiki.headless-dispatch-override-ref/v1",
    handoff_id: document.handoff_id,
    workflow_id: document.workflow_id,
    override_count: document.overrides.length,
  } satisfies HeadlessDispatchOverrideReference;
}
