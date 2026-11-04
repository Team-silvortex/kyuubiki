"use client";

import { WORKBENCH_SCRIPT_ACTIONS, type WorkbenchScriptSnapshot } from "@/lib/scripting/workbench-script-runtime";
import { requestAssistantJson, AssistantConnectionError } from "./model-connection";
export { validateAssistantEndpoint, type AssistantEndpointValidation } from "./model-connection";

export type AssistantSuggestedAction = {
  action: string;
  payload?: Record<string, unknown>;
  reason: string;
};

export type AssistantPlan = {
  summary: string;
  rationale: string;
  suggested_actions: AssistantSuggestedAction[];
};

function extractJsonBlock(value: string): string {
  const fenced = value.match(/```json\s*([\s\S]*?)```/i);
  if (fenced?.[1]) {
    return fenced[1].trim();
  }

  const firstBrace = value.indexOf("{");
  const lastBrace = value.lastIndexOf("}");
  if (firstBrace >= 0 && lastBrace > firstBrace) {
    return value.slice(firstBrace, lastBrace + 1);
  }

  return value.trim();
}

export async function requestWorkbenchAssistantPlan(input: {
  baseUrl: string;
  apiKey?: string;
  model: string;
  prompt: string;
  snapshot: WorkbenchScriptSnapshot;
  localHints: Array<{ id: string; title: string; summary: string; actionLabel: string }>;
  trustedOrigins?: ReadonlySet<string>;
  signal?: AbortSignal;
  transport?: "browser" | "native";
}): Promise<AssistantPlan> {
  if (!input.model.trim() || input.model.length > 256) throw new AssistantConnectionError("credentials");
  const payload = await requestAssistantJson(input, "plan", {
      model: input.model,
      messages: [
        {
          role: "system",
          content:
            "You are the Kyuubiki frontend assistant. Return strict JSON with keys summary, rationale, suggested_actions. suggested_actions must be an array of objects with action, payload, reason. Only suggest actions from the provided action catalog. Keep it concise and operational.",
        },
        {
          role: "user",
          content: JSON.stringify(
            {
              prompt: input.prompt,
              snapshot: input.snapshot,
              local_hints: input.localHints,
              action_catalog: WORKBENCH_SCRIPT_ACTIONS.map((entry) => ({
                id: entry.id,
                category: entry.category,
                summary: entry.summary.en,
                payloadExample: entry.payloadExample ?? {},
              })),
            },
            null,
            2,
          ),
        },
      ],
  }) as {
    choices?: Array<{ message?: { content?: string | null } }>;
  };
  const content = payload?.choices?.[0]?.message?.content;
  if (typeof content !== "string" || !content) throw new AssistantConnectionError("invalid_response");
  try {
    const parsed = JSON.parse(extractJsonBlock(content)) as AssistantPlan;
    if (!parsed || typeof parsed.summary !== "string" || typeof parsed.rationale !== "string"
      || parsed.summary.length > 8000 || parsed.rationale.length > 8000
      || !Array.isArray(parsed.suggested_actions) || parsed.suggested_actions.length > 32) throw new Error();
    const actions = new Set<string>(WORKBENCH_SCRIPT_ACTIONS.map(entry => entry.id));
    for (const entry of parsed.suggested_actions) {
      if (!entry || !actions.has(entry.action) || typeof entry.reason !== "string" || entry.reason.length > 8000
        || (entry.payload !== undefined && (!entry.payload || typeof entry.payload !== "object" || Array.isArray(entry.payload)))) throw new Error();
    }
    return { summary:parsed.summary, rationale:parsed.rationale, suggested_actions:parsed.suggested_actions.map(entry => ({ action:entry.action, reason:entry.reason, payload:entry.payload ?? {} })) };
  } catch { throw new AssistantConnectionError("invalid_response"); }
}
