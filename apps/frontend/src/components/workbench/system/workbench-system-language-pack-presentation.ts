import { getWorkbenchLanguagePackCompatibility } from "@/lib/workbench/helpers";

type PresentedLanguage = "en" | "zh" | "ja" | "es";

type LanguagePackSummary = {
  id: string;
  language: string;
  name: string;
  version: string;
  versionLine?: string;
  targetAppVersion?: string;
  source: "imported" | "downloaded";
  updatedAt: string;
  description?: string;
};

export function buildWorkbenchLanguagePackPresentation(
  language: PresentedLanguage,
  packs: LanguagePackSummary[],
) {
  return packs.map((pack) => {
    const compatibility = getWorkbenchLanguagePackCompatibility(pack);
    const compatibilityLabel =
      language === "zh"
        ? compatibility === "exact"
          ? "兼容性：与当前 Workbench 版本完全匹配"
          : compatibility === "line"
            ? "兼容性：与当前 tamamono 版本线匹配"
            : compatibility === "mismatch"
              ? "兼容性：目标版本与当前 Workbench 不匹配"
              : "兼容性：未声明目标版本，按通用覆盖处理"
        : language === "ja"
          ? compatibility === "exact"
            ? "互換性: 現在の Workbench バージョンに完全一致"
            : compatibility === "line"
              ? "互換性: 現在の tamamono 系統に一致"
              : compatibility === "mismatch"
                ? "互換性: 対象バージョンが現在の Workbench と不一致"
                : "互換性: 対象バージョン未指定の汎用上書き"
          : compatibility === "exact"
            ? "Compatibility: exact match for the current Workbench version"
            : compatibility === "line"
              ? "Compatibility: matches the current tamamono version line"
              : compatibility === "mismatch"
                ? "Compatibility: target version does not match the current Workbench"
                : "Compatibility: unscoped pack, applied as a generic override";
    const targetLabel =
      language === "zh"
        ? `目标：${pack.versionLine ?? "未声明版本线"} · ${pack.targetAppVersion ?? "未声明应用版本"}`
        : language === "ja"
          ? `対象: ${pack.versionLine ?? "系統未指定"} · ${pack.targetAppVersion ?? "アプリ版未指定"}`
          : `Target: ${pack.versionLine ?? "no version line"} · ${pack.targetAppVersion ?? "no app version"}`;

    return {
      ...pack,
      compatibilityLabel,
      targetLabel,
    };
  });
}
