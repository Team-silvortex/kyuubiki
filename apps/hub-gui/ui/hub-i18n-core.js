import { applyHubDocsI18n } from "./hub-i18n-docs.js";
import { applyHubGuidesI18n } from "./hub-i18n-guides.js";
import { applyHubLocalizationI18n } from "./hub-i18n-localization.js";
import { applyHubAssistantI18n } from "./hub-i18n-assistant.js";
import { applyHubWorkloadsI18n } from "./hub-i18n-workloads.js";
import { HUB_I18N_EN } from "./hub-i18n-en.js";
import { createHubI18nEs } from "./hub-i18n-es.js";
import { HUB_I18N_JA } from "./hub-i18n-ja.js";
import { HUB_I18N_ZH } from "./hub-i18n-zh.js";

export const HUB_I18N = {
  en: HUB_I18N_EN,
  zh: HUB_I18N_ZH,
  ja: HUB_I18N_JA,
};

HUB_I18N.es = createHubI18nEs(HUB_I18N.en);

applyHubDocsI18n(HUB_I18N);
applyHubGuidesI18n(HUB_I18N);
applyHubLocalizationI18n(HUB_I18N);
applyHubAssistantI18n(HUB_I18N);
applyHubWorkloadsI18n(HUB_I18N);
