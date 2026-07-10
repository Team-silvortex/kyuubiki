"use client";

import type {
  DraftStep,
  HeadlessActionContract,
  PayloadObject,
} from "@/components/workbench/workbench-headless-workflow-contract";
import { parsePayloadText } from "@/components/workbench/workbench-headless-workflow-contract";
import { buildHeadlessWorkflowPanelCopy } from "@/components/workbench/workbench-headless-workflow-panel-helpers";
import { parseFrontendMacroBridgePayload } from "@/components/workbench/workbench-headless-workflow-panel-state";
import { buildReferenceTokens, localizeWorkflowText } from "@/components/workbench/workbench-headless-workflow-registry";
import { WorkbenchHeadlessWorkflowStepEditor } from "@/components/workbench/workbench-headless-workflow-step-editor";
import type { WorkbenchRecordedMacroDraft, WorkbenchScriptLanguage } from "@/lib/scripting/workbench-script-runtime";

type HeadlessPanelCopy = ReturnType<typeof buildHeadlessWorkflowPanelCopy>;

export function WorkbenchHeadlessWorkflowStepList({
  actionMap,
  language,
  onMoveStep,
  onPayloadTextChange,
  onRemoveStep,
  onRestoreFrontendMacro,
  patchStepPayload,
  steps,
  ui,
}: {
  actionMap: Map<string, HeadlessActionContract>;
  language: WorkbenchScriptLanguage;
  onMoveStep(index: number, direction: -1 | 1): void;
  onPayloadTextChange(stepId: string, payloadText: string): void;
  onRemoveStep(stepId: string): void;
  onRestoreFrontendMacro(draft: WorkbenchRecordedMacroDraft): void;
  patchStepPayload(stepId: string, updater: (payload: PayloadObject | null) => PayloadObject | null): void;
  steps: DraftStep[];
  ui: HeadlessPanelCopy;
}) {
  return (
    <div className="script-panel__catalog">
      {steps.map((step, index) => {
        const descriptor = actionMap.get(step.action);
        const references = buildReferenceTokens(steps, index, actionMap);
        const riskLabel =
          descriptor?.risk === "destructive" ? ui.destructive : descriptor?.risk === "sensitive" ? ui.sensitive : ui.normal;

        return (
          <article className="script-panel__action" key={step.id}>
            <div className="script-panel__action-head">
              <strong>{`${index + 1}. ${step.action}`}</strong>
              <span>{riskLabel}</span>
            </div>
            {descriptor ? <p className="card-copy">{localizeWorkflowText(language, descriptor.summary)}</p> : null}
            <WorkbenchHeadlessWorkflowStepEditor
              bridgeActionListLabel={ui.bridgeActionList}
              bridgeMacroIdLabel={ui.bridgeMacroId}
              bridgePreviewHideLabel={ui.bridgePreviewHide}
              bridgePreviewPayloadLabel={ui.bridgePreviewPayload}
              bridgePreviewShowLabel={ui.bridgePreviewShow}
              bridgeReplayModeHint={ui.bridgeReplayModeHint}
              bridgeReplayModeLabel={ui.bridgeReplayMode}
              bridgeRestoreLabel={ui.bridgeRestore}
              bridgeStepCountLabel={ui.bridgeSteps}
              endpointsHint={ui.endpointsHint}
              endpointsLabel={ui.endpoints}
              contract={descriptor}
              guidanceTitle={ui.guidanceTitle}
              language={language}
              noReferencesLabel={ui.noReferences}
              onRestoreBridgeMacro={
                step.action === "frontend_macro_bridge"
                  ? () => {
                      const restored = parseFrontendMacroBridgePayload(parsePayloadText(step.payloadText));
                      if (restored) onRestoreFrontendMacro(restored);
                    }
                  : undefined
              }
              parsePayloadText={parsePayloadText}
              patchStepPayload={patchStepPayload}
              referenceApplyLabel={ui.referenceApply}
              referenceClearLabel={ui.referenceClear}
              referenceCurrentLabel={ui.referenceCurrent}
              references={references}
              referenceTitle={ui.referenceTitle}
              step={step}
            />

            {step.action === "frontend_macro_bridge" ? (
              <label className="field-label">
                <span>{ui.rawPayloadJson}</span>
                <pre className="script-panel__snapshot">{step.payloadText}</pre>
              </label>
            ) : (
              <label className="field-label">
                <span>{ui.payloadJson}</span>
                <textarea
                  className="script-panel__editor"
                  onChange={(event) => onPayloadTextChange(step.id, event.target.value)}
                  rows={6}
                  spellCheck={false}
                  value={step.payloadText}
                />
              </label>
            )}

            <div className="button-row">
              <button className="ghost-button ghost-button--compact" onClick={() => onMoveStep(index, -1)} type="button">
                {ui.moveUp}
              </button>
              <button className="ghost-button ghost-button--compact" onClick={() => onMoveStep(index, 1)} type="button">
                {ui.moveDown}
              </button>
              <button className="ghost-button ghost-button--compact" onClick={() => onRemoveStep(step.id)} type="button">
                {ui.remove}
              </button>
            </div>
          </article>
        );
      })}
    </div>
  );
}
