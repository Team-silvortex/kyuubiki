"use client";

import type { Dispatch, SetStateAction } from "react";
import type { WorkbenchCopy, WorkbenchLanguage } from "@/components/workbench/workbench-copy";
import type { WorkbenchLanguagePack } from "@/lib/workbench/helpers";
import { WORKBENCH_LANGUAGE_PACK_SCHEMA_VERSION } from "@/lib/workbench/helpers";

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
    name: `${t.languages[language]} custom pack`,
    version: "1.0.0",
    source: "imported",
    description:
      language === "zh"
        ? "从这个模板开始覆盖 Workbench 文案。"
        : language === "ja"
          ? "このテンプレートから Workbench 文言を上書きします。"
          : "Start from this template to override Workbench copy.",
    overrides: {},
  });
  setMessage(
    language === "zh"
      ? "语言包模板已下载。"
      : language === "ja"
        ? "言語パックのテンプレートを出力しました。"
        : "Language pack template downloaded.",
  );
}

export function exportWorkbenchInstalledLanguagePack(params: {
  language: WorkbenchLanguage;
  activeLanguagePack: WorkbenchLanguagePack | null;
  setMessage: (value: string) => void;
}) {
  const { language, activeLanguagePack, setMessage } = params;
  if (!activeLanguagePack) {
    setMessage(
      language === "zh"
        ? "当前语言还没有安装自定义语言包。"
        : language === "ja"
          ? "現在の言語にはまだカスタム言語パックがありません。"
          : "No custom language pack is installed for the current language yet.",
    );
    return;
  }

  triggerWorkbenchJsonDownload(
    `workbench-language-pack-${activeLanguagePack.language}-${activeLanguagePack.id}.json`,
    activeLanguagePack,
  );
  setMessage(
    language === "zh"
      ? "当前语言包已导出。"
      : language === "ja"
        ? "現在の言語パックを出力しました。"
        : "Exported the current language pack.",
  );
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
    if (!raw || typeof raw !== "object" || typeof raw.language !== "string" || typeof raw.name !== "string") {
      throw new Error("invalid-pack");
    }

    const nextPack: WorkbenchLanguagePack = {
      schema_version:
        typeof raw.schema_version === "string" && raw.schema_version.trim()
          ? raw.schema_version.trim()
          : WORKBENCH_LANGUAGE_PACK_SCHEMA_VERSION,
      id: typeof raw.id === "string" && raw.id.trim() ? raw.id.trim() : `${raw.language}-${Date.now()}`,
      language: raw.language,
      name: raw.name,
      version: typeof raw.version === "string" && raw.version.trim() ? raw.version.trim() : "1.0.0",
      source: raw.source === "downloaded" ? "downloaded" : "imported",
      updatedAt: new Date().toISOString(),
      description: typeof raw.description === "string" ? raw.description : undefined,
      overrides:
        raw.overrides && typeof raw.overrides === "object" && !Array.isArray(raw.overrides) ? raw.overrides : {},
    };

    setLanguagePacks((current) => {
      const next = current.filter(
        (pack) => !(pack.id === nextPack.id || (pack.language === nextPack.language && pack.name === nextPack.name)),
      );
      return [nextPack, ...next];
    });

    setMessage(
      language === "zh"
        ? "语言包已导入。"
        : language === "ja"
          ? "言語パックを取り込みました。"
          : "Language pack imported.",
    );
  } catch {
    setMessage(
      language === "zh"
        ? "语言包 JSON 无效。"
        : language === "ja"
          ? "言語パック JSON が無効です。"
          : "Invalid language pack JSON.",
    );
  }
}

export function removeWorkbenchLanguagePack(params: {
  packId: string;
  setLanguagePacks: Dispatch<SetStateAction<WorkbenchLanguagePack[]>>;
  language: WorkbenchLanguage;
  setMessage: (value: string) => void;
}) {
  const { packId, setLanguagePacks, language, setMessage } = params;
  setLanguagePacks((current) => current.filter((pack) => pack.id !== packId));
  setMessage(
    language === "zh"
      ? "语言包已移除。"
      : language === "ja"
        ? "言語パックを削除しました。"
        : "Language pack removed.",
  );
}

