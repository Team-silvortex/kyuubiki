"use client";

export type WorkflowTemplateChainPreferenceSnapshot = {
  favoriteChainIds: string[];
  favoriteChainAliases: Record<string, string>;
};

const FAVORITE_TEMPLATE_CHAIN_STORAGE_KEY =
  "kyuubiki.workflow.favoriteTemplateChains";
const FAVORITE_TEMPLATE_CHAIN_ALIAS_STORAGE_KEY =
  "kyuubiki.workflow.favoriteTemplateChainAliases";

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function readFavoriteChainIds(): string[] {
  if (typeof window === "undefined") return [];
  try {
    const raw = window.localStorage.getItem(FAVORITE_TEMPLATE_CHAIN_STORAGE_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw) as unknown;
    return Array.isArray(parsed)
      ? parsed.filter((value): value is string => typeof value === "string")
      : [];
  } catch {
    return [];
  }
}

function readFavoriteChainAliases(): Record<string, string> {
  if (typeof window === "undefined") return {};
  try {
    const raw = window.localStorage.getItem(FAVORITE_TEMPLATE_CHAIN_ALIAS_STORAGE_KEY);
    if (!raw) return {};
    const parsed = JSON.parse(raw) as unknown;
    if (!isRecord(parsed)) return {};
    return Object.fromEntries(
      Object.entries(parsed).filter(
        (entry): entry is [string, string] => typeof entry[1] === "string",
      ),
    );
  } catch {
    return {};
  }
}

export function readWorkflowTemplateChainPreferences(): WorkflowTemplateChainPreferenceSnapshot {
  return {
    favoriteChainIds: readFavoriteChainIds(),
    favoriteChainAliases: readFavoriteChainAliases(),
  };
}

export function writeWorkflowTemplateChainPreferences(
  snapshot: WorkflowTemplateChainPreferenceSnapshot,
) {
  if (typeof window === "undefined") return;
  window.localStorage.setItem(
    FAVORITE_TEMPLATE_CHAIN_STORAGE_KEY,
    JSON.stringify(snapshot.favoriteChainIds.slice(0, 12)),
  );
  window.localStorage.setItem(
    FAVORITE_TEMPLATE_CHAIN_ALIAS_STORAGE_KEY,
    JSON.stringify(snapshot.favoriteChainAliases),
  );
}

