// Generated from language-packs/workbench/*.json. Do not edit by hand.
type WorkbenchLanguagePackOverrides = Record<string, unknown>;
type WorkbenchLanguagePackLoader = () => Promise<WorkbenchLanguagePackOverrides>;

const WORKBENCH_TRANSLATED_LANGUAGE_PACK_LOADERS: Record<string, WorkbenchLanguagePackLoader> = {
  "ar": () => import("./workbench-language-pack-data/ar").then((module) => module.default),
  "bn": () => import("./workbench-language-pack-data/bn").then((module) => module.default),
  "cs": () => import("./workbench-language-pack-data/cs").then((module) => module.default),
  "da": () => import("./workbench-language-pack-data/da").then((module) => module.default),
  "de": () => import("./workbench-language-pack-data/de").then((module) => module.default),
  "el": () => import("./workbench-language-pack-data/el").then((module) => module.default),
  "fa": () => import("./workbench-language-pack-data/fa").then((module) => module.default),
  "fi": () => import("./workbench-language-pack-data/fi").then((module) => module.default),
  "fr": () => import("./workbench-language-pack-data/fr").then((module) => module.default),
  "he": () => import("./workbench-language-pack-data/he").then((module) => module.default),
  "hi": () => import("./workbench-language-pack-data/hi").then((module) => module.default),
  "id": () => import("./workbench-language-pack-data/id").then((module) => module.default),
  "it": () => import("./workbench-language-pack-data/it").then((module) => module.default),
  "ko": () => import("./workbench-language-pack-data/ko").then((module) => module.default),
  "ms": () => import("./workbench-language-pack-data/ms").then((module) => module.default),
  "nl": () => import("./workbench-language-pack-data/nl").then((module) => module.default),
  "no": () => import("./workbench-language-pack-data/no").then((module) => module.default),
  "pl": () => import("./workbench-language-pack-data/pl").then((module) => module.default),
  "pt-BR": () => import("./workbench-language-pack-data/pt-br").then((module) => module.default),
  "ro": () => import("./workbench-language-pack-data/ro").then((module) => module.default),
  "ru": () => import("./workbench-language-pack-data/ru").then((module) => module.default),
  "sv": () => import("./workbench-language-pack-data/sv").then((module) => module.default),
  "sw": () => import("./workbench-language-pack-data/sw").then((module) => module.default),
  "ta": () => import("./workbench-language-pack-data/ta").then((module) => module.default),
  "th": () => import("./workbench-language-pack-data/th").then((module) => module.default),
  "tr": () => import("./workbench-language-pack-data/tr").then((module) => module.default),
  "uk": () => import("./workbench-language-pack-data/uk").then((module) => module.default),
  "ur": () => import("./workbench-language-pack-data/ur").then((module) => module.default),
  "vi": () => import("./workbench-language-pack-data/vi").then((module) => module.default),
  "zh-TW": () => import("./workbench-language-pack-data/zh-tw").then((module) => module.default),
};

const workbenchLanguagePackCache = new Map<string, Promise<WorkbenchLanguagePackOverrides>>();

export function loadWorkbenchTranslatedLanguagePackOverrides(language: string): Promise<WorkbenchLanguagePackOverrides | null> {
  const loader = WORKBENCH_TRANSLATED_LANGUAGE_PACK_LOADERS[language];
  if (!loader) return Promise.resolve(null);
  const cached = workbenchLanguagePackCache.get(language);
  if (cached) return cached;
  const pending = loader().catch((error) => {
    workbenchLanguagePackCache.delete(language);
    throw error;
  });
  workbenchLanguagePackCache.set(language, pending);
  return pending;
}
