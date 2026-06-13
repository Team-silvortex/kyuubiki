export function loadHubWorkloadLibrary(storageKey) {
  try {
    const raw = window.localStorage.getItem(storageKey);
    const parsed = raw ? JSON.parse(raw) : [];
    return Array.isArray(parsed) ? parsed : [];
  } catch {
    return [];
  }
}

export function persistHubWorkloadLibrary(storageKey, entries, limit) {
  window.localStorage.setItem(storageKey, JSON.stringify(entries.slice(0, limit)));
}

export function workloadIdentity(entry) {
  return [
    String(entry?.sourceKind || "").trim(),
    String(entry?.bundlePath || "").trim(),
    String(entry?.downloadUrl || "").trim(),
    String(entry?.projectId || "").trim(),
  ].join("::");
}

export function normalizeHubWorkloadEntry(entry) {
  const label = String(entry?.label || entry?.projectName || "").trim();
  const sourceKind = String(entry?.sourceKind || "").trim() || "local-bundle";
  const bundlePath = String(entry?.bundlePath || "").trim();
  const downloadUrl = String(entry?.downloadUrl || "").trim();
  const projectId = String(entry?.projectId || "").trim();
  const projectName = String(entry?.projectName || "").trim();

  if (!label && !bundlePath && !downloadUrl && !projectId) {
    return null;
  }

  return {
    id: String(entry?.id || `workload-${Date.now()}-${Math.random().toString(36).slice(2, 10)}`),
    label: label || projectName || bundlePath || downloadUrl || "workload",
    note: String(entry?.note || "").trim(),
    sourceKind,
    sourceLabel: String(entry?.sourceLabel || "").trim(),
    bundlePath,
    downloadUrl,
    projectId,
    projectName,
    schema: String(entry?.schema || "").trim(),
    layout: String(entry?.layout || "").trim(),
    modelCount: Number.isFinite(Number(entry?.modelCount)) ? Number(entry.modelCount) : 0,
    versionCount: Number.isFinite(Number(entry?.versionCount)) ? Number(entry.versionCount) : 0,
    jobCount: Number.isFinite(Number(entry?.jobCount)) ? Number(entry.jobCount) : 0,
    resultCount: Number.isFinite(Number(entry?.resultCount)) ? Number(entry.resultCount) : 0,
    analysisDomains: Array.isArray(entry?.analysisDomains)
      ? entry.analysisDomains.filter((value) => typeof value === "string")
      : Array.isArray(entry?.analysis_domains)
        ? entry.analysis_domains.filter((value) => typeof value === "string")
        : [],
    analysisFamilies: Array.isArray(entry?.analysisFamilies)
      ? entry.analysisFamilies.filter((value) => typeof value === "string")
      : Array.isArray(entry?.analysis_families)
        ? entry.analysis_families.filter((value) => typeof value === "string")
        : [],
    thermalIntents: Array.isArray(entry?.thermalIntents)
      ? entry.thermalIntents.filter((value) => typeof value === "string")
      : Array.isArray(entry?.thermal_intents)
        ? entry.thermal_intents.filter((value) => typeof value === "string")
        : [],
    downloadedAt: String(entry?.downloadedAt || "").trim(),
    attachedAt: String(entry?.attachedAt || "").trim(),
    addedAt: String(entry?.addedAt || "").trim() || new Date().toISOString(),
    updatedAt: String(entry?.updatedAt || "").trim() || new Date().toISOString(),
  };
}

export function mergeHubWorkloadLibrary(existingEntries, incomingEntries, limit) {
  const merged = [];

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
    if (merged.length >= limit) {
      break;
    }
  }

  return merged;
}

export function projectSummaryFromInspectPayload(raw) {
  const parsed = JSON.parse(raw);
  return {
    projectId: String(parsed?.project_id || "").trim(),
    projectName: String(parsed?.project_name || "").trim(),
    schema: String(parsed?.schema || "").trim(),
    layout: String(parsed?.layout || "").trim(),
    modelCount: Number(parsed?.model_count || 0),
    versionCount: Number(parsed?.version_count || 0),
    jobCount: Number(parsed?.job_count || 0),
    resultCount: Number(parsed?.result_count || 0),
    analysisDomains: Array.isArray(parsed?.analysis_domains) ? parsed.analysis_domains.filter((value) => typeof value === "string") : [],
    analysisFamilies: Array.isArray(parsed?.analysis_families) ? parsed.analysis_families.filter((value) => typeof value === "string") : [],
    thermalIntents: Array.isArray(parsed?.thermal_intents) ? parsed.thermal_intents.filter((value) => typeof value === "string") : [],
  };
}

export function validateHubCatalogUrl(value) {
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

export function validateRemoteWorkloadCatalogPayload(payload) {
  if (!payload || typeof payload !== "object" || Array.isArray(payload)) {
    return { ok: false, reason: "Catalog payload must be a JSON object." };
  }

  if (payload.schema_version !== "kyuubiki.workload-catalog/v1") {
    return {
      ok: false,
      reason: "Catalog schema_version must be kyuubiki.workload-catalog/v1.",
    };
  }

  if (!Array.isArray(payload.workloads)) {
    return { ok: false, reason: "Catalog workloads must be an array." };
  }

  for (const [index, workload] of payload.workloads.entries()) {
    if (!workload || typeof workload !== "object" || Array.isArray(workload)) {
      return { ok: false, reason: `Workload ${index + 1} must be an object.` };
    }

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

export function normalizeRemoteWorkloadCatalogPayload(payload, catalogUrl) {
  const list = Array.isArray(payload)
    ? payload
    : Array.isArray(payload?.workloads)
      ? payload.workloads
      : Array.isArray(payload?.items)
        ? payload.items
        : [];

  return list
    .map((entry) =>
      normalizeHubWorkloadEntry({
        label: entry?.label || entry?.name || entry?.projectName || entry?.project_name,
        note: entry?.note || entry?.description || `Synced from ${catalogUrl}`,
        sourceKind: "remote-catalog",
        sourceLabel: entry?.sourceLabel || payload?.sourceLabel || catalogUrl,
        bundlePath: entry?.bundlePath || entry?.bundle_path || "",
        downloadUrl: entry?.downloadUrl || entry?.download_url || catalogUrl,
        projectId: entry?.projectId || entry?.project_id || "",
        projectName: entry?.projectName || entry?.project_name || "",
        schema: entry?.schema || "",
        layout: entry?.layout || "",
        modelCount: entry?.modelCount || entry?.model_count || 0,
        versionCount: entry?.versionCount || entry?.version_count || 0,
        jobCount: entry?.jobCount || entry?.job_count || 0,
        resultCount: entry?.resultCount || entry?.result_count || 0,
        analysisDomains: entry?.analysisDomains || entry?.analysis_domains || [],
        analysisFamilies: entry?.analysisFamilies || entry?.analysis_families || [],
        thermalIntents: entry?.thermalIntents || entry?.thermal_intents || [],
      }),
    )
    .filter(Boolean);
}

export function inferDownloadFilename(url, fallback = "kyuubiki-workload.kyuubiki") {
  try {
    const parsed = new URL(String(url || "").trim());
    const pathname = parsed.pathname.split("/").filter(Boolean).at(-1);
    return pathname || fallback;
  } catch {
    return fallback;
  }
}

export function workloadSourceBadge(entry) {
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

export function workloadProvenanceHost(value) {
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

export function workloadProvenanceLabel(entry) {
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
    return entry.sourceLabel;
  }

  return "Hub local registration";
}

export function workloadDomainLabel(domain) {
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

export function workloadFamilyLabel(family) {
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
