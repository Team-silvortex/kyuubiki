import type { WorkbenchLanguagePack } from "@/lib/workbench/helpers";

export type WorkbenchLanguagePackCatalogEntry = {
  id: string;
  language: string;
  name: string;
  status: string;
};

const REMOTE_READY_STATUS = {
  en: "Ready for local import; remote download source pending",
  zh: "可本地导入；远程下载源待接入",
  ja: "ローカル取込に対応済み・リモート配布源は今後接続",
} as const;

const WORKBENCH_LANGUAGE_PACK_CATALOG: WorkbenchLanguagePackCatalogEntry[] = [
  {
    id: "workbench-fr-core-1.15",
    language: "fr",
    name: "French Workbench Core",
    status: REMOTE_READY_STATUS.en,
  },
  {
    id: "workbench-ko-core-1.15",
    language: "ko",
    name: "Korean Workbench Core",
    status: REMOTE_READY_STATUS.en,
  },
];

function localizeStatus(language: string) {
  if (language === "zh") return REMOTE_READY_STATUS.zh;
  if (language === "ja") return REMOTE_READY_STATUS.ja;
  return REMOTE_READY_STATUS.en;
}

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
  switch (packId) {
    case "workbench-fr-core-1.15":
      return {
        schema_version: "kyuubiki.language-pack/v1",
        id: "workbench-fr-core-1.15",
        language: "fr",
        targetSurface: "workbench",
        name: "French Workbench Core",
        version: "1.19.0",
        versionLine: "tamamono 1.x",
        targetAppVersion: "1.19.0",
        source: "downloaded",
        updatedAt: "2026-06-30T00:00:00.000Z",
        description: "Core French UI overrides for Workbench navigation, workflow, store, and system surfaces.",
        overrides: {
          title: "Workbench Kyuubiki",
          subtitle: "Espace de travail pour la simulation, les workflows et les agents.",
          rail: {
            study: "Etude",
            model: "Espace",
            workflow: "Workflow",
            store: "Store",
            library: "Historique",
            system: "Systeme",
          },
          sections: {
            study: "Configuration d'etude",
            model: "Espace de travail",
            workflow: "Studio de workflow",
            store: "Store du projet",
            library: "Historique des taches",
            system: "Systeme",
          },
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
        },
      };
    case "workbench-ko-core-1.15":
      return {
        schema_version: "kyuubiki.language-pack/v1",
        id: "workbench-ko-core-1.15",
        language: "ko",
        targetSurface: "workbench",
        name: "Korean Workbench Core",
        version: "1.19.0",
        versionLine: "tamamono 1.x",
        targetAppVersion: "1.19.0",
        source: "downloaded",
        updatedAt: "2026-06-30T00:00:00.000Z",
        description: "Core Korean UI overrides for Workbench navigation, workflow, store, and system surfaces.",
        overrides: {
          title: "Kyuubiki 워크벤치",
          subtitle: "시뮬레이션, 워크플로, 에이전트를 위한 작업 공간입니다.",
          rail: {
            study: "연구",
            model: "작업공간",
            workflow: "워크플로",
            store: "스토어",
            library: "기록",
            system: "시스템",
          },
          sections: {
            study: "연구 설정",
            model: "작업공간",
            workflow: "워크플로 스튜디오",
            store: "프로젝트 스토어",
            library: "작업 기록",
            system: "시스템",
          },
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
        },
      };
    default:
      return null;
  }
}
