import { describeDesktopLanguage, loadDesktopLanguagePack } from "./shared/language-pack-loader.js";

function isPlainObject(value) {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

function installerShellCopyFromHubPack(pack, baseCopy) {
  const language = typeof pack?.language === "string" ? pack.language : "en";
  const label = describeDesktopLanguage(language);
  const overrides = isPlainObject(pack?.overrides) ? pack.overrides : {};
  const shell = isPlainObject(overrides.shell) ? overrides.shell : {};
  const sections = isPlainObject(overrides.sections) ? overrides.sections : {};
  const deploy = isPlainObject(sections.deploy) ? sections.deploy : {};
  const projects = isPlainObject(sections.projects) ? sections.projects : {};
  return {
    ...baseCopy,
    language: typeof shell.language === "string" ? shell.language : label,
    roleChip: `${label} bootstrap shell`,
    description:
      typeof deploy.copy === "string"
        ? deploy.copy
        : typeof projects.copy === "string"
          ? projects.copy
          : pack?.description || baseCopy.description,
    actions: {
      ...baseCopy.actions,
      serviceStatus: typeof shell.actionStatus === "string" ? shell.actionStatus : baseCopy.actions.serviceStatus,
      validateEnv: typeof shell.validateEnv === "string" ? shell.validateEnv : baseCopy.actions.validateEnv,
      bootstrap: typeof shell.startLocal === "string" ? shell.startLocal : baseCopy.actions.bootstrap,
    },
    headings: {
      ...baseCopy.headings,
      setup: typeof projects.title === "string" ? projects.title : baseCopy.headings.setup,
      release: typeof deploy.title === "string" ? deploy.title : baseCopy.headings.release,
    },
    completion: `${label} starter language pack loaded.`,
    ready: "Starter coverage is active. Restart already-open desktop shells if a long-running view does not refresh.",
  };
}

export function createInstallerLanguagePackSupport({ installerShellCopy, normalizeDesktopLanguage }) {
  const lazyInstallerShellCopy = {};

  async function ensureInstallerLanguagePack(language) {
    const normalized = normalizeDesktopLanguage(language);
    if (installerShellCopy[normalized]) {
      return { status: "builtin", message: `${describeDesktopLanguage(normalized)} is built in.` };
    }
    const result = await loadDesktopLanguagePack("hub", normalized);
    if (result.status === "loaded" && result.pack) {
      lazyInstallerShellCopy[normalized] = installerShellCopyFromHubPack(result.pack, installerShellCopy.en);
    }
    return result;
  }

  function lazyShellCopyFor(language) {
    return lazyInstallerShellCopy[normalizeDesktopLanguage(language)] || null;
  }

  return { ensureInstallerLanguagePack, lazyShellCopyFor };
}
