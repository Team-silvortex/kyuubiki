"use client";

import { applyImportedWorkbenchModel } from "@/components/workbench/workbench-model-load";
import { parsePlaygroundModel } from "@/lib/models";

type ImportedModelDeps = Parameters<typeof applyImportedWorkbenchModel>[1];

export async function fetchWorkbenchTextWithTimeout(
  url: string,
  labels: { requestTimedOut: string },
  timeoutMs = 12_000,
) {
  const controller = new AbortController();
  const timeoutId = window.setTimeout(() => controller.abort(), timeoutMs);

  try {
    const response = await fetch(url, {
      cache: "no-store",
      signal: controller.signal,
    });

    if (!response.ok) {
      throw new Error(`request failed: ${response.status}`);
    }

    return await response.text();
  } catch (error) {
    if (error instanceof DOMException && error.name === "AbortError") {
      throw new Error(labels.requestTimedOut);
    }
    throw error;
  } finally {
    window.clearTimeout(timeoutId);
  }
}

export async function importWorkbenchModelFile(params: {
  file: File | undefined;
  labels: {
    importAction: string;
    importedModel: string;
    importFailed: string;
  };
  recordHistory: (label: string) => void;
  applyImportedModel: ImportedModelDeps;
  setMessage: (value: string) => void;
}) {
  const { file, labels, recordHistory, applyImportedModel, setMessage } = params;
  if (!file) return;

  try {
    const imported = parsePlaygroundModel(await file.text());
    recordHistory(labels.importAction);
    applyImportedWorkbenchModel(imported, applyImportedModel);
    setMessage(`${labels.importedModel}: ${imported.name}`);
  } catch (error) {
    setMessage(error instanceof Error ? `${labels.importFailed}: ${error.message}` : labels.importFailed);
  }
}

export async function openWorkbenchSample(params: {
  href: string;
  labels: {
    sampleAction: string;
    importedModel: string;
    importFailed: string;
    requestTimedOut: string;
  };
  recordHistory: (label: string) => void;
  applyImportedModel: ImportedModelDeps;
  setMessage: (value: string) => void;
}) {
  const { href, labels, recordHistory, applyImportedModel, setMessage } = params;
  try {
    const text = await fetchWorkbenchTextWithTimeout(href, { requestTimedOut: labels.requestTimedOut });
    const imported = parsePlaygroundModel(text);
    recordHistory(labels.sampleAction);
    applyImportedWorkbenchModel(imported, applyImportedModel);
    setMessage(`${labels.importedModel}: ${imported.name}`);
  } catch (error) {
    setMessage(error instanceof Error ? `${labels.importFailed}: ${error.message}` : labels.importFailed);
  }
}

