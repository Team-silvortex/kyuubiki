"use client";

import { useState } from "react";
import type { WorkbenchScriptPanelCopyEntry } from "@/components/workbench/workbench-script-panel-copy";
import {
  isWorkbenchScriptActionHighRisk,
  type WorkbenchMacroPresetRecord,
  type WorkbenchScriptActionDefinition,
  type WorkbenchScriptLanguage,
  type WorkbenchScriptMacroDefinition,
} from "@/lib/scripting/workbench-script-runtime";

type CatalogMode = "presets" | "actions" | "macros";

type WorkbenchScriptCatalogPanelProps = {
  actions: WorkbenchScriptActionDefinition[];
  copy: WorkbenchScriptPanelCopyEntry;
  deletePreset: (preset: WorkbenchMacroPresetRecord) => void;
  exportPresetJson: (preset: WorkbenchMacroPresetRecord) => void;
  insertAction: (action: WorkbenchScriptActionDefinition) => void;
  insertMacro: (macroId: string, payload?: Record<string, unknown>) => void;
  insertPreset: (preset: WorkbenchMacroPresetRecord) => void;
  language: WorkbenchScriptLanguage;
  macros: WorkbenchScriptMacroDefinition[];
  presetName: string;
  presetRecords: WorkbenchMacroPresetRecord[];
  saveCurrentPreset: () => void;
  selectedProjectId: string | null;
  setPresetName: (value: string) => void;
};

function stringifyPayload(payload: Record<string, unknown> | undefined): string {
  return payload ? JSON.stringify(payload) : "{}";
}

export function WorkbenchScriptCatalogPanel({
  actions,
  copy,
  deletePreset,
  exportPresetJson,
  insertAction,
  insertMacro,
  insertPreset,
  language,
  macros,
  presetName,
  presetRecords,
  saveCurrentPreset,
  selectedProjectId,
  setPresetName,
}: WorkbenchScriptCatalogPanelProps) {
  const [mode, setMode] = useState<CatalogMode>("presets");

  return (
    <section className="sidebar-card sidebar-card--compact">
      <div className="card-head">
        <h2>{copy.catalog}</h2>
        <span>{mode === "presets" ? presetRecords.length : mode === "actions" ? actions.length : macros.length}</span>
      </div>
      <p className="card-copy">{copy.catalogHint}</p>
      <div className="panel-tabs panel-tabs--wide">
        <button className={`panel-tab${mode === "presets" ? " panel-tab--active" : ""}`} onClick={() => setMode("presets")} type="button">
          {copy.presetsMode}
        </button>
        <button className={`panel-tab${mode === "actions" ? " panel-tab--active" : ""}`} onClick={() => setMode("actions")} type="button">
          {copy.actionsMode}
        </button>
        <button className={`panel-tab${mode === "macros" ? " panel-tab--active" : ""}`} onClick={() => setMode("macros")} type="button">
          {copy.macrosMode}
        </button>
      </div>

      {mode === "presets" ? (
        <>
          {!selectedProjectId ? <p className="card-copy">{copy.noProjectSelected}</p> : null}
          <label className="field-label">
            <span>{copy.presetName}</span>
            <input
              className="text-input"
              onChange={(event) => setPresetName(event.target.value)}
              placeholder={copy.presetNamePlaceholder}
              type="text"
              value={presetName}
            />
          </label>
          <div className="button-row">
            <button className="ghost-button" disabled={!selectedProjectId} onClick={saveCurrentPreset} type="button">
              {copy.savePreset}
            </button>
          </div>
          {presetRecords.length === 0 ? (
            <p className="card-copy">{selectedProjectId ? copy.noPresets : copy.noProjectSelected}</p>
          ) : (
            <div className="script-panel__catalog">
              {presetRecords.map((preset) => (
                <article className="script-panel__action" key={preset.presetId}>
                  <div className="script-panel__action-head">
                    <strong>{preset.name}</strong>
                    <span>{preset.updatedAt}</span>
                  </div>
                  <div className="script-panel__payload">
                    <span>ID</span>
                    <code>{preset.macro.id}</code>
                  </div>
                  <div className="script-panel__payload">
                    <span>{copy.stepsLabel}</span>
                    <code>{String(preset.macro.steps.length)}</code>
                  </div>
                  <div className="button-row">
                    <button className="ghost-button ghost-button--compact" onClick={() => insertPreset(preset)} type="button">
                      {copy.insertPreset}
                    </button>
                    <button className="ghost-button ghost-button--compact" onClick={() => exportPresetJson(preset)} type="button">
                      {copy.exportPresetJson}
                    </button>
                    <button className="ghost-button ghost-button--compact" onClick={() => deletePreset(preset)} type="button">
                      {copy.deletePreset}
                    </button>
                  </div>
                </article>
              ))}
            </div>
          )}
        </>
      ) : null}

      {mode === "actions" ? (
        <div className="script-panel__catalog">
          {actions.map((action) => (
            <article className="script-panel__action" key={action.id}>
              <div className="script-panel__action-head">
                <strong>{action.id}</strong>
                <span>
                  {copy.categories[action.category as keyof typeof copy.categories] ?? action.category}
                  {action.risk === "destructive" ? ` · ${copy.riskDestructive}` : action.risk === "sensitive" ? ` · ${copy.riskSensitive}` : ` · ${copy.riskNormal}`}
                </span>
              </div>
              <p className="card-copy">{language === "zh" ? action.summary.zh : action.summary.en}</p>
              {isWorkbenchScriptActionHighRisk(action.id) ? <p className="card-copy">{copy.confirmationRequired}</p> : null}
              <div className="script-panel__payload">
                <span>{copy.payload}</span>
                <code>{stringifyPayload(action.payloadExample)}</code>
              </div>
              <div className="button-row">
                <button className="ghost-button ghost-button--compact" onClick={() => insertAction(action)} type="button">
                  {copy.insertLabel}
                </button>
              </div>
            </article>
          ))}
        </div>
      ) : null}

      {mode === "macros" ? (
        <div className="script-panel__catalog">
          {macros.map((macro) => (
            <article className="script-panel__action" key={macro.id}>
              <div className="script-panel__action-head">
                <strong>{macro.id}</strong>
                <span>
                  {copy.categories[macro.category as keyof typeof copy.categories] ?? macro.category}
                  {macro.risk === "destructive" ? ` · ${copy.riskDestructive}` : macro.risk === "sensitive" ? ` · ${copy.riskSensitive}` : ` · ${copy.riskNormal}`}
                </span>
              </div>
              <p className="card-copy">{language === "zh" ? macro.summary.zh : macro.summary.en}</p>
              {macro.requiresConfirmation ? <p className="card-copy">{copy.confirmationRequired}</p> : null}
              <div className="script-panel__payload">
                <span>{copy.payload}</span>
                <code>{stringifyPayload(macro.payloadExample)}</code>
              </div>
              <div className="script-panel__payload">
                <span>{copy.stepsLabel}</span>
                <code>{macro.steps.map((step) => step.action).join(" -> ")}</code>
              </div>
              {macro.payloadExample ? <p className="card-copy">{copy.macroPayloadHint}</p> : null}
              <div className="button-row">
                <button className="ghost-button ghost-button--compact" onClick={() => insertMacro(macro.id, macro.payloadExample)} type="button">
                  {copy.insertLabel}
                </button>
              </div>
            </article>
          ))}
        </div>
      ) : null}
    </section>
  );
}
