"use client";

import { useMemo, useState } from "react";
import type { FrontendMacroAssetRecord } from "@/components/workbench/workbench-headless-workflow-panel";
import type { WorkbenchScriptPanelCopyEntry } from "@/components/workbench/workbench-script-panel-copy";

type WorkbenchScriptAuthorPanelProps = {
  copy: WorkbenchScriptPanelCopyEntry;
  deriveFrontendMacroAsset: (asset: FrontendMacroAssetRecord) => void;
  exportMacroDraftJson: () => void;
  frontendMacroAssets: FrontendMacroAssetRecord[];
  importMacroJson: (file: File | undefined) => Promise<void>;
  insertFrontendMacroAsset: (asset: FrontendMacroAssetRecord) => void;
  insertMacroDraftFromLog: () => void;
  recordingMode: boolean;
  scriptCode: string;
  setScriptCode: (value: string) => void;
};

type AuthorMode = "script" | "record";

function describeAssetSource(asset: FrontendMacroAssetRecord, copy: WorkbenchScriptPanelCopyEntry) {
  if (asset.source === "bridge_restore") return copy.assetSourceBridge;
  if (asset.source === "snapshot_derived") return copy.assetSourceDerived;
  return copy.assetSourceTimeline;
}

function stablePayload(value: Record<string, unknown> | undefined) {
  return JSON.stringify(value ?? {}, null, 2);
}

export function WorkbenchScriptAuthorPanel({
  copy,
  deriveFrontendMacroAsset,
  exportMacroDraftJson,
  frontendMacroAssets,
  importMacroJson,
  insertFrontendMacroAsset,
  insertMacroDraftFromLog,
  recordingMode,
  scriptCode,
  setScriptCode,
}: WorkbenchScriptAuthorPanelProps) {
  const [mode, setMode] = useState<AuthorMode>("script");
  const [compareLeftId, setCompareLeftId] = useState<string | null>(null);
  const [compareRightId, setCompareRightId] = useState<string | null>(null);
  const compareLeftAsset = frontendMacroAssets.find((asset) => asset.assetId === compareLeftId) ?? null;
  const compareRightAsset = frontendMacroAssets.find((asset) => asset.assetId === compareRightId) ?? null;
  const comparisonRows = useMemo(() => {
    if (!compareLeftAsset || !compareRightAsset) return [];
    const maxLength = Math.max(compareLeftAsset.draft.steps.length, compareRightAsset.draft.steps.length);
    return Array.from({ length: maxLength }, (_, index) => {
      const leftStep = compareLeftAsset.draft.steps[index];
      const rightStep = compareRightAsset.draft.steps[index];
      const leftAction = leftStep?.action ?? null;
      const rightAction = rightStep?.action ?? null;
      const leftPayload = stablePayload(leftStep?.payload);
      const rightPayload = stablePayload(rightStep?.payload);
      const actionChanged = leftAction !== rightAction;
      const payloadChanged = leftPayload !== rightPayload;
      const status = !leftStep || !rightStep ? "missing" : actionChanged || payloadChanged ? "changed" : "same";
      return {
        index,
        leftAction,
        leftPayload,
        payloadChanged,
        rightAction,
        rightPayload,
        status,
      };
    });
  }, [compareLeftAsset, compareRightAsset]);
  const changedStepCount = comparisonRows.filter((row) => row.status !== "same").length;

  return (
    <section className="sidebar-card sidebar-card--compact">
      <div className="card-head">
        <h2>{copy.author}</h2>
        <span>{mode === "script" ? "Pyodide" : copy.recordMode}</span>
      </div>
      <div className="panel-tabs panel-tabs--wide">
        <button className={`panel-tab${mode === "script" ? " panel-tab--active" : ""}`} onClick={() => setMode("script")} type="button">
          {copy.scriptMode}
        </button>
        <button className={`panel-tab${mode === "record" ? " panel-tab--active" : ""}`} onClick={() => setMode("record")} type="button">
          {copy.recordMode}
        </button>
      </div>
      {mode === "script" ? (
        <>
          <p className="card-copy">{copy.authorScriptHint}</p>
          <textarea
            className="script-panel__editor"
            rows={18}
            spellCheck={false}
            value={scriptCode}
            onChange={(event) => setScriptCode(event.target.value)}
          />
        </>
      ) : (
        <>
          <p className="card-copy">{copy.authorRecordHint}</p>
          <div className="button-row">
            <button className="ghost-button" onClick={insertMacroDraftFromLog} type="button">
              {copy.insertMacroDraft}
            </button>
            <button className="ghost-button" onClick={exportMacroDraftJson} type="button">
              {copy.exportMacroJson}
            </button>
            <label className="ghost-button" style={{ display: "inline-flex", alignItems: "center", cursor: "pointer" }}>
              {copy.importMacroJson}
              <input
                accept="application/json,.json"
                hidden
                onChange={(event) => {
                  void importMacroJson(event.target.files?.[0]);
                  event.currentTarget.value = "";
                }}
                type="file"
              />
            </label>
          </div>
          {recordingMode ? <p className="card-copy">{copy.recordingActive}</p> : null}
          <div className="card-subhead">
            <strong>{copy.frontendMacroAssets}</strong>
            <span>{frontendMacroAssets.length}</span>
          </div>
          {frontendMacroAssets.length === 0 ? (
            <p className="card-copy">{copy.frontendMacroAssetsEmpty}</p>
          ) : (
            <div className="script-panel__catalog">
              {frontendMacroAssets.map((asset) => (
                <article
                  className={`script-panel__action${asset.assetId === compareLeftId || asset.assetId === compareRightId ? " script-panel__action--selected" : ""}`}
                  key={asset.assetId}
                >
                  <div className="script-panel__action-head">
                    <strong>{asset.draft.id}</strong>
                    <span>{`${copy.stepsLabel}: ${asset.draft.steps.length}`}</span>
                  </div>
                  <div className="script-panel__payload">
                    <span>{copy.assetSnapshotId}</span>
                    <code>{asset.assetId}</code>
                  </div>
                  <div className="script-panel__payload">
                    <span>{copy.assetSource}</span>
                    <code>{describeAssetSource(asset, copy)}</code>
                  </div>
                  <div className="script-panel__payload">
                    <span>{copy.assetUpdatedAt}</span>
                    <code>{asset.updatedAt}</code>
                  </div>
                  <div className="button-row">
                    <button className="ghost-button ghost-button--compact" onClick={() => setCompareLeftId(asset.assetId)} type="button">
                      {copy.compareLeftAsset}
                    </button>
                    <button className="ghost-button ghost-button--compact" onClick={() => setCompareRightId(asset.assetId)} type="button">
                      {copy.compareRightAsset}
                    </button>
                  </div>
                  <div className="button-row">
                    <button className="ghost-button ghost-button--compact" onClick={() => deriveFrontendMacroAsset(asset)} type="button">
                      {copy.deriveFrontendMacroAsset}
                    </button>
                    <button className="ghost-button ghost-button--compact" onClick={() => insertFrontendMacroAsset(asset)} type="button">
                      {copy.restoreBridgeMacroToFrontend}
                    </button>
                  </div>
                </article>
              ))}
            </div>
          )}
          <div className="card-subhead">
            <strong>{copy.snapshotCompare}</strong>
            <span>{comparisonRows.length > 0 ? changedStepCount : 0}</span>
          </div>
          {!compareLeftAsset || !compareRightAsset ? (
            <p className="card-copy">{copy.snapshotCompareEmpty}</p>
          ) : (
            <div className="stack-block">
              <div className="button-row button-row--adaptive">
                <article className="script-panel__action">
                  <div className="script-panel__action-head">
                    <strong>{copy.compareLeftAsset}</strong>
                    <span>{`${copy.stepsLabel}: ${compareLeftAsset.draft.steps.length}`}</span>
                  </div>
                  <div className="script-panel__payload">
                    <span>{copy.assetSnapshotId}</span>
                    <code>{compareLeftAsset.assetId}</code>
                  </div>
                  <div className="script-panel__payload">
                    <span>{copy.assetSource}</span>
                    <code>{describeAssetSource(compareLeftAsset, copy)}</code>
                  </div>
                </article>
                <article className="script-panel__action">
                  <div className="script-panel__action-head">
                    <strong>{copy.compareRightAsset}</strong>
                    <span>{`${copy.stepsLabel}: ${compareRightAsset.draft.steps.length}`}</span>
                  </div>
                  <div className="script-panel__payload">
                    <span>{copy.assetSnapshotId}</span>
                    <code>{compareRightAsset.assetId}</code>
                  </div>
                  <div className="script-panel__payload">
                    <span>{copy.assetSource}</span>
                    <code>{describeAssetSource(compareRightAsset, copy)}</code>
                  </div>
                </article>
              </div>
              <p className="card-copy">{`${copy.snapshotCompareChangedSteps}: ${changedStepCount}`}</p>
              <div className="button-row">
                <button className="ghost-button ghost-button--compact" onClick={() => setCompareLeftId(null)} type="button">
                  {copy.clearCompareLeft}
                </button>
                <button className="ghost-button ghost-button--compact" onClick={() => setCompareRightId(null)} type="button">
                  {copy.clearCompareRight}
                </button>
              </div>
              <div className="script-panel__catalog">
                {comparisonRows.map((row) => (
                  <article
                    className={`script-panel__action${row.status === "changed" ? " script-panel__action--selected" : row.status === "missing" ? " script-panel__action--failed" : ""}`}
                    key={`${compareLeftAsset.assetId}-${compareRightAsset.assetId}-${row.index}`}
                  >
                    <div className="script-panel__action-head">
                      <strong>{`${copy.stepsLabel} ${row.index + 1}`}</strong>
                      <span>
                        {row.status === "same" ? copy.snapshotCompareSame : row.status === "missing" ? copy.snapshotCompareMissing : copy.snapshotCompareChanged}
                      </span>
                    </div>
                    <div className="button-row button-row--adaptive" style={{ alignItems: "stretch" }}>
                      <div className="stack-block">
                        <div className="script-panel__payload">
                          <span>{copy.compareLeftAsset}</span>
                          <code>{row.leftAction ?? copy.snapshotCompareStepMissing}</code>
                        </div>
                        <div className="script-panel__payload">
                          <span>{copy.payload}</span>
                          <code>{row.leftPayload}</code>
                        </div>
                      </div>
                      <div className="stack-block">
                        <div className="script-panel__payload">
                          <span>{copy.compareRightAsset}</span>
                          <code>{row.rightAction ?? copy.snapshotCompareStepMissing}</code>
                        </div>
                        <div className="script-panel__payload">
                          <span>{copy.payload}</span>
                          <code>{row.rightPayload}</code>
                        </div>
                      </div>
                    </div>
                    {row.status === "changed" ? (
                      <p className="card-copy">{row.payloadChanged ? copy.snapshotComparePayloadChanged : copy.snapshotCompareActionChanged}</p>
                    ) : null}
                  </article>
                ))}
              </div>
            </div>
          )}
        </>
      )}
    </section>
  );
}
