import { getWorkbenchLanguagePackCompatibility } from "@/lib/workbench/helpers";
import { getWorkbenchLanguagePackSystemCopy } from "@/components/workbench/workbench-language-pack-system-copy";

type PresentedLanguage = string;

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
    const copy = getWorkbenchLanguagePackSystemCopy(language);
    const compatibilityLabel =
      compatibility === "exact"
        ? copy.compatibilityExact
        : compatibility === "line"
          ? copy.compatibilityLine
          : compatibility === "mismatch"
            ? copy.compatibilityMismatch
            : copy.compatibilityGeneric;
    const targetLabel = `${copy.targetPrefix}: ${pack.versionLine ?? copy.noVersionLine} · ${
      pack.targetAppVersion ?? copy.noAppVersion
    }`;

    return {
      ...pack,
      compatibilityLabel,
      targetLabel,
    };
  });
}
