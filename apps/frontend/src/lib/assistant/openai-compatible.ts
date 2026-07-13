"use client";

import { WORKBENCH_SCRIPT_ACTIONS, type WorkbenchScriptSnapshot } from "@/lib/scripting/workbench-script-runtime";

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

export type AssistantEndpointValidation = {
  ok: boolean;
  normalized?: string;
  reason?: string;
  requiresTrust?: boolean;
  origin?: string;
};

function isLoopbackAssistantHost(hostname: string) {
  const normalized = hostname.toLowerCase();
  return normalized === "localhost" || normalized === "127.0.0.1" || normalized === "::1" || normalized === "[::1]";
}

export function validateAssistantEndpoint(baseUrl: string): AssistantEndpointValidation {
  const trimmed = baseUrl.trim();
  if (!trimmed) {
    return { ok: false, reason: "assistant base URL is required" };
  }

  let parsed: URL;
  try {
    parsed = new URL(trimmed);
  } catch {
    return { ok: false, reason: "assistant base URL must be an absolute URL" };
  }

  const protocol = parsed.protocol.toLowerCase();
  const isLoopback = isLoopbackAssistantHost(parsed.hostname);
  if (protocol === "http:" && !isLoopback) {
    return { ok: false, reason: "remote assistant HTTP endpoints are not allowed; use HTTPS" };
  }
  if (protocol !== "https:" && !(protocol === "http:" && isLoopback)) {
    return { ok: false, reason: "assistant base URL must use HTTPS, or HTTP only for loopback" };
  }

  return {
    ok: true,
    normalized: parsed.origin + parsed.pathname.replace(/\/+$/, ""),
    requiresTrust: !isLoopback,
    origin: parsed.origin.toLowerCase(),
  };
}

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
}): Promise<AssistantPlan> {
  const validation = validateAssistantEndpoint(input.baseUrl);
  if (!validation.ok || !validation.normalized) {
    throw new Error(validation.reason ?? "assistant base URL is not allowed");
  }
  if (validation.requiresTrust && validation.origin && !input.trustedOrigins?.has(validation.origin)) {
    throw new Error(`assistant host must be explicitly trusted before use: ${validation.origin}`);
  }

  const endpoint = `${validation.normalized}/chat/completions`;
  const response = await fetch(endpoint, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      ...(input.apiKey?.trim() ? { Authorization: `Bearer ${input.apiKey.trim()}` } : {}),
    },
    body: JSON.stringify({
      model: input.model,
      temperature: 0.2,
      response_format: { type: "json_object" },
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
    }),
  });

  if (!response.ok) {
    const body = await response.text();
    throw new Error(`assistant request failed (${response.status}): ${body.slice(0, 240)}`);
  }

  const payload = (await response.json()) as {
    choices?: Array<{ message?: { content?: string | null } }>;
  };
  const content = payload.choices?.[0]?.message?.content;
  if (!content) {
    throw new Error("assistant response did not include a message body");
  }

  const parsed = JSON.parse(extractJsonBlock(content)) as Partial<AssistantPlan>;
  return {
    summary: String(parsed.summary ?? ""),
    rationale: String(parsed.rationale ?? ""),
    suggested_actions: Array.isArray(parsed.suggested_actions)
      ? parsed.suggested_actions.map((entry) => ({
          action: String((entry as { action?: string }).action ?? ""),
          payload:
            entry && typeof entry === "object" && "payload" in entry && typeof entry.payload === "object" && entry.payload
              ? (entry.payload as Record<string, unknown>)
              : {},
          reason: String((entry as { reason?: string }).reason ?? ""),
        }))
      : [],
  };
}
