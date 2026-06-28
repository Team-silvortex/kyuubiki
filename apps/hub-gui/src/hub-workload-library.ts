type UnknownRecord = Record<string, unknown>;

export type HubWorkloadEntry = {
  id: string;
  label: string;
  note: string;
  sourceKind: string;
  sourceLabel: string;
  bundlePath: string;
  downloadUrl: string;
  projectId: string;
  projectName: string;
  schema: string;
  layout: string;
  modelCount: number;
  versionCount: number;
  jobCount: number;
  resultCount: number;
  analysisDomains: string[];
  analysisFamilies: string[];
  thermalIntents: string[];
  downloadedAt: string;
  attachedAt: string;
  addedAt: string;
  updatedAt: string;
};

export type HubCatalogValidation =
  | { ok: true; normalized?: string }
  | { ok: false; reason: string };

function asRecord(value: unknown): UnknownRecord {
  return value && typeof value === "object" ? (value as UnknownRecord) : {};
}

function stringArray(value: unknown): string[] {
  return Array.isArray(value) ? value.filter((item): item is string => typeof item === "string") : [];
}

function stringValue(value: unknown): string {
  return String(value || "").trim();
}

function numberValue(value: unknown): number {
  return Number.isFinite(Number(value)) ? Number(value) : 0;
}

export function loadHubWorkloadLibrary(storageKey: string): unknown[] {
  try {
    const raw = window.localStorage.getItem(storageKey);
    const parsed = raw ? JSON.parse(raw) : [];
    return Array.isArray(parsed) ? parsed : [];
  } catch {
    return [];
  }
}

export function persistHubWorkloadLibrary(storageKey: string, entries: unknown[], limit: number): void {
  window.localStorage.setItem(storageKey, JSON.stringify(entries.slice(0, limit)));
}

export function workloadIdentity(entry: unknown): string {
  const candidate = asRecord(entry);
  return [
    stringValue(candidate.sourceKind),
    stringValue(candidate.bundlePath),
    stringValue(candidate.downloadUrl),
    stringValue(candidate.projectId),
  ].join("::");
}

export function normalizeHubWorkloadEntry(entry: unknown): HubWorkloadEntry | null {
  const candidate = asRecord(entry);
  const label = stringValue(candidate.label || candidate.projectName);
  const sourceKind = stringValue(candidate.sourceKind) || "local-bundle";
  const bundlePath = stringValue(candidate.bundlePath);
  const downloadUrl = stringValue(candidate.downloadUrl);
  const projectId = stringValue(candidate.projectId);
  const projectName = stringValue(candidate.projectName);

  if (!label && !bundlePath && !downloadUrl && !projectId) {
    return null;
  }

  return {
    id: String(candidate.id || `workload-${Date.now()}-${Math.random().toString(36).slice(2, 10)}`),
    label: label || projectName || bundlePath || downloadUrl || "workload",
    note: stringValue(candidate.note),
    sourceKind,
    sourceLabel: stringValue(candidate.sourceLabel),
    bundlePath,
    downloadUrl,
    projectId,
    projectName,
    schema: stringValue(candidate.schema),
    layout: stringValue(candidate.layout),
    modelCount: numberValue(candidate.modelCount),
    versionCount: numberValue(candidate.versionCount),
    jobCount: numberValue(candidate.jobCount),
    resultCount: numberValue(candidate.resultCount),
    analysisDomains: stringArray(candidate.analysisDomains || candidate.analysis_domains),
    analysisFamilies: stringArray(candidate.analysisFamilies || candidate.analysis_families),
    thermalIntents: stringArray(candidate.thermalIntents || candidate.thermal_intents),
    downloadedAt: stringValue(candidate.downloadedAt),
    attachedAt: stringValue(candidate.attachedAt),
    addedAt: stringValue(candidate.addedAt) || new Date().toISOString(),
    updatedAt: stringValue(candidate.updatedAt) || new Date().toISOString(),
  };
}

export function mergeHubWorkloadLibrary(
  existingEntries: unknown[],
  incomingEntries: unknown[],
  limit?: number,
): HubWorkloadEntry[] {
  const merged: HubWorkloadEntry[] = [];

  for (const candidate of [...incomingEntries, ...existingEntries]) {
    const normalized = normalizeHubWorkloadEntry(candidate);
    if (!normalized) {
      continue;
    }

    const duplicateIndex = merged.findIndex((entry) => workloadIdentity(entry) === workloadIdentity(normalized));
    if (duplicateIndex >= 0) {
      continue;
    }

    merged.push(normalized);
    if (typeof limit === "number" && merged.length >= limit) {
      break;
    }
  }

  return merged;
}

export function projectSummaryFromInspectPayload(raw: string): UnknownRecord {
  const parsed = asRecord(JSON.parse(raw));
  return {
    projectId: stringValue(parsed.project_id),
    projectName: stringValue(parsed.project_name),
    schema: stringValue(parsed.schema),
    layout: stringValue(parsed.layout),
    modelCount: Number(parsed.model_count || 0),
    versionCount: Number(parsed.version_count || 0),
    jobCount: Number(parsed.job_count || 0),
    resultCount: Number(parsed.result_count || 0),
    analysisDomains: stringArray(parsed.analysis_domains),
    analysisFamilies: stringArray(parsed.analysis_families),
    thermalIntents: stringArray(parsed.thermal_intents),
  };
}

export function validateHubCatalogUrl(value: unknown): HubCatalogValidation {
  const normalized = String(value || "").trim();
  if (!normalized) {
    return { ok: false, reason: "Fill in a workload catalog URL first." };
  }

  try {
    const parsed = new URL(normalized);
    const protocol = parsed.protocol.toLowerCase();
    const hostname = parsed.hostname.toLowerCase();
    const isLoopback =
      hostname === "localhost" || hostname === "127.0.0.1" || hostname === "::1" || hostname === "[::1]";
    if (protocol === "https:" || (protocol === "http:" && isLoopback)) {
      return { ok: true, normalized };
    }
    return {
      ok: false,
      reason: "Catalog URL must use https, or http only for localhost / 127.0.0.1 / ::1.",
    };
  } catch {
    return { ok: false, reason: "Catalog URL must be a valid absolute URL." };
  }
}

export function validateRemoteWorkloadCatalogPayload(payload: unknown): HubCatalogValidation {
  if (!payload || typeof payload !== "object" || Array.isArray(payload)) {
    return { ok: false, reason: "Catalog payload must be a JSON object." };
  }

  const catalog = payload as UnknownRecord;
  if (catalog.schema_version !== "kyuubiki.workload-catalog/v1") {
    return {
      ok: false,
      reason: "Catalog schema_version must be kyuubiki.workload-catalog/v1.",
    };
  }

  if (!Array.isArray(catalog.workloads)) {
    return { ok: false, reason: "Catalog workloads must be an array." };
  }

  for (const [index, rawWorkload] of catalog.workloads.entries()) {
    if (!rawWorkload || typeof rawWorkload !== "object" || Array.isArray(rawWorkload)) {
      return { ok: false, reason: `Workload ${index + 1} must be an object.` };
    }

    const workload = rawWorkload as UnknownRecord;
    if (!String(workload.label || "").trim()) {
      return { ok: false, reason: `Workload ${index + 1} is missing label.` };
    }

    const hasRequiredLocator =
      String(workload.download_url || "").trim() ||
      String(workload.bundle_path || "").trim() ||
      String(workload.project_id || "").trim();
    if (!hasRequiredLocator) {
      return {
        ok: false,
        reason: `Workload ${index + 1} must define download_url, bundle_path, or project_id.`,
      };
    }

    if (
      workload.analysis_domains !== undefined &&
      (!Array.isArray(workload.analysis_domains) ||
        workload.analysis_domains.some(
          (value) =>
            typeof value !== "string" ||
            !["mechanical", "thermal", "thermo_mechanical"].includes(value),
        ))
    ) {
      return {
        ok: false,
        reason: `Workload ${index + 1} has invalid analysis_domains.`,
      };
    }

    if (
      workload.analysis_families !== undefined &&
      (!Array.isArray(workload.analysis_families) ||
        workload.analysis_families.some(
          (value) =>
            typeof value !== "string" ||
            !["axial_and_springs", "beams_and_frames", "trusses", "planes"].includes(value),
        ))
    ) {
      return {
        ok: false,
        reason: `Workload ${index + 1} has invalid analysis_families.`,
      };
    }

    if (
      workload.thermal_intents !== undefined &&
      (!Array.isArray(workload.thermal_intents) ||
        workload.thermal_intents.some((value) => typeof value !== "string"))
    ) {
      return {
        ok: false,
        reason: `Workload ${index + 1} has invalid thermal_intents.`,
      };
    }
  }

  return { ok: true };
}

export function normalizeRemoteWorkloadCatalogPayload(
  payload: unknown,
  catalogUrl: string,
): HubWorkloadEntry[] {
  const catalog = asRecord(payload);
  const list = Array.isArray(payload)
    ? payload
    : Array.isArray(catalog.workloads)
      ? catalog.workloads
      : Array.isArray(catalog.items)
        ? catalog.items
        : [];

  return list
    .map((entry) => {
      const candidate = asRecord(entry);
      return normalizeHubWorkloadEntry({
        label: candidate.label || candidate.name || candidate.projectName || candidate.project_name,
        note: candidate.note || candidate.description || `Synced from ${catalogUrl}`,
        sourceKind: "remote-catalog",
        sourceLabel: candidate.sourceLabel || catalog.sourceLabel || catalogUrl,
        bundlePath: candidate.bundlePath || candidate.bundle_path || "",
        downloadUrl: candidate.downloadUrl || candidate.download_url || catalogUrl,
        projectId: candidate.projectId || candidate.project_id || "",
        projectName: candidate.projectName || candidate.project_name || "",
        schema: candidate.schema || "",
        layout: candidate.layout || "",
        modelCount: candidate.modelCount || candidate.model_count || 0,
        versionCount: candidate.versionCount || candidate.version_count || 0,
        jobCount: candidate.jobCount || candidate.job_count || 0,
        resultCount: candidate.resultCount || candidate.result_count || 0,
        analysisDomains: candidate.analysisDomains || candidate.analysis_domains || [],
        analysisFamilies: candidate.analysisFamilies || candidate.analysis_families || [],
        thermalIntents: candidate.thermalIntents || candidate.thermal_intents || [],
      });
    })
    .filter((entry): entry is HubWorkloadEntry => Boolean(entry));
}

export function inferDownloadFilename(url: unknown, fallback = "kyuubiki-workload.kyuubiki"): string {
  try {
    const parsed = new URL(String(url || "").trim());
    const pathname = parsed.pathname.split("/").filter(Boolean).at(-1);
    return pathname || fallback;
  } catch {
    return fallback;
  }
}

export function workloadSourceBadge(entry: UnknownRecord): [string, string] {
  if (entry.sourceKind === "remote-catalog" && entry.bundlePath) {
    return ["attached local", "desktop-shell-state desktop-shell-state--healthy"];
  }

  if (entry.sourceKind === "remote-catalog" && entry.downloadedAt) {
    return ["downloaded", "desktop-shell-state desktop-shell-state--warning"];
  }

  switch (entry.sourceKind) {
    case "remote-catalog":
      return ["remote catalog", "desktop-shell-state desktop-shell-state--healthy"];
    case "imported-library":
      return ["imported", "desktop-shell-state desktop-shell-state--warning"];
    default:
      return ["local bundle", "desktop-shell-state desktop-shell-state--idle"];
  }
}

export function workloadProvenanceHost(value: unknown): string {
  const normalized = String(value || "").trim();
  if (!normalized) {
    return "";
  }

  try {
    return new URL(normalized).host;
  } catch {
    return "";
  }
}

export function workloadProvenanceLabel(entry: UnknownRecord): string {
  if (entry.sourceKind === "remote-catalog") {
    if (entry.sourceLabel === "Kyuubiki Control Plane") {
      return "first-party control plane catalog";
    }
    const hostHint = workloadProvenanceHost(entry.sourceLabel || entry.downloadUrl || "");
    if (hostHint) {
      return `custom remote catalog · ${hostHint}`;
    }
    return `custom remote catalog${entry.sourceLabel ? ` · ${entry.sourceLabel}` : ""}`;
  }

  if (entry.sourceKind === "imported-library") {
    return "imported library snapshot";
  }

  if (entry.sourceLabel) {
    return String(entry.sourceLabel);
  }

  return "Hub local registration";
}

export function workloadDomainLabel(domain: unknown): string {
  switch (domain) {
    case "mechanical":
      return "Mechanical";
    case "thermal":
      return "Thermal";
    case "thermo_mechanical":
      return "Thermo-mechanical";
    default:
      return String(domain || "").trim();
  }
}

export function workloadFamilyLabel(family: unknown): string {
  switch (family) {
    case "axial_and_springs":
      return "Axial & Springs";
    case "beams_and_frames":
      return "Beams & Frames";
    case "trusses":
      return "Trusses";
    case "planes":
      return "Planes";
    default:
      return String(family || "").trim();
  }
}
