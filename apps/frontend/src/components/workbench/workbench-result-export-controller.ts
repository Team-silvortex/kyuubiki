"use client";

import { serializeResultCsv } from "@/lib/workbench/result-csv";

type HotspotRow = {
  id: string;
  value: number | string;
  summary?: string;
};

type ForceRow = {
  id: string;
  node_i: string;
  node_j: string;
  axial_force_i?: number | string;
  shear_force_i?: number | string;
  moment_i?: number | string;
  axial_force_j?: number | string;
  shear_force_j?: number | string;
  moment_j?: number | string;
};

type WorkbenchResultExportDeps = {
  result: unknown | null;
  studyKind: any;
  loadedModelName: string;
  job: any;
  isPlane: boolean;
  isFrameLike: boolean;
  isBeam: boolean;
  isSpring: boolean;
  isThermal: boolean;
  isThermalBar: boolean;
  isThermalTruss2d: boolean;
  isThermalTruss3d: boolean;
  planeResultField: string;
  activeLineResultField: string;
  planeHotspotElements: HotspotRow[];
  frameHotspotElements: HotspotRow[];
  frameForceRows: Array<ForceRow | Record<string, unknown>>;
  downloadTextFile: (name: string, contents: string) => void;
  setMessage: (value: string) => void;
  labels: {
    noResultToExport: string;
    resultJsonDownloaded: string;
    resultCsvDownloaded: string;
  };
};

export function downloadWorkbenchResultJson({
  result,
  studyKind,
  loadedModelName,
  job,
  downloadTextFile,
  setMessage,
  labels,
}: WorkbenchResultExportDeps) {
  if (!result) {
    setMessage(labels.noResultToExport);
    return;
  }

  const payload = {
    exported_at: new Date().toISOString(),
    study_kind: studyKind,
    model_name: loadedModelName,
    job,
    result,
  };

  downloadTextFile(`${loadedModelName || "kyuubiki-study"}-result.json`, JSON.stringify(payload, null, 2));
  setMessage(labels.resultJsonDownloaded);
}

export function downloadWorkbenchResultCsv({
  result,
  studyKind,
  loadedModelName,
  job,
  downloadTextFile,
  setMessage,
  labels,
}: WorkbenchResultExportDeps) {
  if (!result) {
    setMessage(labels.noResultToExport);
    return;
  }

  const csv = serializeResultCsv(studyKind, job, result as any);
  downloadTextFile(`${loadedModelName || "kyuubiki-study"}-result.csv`, csv);
  setMessage(labels.resultCsvDownloaded);
}

export function downloadWorkbenchPlaneHotspotSummary({
  isPlane,
  planeHotspotElements,
  planeResultField,
  loadedModelName,
  downloadTextFile,
  setMessage,
  labels,
}: WorkbenchResultExportDeps) {
  if (!isPlane || planeHotspotElements.length === 0) {
    setMessage(labels.noResultToExport);
    return;
  }

  const lines = [
    ["rank", "id", "field", "value"].join(","),
    ...planeHotspotElements.map((entry, index) => [index + 1, entry.id, planeResultField, entry.value].join(",")),
  ];

  downloadTextFile(`${loadedModelName || "kyuubiki-study"}-${planeResultField}-hotspots.csv`, lines.join("\n"));
  setMessage(labels.resultCsvDownloaded);
}

export function downloadWorkbenchFrameHotspotSummary({
  isFrameLike,
  isBeam,
  isSpring,
  isThermal,
  frameHotspotElements,
  activeLineResultField,
  loadedModelName,
  downloadTextFile,
  setMessage,
  labels,
}: WorkbenchResultExportDeps) {
  if (!(isFrameLike || isBeam || isSpring || isThermal) || frameHotspotElements.length === 0) {
    setMessage(labels.noResultToExport);
    return;
  }

  const lines = [
    ["rank", "id", "field", "value", "end_forces"].join(","),
    ...frameHotspotElements.map((entry, index) => [index + 1, entry.id, activeLineResultField, entry.value, `"${entry.summary ?? ""}"`].join(",")),
  ];

  downloadTextFile(`${loadedModelName || "kyuubiki-study"}-${activeLineResultField}-hotspots.csv`, lines.join("\n"));
  setMessage(labels.resultCsvDownloaded);
}

export function downloadWorkbenchFrameForceSummary({
  isFrameLike,
  isBeam,
  isSpring,
  isThermal,
  isThermalBar,
  isThermalTruss2d,
  isThermalTruss3d,
  frameForceRows,
  loadedModelName,
  downloadTextFile,
  setMessage,
  labels,
}: WorkbenchResultExportDeps) {
  if (!(isFrameLike || isBeam || isSpring || isThermal) || frameForceRows.length === 0) {
    setMessage(labels.noResultToExport);
    return;
  }

  const lines = [
    ["id", "node_i", "node_j", "axial_force_i", "shear_force_i", "moment_i", "axial_force_j", "shear_force_j", "moment_j"].join(","),
    ...frameForceRows.map((element) =>
      [
        String((element as any).id ?? ""),
        String((element as any).node_i ?? ""),
        String((element as any).node_j ?? ""),
        String((element as any).axial_force_i ?? (element as any).axialForceI ?? ""),
        String((element as any).shear_force_i ?? (element as any).shearForceI ?? ""),
        String((element as any).moment_i ?? (element as any).momentI ?? ""),
        String((element as any).axial_force_j ?? (element as any).axialForceJ ?? ""),
        String((element as any).shear_force_j ?? (element as any).shearForceJ ?? ""),
        String((element as any).moment_j ?? (element as any).momentJ ?? ""),
      ].join(","),
    ),
  ];

  downloadTextFile(
    `${loadedModelName || "kyuubiki-study"}-${isThermalBar ? "thermal-bar" : isThermalTruss2d || isThermalTruss3d ? "thermal-truss" : isSpring ? "spring" : isBeam ? "beam" : "frame"}-member-forces.csv`,
    lines.join("\n"),
  );
  setMessage(labels.resultCsvDownloaded);
}
