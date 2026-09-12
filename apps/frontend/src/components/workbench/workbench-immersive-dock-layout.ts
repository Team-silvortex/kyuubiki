export type ImmersiveDockPreferences = { width?: number; height?: number };
export type ImmersiveDockGeometry = { width: number; height: number; stacked: boolean; minMainSize?: number; gutter?: number };
const clamp = (value: number, min: number, max: number) => Math.max(min, Math.min(max, value));
const positive = (value: number | undefined, fallback: number) =>
  typeof value === "number" && Number.isFinite(value) && value > 0 ? value : fallback;

export function resolveImmersiveDockLayout(preferences: ImmersiveDockPreferences, geometry: ImmersiveDockGeometry) {
  const width = positive(geometry.width, 0), height = positive(geometry.height, 0);
  const key: keyof ImmersiveDockPreferences = geometry.stacked ? "height" : "width";
  // Both panel paddings, the separator and two grid gaps are outside the two regions.
  const budget = Math.max(0, (geometry.stacked ? height : width) - positive(geometry.gutter, 32));
  const proportionMax = geometry.stacked ? budget * 0.45 : Math.min(460, budget * 0.4);
  const max = Math.max(0, Math.min(proportionMax, budget - positive(geometry.minMainSize, 0)));
  const min = Math.min(geometry.stacked ? 180 : 220, max);
  const fallback = geometry.stacked ? budget * 0.42 : clamp(width * 0.27, 260, 340);
  return { key, min, max, size: clamp(positive(preferences[key], fallback), min, max) };
}

export function resizeImmersiveDock(preferences: ImmersiveDockPreferences, geometry: ImmersiveDockGeometry, value: number) {
  const layout = resolveImmersiveDockLayout(preferences, geometry);
  return { ...preferences, [layout.key]: clamp(Number.isFinite(value) ? value : layout.size, layout.min, layout.max) };
}
