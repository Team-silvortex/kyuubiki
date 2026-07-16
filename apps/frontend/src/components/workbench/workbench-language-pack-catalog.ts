import {
  WORKBENCH_LANGUAGE_PACK_SCHEMA_VERSION,
  WORKBENCH_LANGUAGE_PACK_TARGET_APP_VERSION,
  WORKBENCH_LANGUAGE_PACK_VERSION_LINE,
  type WorkbenchLanguagePack,
} from "@/lib/workbench/helpers";

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
  en: "Ready for local import; remote download source pending",
  zh: "可本地导入；远程下载源待接入",
  ja: "ローカル取込に対応済み・リモート配布源は今後接続",
} as const;

const UPDATED_AT = "2026-07-13T00:00:00.000Z";

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

function buildStarterOverrides(locale: MainstreamLanguagePackLocale): Record<string, unknown> {
  if (locale.language === "fr") {
    return {
      title: "Workbench Kyuubiki",
      subtitle: "Espace de travail pour la simulation, les workflows et les agents.",
      rail: { study: "Etude", model: "Espace", workflow: "Workflow", store: "Store", library: "Historique", system: "Systeme" },
      sections: { study: "Configuration d'etude", model: "Espace de travail", workflow: "Studio de workflow", store: "Store du projet", library: "Historique des taches", system: "Systeme" },
      workflowBuilderPage: "Constructeur",
      workflowRunsPage: "Executions",
      workflowCatalogTitle: "Catalogue de workflows",
      workflowTemplateChainLibraryLabel: "Bibliotheque de chaines",
      languagePacksTitle: "Packs de langue installes",
      languagePacksHint: "Importez des packs JSON locaux maintenant, avec la meme surface prete pour une distribution distante.",
      languagePacksEmptyLabel: "Aucun pack de langue personnalise n'est encore installe.",
      languagePackName: "Nom",
      languagePackVersion: "Version",
      languagePackSourceImported: "Importe",
      languagePackSourceDownloaded: "Telecharge",
      languagePackDownloadTemplate: "Telecharger le modele",
      languagePackExportInstalled: "Exporter le pack installe",
      languagePackImport: "Importer un pack",
      languagePackRemove: "Retirer",
      languagePackCatalogTitle: "Catalogue futur",
      languagePackCatalogHint: "Ces emplacements restent visibles pour brancher plus tard le telechargement de packs distants.",
      languagePackCatalogAction: "Bientot",
    };
  }

  if (locale.language === "ko") {
    return {
      title: "Kyuubiki 워크벤치",
      subtitle: "시뮬레이션, 워크플로, 에이전트를 위한 작업 공간입니다.",
      rail: { study: "연구", model: "작업공간", workflow: "워크플로", store: "스토어", library: "기록", system: "시스템" },
      sections: { study: "연구 설정", model: "작업공간", workflow: "워크플로 스튜디오", store: "프로젝트 스토어", library: "작업 기록", system: "시스템" },
      workflowBuilderPage: "빌더",
      workflowRunsPage: "실행",
      workflowCatalogTitle: "워크플로 카탈로그",
      workflowTemplateChainLibraryLabel: "체인 라이브러리",
      languagePacksTitle: "설치된 언어 팩",
      languagePacksHint: "지금은 로컬 JSON 팩을 가져오고, 나중의 원격 배포도 같은 화면에서 이어갑니다.",
      languagePacksEmptyLabel: "아직 설치된 사용자 언어 팩이 없습니다.",
      languagePackName: "이름",
      languagePackVersion: "버전",
      languagePackSourceImported: "가져옴",
      languagePackSourceDownloaded: "다운로드됨",
      languagePackDownloadTemplate: "템플릿 다운로드",
      languagePackExportInstalled: "설치된 팩 내보내기",
      languagePackImport: "언어 팩 가져오기",
      languagePackRemove: "제거",
      languagePackCatalogTitle: "향후 카탈로그",
      languagePackCatalogHint: "이 영역은 원격 언어 팩 다운로드를 같은 화면에 연결하기 위해 미리 남겨둡니다.",
      languagePackCatalogAction: "준비 중",
    };
  }

  return {
    title: `Kyuubiki Workbench - ${locale.nativeName}`,
    subtitle: `${locale.englishName} starter language pack for simulation, workflow, and agent surfaces.`,
    rail: { study: "Study", model: "Model", workflow: "Workflow", store: "Store", library: "History", system: "System" },
    sections: { study: "Study setup", model: "Model space", workflow: "Workflow studio", store: "Project store", library: "Task history", system: "System" },
    workflowBuilderPage: "Builder",
    workflowRunsPage: "Runs",
    workflowCatalogTitle: `${locale.englishName} workflow catalog`,
    workflowTemplateChainLibraryLabel: `${locale.nativeName} chain library`,
    languagePacksTitle: `${locale.englishName} language packs`,
    languagePacksHint: `Install this ${locale.englishName} starter pack locally now; remote sources can reuse the same envelope later.`,
    languagePacksEmptyLabel: `No ${locale.englishName} language pack is installed yet.`,
    languagePackName: "Name",
    languagePackVersion: "Version",
    languagePackSourceImported: "Imported",
    languagePackSourceDownloaded: "Downloaded",
    languagePackDownloadTemplate: "Download template",
    languagePackExportInstalled: "Export installed pack",
    languagePackImport: "Import pack",
    languagePackRemove: "Remove",
    languagePackCatalogTitle: `${locale.nativeName} catalog`,
    languagePackCatalogHint: `Starter coverage for ${locale.englishName}; full product copy can expand incrementally without changing the pack contract.`,
    languagePackCatalogAction: "Install",
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
    description: `Starter ${locale.englishName} UI overrides for Workbench navigation, workflow, store, and system surfaces.`,
    overrides: buildStarterOverrides(locale),
  };
}
