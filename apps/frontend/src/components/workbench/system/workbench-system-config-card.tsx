"use client";

type Theme = "linen" | "marine" | "graphite";
type Language = "en" | "zh";
type FrontendRuntimeMode = "orchestrated_gui" | "direct_mesh_gui";
type DirectMeshSelectionMode = "healthiest" | "first_reachable";

type WorkbenchSystemConfigCardProps = {
  title: string;
  status: string;
  themeLabel: string;
  languageLabel: string;
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
  frontendModeOptions: Array<{ value: FrontendRuntimeMode; label: string }>;
  directMeshStrategyOptions: Array<{ value: DirectMeshSelectionMode; label: string }>;
  onThemeChange: (value: Theme) => void;
  onLanguageChange: (value: Language) => void;
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
  themeLabel,
  languageLabel,
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
  frontendModeOptions,
  directMeshStrategyOptions,
  onThemeChange,
  onLanguageChange,
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
  return (
    <section className="sidebar-card sidebar-card--compact">
      <div className="card-head">
        <h2>{title}</h2>
        <span>{status}</span>
      </div>
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
      <p className="card-copy">{browserLimitsNote}</p>
      <div className="button-row">
        <button className="ghost-button" onClick={onExportDatabase} type="button">
          {exportDatabaseLabel}
        </button>
      </div>
    </section>
  );
}
