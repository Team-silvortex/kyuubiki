import type { PayloadObject } from "@/components/workbench/workbench-headless-workflow-contract";
import type { WorkbenchRecordedMacroDraft } from "@/lib/scripting/workbench-script-runtime";

export type FrontendMacroAssetRecord = {
  assetId: string;
  draft: WorkbenchRecordedMacroDraft;
  source: "timeline_selection" | "bridge_restore" | "snapshot_derived";
  updatedAt: string;
};

export function buildFrontendMacroBridgePayload(draft: WorkbenchRecordedMacroDraft): PayloadObject {
  return {
    macro_id: draft.id,
    replay_mode: "bridge",
    step_count: draft.steps.length,
    steps: draft.steps.map((step) => ({
      action: step.action,
      payload: step.payload ?? {},
    })),
  };
}

export function parseFrontendMacroBridgePayload(payload: PayloadObject | null): WorkbenchRecordedMacroDraft | null {
  if (!payload) return null;
  const macroId = typeof payload.macro_id === "string" && payload.macro_id.trim()
    ? payload.macro_id
    : "macro/frontend-bridge-restored";
  const steps = Array.isArray(payload.steps)
    ? payload.steps.flatMap((step) => {
        if (!step || typeof step !== "object") return [];
        const candidate = step as { action?: unknown; payload?: unknown };
        if (typeof candidate.action !== "string") return [];
        return [
          {
            action: candidate.action,
            ...(candidate.payload && typeof candidate.payload === "object" && !Array.isArray(candidate.payload)
              ? { payload: candidate.payload as Record<string, unknown> }
              : {}),
          },
        ];
      })
    : [];
  return steps.length > 0 ? { id: macroId, steps } : null;
}

export function moveItem<T>(items: T[], fromIndex: number, toIndex: number) {
  if (toIndex < 0 || toIndex >= items.length || fromIndex === toIndex) return items;
  const next = [...items];
  const [item] = next.splice(fromIndex, 1);
  next.splice(toIndex, 0, item);
  return next;
}
