export function extractAnalysisMetadata(value: unknown): {
  analysis_domain?: "mechanical" | "thermal" | "thermo_mechanical";
  analysis_family?: "axial_and_springs" | "beams_and_frames" | "trusses" | "planes";
  thermal_intent?: string[];
} {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    return {};
  }
  const metadata = (value as { analysis_metadata?: unknown }).analysis_metadata;
  if (!metadata || typeof metadata !== "object" || Array.isArray(metadata)) {
    return {};
  }
  const record = metadata as {
    domain?: unknown;
    family?: unknown;
    thermal_intent?: unknown;
  };
  return {
    analysis_domain:
      record.domain === "mechanical" || record.domain === "thermal" || record.domain === "thermo_mechanical" ? record.domain : undefined,
    analysis_family:
      record.family === "axial_and_springs" ||
      record.family === "beams_and_frames" ||
      record.family === "trusses" ||
      record.family === "planes"
        ? record.family
        : undefined,
    thermal_intent: Array.isArray(record.thermal_intent) ? record.thermal_intent.filter((item): item is string => typeof item === "string") : undefined,
  };
}
