"use client";

import type { Dispatch, SetStateAction } from "react";
import type { WorkbenchCopy, WorkbenchLanguage } from "@/components/workbench/workbench-copy";
import type { WorkbenchLanguagePack } from "@/lib/workbench/helpers";
import {
  WORKBENCH_LANGUAGE_PACK_SCHEMA_VERSION,
  WORKBENCH_LANGUAGE_PACK_TARGET_APP_VERSION,
  WORKBENCH_LANGUAGE_PACK_VERSION_LINE,
  getWorkbenchLanguagePackCompatibility,
} from "@/lib/workbench/helpers";
import { getBuiltinWorkbenchLanguagePack } from "@/components/workbench/workbench-language-pack-catalog";
import { getWorkbenchLanguagePackSystemCopy } from "@/components/workbench/workbench-language-pack-system-copy";

const UNSAFE_LANGUAGE_PACK_TEXT_PATTERNS = [
  "<",
  ">",
  "javascript:",
  "data:text/html",
  "onerror=",
  "onclick=",
  "innerhtml",
  "document.cookie",
  "localstorage",
  "eval(",
] as const;

function hasUnsafeLanguagePackText(value: unknown): boolean {
  if (typeof value === "string") {
    const normalized = value.toLowerCase();
    return UNSAFE_LANGUAGE_PACK_TEXT_PATTERNS.some((pattern) => normalized.includes(pattern));
  }
  if (Array.isArray(value)) return value.some(hasUnsafeLanguagePackText);
  if (value && typeof value === "object") {
    return Object.values(value as Record<string, unknown>).some(hasUnsafeLanguagePackText);
  }
  return false;
}

function setUnsafeLanguagePackMessage(language: WorkbenchLanguage, setMessage: (value: string) => void) {
  setMessage(getWorkbenchLanguagePackSystemCopy(language).unsafeRejected);
}

function triggerWorkbenchJsonDownload(filename: string, payload: Record<string, unknown>) {
  if (typeof window === "undefined") return;
  const blob = new Blob([`${JSON.stringify(payload, null, 2)}\n`], { type: "application/json" });
  const url = window.URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = filename;
  anchor.click();
  window.URL.revokeObjectURL(url);
}

export function downloadWorkbenchLanguagePackTemplate(params: {
  language: WorkbenchLanguage;
  copy: WorkbenchCopy;
  setMessage: (value: string) => void;
}) {
  const { language, copy: t, setMessage } = params;
  triggerWorkbenchJsonDownload(`workbench-language-pack-${language}.json`, {
    schema_version: WORKBENCH_LANGUAGE_PACK_SCHEMA_VERSION,
    id: `${language}-custom-pack`,
    language,
    targetSurface: "workbench",
    name: `${t.languages[language as keyof typeof t.languages] ?? language.toUpperCase()} custom pack`,
    version: "2.0.0",
    versionLine: WORKBENCH_LANGUAGE_PACK_VERSION_LINE,
    targetAppVersion: WORKBENCH_LANGUAGE_PACK_TARGET_APP_VERSION,
    source: "imported",
    description: getWorkbenchLanguagePackSystemCopy(language).templateDescription,
    overrides: {},
  });
  setMessage(getWorkbenchLanguagePackSystemCopy(language).templateDownloaded);
}

export function exportWorkbenchInstalledLanguagePack(params: {
  language: WorkbenchLanguage;
  activeLanguagePack: WorkbenchLanguagePack | null;
  setMessage: (value: string) => void;
}) {
  const { language, activeLanguagePack, setMessage } = params;
  if (!activeLanguagePack) {
    setMessage(getWorkbenchLanguagePackSystemCopy(language).noCustomPack);
    return;
  }

  triggerWorkbenchJsonDownload(
    `workbench-language-pack-${activeLanguagePack.language}-${activeLanguagePack.id}.json`,
    activeLanguagePack,
  );
  setMessage(getWorkbenchLanguagePackSystemCopy(language).exported);
}

export async function importWorkbenchLanguagePack(params: {
  file: File;
  language: WorkbenchLanguage;
  setLanguagePacks: Dispatch<SetStateAction<WorkbenchLanguagePack[]>>;
  setMessage: (value: string) => void;
}) {
  const { file, language, setLanguagePacks, setMessage } = params;
  try {
    const raw = JSON.parse(await file.text()) as Partial<WorkbenchLanguagePack> & {
      overrides?: Record<string, unknown>;
    };
    installWorkbenchLanguagePackPayload({ raw, language, setLanguagePacks, setMessage });
  } catch {
    setMessage(getWorkbenchLanguagePackSystemCopy(language).invalidJson);
  }
}

export function installWorkbenchLanguagePackPayload(params: {
  raw: Partial<WorkbenchLanguagePack> & { overrides?: Record<string, unknown> };
  language: WorkbenchLanguage;
  setLanguagePacks: Dispatch<SetStateAction<WorkbenchLanguagePack[]>>;
  setMessage: (value: string) => void;
}) {
  const { raw, language, setLanguagePacks, setMessage } = params;
  if (!raw || typeof raw !== "object" || typeof raw.language !== "string" || typeof raw.name !== "string") {
    throw new Error("invalid-pack");
  }
  if (raw.targetSurface !== undefined && raw.targetSurface !== "workbench") {
    throw new Error("wrong-surface");
  }
  if (hasUnsafeLanguagePackText(raw)) {
    setUnsafeLanguagePackMessage(language, setMessage);
    return;
  }

  const nextPack: WorkbenchLanguagePack = {
    schema_version:
      typeof raw.schema_version === "string" && raw.schema_version.trim()
        ? raw.schema_version.trim()
        : WORKBENCH_LANGUAGE_PACK_SCHEMA_VERSION,
    id: typeof raw.id === "string" && raw.id.trim() ? raw.id.trim() : `${raw.language}-${Date.now()}`,
    language: raw.language,
    targetSurface: "workbench",
    name: raw.name,
    version: typeof raw.version === "string" && raw.version.trim() ? raw.version.trim() : "2.0.0",
    versionLine: typeof raw.versionLine === "string" && raw.versionLine.trim() ? raw.versionLine.trim() : undefined,
    targetAppVersion:
      typeof raw.targetAppVersion === "string" && raw.targetAppVersion.trim() ? raw.targetAppVersion.trim() : undefined,
    source: raw.source === "downloaded" ? "downloaded" : "imported",
    updatedAt: typeof raw.updatedAt === "string" && raw.updatedAt.trim() ? raw.updatedAt.trim() : new Date().toISOString(),
    description: typeof raw.description === "string" ? raw.description : undefined,
    overrides:
      raw.overrides && typeof raw.overrides === "object" && !Array.isArray(raw.overrides) ? raw.overrides : {},
  };

  const compatibility = getWorkbenchLanguagePackCompatibility(nextPack);

  setLanguagePacks((current) => {
    const next = current.filter(
      (pack) => !(pack.id === nextPack.id || (pack.language === nextPack.language && pack.name === nextPack.name)),
    );
    return [nextPack, ...next];
  });

  const copy = getWorkbenchLanguagePackSystemCopy(language);
  setMessage(compatibility === "mismatch" ? copy.importedMismatch : copy.imported);
}

export function installBuiltinWorkbenchLanguagePack(params: {
  packId: string;
  language: WorkbenchLanguage;
  setLanguagePacks: Dispatch<SetStateAction<WorkbenchLanguagePack[]>>;
  setMessage: (value: string) => void;
}) {
  const { packId, language, setLanguagePacks, setMessage } = params;
  const pack = getBuiltinWorkbenchLanguagePack(packId);
  if (!pack) {
    setMessage(getWorkbenchLanguagePackSystemCopy(language).notFound);
    return;
  }

  installWorkbenchLanguagePackPayload({ raw: pack, language, setLanguagePacks, setMessage });
}

export function removeWorkbenchLanguagePack(params: {
  packId: string;
  setLanguagePacks: Dispatch<SetStateAction<WorkbenchLanguagePack[]>>;
  language: WorkbenchLanguage;
  setMessage: (value: string) => void;
}) {
  const { packId, setLanguagePacks, language, setMessage } = params;
  setLanguagePacks((current) => current.filter((pack) => pack.id !== packId));
  setMessage(getWorkbenchLanguagePackSystemCopy(language).removed);
}
