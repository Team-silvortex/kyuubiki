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

const MAINSTREAM_WORKBENCH_LANGUAGE_LABELS: Record<string, string> = {
  ar: "العربية · Arabic",
  bn: "বাংলা · Bengali",
  cs: "Čeština · Czech",
  da: "Dansk · Danish",
  de: "Deutsch · German",
  el: "Ελληνικά · Greek",
  fa: "فارسی · Persian",
  fi: "Suomi · Finnish",
  fr: "Français · French",
  he: "עברית · Hebrew",
  hi: "हिन्दी · Hindi",
  id: "Bahasa Indonesia · Indonesian",
  it: "Italiano · Italian",
  ko: "한국어 · Korean",
  ms: "Bahasa Melayu · Malay",
  nl: "Nederlands · Dutch",
  no: "Norsk · Norwegian",
  pl: "Polski · Polish",
  "pt-BR": "Português (Brasil)",
  ro: "Română · Romanian",
  ru: "Русский · Russian",
  sv: "Svenska · Swedish",
  sw: "Kiswahili · Swahili",
  ta: "தமிழ் · Tamil",
  th: "ไทย · Thai",
  tr: "Türkçe · Turkish",
  uk: "Українська · Ukrainian",
  ur: "اردو · Urdu",
  vi: "Tiếng Việt · Vietnamese",
  "zh-TW": "繁體中文 · Traditional Chinese",
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
  const mainstreamLabel = MAINSTREAM_WORKBENCH_LANGUAGE_LABELS[language];
  if (mainstreamLabel) return mainstreamLabel;

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
    ...Object.keys(MAINSTREAM_WORKBENCH_LANGUAGE_LABELS),
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
