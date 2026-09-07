export type WorkbenchPanelSize = "sidebar" | "inspector" | "report";
export type WorkbenchPanelPreferences = Partial<Record<WorkbenchPanelSize, number>>;
export type WorkbenchPanelGeometry = { width: number; height: number; gap: number; workflow?: boolean };
export const WORKBENCH_PANEL_LAYOUT_KEY = "kyuubiki.workbench.panelLayout.v1";

const finite = (value: number, fallback: number) => Number.isFinite(value) ? value : fallback;
const clamp = (value: number, min: number, max: number) => Math.max(min, Math.min(max, value));

export function parseWorkbenchPanelPreferences(raw: string | null): WorkbenchPanelPreferences {
  if (!raw || raw.length > 1024) return {};
  try {
    const data = JSON.parse(raw);
    if (!data || data.version !== 1 || !data.sizes || Array.isArray(data.sizes)) return {};
    const sizes: WorkbenchPanelPreferences = {};
    for (const key of ["sidebar", "inspector", "report"] as const) {
      const value = data.sizes[key];
      if (typeof value === "number" && Number.isFinite(value) && value > 0 && value <= 5000) sizes[key] = value;
    }
    return sizes;
  } catch {
    return {};
  }
}

export function resolveWorkbenchPanelLayout(preferences: WorkbenchPanelPreferences, geometry: WorkbenchPanelGeometry) {
  const width = Math.max(0, finite(geometry.width, 0));
  const height = Math.max(0, finite(geometry.height, 0));
  const gap = clamp(finite(geometry.gap, 8), 0, 32);
  const budget = Math.max(0, width - 70 - 3 * gap);
  const minimumMain = Math.min(budget, Math.min(560, Math.max(360, budget * 0.5)));
  const sideBudget = budget - minimumMain;
  const sidebarMin = Math.min(176, sideBudget * 176 / 340);
  const inspectorMin = Math.min(164, sideBudget - sidebarMin);
  let sidebar = clamp(finite(preferences.sidebar ?? (geometry.workflow ? 340 : width * 0.16), 220), sidebarMin, 520);
  let inspector = clamp(finite(preferences.inspector ?? Math.max(200, width * 0.14), 200), inspectorMin, 440);
  // Window constraints are transient: do not overwrite the user's preferred sizes.
  const extra = sidebar + inspector - sidebarMin - inspectorMin;
  if (sidebar + inspector > sideBudget && extra > 0) {
    const ratio = Math.max(0, (sideBudget - sidebarMin - inspectorMin) / extra);
    sidebar = sidebarMin + (sidebar - sidebarMin) * ratio;
    inspector = inspectorMin + (inspector - inspectorMin) * ratio;
  }
  const reportMax = Math.max(0, Math.min(height * 0.45, height - Math.min(300, height * 0.6)));
  const reportMin = Math.min(120, reportMax);
  const report = clamp(finite(preferences.report ?? clamp(height * 0.25, 150, 240), 150), reportMin, reportMax);
  return {
    sizes: { sidebar, inspector, report },
    limits: {
      sidebar: { min: sidebarMin, max: Math.max(sidebarMin, Math.min(520, sideBudget - inspector)) },
      inspector: { min: inspectorMin, max: Math.max(inspectorMin, Math.min(440, sideBudget - sidebar)) },
      report: { min: reportMin, max: reportMax },
    },
    minimumMain,
  };
}

export function resizeWorkbenchPanel(
  preferences: WorkbenchPanelPreferences, geometry: WorkbenchPanelGeometry, panel: WorkbenchPanelSize, value: number,
): WorkbenchPanelPreferences {
  const layout = resolveWorkbenchPanelLayout(preferences, geometry);
  const { min, max } = layout.limits[panel];
  return { ...preferences, [panel]: clamp(finite(value, layout.sizes[panel]), min, max) };
}
