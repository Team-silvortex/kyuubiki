const loaders = {
  "ar": () => import("./installer-shell-locales/ar.js"),
  "bn": () => import("./installer-shell-locales/bn.js"),
  "cs": () => import("./installer-shell-locales/cs.js"),
  "da": () => import("./installer-shell-locales/da.js"),
  "de": () => import("./installer-shell-locales/de.js"),
  "el": () => import("./installer-shell-locales/el.js"),
  "fa": () => import("./installer-shell-locales/fa.js"),
  "fi": () => import("./installer-shell-locales/fi.js"),
  "fr": () => import("./installer-shell-locales/fr.js"),
  "he": () => import("./installer-shell-locales/he.js"),
  "hi": () => import("./installer-shell-locales/hi.js"),
  "id": () => import("./installer-shell-locales/id.js"),
  "it": () => import("./installer-shell-locales/it.js"),
  "ko": () => import("./installer-shell-locales/ko.js"),
  "ms": () => import("./installer-shell-locales/ms.js"),
  "nl": () => import("./installer-shell-locales/nl.js"),
  "no": () => import("./installer-shell-locales/no.js"),
  "pl": () => import("./installer-shell-locales/pl.js"),
  "pt-BR": () => import("./installer-shell-locales/pt-br.js"),
  "ro": () => import("./installer-shell-locales/ro.js"),
  "ru": () => import("./installer-shell-locales/ru.js"),
  "sv": () => import("./installer-shell-locales/sv.js"),
  "sw": () => import("./installer-shell-locales/sw.js"),
  "ta": () => import("./installer-shell-locales/ta.js"),
  "th": () => import("./installer-shell-locales/th.js"),
  "tr": () => import("./installer-shell-locales/tr.js"),
  "uk": () => import("./installer-shell-locales/uk.js"),
  "ur": () => import("./installer-shell-locales/ur.js"),
  "vi": () => import("./installer-shell-locales/vi.js"),
  "zh-TW": () => import("./installer-shell-locales/zh-tw.js"),
};

export async function loadInstallerShellTranslation(language) {
  const load = loaders[language];
  if (!load) return null;
  return (await load()).default;
}
