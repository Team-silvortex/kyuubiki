"use client";

export type WorkflowTemplateChainPreferenceSnapshot = {
  favoriteChainIds: string[];
  favoriteChainAliases: Record<string, string>;
};

export const FAVORITE_TEMPLATE_CHAIN_STORAGE_KEY =
  "kyuubiki.workflow.favoriteTemplateChains";
export const FAVORITE_TEMPLATE_CHAIN_ALIAS_STORAGE_KEY =
  "kyuubiki.workflow.favoriteTemplateChainAliases";
export const FAVORITE_TEMPLATE_CHAIN_LIMIT = 12;

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function readFavoriteChainIds(): string[] {
  if (typeof window === "undefined") return [];
  try {
    const raw = window.localStorage.getItem(FAVORITE_TEMPLATE_CHAIN_STORAGE_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw) as unknown;
    if (!Array.isArray(parsed)) return [];
    return [...new Set(
      parsed.filter(
        (value): value is string => typeof value === "string" && value.trim().length > 0,
      ),
    )].slice(0, FAVORITE_TEMPLATE_CHAIN_LIMIT);
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
  const favoriteChainIds = readFavoriteChainIds();
  const favoriteIds = new Set(favoriteChainIds);
  const favoriteChainAliases = Object.fromEntries(
    Object.entries(readFavoriteChainAliases()).filter(
      ([chainId, alias]) => favoriteIds.has(chainId) && alias.trim().length > 0,
    ),
  );
  return { favoriteChainIds, favoriteChainAliases };
}

export function writeWorkflowTemplateChainPreferences(
  snapshot: WorkflowTemplateChainPreferenceSnapshot,
): boolean {
  if (typeof window === "undefined") return false;
  const favoriteChainIds = [...new Set(
    (Array.isArray(snapshot.favoriteChainIds) ? snapshot.favoriteChainIds : []).filter(
      (chainId): chainId is string =>
        typeof chainId === "string" && chainId.trim().length > 0,
    ),
  )].slice(0, FAVORITE_TEMPLATE_CHAIN_LIMIT);
  const favoriteIds = new Set(favoriteChainIds);
  const favoriteChainAliases = Object.fromEntries(
    Object.entries(isRecord(snapshot.favoriteChainAliases) ? snapshot.favoriteChainAliases : {})
      .filter(
        (entry): entry is [string, string] =>
          favoriteIds.has(entry[0]) &&
          typeof entry[1] === "string" &&
          entry[1].trim().length > 0,
      ),
  );
  let previousFavoriteIds: string | null | undefined;
  let previousAliases: string | null | undefined;
  function restore(key: string, previous: string | null | undefined) {
    if (previous === undefined) return;
    try {
      if (previous === null) window.localStorage.removeItem(key);
      else window.localStorage.setItem(key, previous);
    } catch {
      // The caller still receives false when a read-only or quota-limited store cannot roll back.
    }
  }
  try {
    previousFavoriteIds = window.localStorage.getItem(
      FAVORITE_TEMPLATE_CHAIN_STORAGE_KEY,
    );
    previousAliases = window.localStorage.getItem(
      FAVORITE_TEMPLATE_CHAIN_ALIAS_STORAGE_KEY,
    );
    window.localStorage.setItem(
      FAVORITE_TEMPLATE_CHAIN_STORAGE_KEY,
      JSON.stringify(favoriteChainIds),
    );
    window.localStorage.setItem(
      FAVORITE_TEMPLATE_CHAIN_ALIAS_STORAGE_KEY,
      JSON.stringify(favoriteChainAliases),
    );
    return true;
  } catch {
    restore(FAVORITE_TEMPLATE_CHAIN_STORAGE_KEY, previousFavoriteIds);
    restore(FAVORITE_TEMPLATE_CHAIN_ALIAS_STORAGE_KEY, previousAliases);
    return false;
  }
}
