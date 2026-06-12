"use client";

import { useState } from "react";
import type { WorkbenchScriptPanelCopyEntry } from "@/components/workbench/workbench-script-panel-copy";
import {
  getWorkbenchScriptSnippetParameterDefaults,
  getWorkbenchScriptSnippetLabel,
  getWorkbenchScriptSnippetSummary,
  isWorkbenchScriptActionHighRisk,
  type WorkbenchMacroPresetRecord,
  type WorkbenchScriptActionDefinition,
  type WorkbenchScriptLanguage,
  type WorkbenchScriptMacroDefinition,
  type WorkbenchScriptSnippetDefinition,
  type WorkbenchScriptSnippetParameters,
  type WorkbenchScriptSnippetPresetRecord,
} from "@/lib/scripting/workbench-script-runtime";

type CatalogMode = "presets" | "actions" | "macros" | "snippets";

type WorkbenchScriptCatalogPanelProps = {
  actions: WorkbenchScriptActionDefinition[];
  copy: WorkbenchScriptPanelCopyEntry;
  deletePreset: (preset: WorkbenchMacroPresetRecord) => void;
  exportPresetJson: (preset: WorkbenchMacroPresetRecord) => void;
  exportSnippetPresetJson: (preset: WorkbenchScriptSnippetPresetRecord) => void;
  importSnippetPresetJson: (snippet: WorkbenchScriptSnippetDefinition, file: File | undefined) => Promise<void>;
  insertAction: (action: WorkbenchScriptActionDefinition) => void;
  insertMacro: (macroId: string, payload?: Record<string, unknown>) => void;
  insertPreset: (preset: WorkbenchMacroPresetRecord) => void;
  insertSnippet: (snippet: WorkbenchScriptSnippetDefinition, parameters?: WorkbenchScriptSnippetParameters) => void;
  insertSnippetPreset: (preset: WorkbenchScriptSnippetPresetRecord) => void;
  language: WorkbenchScriptLanguage;
  macros: WorkbenchScriptMacroDefinition[];
  deleteSnippetPreset: (preset: WorkbenchScriptSnippetPresetRecord) => void;
  presetName: string;
  presetRecords: WorkbenchMacroPresetRecord[];
  saveSnippetPreset: (snippet: WorkbenchScriptSnippetDefinition, parameters: WorkbenchScriptSnippetParameters) => void;
  saveCurrentPreset: () => void;
  selectedProjectId: string | null;
  setPresetName: (value: string) => void;
  snippets: WorkbenchScriptSnippetDefinition[];
  snippetPresetRecords: WorkbenchScriptSnippetPresetRecord[];
};

function stringifyPayload(payload: Record<string, unknown> | undefined): string {
  return payload ? JSON.stringify(payload) : "{}";
}

function getSnippetModeLabel(language: WorkbenchScriptLanguage) {
  if (language === "zh") return "配方";
  if (language === "ja") return "スニペット";
  if (language === "es") return "Recetas";
  return "Snippets";
}

function getSnippetCategoryLabel(category: WorkbenchScriptSnippetDefinition["category"], language: WorkbenchScriptLanguage) {
  if (language === "zh") {
    if (category === "runtime") return "运行时";
    if (category === "workflow") return "工作流";
    if (category === "inspection") return "检查";
    return "导航";
  }
  return category;
}

function getSnippetPresetLabel(language: WorkbenchScriptLanguage) {
  return language === "zh" ? "配方预设" : language === "ja" ? "スニペットプリセット" : language === "es" ? "Presets de receta" : "Snippet presets";
}

function getSnippetJsonLabel(language: WorkbenchScriptLanguage) {
  return language === "zh" ? "参数 JSON" : language === "ja" ? "パラメータ JSON" : language === "es" ? "JSON de parametros" : "Parameter JSON";
}

function getInsertConfiguredLabel(language: WorkbenchScriptLanguage) {
  return language === "zh" ? "按当前参数插入" : language === "ja" ? "現在の設定で挿入" : language === "es" ? "Insertar con estos parametros" : "Insert configured";
}

function getSaveSnippetPresetLabel(language: WorkbenchScriptLanguage) {
  return language === "zh" ? "存为预设" : language === "ja" ? "プリセット保存" : language === "es" ? "Guardar preset" : "Save preset";
}

function getSnippetPresetEmptyLabel(language: WorkbenchScriptLanguage) {
  return language === "zh" ? "当前项目下还没有这条配方的预设。" : language === "ja" ? "このスニペットのプリセットはまだありません。" : language === "es" ? "Todavia no hay presets para este snippet." : "No presets saved for this snippet yet.";
}

function getSnippetJsonErrorLabel(language: WorkbenchScriptLanguage) {
  return language === "zh" ? "参数 JSON 无效，无法插入或保存。" : language === "ja" ? "パラメータ JSON が無効です。" : language === "es" ? "El JSON de parametros no es valido." : "Parameter JSON is invalid.";
}

function getImportSnippetPresetLabel(language: WorkbenchScriptLanguage) {
  return language === "zh" ? "导入预设" : language === "ja" ? "プリセット読込" : language === "es" ? "Importar preset" : "Import preset";
}

export function WorkbenchScriptCatalogPanel({
  actions,
  copy,
  deletePreset,
  exportPresetJson,
  exportSnippetPresetJson,
  importSnippetPresetJson,
  insertAction,
  insertMacro,
  insertPreset,
  insertSnippetPreset,
  insertSnippet,
  language,
  macros,
  deleteSnippetPreset,
  presetName,
  presetRecords,
  saveSnippetPreset,
  saveCurrentPreset,
  selectedProjectId,
  setPresetName,
  snippets,
  snippetPresetRecords,
}: WorkbenchScriptCatalogPanelProps) {
  const [mode, setMode] = useState<CatalogMode>("presets");
  const [snippetParameterDrafts, setSnippetParameterDrafts] = useState<Record<string, string>>({});
  const [snippetErrors, setSnippetErrors] = useState<Record<string, string | null>>({});

  const readSnippetDraft = (snippet: WorkbenchScriptSnippetDefinition) =>
    snippetParameterDrafts[snippet.id] ?? JSON.stringify(getWorkbenchScriptSnippetParameterDefaults(snippet), null, 2);

  const parseSnippetParameters = (snippet: WorkbenchScriptSnippetDefinition) => {
    try {
      const parsed = JSON.parse(readSnippetDraft(snippet)) as unknown;
      if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) {
        throw new Error(getSnippetJsonErrorLabel(language));
      }
      setSnippetErrors((current) => ({ ...current, [snippet.id]: null }));
      return parsed as WorkbenchScriptSnippetParameters;
    } catch {
      setSnippetErrors((current) => ({ ...current, [snippet.id]: getSnippetJsonErrorLabel(language) }));
      return null;
    }
  };

  return (
    <section className="sidebar-card sidebar-card--compact">
      <div className="card-head">
        <h2>{copy.catalog}</h2>
        <span>{mode === "presets" ? presetRecords.length : mode === "actions" ? actions.length : mode === "macros" ? macros.length : snippets.length}</span>
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
        <button className={`panel-tab${mode === "snippets" ? " panel-tab--active" : ""}`} onClick={() => setMode("snippets")} type="button">
          {getSnippetModeLabel(language)}
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

      {mode === "snippets" ? (
        <div className="script-panel__catalog">
          {snippets.map((snippet) => (
            <article className="script-panel__action" key={snippet.id}>
              <div className="script-panel__action-head">
                <strong>{snippet.id}</strong>
                <span>{getSnippetCategoryLabel(snippet.category, language)}</span>
              </div>
              <p className="card-copy">{getWorkbenchScriptSnippetLabel(snippet, language)}</p>
              <p className="card-copy">{getWorkbenchScriptSnippetSummary(snippet, language)}</p>
              {snippet.parameters && snippet.parameters.length > 0 ? (
                <label className="field-label">
                  <span>{getSnippetJsonLabel(language)}</span>
                  <textarea
                    className="script-panel__editor"
                    rows={Math.min(Math.max(snippet.parameters.length + 2, 4), 8)}
                    spellCheck={false}
                    value={readSnippetDraft(snippet)}
                    onChange={(event) => setSnippetParameterDrafts((current) => ({ ...current, [snippet.id]: event.target.value }))}
                  />
                </label>
              ) : null}
              {snippetErrors[snippet.id] ? <p className="card-copy">{snippetErrors[snippet.id]}</p> : null}
              <div className="script-panel__payload">
                <span>{copy.lineCount}</span>
                <code>{String(snippet.code.split("\n").length)}</code>
              </div>
              <div className="button-row">
                <button
                  className="ghost-button ghost-button--compact"
                  onClick={() => {
                    const parameters = parseSnippetParameters(snippet);
                    if (!parameters) return;
                    insertSnippet(snippet, parameters);
                  }}
                  type="button"
                >
                  {getInsertConfiguredLabel(language)}
                </button>
                <button
                  className="ghost-button ghost-button--compact"
                  disabled={!selectedProjectId}
                  onClick={() => {
                    const parameters = parseSnippetParameters(snippet);
                    if (!parameters) return;
                    saveSnippetPreset(snippet, parameters);
                  }}
                  type="button"
                >
                  {getSaveSnippetPresetLabel(language)}
                </button>
                <button className="ghost-button ghost-button--compact" onClick={() => insertSnippet(snippet)} type="button">
                  {copy.insertLabel}
                </button>
                <label className="ghost-button ghost-button--compact" style={{ display: "inline-flex", alignItems: "center", cursor: "pointer" }}>
                  {getImportSnippetPresetLabel(language)}
                  <input
                    accept="application/json,.json"
                    hidden
                    onChange={(event) => {
                      void importSnippetPresetJson(snippet, event.target.files?.[0]);
                      event.currentTarget.value = "";
                    }}
                    type="file"
                  />
                </label>
              </div>
              <div className="card-subhead">
                <strong>{getSnippetPresetLabel(language)}</strong>
                <span>{snippetPresetRecords.filter((preset) => preset.snippetId === snippet.id).length}</span>
              </div>
              {snippetPresetRecords.filter((preset) => preset.snippetId === snippet.id).length === 0 ? (
                <p className="card-copy">{getSnippetPresetEmptyLabel(language)}</p>
              ) : (
                <div className="script-panel__catalog">
                  {snippetPresetRecords
                    .filter((preset) => preset.snippetId === snippet.id)
                    .map((preset) => (
                      <article className="script-panel__action" key={preset.presetId}>
                        <div className="script-panel__action-head">
                          <strong>{preset.name}</strong>
                          <span>{preset.updatedAt}</span>
                        </div>
                        <div className="script-panel__payload">
                          <span>{copy.payload}</span>
                          <code>{JSON.stringify(preset.parameters)}</code>
                        </div>
                        <div className="button-row">
                          <button className="ghost-button ghost-button--compact" onClick={() => insertSnippetPreset(preset)} type="button">
                            {copy.insertPreset}
                          </button>
                          <button className="ghost-button ghost-button--compact" onClick={() => exportSnippetPresetJson(preset)} type="button">
                            {copy.exportPresetJson}
                          </button>
                          <button className="ghost-button ghost-button--compact" onClick={() => deleteSnippetPreset(preset)} type="button">
                            {copy.deletePreset}
                          </button>
                        </div>
                      </article>
                    ))}
                </div>
              )}
            </article>
          ))}
        </div>
      ) : null}
    </section>
  );
}
