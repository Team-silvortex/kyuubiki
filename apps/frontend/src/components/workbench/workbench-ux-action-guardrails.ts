import type { WorkbenchScriptActionDefinition } from "@/lib/scripting/workbench-script-runtime";
import type { WorkbenchUxGuardrailSummary } from "@/components/workbench/workbench-ux-guardrails";

export type WorkbenchUxActionGuardrailDecision = {
  allowed: boolean;
  reason: string | null;
  blockingItemId: string | null;
};

const RUNTIME_REQUIRED_CATEGORIES = new Set(["job", "project", "model", "data"]);
const LOCAL_SAFE_CATEGORIES = new Set(["navigation", "selection", "history", "viewport", "settings"]);

export function evaluateWorkbenchUxActionGuardrail(params: {
  action: string;
  definition: WorkbenchScriptActionDefinition | null;
  summary?: WorkbenchUxGuardrailSummary | null;
}): WorkbenchUxActionGuardrailDecision {
  const { action, definition, summary } = params;
  if (!summary || summary.blockedActionCount === 0) return allow();

  const category = definition?.category ?? action.split("/")[0] ?? "";
  if (LOCAL_SAFE_CATEGORIES.has(category)) return allow();

  const blockingItem = summary.items.find((item) => item.tone === "block") ?? null;
  if (!blockingItem) return allow();

  if (RUNTIME_REQUIRED_CATEGORIES.has(category) || definition?.risk === "destructive" || definition?.risk === "sensitive") {
    return {
      allowed: false,
      blockingItemId: blockingItem.id,
      reason: `${blockingItem.title}: ${blockingItem.nextAction}`,
    };
  }

  return allow();
}

function allow(): WorkbenchUxActionGuardrailDecision {
  return { allowed: true, reason: null, blockingItemId: null };
}
