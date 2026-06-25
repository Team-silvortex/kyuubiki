export const BUILTIN_WORKBENCH_LANGUAGE_CODES = ["en", "zh", "ja", "es"] as const;

export type BuiltInWorkbenchLanguage = (typeof BUILTIN_WORKBENCH_LANGUAGE_CODES)[number];
export type WorkbenchLanguage = string;

type WorkbenchLanguagePackSummary = {
  language: string;
  name: string;
};

type WorkbenchLanguageOptionCopy = {
  languages?: Record<string, string>;
};

export function isBuiltInWorkbenchLanguage(language: string): language is BuiltInWorkbenchLanguage {
  return BUILTIN_WORKBENCH_LANGUAGE_CODES.includes(language as BuiltInWorkbenchLanguage);
}

function describeWorkbenchLanguage(
  language: string,
  copy: WorkbenchLanguageOptionCopy,
  packs: WorkbenchLanguagePackSummary[],
) {
  const builtInLabel = copy.languages?.[language];
  if (typeof builtInLabel === "string" && builtInLabel.trim()) return builtInLabel;

  const pack = packs.find((entry) => entry.language === language && entry.name.trim());
  if (pack) return `${pack.name} (${language.toUpperCase()})`;

  return language.toUpperCase();
}

export function buildWorkbenchLanguageOptions(params: {
  copy: WorkbenchLanguageOptionCopy;
  languagePacks: WorkbenchLanguagePackSummary[];
  currentLanguage: WorkbenchLanguage;
}) {
  const { copy, languagePacks, currentLanguage } = params;
  const languages = [
    ...BUILTIN_WORKBENCH_LANGUAGE_CODES,
    ...languagePacks.map((pack) => pack.language),
    currentLanguage,
  ];
  return [...new Set(languages)]
    .filter((language) => typeof language === "string" && language.trim())
    .map((language) => ({
      value: language,
      label: describeWorkbenchLanguage(language, copy, languagePacks),
    }));
}
