"use client";

import type { PayloadObject } from "@/components/workbench/workbench-headless-workflow-contract";
import { buildHeadlessWorkflowPanelCopy } from "@/components/workbench/workbench-headless-workflow-panel-helpers";
import type { FrontendMacroAssetRecord } from "@/components/workbench/workbench-headless-workflow-panel-state";
import {
  HEADLESS_ACTIONS,
  HEADLESS_WORKFLOW_TEMPLATES,
  localizeWorkflowText,
} from "@/components/workbench/workbench-headless-workflow-registry";
import type { WorkbenchScriptLanguage } from "@/lib/scripting/workbench-script-runtime";

type HeadlessPanelCopy = ReturnType<typeof buildHeadlessWorkflowPanelCopy>;

export function WorkbenchHeadlessFrontendAssetCatalog({
  assets,
  onDerive,
  onInsertBridge,
  ui,
}: {
  assets: FrontendMacroAssetRecord[];
  onDerive(asset: FrontendMacroAssetRecord): void;
  onInsertBridge(asset: FrontendMacroAssetRecord): void;
  ui: HeadlessPanelCopy;
}) {
  return (
    <>
      <div className="card-subhead">
        <strong>{ui.frontendAssets}</strong>
        <span>{assets.length}</span>
      </div>
      <p className="card-copy">{ui.frontendAssetsHint}</p>
      {assets.length === 0 ? (
        <p className="card-copy">{ui.frontendAssetEmpty}</p>
      ) : (
        <div className="script-panel__catalog">
          {assets.map((asset) => (
            <article className="script-panel__action" key={asset.assetId}>
              <div className="script-panel__action-head">
                <strong>{asset.draft.id}</strong>
                <span>{`${ui.bridgeSteps}: ${asset.draft.steps.length}`}</span>
              </div>
              <div className="script-panel__payload">
                <span>{ui.assetSnapshotId}</span>
                <code>{asset.assetId}</code>
              </div>
              <div className="script-panel__payload">
                <span>{ui.assetSource}</span>
                <code>
                  {asset.source === "bridge_restore"
                    ? ui.assetSourceBridge
                    : asset.source === "snapshot_derived"
                      ? ui.assetSourceDerived
                      : ui.assetSourceTimeline}
                </code>
              </div>
              <div className="script-panel__payload">
                <span>{ui.assetUpdatedAt}</span>
                <code>{asset.updatedAt}</code>
              </div>
              <div className="button-row">
                <button className="ghost-button ghost-button--compact" onClick={() => onDerive(asset)} type="button">
                  {ui.bridgeDerive}
                </button>
                <button className="ghost-button ghost-button--compact" onClick={() => onInsertBridge(asset)} type="button">
                  {ui.bridgeInsert}
                </button>
              </div>
            </article>
          ))}
        </div>
      )}
    </>
  );
}

export function WorkbenchHeadlessTemplateCatalog({
  language,
  onLoadTemplate,
  ui,
}: {
  language: WorkbenchScriptLanguage;
  onLoadTemplate(template: (typeof HEADLESS_WORKFLOW_TEMPLATES)[number]): void;
  ui: HeadlessPanelCopy;
}) {
  return (
    <div className="script-panel__catalog">
      {HEADLESS_WORKFLOW_TEMPLATES.map((template) => (
        <article className="script-panel__action" key={template.id}>
          <div className="script-panel__action-head">
            <strong>{localizeWorkflowText(language, template.title)}</strong>
            <span>{template.steps.length}</span>
          </div>
          <p className="card-copy">{localizeWorkflowText(language, template.description)}</p>
          <div className="button-row">
            <button className="ghost-button ghost-button--compact" onClick={() => onLoadTemplate(template)} type="button">
              {ui.loadTemplate}
            </button>
          </div>
        </article>
      ))}
    </div>
  );
}

export function WorkbenchHeadlessActionButtons({
  onInsertAction,
}: {
  onInsertAction(actionId: string, payload: PayloadObject): void;
}) {
  return (
    <div className="button-row">
      {HEADLESS_ACTIONS.map((action) => (
        <button
          className="ghost-button ghost-button--compact"
          key={action.id}
          onClick={() => onInsertAction(action.id, action.payloadExample)}
          type="button"
        >
          {action.id}
        </button>
      ))}
    </div>
  );
}
