export type MaterialPreset = {
  label: string;
  value: string;
  modulusGpa: number | null;
};

export const MATERIAL_PRESETS: MaterialPreset[] = [
  { label: "Steel", value: "210", modulusGpa: 210 },
  { label: "Aluminum", value: "70", modulusGpa: 70 },
  { label: "Titanium", value: "116", modulusGpa: 116 },
  { label: "Concrete", value: "30", modulusGpa: 30 },
  { label: "Carbon fiber", value: "135", modulusGpa: 135 },
  { label: "Custom", value: "custom", modulusGpa: null },
];

export function materialLabel(value: string): string {
  return MATERIAL_PRESETS.find((preset) => preset.value === value)?.label ?? "Custom";
}
