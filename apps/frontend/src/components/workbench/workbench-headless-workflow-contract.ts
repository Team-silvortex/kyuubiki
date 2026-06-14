"use client";

export type PayloadObject = Record<string, unknown>;

export type DraftStep = {
  id: string;
  action: string;
  payloadText: string;
};

export type HeadlessReferenceToken = {
  label: string;
  outputKey: string;
  template: string;
};

export type HeadlessInputPort = {
  key: string;
  bindable?: boolean;
  label: string;
  required?: boolean;
};

export type HeadlessOutputPort = {
  key: string;
  label: string;
};

export type HeadlessLocalizedText = Record<string, string>;

export type HeadlessActionGuidanceNote = {
  label: HeadlessLocalizedText;
  value: HeadlessLocalizedText;
};

export type HeadlessActionContract = {
  id: string;
  risk: "normal" | "sensitive" | "destructive";
  summary: HeadlessLocalizedText;
  payloadExample: PayloadObject;
  inputSchema: HeadlessInputPort[];
  outputSchema: HeadlessOutputPort[];
  guidanceNotes?: HeadlessActionGuidanceNote[];
};

export type HeadlessWorkflowTemplate = {
  id: string;
  title: HeadlessLocalizedText;
  description: HeadlessLocalizedText;
  steps: Array<{ action: string; payload: PayloadObject }>;
};

export function formatPayload(payload: PayloadObject) {
  return JSON.stringify(payload, null, 2);
}

export function parsePayloadText(payloadText: string): PayloadObject | null {
  try {
    const parsed = JSON.parse(payloadText) as unknown;
    return parsed && typeof parsed === "object" && !Array.isArray(parsed) ? (parsed as PayloadObject) : null;
  } catch {
    return null;
  }
}

export function updatePayloadField(payload: PayloadObject | null, key: string, value: unknown) {
  const next = { ...(payload ?? {}) };
  if (value === "" || value === undefined) {
    delete next[key];
  } else {
    next[key] = value;
  }
  return next;
}

export function localizeHeadlessText(language: string, value: HeadlessLocalizedText) {
  return value[language] ?? value.en ?? "";
}
