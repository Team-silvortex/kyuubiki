import {
  WORKBENCH_LANGUAGE_PACK_SCHEMA_VERSION,
  WORKBENCH_LANGUAGE_PACK_TARGET_APP_VERSION,
  WORKBENCH_LANGUAGE_PACK_VERSION_LINE,
  type WorkbenchLanguagePack,
} from "@/lib/workbench/helpers";
import { WORKBENCH_TRANSLATED_LANGUAGE_PACK_OVERRIDES } from "@/components/workbench/workbench-language-pack-catalog-data";

export type WorkbenchLanguagePackCatalogEntry = {
  id: string;
  language: string;
  name: string;
  status: string;
};

type MainstreamLanguagePackLocale = {
  language: string;
  englishName: string;
  nativeName: string;
};

export const WORKBENCH_MAINSTREAM_LANGUAGE_PACK_LOCALES: MainstreamLanguagePackLocale[] = [
  { language: "ar", englishName: "Arabic", nativeName: "العربية" },
  { language: "bn", englishName: "Bengali", nativeName: "বাংলা" },
  { language: "cs", englishName: "Czech", nativeName: "Čeština" },
  { language: "da", englishName: "Danish", nativeName: "Dansk" },
  { language: "de", englishName: "German", nativeName: "Deutsch" },
  { language: "el", englishName: "Greek", nativeName: "Ελληνικά" },
  { language: "fa", englishName: "Persian", nativeName: "فارسی" },
  { language: "fi", englishName: "Finnish", nativeName: "Suomi" },
  { language: "fr", englishName: "French", nativeName: "Français" },
  { language: "he", englishName: "Hebrew", nativeName: "עברית" },
  { language: "hi", englishName: "Hindi", nativeName: "हिन्दी" },
  { language: "id", englishName: "Indonesian", nativeName: "Bahasa Indonesia" },
  { language: "it", englishName: "Italian", nativeName: "Italiano" },
  { language: "ko", englishName: "Korean", nativeName: "한국어" },
  { language: "ms", englishName: "Malay", nativeName: "Bahasa Melayu" },
  { language: "nl", englishName: "Dutch", nativeName: "Nederlands" },
  { language: "no", englishName: "Norwegian", nativeName: "Norsk" },
  { language: "pl", englishName: "Polish", nativeName: "Polski" },
  { language: "pt-BR", englishName: "Portuguese (Brazil)", nativeName: "Português (Brasil)" },
  { language: "ro", englishName: "Romanian", nativeName: "Română" },
  { language: "ru", englishName: "Russian", nativeName: "Русский" },
  { language: "sv", englishName: "Swedish", nativeName: "Svenska" },
  { language: "sw", englishName: "Swahili", nativeName: "Kiswahili" },
  { language: "ta", englishName: "Tamil", nativeName: "தமிழ்" },
  { language: "th", englishName: "Thai", nativeName: "ไทย" },
  { language: "tr", englishName: "Turkish", nativeName: "Türkçe" },
  { language: "uk", englishName: "Ukrainian", nativeName: "Українська" },
  { language: "ur", englishName: "Urdu", nativeName: "اردو" },
  { language: "vi", englishName: "Vietnamese", nativeName: "Tiếng Việt" },
  { language: "zh-TW", englishName: "Traditional Chinese", nativeName: "繁體中文" },
];

const REMOTE_READY_STATUS = {
  en: "Translated pack ready for local import; remote download source pending",
  zh: "翻译包可本地导入；远程下载源待接入",
  ja: "翻訳済みパックをローカル取込可能・リモート配布源は今後接続",
} as const;

const UPDATED_AT = "2026-07-16T00:00:00.000Z";

function languageSlug(language: string) {
  return language.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "");
}

function workbenchPackId(language: string) {
  return `workbench-${languageSlug(language)}-core-2.0`;
}

function localizeStatus(language: string) {
  if (language === "zh") return REMOTE_READY_STATUS.zh;
  if (language === "ja") return REMOTE_READY_STATUS.ja;
  return REMOTE_READY_STATUS.en;
}

function buildCatalogEntry(locale: MainstreamLanguagePackLocale): WorkbenchLanguagePackCatalogEntry {
  return {
    id: workbenchPackId(locale.language),
    language: locale.language,
    name: `${locale.englishName} Workbench Core`,
    status: REMOTE_READY_STATUS.en,
  };
}

const WORKBENCH_LANGUAGE_PACK_CATALOG = WORKBENCH_MAINSTREAM_LANGUAGE_PACK_LOCALES.map(buildCatalogEntry);

export function buildWorkbenchLanguagePackCatalogRows(language: string): WorkbenchLanguagePackCatalogEntry[] {
  return WORKBENCH_LANGUAGE_PACK_CATALOG.map((entry) => ({
    ...entry,
    status: localizeStatus(language),
  }));
}

export function findWorkbenchLanguagePackCatalogEntry(packId: string): WorkbenchLanguagePackCatalogEntry | null {
  return WORKBENCH_LANGUAGE_PACK_CATALOG.find((entry) => entry.id === packId) ?? null;
}

export function getBuiltinWorkbenchLanguagePack(packId: string): WorkbenchLanguagePack | null {
  const locale = WORKBENCH_MAINSTREAM_LANGUAGE_PACK_LOCALES.find((entry) => workbenchPackId(entry.language) === packId);
  if (!locale) return null;

  return {
    schema_version: WORKBENCH_LANGUAGE_PACK_SCHEMA_VERSION,
    id: workbenchPackId(locale.language),
    language: locale.language,
    targetSurface: "workbench",
    name: `${locale.englishName} Workbench Core`,
    version: WORKBENCH_LANGUAGE_PACK_TARGET_APP_VERSION,
    versionLine: WORKBENCH_LANGUAGE_PACK_VERSION_LINE,
    targetAppVersion: WORKBENCH_LANGUAGE_PACK_TARGET_APP_VERSION,
    source: "downloaded",
    updatedAt: UPDATED_AT,
    description: `${locale.englishName} UI translations for Workbench navigation, workflow, store, and system surfaces.`,
    overrides: WORKBENCH_TRANSLATED_LANGUAGE_PACK_OVERRIDES[locale.language] ?? {},
  };
}
