"use client";

import { WorkbenchAssistantPanel } from "./workbench-assistant-panel";
import type { WorkbenchCopy } from "./workbench-copy";

type AssistantTransaction = {
  id: string;
  summary: string;
  createdAt: string;
  executedActions: string[];
};

type AssistantFloatProps = {
  t: Pick<
    WorkbenchCopy,
    | "assistant"
    | "assistantClose"
    | "assistantOpen"
    | "assistantLauncherHint"
    | "assistantFloatingTitle"
    | "assistantFloatingSubtitle"
    | "none"
    | "no"
    | "yes"
    | "frontendModes"
    | "kinds"
  >;
  assistantWindowOpen: boolean;
  setAssistantWindowOpen: (updater: boolean | ((current: boolean) => boolean)) => void;
  jobStatus?: string;
  hasAnyResult: boolean;
  frontendRuntimeMode: keyof WorkbenchCopy["frontendModes"];
  studyKind: keyof WorkbenchCopy["kinds"];
  language: "en" | "zh" | "ja" | "es";
  assistantApiKey: string;
  assistantApiBaseUrl: string;
  assistantModel: string;
  assistantCards: React.ComponentProps<typeof WorkbenchAssistantPanel>["localCards"];
  assistantMode: React.ComponentProps<typeof WorkbenchAssistantPanel>["mode"];
  assistantPromptPresets: React.ComponentProps<typeof WorkbenchAssistantPanel>["promptPresets"];
  assistantTransactions: AssistantTransaction[];
  executeAssistantPlan: (
    actions: Array<{ action: string; payload?: Record<string, unknown>; reason?: string }>,
    summary: string,
    invokeScriptAction: (action: string, payload?: Record<string, unknown>, reason?: string) => Promise<Record<string, unknown>>,
  ) => Promise<unknown>;
  invokeScriptAction: (action: string, payload?: Record<string, unknown>, reason?: string) => Promise<Record<string, unknown>>;
  setAssistantApiKey: (value: string) => void;
  setAssistantApiBaseUrl: (value: string) => void;
  setAssistantModel: (value: string) => void;
  setAssistantMode: (value: React.ComponentProps<typeof WorkbenchAssistantPanel>["mode"]) => void;
  requestLlmAssistantPlan: React.ComponentProps<typeof WorkbenchAssistantPanel>["onRequestPlan"];
  rollbackAssistantTransaction: (id: string) => void;
};

export function WorkbenchAssistantFloat({
  t,
  assistantWindowOpen,
  setAssistantWindowOpen,
  jobStatus,
  hasAnyResult,
  frontendRuntimeMode,
  studyKind,
  language,
  assistantApiKey,
  assistantApiBaseUrl,
  assistantModel,
  assistantCards,
  assistantMode,
  assistantPromptPresets,
  assistantTransactions,
  executeAssistantPlan,
  invokeScriptAction,
  setAssistantApiKey,
  setAssistantApiBaseUrl,
  setAssistantModel,
  setAssistantMode,
  requestLlmAssistantPlan,
  rollbackAssistantTransaction,
}: AssistantFloatProps) {
  return (
    <>
      <button
        aria-expanded={assistantWindowOpen}
        aria-label={assistantWindowOpen ? t.assistantClose : t.assistantOpen}
        className={`assistant-float-launcher${assistantWindowOpen ? " assistant-float-launcher--open" : ""}`}
        onClick={() => setAssistantWindowOpen((current) => !current)}
        type="button"
      >
        <img alt="" className="assistant-float-launcher__mark" src="/kyuubiki.png" />
        <span className="assistant-float-launcher__copy">
          <strong>{t.assistant}</strong>
          <small>{assistantWindowOpen ? t.assistantClose : t.assistantLauncherHint}</small>
        </span>
      </button>

      {assistantWindowOpen ? (
        <aside aria-label={t.assistant} className="assistant-float-panel panel" role="dialog">
          <div className="assistant-float-panel__header">
            <div className="assistant-float-panel__headline">
              <img alt="" className="assistant-float-panel__mark" src="/kyuubiki.png" />
              <div>
                <strong>{t.assistantFloatingTitle}</strong>
                <p>{t.assistantFloatingSubtitle}</p>
              </div>
            </div>
            <button className="ghost-button ghost-button--compact" onClick={() => setAssistantWindowOpen(false)} type="button">
              {t.assistantClose}
            </button>
          </div>
          <div className="assistant-float-panel__body panel-scroll-window">
            <WorkbenchAssistantPanel
              currentJobLabel={jobStatus ?? t.none}
              currentResultLabel={hasAnyResult ? t.yes : t.no}
              currentRuntimeLabel={t.frontendModes[frontendRuntimeMode]}
              currentStudyLabel={t.kinds[studyKind]}
              language={language}
              llmApiKey={assistantApiKey}
              llmBaseUrl={assistantApiBaseUrl}
              llmModel={assistantModel}
              localCards={assistantCards}
              mode={assistantMode}
              promptPresets={assistantPromptPresets}
              transactions={assistantTransactions.map((entry) => ({
                id: entry.id,
                summary: entry.summary,
                createdAt: entry.createdAt,
                executedActions: entry.executedActions,
              }))}
              variant="floating"
              onExecuteLlmAction={async (action, payload, reason) => {
                await executeAssistantPlan([{ action, payload, reason }], reason ?? action, invokeScriptAction);
              }}
              onExecuteLlmPlan={async (actions, summary) => {
                await executeAssistantPlan(actions, summary, invokeScriptAction);
              }}
              onLlmApiKeyChange={setAssistantApiKey}
              onLlmBaseUrlChange={setAssistantApiBaseUrl}
              onLlmModelChange={setAssistantModel}
              onModeChange={setAssistantMode}
              onRequestPlan={requestLlmAssistantPlan}
              onRollbackTransaction={rollbackAssistantTransaction}
            />
          </div>
        </aside>
      ) : null}
    </>
  );
}
