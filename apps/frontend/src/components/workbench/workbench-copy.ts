import { copyEnCore } from "@/components/workbench/workbench-copy-en-core";
import { copyEnExtended } from "@/components/workbench/workbench-copy-en-extended";
import { copyJa } from "@/components/workbench/workbench-copy-ja";
import { copyZhCore } from "@/components/workbench/workbench-copy-zh-core";
import { copyZhExtended } from "@/components/workbench/workbench-copy-zh-extended";

export type WorkbenchLanguage = "en" | "zh" | "ja" | "es";

type WidenLiteral<T> =
  T extends string ? string
  : T extends number ? number
  : T extends boolean ? boolean
  : T extends readonly (infer U)[] ? readonly WidenLiteral<U>[]
  : T extends object ? { [K in keyof T]: WidenLiteral<T[K]> }
  : T;

const copyEn = {
  ...copyEnCore,
  ...copyEnExtended,
} as const;

const copyZh = {
  ...copyZhCore,
  ...copyZhExtended,
} as const;

export type WorkbenchCopy = WidenLiteral<typeof copyEn>;

export const copyByLanguage: Record<WorkbenchLanguage, WorkbenchCopy> = {
  en: copyEn,
  zh: copyZh,
  ja: copyJa,
  es: {
    ...copyEn,
    language: "Idioma",
    languages: {
      ...copyEn.languages,
      es: "Español",
    },
  },
};

export function humanizeSolverFailure(message: string | null | undefined, languageCopy: WorkbenchCopy) {
  if (!message) return null;

  if (message.includes("watchdog marked job stalled")) {
    return languageCopy.translatedWatchdogStalled;
  }

  if (message.includes("watchdog timed out job")) {
    return languageCopy.translatedWatchdogTimedOut;
  }

  if (message.includes("job execution timed out")) {
    return languageCopy.translatedExecutionTimedOut;
  }

  if (message.includes("small-deformation limit")) {
    return `${languageCopy.translatedSmallDeformation} ${languageCopy.translatedConnectivity}`;
  }

  if (message.includes("system is singular")) {
    return `${languageCopy.translatedSingular} ${languageCopy.translatedConnectivity}`;
  }

  if (message.includes("supports or connectivity")) {
    return languageCopy.translatedConnectivity;
  }

  if (message.includes(":timeout") || message.includes("all_agents_failed")) {
    return languageCopy.translatedAgentTimeout;
  }

  return message;
}
