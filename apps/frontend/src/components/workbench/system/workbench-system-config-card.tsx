"use client";

import { useState } from "react";

type Theme = "linen" | "marine" | "graphite";
type Language = string;
type FrontendRuntimeMode = "orchestrated_gui" | "direct_mesh_gui";
type DirectMeshSelectionMode = "healthiest" | "first_reachable";
type ConfigPage = "workspace" | "routing" | "access" | "governance" | "packs";

type WorkbenchSystemConfigCardProps = {
  title: string;
  status: string;
  workspacePageLabel: string;
  routingPageLabel: string;
  accessPageLabel: string;
  governancePageLabel: string;
  packsPageLabel: string;
  themeLabel: string;
  languageLabel: string;
  languagePacksTitle: string;
  languagePacksHint: string;
  languagePacksEmptyLabel: string;
  languagePackNameLabel: string;
  languagePackVersionLabel: string;
  languagePackSourceImportedLabel: string;
  languagePackSourceDownloadedLabel: string;
  languagePackDownloadTemplateLabel: string;
  languagePackExportInstalledLabel: string;
  languagePackImportLabel: string;
  languagePackRemoveLabel: string;
  languagePackCatalogTitle: string;
  languagePackCatalogHint: string;
  languagePackCatalogActionLabel: string;
  frontendModeLabel: string;
  directMeshStrategyLabel: string;
  directMeshEndpointsLabel: string;
  directMeshEndpointsHelp: string;
  controlPlaneTokenLabel: string;
  controlPlaneTokenHelp: string;
  controlPlaneTokenPlaceholder: string;
  clusterTokenLabel: string;
  clusterTokenHelp: string;
  clusterTokenPlaceholder: string;
  directMeshTokenLabel: string;
  directMeshTokenHelp: string;
  directMeshTokenPlaceholder: string;
  shortcutHintsLabel: string;
  shortcutHintsHelp: string;
  immersiveGuardLabel: string;
  immersiveGuardHelp: string;
  browserLimitsNote: string;
  exportDatabaseLabel: string;
  governanceTitle: string;
  governanceHint: string;
  governanceRows: Array<{ label: string; value: string }>;
  governanceJson: string;
  theme: Theme;
  language: Language;
  frontendRuntimeMode: FrontendRuntimeMode;
  directMeshSelectionMode: DirectMeshSelectionMode;
  directMeshEndpointsText: string;
  controlPlaneApiToken: string;
  clusterApiToken: string;
  directMeshApiToken: string;
  showShortcutHints: boolean;
  immersiveGuardrails: boolean;
  themeOptions: Array<{ value: Theme; label: string }>;
  languageOptions: Array<{ value: Language; label: string }>;
  installedLanguagePacks: Array<{
    id: string;
    language: string;
    name: string;
    version: string;
    versionLine?: string;
    targetAppVersion?: string;
    source: "imported" | "downloaded";
    updatedAt: string;
    description?: string;
    compatibilityLabel: string;
    targetLabel: string;
  }>;
  catalogLanguagePacks: Array<{ id: string; language: string; name: string; status: string }>;
  frontendModeOptions: Array<{ value: FrontendRuntimeMode; label: string }>;
  directMeshStrategyOptions: Array<{ value: DirectMeshSelectionMode; label: string }>;
  onThemeChange: (value: Theme) => void;
  onLanguageChange: (value: Language) => void;
  onDownloadLanguagePackTemplate: () => void;
  onExportInstalledLanguagePack: () => void;
  onImportLanguagePack: (file: File) => void;
  onRemoveLanguagePack: (packId: string) => void;
  onFrontendRuntimeModeChange: (value: FrontendRuntimeMode) => void;
  onDirectMeshSelectionModeChange: (value: DirectMeshSelectionMode) => void;
  onDirectMeshEndpointsTextChange: (value: string) => void;
  onControlPlaneApiTokenChange: (value: string) => void;
  onClusterApiTokenChange: (value: string) => void;
  onDirectMeshApiTokenChange: (value: string) => void;
  onShowShortcutHintsChange: (value: boolean) => void;
  onImmersiveGuardrailsChange: (value: boolean) => void;
  onExportDatabase: () => void;
};

export function WorkbenchSystemConfigCard({
  title,
  status,
  workspacePageLabel,
  routingPageLabel,
  accessPageLabel,
  governancePageLabel,
  packsPageLabel,
  themeLabel,
  languageLabel,
  languagePacksTitle,
  languagePacksHint,
  languagePacksEmptyLabel,
  languagePackNameLabel,
  languagePackVersionLabel,
  languagePackSourceImportedLabel,
  languagePackSourceDownloadedLabel,
  languagePackDownloadTemplateLabel,
  languagePackExportInstalledLabel,
  languagePackImportLabel,
  languagePackRemoveLabel,
  languagePackCatalogTitle,
  languagePackCatalogHint,
  languagePackCatalogActionLabel,
  frontendModeLabel,
  directMeshStrategyLabel,
  directMeshEndpointsLabel,
  directMeshEndpointsHelp,
  controlPlaneTokenLabel,
  controlPlaneTokenHelp,
  controlPlaneTokenPlaceholder,
  clusterTokenLabel,
  clusterTokenHelp,
  clusterTokenPlaceholder,
  directMeshTokenLabel,
  directMeshTokenHelp,
  directMeshTokenPlaceholder,
  shortcutHintsLabel,
  shortcutHintsHelp,
  immersiveGuardLabel,
  immersiveGuardHelp,
  browserLimitsNote,
  exportDatabaseLabel,
  governanceTitle,
  governanceHint,
  governanceRows,
  governanceJson,
  theme,
  language,
  frontendRuntimeMode,
  directMeshSelectionMode,
  directMeshEndpointsText,
  controlPlaneApiToken,
  clusterApiToken,
  directMeshApiToken,
  showShortcutHints,
  immersiveGuardrails,
  themeOptions,
  languageOptions,
  installedLanguagePacks,
  catalogLanguagePacks,
  frontendModeOptions,
  directMeshStrategyOptions,
  onThemeChange,
  onLanguageChange,
  onDownloadLanguagePackTemplate,
  onExportInstalledLanguagePack,
  onImportLanguagePack,
  onRemoveLanguagePack,
  onFrontendRuntimeModeChange,
  onDirectMeshSelectionModeChange,
  onDirectMeshEndpointsTextChange,
  onControlPlaneApiTokenChange,
  onClusterApiTokenChange,
  onDirectMeshApiTokenChange,
  onShowShortcutHintsChange,
  onImmersiveGuardrailsChange,
  onExportDatabase,
}: WorkbenchSystemConfigCardProps) {
  const [page, setPage] = useState<ConfigPage>("workspace");

  return (
    <section className="sidebar-card sidebar-card--compact">
      <div className="card-head">
        <h2>{title}</h2>
        <span>{status}</span>
      </div>
      <div className="panel-tabs panel-tabs--wide">
        <button className={`panel-tab${page === "workspace" ? " panel-tab--active" : ""}`} onClick={() => setPage("workspace")} type="button">
          {workspacePageLabel}
        </button>
        <button className={`panel-tab${page === "routing" ? " panel-tab--active" : ""}`} onClick={() => setPage("routing")} type="button">
          {routingPageLabel}
        </button>
        <button className={`panel-tab${page === "access" ? " panel-tab--active" : ""}`} onClick={() => setPage("access")} type="button">
          {accessPageLabel}
        </button>
        <button className={`panel-tab${page === "governance" ? " panel-tab--active" : ""}`} onClick={() => setPage("governance")} type="button">
          {governancePageLabel}
        </button>
        <button className={`panel-tab${page === "packs" ? " panel-tab--active" : ""}`} onClick={() => setPage("packs")} type="button">
          {packsPageLabel}
        </button>
      </div>
      {page === "workspace" ? (
        <div className="form-grid compact">
          <label>
            <span>{themeLabel}</span>
            <select value={theme} onChange={(event) => onThemeChange(event.target.value as Theme)}>
              {themeOptions.map((option) => (
                <option key={option.value} value={option.value}>
                  {option.label}
                </option>
              ))}
            </select>
          </label>
          <label>
            <span>{languageLabel}</span>
            <select value={language} onChange={(event) => onLanguageChange(event.target.value as Language)}>
              {languageOptions.map((option) => (
                <option key={option.value} value={option.value}>
                  {option.label}
                </option>
              ))}
            </select>
          </label>
          <label className="toggle-row">
            <div>
              <span>{shortcutHintsLabel}</span>
              <small className="field-hint">{shortcutHintsHelp}</small>
            </div>
            <input type="checkbox" checked={showShortcutHints} onChange={(event) => onShowShortcutHintsChange(event.target.checked)} />
          </label>
          <label className="toggle-row">
            <div>
              <span>{immersiveGuardLabel}</span>
              <small className="field-hint">{immersiveGuardHelp}</small>
            </div>
            <input type="checkbox" checked={immersiveGuardrails} onChange={(event) => onImmersiveGuardrailsChange(event.target.checked)} />
          </label>
        </div>
      ) : null}
      {page === "routing" ? (
        <div className="form-grid compact">
          <label>
            <span>{frontendModeLabel}</span>
            <select value={frontendRuntimeMode} onChange={(event) => onFrontendRuntimeModeChange(event.target.value as FrontendRuntimeMode)}>
              {frontendModeOptions.map((option) => (
                <option key={option.value} value={option.value}>
                  {option.label}
                </option>
              ))}
            </select>
          </label>
          <label>
            <span>{directMeshStrategyLabel}</span>
            <select value={directMeshSelectionMode} onChange={(event) => onDirectMeshSelectionModeChange(event.target.value as DirectMeshSelectionMode)}>
              {directMeshStrategyOptions.map((option) => (
                <option key={option.value} value={option.value}>
                  {option.label}
                </option>
              ))}
            </select>
          </label>
          <label className="field-span-2">
            <span>{directMeshEndpointsLabel}</span>
            <small className="field-hint">{directMeshEndpointsHelp}</small>
            <textarea rows={3} value={directMeshEndpointsText} onChange={(event) => onDirectMeshEndpointsTextChange(event.target.value)} />
          </label>
        </div>
      ) : null}
      {page === "access" ? (
        <div className="form-grid compact">
          <label className="field-span-2">
            <span>{controlPlaneTokenLabel}</span>
            <small className="field-hint">{controlPlaneTokenHelp}</small>
            <input type="password" value={controlPlaneApiToken} onChange={(event) => onControlPlaneApiTokenChange(event.target.value)} placeholder={controlPlaneTokenPlaceholder} />
          </label>
          <label className="field-span-2">
            <span>{clusterTokenLabel}</span>
            <small className="field-hint">{clusterTokenHelp}</small>
            <input type="password" value={clusterApiToken} onChange={(event) => onClusterApiTokenChange(event.target.value)} placeholder={clusterTokenPlaceholder} />
          </label>
          <label className="field-span-2">
            <span>{directMeshTokenLabel}</span>
            <small className="field-hint">{directMeshTokenHelp}</small>
            <input type="password" value={directMeshApiToken} onChange={(event) => onDirectMeshApiTokenChange(event.target.value)} placeholder={directMeshTokenPlaceholder} />
          </label>
        </div>
      ) : null}
      {page === "governance" ? (
        <div className="stack-block">
          <div className="card-copy">
            <strong>{governanceTitle}</strong>
            <p>{governanceHint}</p>
          </div>
          <div className="sidebar-list">
            {governanceRows.map((row) => (
              <div className="sidebar-list__row" key={row.label}>
                <span>{row.label}</span>
                <strong>{row.value}</strong>
              </div>
            ))}
          </div>
          <pre className="code-block" style={{ margin: 0, whiteSpace: "pre-wrap", wordBreak: "break-word" }}>
            {governanceJson}
          </pre>
        </div>
      ) : null}
      {page === "packs" ? (
        <div className="stack-block">
          <div className="card-copy">
            <strong>{languagePacksTitle}</strong>
            <p>{languagePacksHint}</p>
          </div>
          <div className="button-row">
            <button className="ghost-button" onClick={onDownloadLanguagePackTemplate} type="button">
              {languagePackDownloadTemplateLabel}
            </button>
            <button className="ghost-button" onClick={onExportInstalledLanguagePack} type="button">
              {languagePackExportInstalledLabel}
            </button>
            <label className="ghost-button file-button">
              <span>{languagePackImportLabel}</span>
              <input
                accept="application/json,.json"
                onChange={(event) => {
                  const file = event.target.files?.[0];
                  if (file) onImportLanguagePack(file);
                  event.currentTarget.value = "";
                }}
                type="file"
              />
            </label>
          </div>
          {installedLanguagePacks.length === 0 ? (
            <p className="card-copy">{languagePacksEmptyLabel}</p>
          ) : (
            <div className="sidebar-list">
              {installedLanguagePacks.map((pack) => (
                <article className="history-item" key={pack.id}>
                  <div>
                    <strong>{pack.language.toUpperCase()} · {pack.name}</strong>
                    <p className="history-meta">
                      {languagePackNameLabel}: {pack.name} · {languagePackVersionLabel}: {pack.version} ·{" "}
                      {pack.source === "downloaded" ? languagePackSourceDownloadedLabel : languagePackSourceImportedLabel}
                    </p>
                    <p className="history-meta">{pack.targetLabel}</p>
                    <p className="history-meta">{pack.compatibilityLabel}</p>
                    {pack.description ? <p className="history-meta">{pack.description}</p> : null}
                    <p className="history-meta">{pack.updatedAt}</p>
                  </div>
                  <div className="history-actions">
                    <button className="ghost-button ghost-button--compact" onClick={() => onRemoveLanguagePack(pack.id)} type="button">
                      {languagePackRemoveLabel}
                    </button>
                  </div>
                </article>
              ))}
            </div>
          )}
          <div className="card-copy">
            <strong>{languagePackCatalogTitle}</strong>
            <p>{languagePackCatalogHint}</p>
          </div>
          <div className="sidebar-list">
            {catalogLanguagePacks.map((pack) => (
              <article className="history-item" key={pack.id}>
                <div>
                  <strong>{pack.language.toUpperCase()} · {pack.name}</strong>
                  <p className="history-meta">{pack.status}</p>
                </div>
                <div className="history-actions">
                  <button className="ghost-button ghost-button--compact" disabled type="button">
                    {languagePackCatalogActionLabel}
                  </button>
                </div>
              </article>
            ))}
          </div>
        </div>
      ) : null}
      <p className="card-copy">{browserLimitsNote}</p>
      <div className="button-row">
        <button className="ghost-button" onClick={onExportDatabase} type="button">
          {exportDatabaseLabel}
        </button>
      </div>
    </section>
  );
}
