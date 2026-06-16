"use client";

import { useEffect, useState } from "react";
import type { WorkbenchScriptPanelCopyEntry } from "@/components/workbench/workbench-script-panel-copy";
import { parseWorkbenchScriptLayoutReportSummary } from "@/components/workbench/workbench-script-layout-report";
import type { WorkbenchScriptActionLogEntry, WorkbenchScriptSnapshot } from "@/lib/scripting/workbench-script-runtime";

type InspectMode = "output" | "timeline" | "snapshot";

type WorkbenchScriptInspectPanelProps = {
  actionCatalogCount: number;
  actionLog: WorkbenchScriptActionLogEntry[];
  continueTimelineFromEntry: (entry: WorkbenchScriptActionLogEntry) => void;
  copy: WorkbenchScriptPanelCopyEntry;
  insertTimelineStep: (entry: WorkbenchScriptActionLogEntry) => void;
  insertTimelineMacroDraft: (entry: WorkbenchScriptActionLogEntry, includedEntryIds?: string[]) => void;
  sendTimelineMacroToHeadless: (entry: WorkbenchScriptActionLogEntry, includedEntryIds?: string[]) => void;
  saveTimelineMacroPreset: (entry: WorkbenchScriptActionLogEntry, includedEntryIds?: string[]) => void;
  macroCatalogCount: number;
  output: string[];
  scriptCode: string;
  snapshot: WorkbenchScriptSnapshot;
};

export function WorkbenchScriptInspectPanel({
  actionCatalogCount,
  actionLog,
  continueTimelineFromEntry,
  copy,
  insertTimelineStep,
  insertTimelineMacroDraft,
  sendTimelineMacroToHeadless,
  saveTimelineMacroPreset,
  macroCatalogCount,
  output,
  scriptCode,
  snapshot,
}: WorkbenchScriptInspectPanelProps) {
  const [mode, setMode] = useState<InspectMode>("output");
  const [selectedActionId, setSelectedActionId] = useState<string | null>(null);
  const [includedEntryIds, setIncludedEntryIds] = useState<string[]>([]);
  const selectedEntry = actionLog.find((entry) => entry.id === selectedActionId) ?? actionLog[0] ?? null;
  const selectedEntryIndex = selectedEntry ? actionLog.findIndex((entry) => entry.id === selectedEntry.id) : -1;
  const selectedTimelineRange = selectedEntryIndex >= 0 ? actionLog.slice(0, selectedEntryIndex + 1).reverse() : [];
  const selectableMacroSteps = selectedTimelineRange.filter(
    (entry) =>
      entry.action !== "macro/run" &&
      ((entry.source === "manual" && entry.status === "completed") || entry.status === "started"),
  );
  const selectedMacroSteps = selectableMacroSteps.filter((entry) => includedEntryIds.includes(entry.id));
  const layoutReportSummary = parseWorkbenchScriptLayoutReportSummary(output);

  useEffect(() => {
    setIncludedEntryIds(selectableMacroSteps.map((entry) => entry.id));
  }, [selectedActionId, actionLog.length]);

  const statusLabel = (status: WorkbenchScriptActionLogEntry["status"]) =>
    status === "failed" ? copy.timelineFailed : status === "completed" ? copy.timelineCompleted : copy.timelineStarted;
  const failureReason = (entry: WorkbenchScriptActionLogEntry) => {
    if (entry.status !== "failed") return null;
    if (entry.note && entry.note.trim()) return entry.note;
    if (entry.result) return JSON.stringify(entry.result);
    if (entry.payload) return JSON.stringify(entry.payload);
    return entry.summary;
  };
  const includeAllSteps = () => {
    setIncludedEntryIds(selectableMacroSteps.map((entry) => entry.id));
  };
  const clearIncludedSteps = () => {
    setIncludedEntryIds([]);
  };
  const includeFailureWindow = () => {
    const failureIndices = selectedTimelineRange
      .map((entry, index) => (entry.status === "failed" ? index : -1))
      .filter((index) => index >= 0);
    if (failureIndices.length === 0) {
      setIncludedEntryIds([]);
      return;
    }
    const nextIds = new Set<string>();
    for (const index of failureIndices) {
      for (const neighborIndex of [index - 1, index, index + 1]) {
        const candidate = selectedTimelineRange[neighborIndex];
        if (!candidate) continue;
        if (!selectableMacroSteps.some((entry) => entry.id === candidate.id)) continue;
        nextIds.add(candidate.id);
      }
    }
    setIncludedEntryIds([...nextIds]);
  };
  const toggleIncludedStep = (entryId: string) => {
    setIncludedEntryIds((current) => (current.includes(entryId) ? current.filter((id) => id !== entryId) : [...current, entryId]));
  };

  return (
    <section className="sidebar-card sidebar-card--compact">
      <div className="card-head">
        <h2>{copy.inspect}</h2>
        <span>{mode === "output" ? copy.output : mode === "timeline" ? copy.timelineMode : copy.snapshot}</span>
      </div>
      <div className="panel-tabs panel-tabs--wide">
        <button className={`panel-tab${mode === "output" ? " panel-tab--active" : ""}`} onClick={() => setMode("output")} type="button">
          {copy.outputMode}
        </button>
        <button className={`panel-tab${mode === "timeline" ? " panel-tab--active" : ""}`} onClick={() => setMode("timeline")} type="button">
          {copy.timelineMode}
        </button>
        <button className={`panel-tab${mode === "snapshot" ? " panel-tab--active" : ""}`} onClick={() => setMode("snapshot")} type="button">
          {copy.snapshotMode}
        </button>
      </div>
      <div className="sidebar-list">
        <div>
          <span>{copy.stateKeys}</span>
          <strong>{Object.keys(snapshot).length}</strong>
        </div>
        <div>
          <span>{copy.actionCount}</span>
          <strong>{actionCatalogCount + macroCatalogCount}</strong>
        </div>
        <div>
          <span>{copy.lineCount}</span>
          <strong>{scriptCode.split("\n").length}</strong>
        </div>
      </div>
      {mode === "output" ? (
        <>
          <p className="card-copy">{copy.inspectOutputHint}</p>
          {layoutReportSummary ? (
            <section className="sidebar-card sidebar-card--compact" style={{ marginBottom: "0.75rem" }}>
              <div className="card-head">
                <h2>{snapshot.language === "zh" ? "布局摘要" : snapshot.language === "ja" ? "レイアウト要約" : "Layout Summary"}</h2>
                <span>{layoutReportSummary.status}</span>
              </div>
              <div className="sidebar-list sidebar-list--metrics">
                <div className="sidebar-list__row"><span>reported at</span><strong>{layoutReportSummary.reportedAt ?? "--"}</strong></div>
                <div className="sidebar-list__row"><span>failure code</span><strong>{layoutReportSummary.failureCode ?? "--"}</strong></div>
                <div className="sidebar-list__row"><span>failure</span><strong>{layoutReportSummary.failureReason ?? "--"}</strong></div>
                <div className="sidebar-list__row"><span>recovery</span><strong>{layoutReportSummary.recoverySuggestion ?? "--"}</strong></div>
                <div className="sidebar-list__row"><span>anchors</span><strong>{layoutReportSummary.anchors ?? "--"}</strong></div>
                <div className="sidebar-list__row"><span>active sidebar</span><strong>{layoutReportSummary.activeSidebar ?? "--"}</strong></div>
                <div className="sidebar-list__row"><span>runtime tabs</span><strong>{layoutReportSummary.runtimeTabCount ?? "--"}</strong></div>
                <div className="sidebar-list__row"><span>immersive</span><strong>{layoutReportSummary.immersiveMode ?? "--"}</strong></div>
                <div className="sidebar-list__row"><span>overview tab</span><strong>{layoutReportSummary.overviewTabLabel ?? "--"}</strong></div>
                <div className="sidebar-list__row"><span>3d selection</span><strong>{layoutReportSummary.selectedTruss3dNodes ?? "--"}</strong></div>
              </div>
            </section>
          ) : null}
          {output.length === 0 ? <p className="card-copy">{copy.noOutput}</p> : <pre className="script-panel__output">{output.join("\n")}</pre>}
        </>
      ) : null}
      {mode === "timeline" ? (
        <>
          <p className="card-copy">{copy.inspectTimelineHint}</p>
          {actionLog.length === 0 ? (
            <p className="card-copy">{copy.noActionLog}</p>
          ) : (
            <>
              <div className="script-panel__catalog">
                {actionLog.map((entry, index) => (
                  <article
                    className={`script-panel__action${entry.status === "failed" ? " script-panel__action--failed" : ""}${selectedEntry?.id === entry.id ? " script-panel__action--selected" : ""}`}
                    key={entry.id}
                  >
                    <div className="script-panel__action-head">
                      <strong>{`${index + 1}. ${entry.action}`}</strong>
                      <span>{statusLabel(entry.status)}</span>
                    </div>
                    <p className="card-copy">{entry.summary}</p>
                    <div className="script-panel__payload">
                      <span>{copy.actionSource}</span>
                      <code>{entry.source ?? "--"}</code>
                    </div>
                    {entry.status === "failed" ? (
                      <div className="script-panel__payload">
                        <span>{copy.failureReason}</span>
                        <code>{failureReason(entry) ?? "--"}</code>
                      </div>
                    ) : null}
                    <div className="script-panel__payload">
                      <span>Time</span>
                      <code>{entry.at}</code>
                    </div>
                    <div className="button-row">
                      <button className="ghost-button ghost-button--compact" onClick={() => setSelectedActionId(entry.id)} type="button">
                        {copy.inspect}
                      </button>
                      <button className="ghost-button ghost-button--compact" onClick={() => insertTimelineStep(entry)} type="button">
                        {copy.insertActionStep}
                      </button>
                    </div>
                  </article>
                ))}
              </div>
              <p className="card-copy">{copy.inspectTimelineDetailHint}</p>
              {selectedEntry ? (
                <article className={`script-panel__action${selectedEntry.status === "failed" ? " script-panel__action--failed" : ""}`}>
                  <div className="script-panel__action-head">
                    <strong>{selectedEntry.action}</strong>
                    <span>{statusLabel(selectedEntry.status)}</span>
                  </div>
                  <p className="card-copy">{selectedEntry.summary}</p>
                  <div className="script-panel__payload">
                    <span>{copy.actionSource}</span>
                    <code>{selectedEntry.source ?? "--"}</code>
                  </div>
                  <div className="script-panel__payload">
                    <span>{copy.actionPayload}</span>
                    <code>{selectedEntry.payload ? JSON.stringify(selectedEntry.payload) : "--"}</code>
                  </div>
                  <div className="script-panel__payload">
                    <span>{copy.actionResult}</span>
                    <code>{selectedEntry.result ? JSON.stringify(selectedEntry.result) : "--"}</code>
                  </div>
                  <div className="script-panel__payload">
                    <span>{copy.actionNote}</span>
                    <code>{selectedEntry.note ?? "--"}</code>
                  </div>
                  {selectedEntry.status === "failed" ? (
                    <div className="script-panel__payload">
                      <span>{copy.failureReason}</span>
                      <code>{failureReason(selectedEntry) ?? "--"}</code>
                    </div>
                  ) : null}
                  <div className="script-panel__payload">
                    <span>Time</span>
                    <code>{selectedEntry.at}</code>
                  </div>
                  <div className="card-subhead">
                    <strong>{copy.timelineRangePreview}</strong>
                    <span>{`${copy.timelinePreviewSteps}: ${selectedTimelineRange.length} / ${copy.timelinePreviewMacroSteps}: ${selectedMacroSteps.length}`}</span>
                  </div>
                  <div className="button-row">
                    <button className="ghost-button ghost-button--compact" onClick={includeAllSteps} type="button">
                      {copy.selectAllTimelineSteps}
                    </button>
                    <button className="ghost-button ghost-button--compact" onClick={clearIncludedSteps} type="button">
                      {copy.clearTimelineSteps}
                    </button>
                    <button className="ghost-button ghost-button--compact" onClick={includeFailureWindow} type="button">
                      {copy.focusFailureWindow}
                    </button>
                  </div>
                  {selectedTimelineRange.length > 0 ? (
                    <div className="script-panel__catalog">
                      {selectedTimelineRange.map((entry, index) => (
                        <article className="script-panel__action" key={`preview-${entry.id}`}>
                          <div className="script-panel__action-head">
                            <strong>{`${index + 1}. ${entry.action}`}</strong>
                            <span>{statusLabel(entry.status)}</span>
                          </div>
                          <div className="script-panel__payload">
                            <span>{copy.actionSource}</span>
                            <code>{entry.source ?? "--"}</code>
                          </div>
                          <div className="script-panel__payload">
                            <span>{copy.timelinePreviewIncluded}</span>
                            <code>
                              {selectableMacroSteps.some((candidate) => candidate.id === entry.id)
                                ? selectedMacroSteps.some((candidate) => candidate.id === entry.id)
                                  ? copy.timelinePreviewIncludedYes
                                  : copy.timelinePreviewIncludedNo
                                : copy.timelinePreviewNotEligible}
                            </code>
                          </div>
                          {selectableMacroSteps.some((candidate) => candidate.id === entry.id) ? (
                            <div className="button-row">
                              <button className="ghost-button ghost-button--compact" onClick={() => toggleIncludedStep(entry.id)} type="button">
                                {selectedMacroSteps.some((candidate) => candidate.id === entry.id) ? copy.excludeTimelineStep : copy.includeTimelineStep}
                              </button>
                            </div>
                          ) : null}
                        </article>
                      ))}
                    </div>
                  ) : null}
                  <div className="button-row">
                    <button className="ghost-button ghost-button--compact" onClick={() => insertTimelineMacroDraft(selectedEntry, includedEntryIds)} type="button">
                      {copy.insertTimelineMacroDraft}
                    </button>
                    <button className="ghost-button ghost-button--compact" onClick={() => sendTimelineMacroToHeadless(selectedEntry, includedEntryIds)} type="button">
                      {copy.sendTimelineMacroToHeadless}
                    </button>
                    <button className="ghost-button ghost-button--compact" onClick={() => saveTimelineMacroPreset(selectedEntry, includedEntryIds)} type="button">
                      {copy.saveTimelineMacroPreset}
                    </button>
                    <button className="ghost-button ghost-button--compact" onClick={() => continueTimelineFromEntry(selectedEntry)} type="button">
                      {copy.continueFromStep}
                    </button>
                  </div>
                </article>
              ) : (
                <p className="card-copy">{copy.noTimelineSelection}</p>
              )}
            </>
          )}
        </>
      ) : null}
      {mode === "snapshot" ? (
        <>
          <p className="card-copy">{copy.inspectSnapshotHint}</p>
          <pre className="script-panel__snapshot">{JSON.stringify(snapshot, null, 2)}</pre>
        </>
      ) : null}
    </section>
  );
}
